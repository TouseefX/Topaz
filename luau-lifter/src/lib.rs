pub mod deserializer;
pub mod instruction;
mod lifter;
pub mod op_code;

pub mod builtins;

use ast::{
    inline_gotos::inline_short_gotos, local_declarations::LocalDeclarer,
    name_locals::name_locals, replace_locals::replace_locals, Traverse,
};

use ast::post_process;
use by_address::ByAddress;
use cfg::{
    function::Function,
    ssa::{
        self,
        structuring::{structure_conditionals, structure_jumps},
    },
};
use indexmap::IndexMap;

use lifter::Lifter;

use parking_lot::Mutex;
use petgraph::algo::dominators::simple_fast;
use petgraph::visit::Dfs;

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::time::{Duration, Instant};
use triomphe::Arc;

/// Decompile using the **luaur-compatible** plain-opcode path.
///
/// [luaur](https://github.com/pjankiewicz/luaur) is the full Luau engine port
/// (compiler, VM, typechecker, native codegen). Its loader accepts the same
/// open-source bytecode version range as C++ Luau (`LBC_VERSION` **3..=11**).
///
/// Topaz cannot feed `luau_load` into the decompiler IR directly (the VM
/// resolves imports into live values). Instead we:
///   1. Prefer **plain-opcode** deserialize with encode key `1` when the
///      blob's version is in luaur's range — matching what luaur/`luau_load`
///      expects for unencoded dumps and `luau-compile` output.
///   2. Fall back to the **native** deserializer with encode-key detection
///      (Roblox client dumps use key **203**).
///
/// This keeps luaur as the version/policy authority while preserving a
/// working decompile path for Roblox-encoded bytecode.
pub fn decompile_bytecode_via_luaur(bytecode: &[u8], encode_key: u8) -> String {
    // Force loadsafe_ir (luaur-aligned raw IR). Key 0 → 1 (plain).
    let key = if encode_key == 0 { 1 } else { encode_key };
    match deserializer::loadsafe_ir::decode_chunk(bytecode, key) {
        Ok(c) => decompile_from_chunk(c, key),
        Err(e) => format!("failed to deserialize bytecode: {e}"),
    }
}

/// Back-compat alias: older CLI flag `--ruau` now maps here.
#[deprecated(note = "use decompile_bytecode_via_luaur or decompile_bytecode_default")]
pub fn decompile_bytecode_via_ruau(bytecode: &[u8], encode_key: u8) -> String {
    decompile_bytecode_via_luaur(bytecode, encode_key)
}

/// Default Luau decompilation path.
///
/// **One IR decoder** ([`deserializer::loadsafe_ir`]) for both plain and
/// Roblox-encoded dumps; only the encode key changes:
///
/// 1. Try **plain** opcodes (`encode_key = 1`) — what luaur / `luau-compile`
///    emit. If that fails, the blob is often not invalid: Roblox clients
///    shuffle the instruction op-byte with a key (commonly **203**), which
///    a plain-only loader would treat as garbage.
/// 2. **Detect encode key** (preferred, 203, 1) and decode again through
///    the **same** loadsafe_ir path so constants stay raw (`Import(iid)`,
///    table shapes, …) rather than branching into a second parser.
///
/// encode_key only affects instruction op-bytes (`op' = op * key`); string
/// tables and constant payloads are not keyed.
pub fn decompile_bytecode_default(bytecode: &[u8], encode_key: u8) -> String {
    ast::reset_local_id_counter();
    match decode_best(bytecode, encode_key) {
        Ok((c, key)) => decompile_from_chunk(c, key),
        Err(e) => format!("failed to deserialize bytecode: {e}"),
    }
}

/// Force loadsafe_ir with the given key (plain if 0/1).
#[allow(dead_code)]
fn try_decompile_luaur_plain(bytecode: &[u8]) -> Option<String> {
    let key = 1u8;
    match deserializer::loadsafe_ir::decode_chunk(bytecode, key) {
        Ok(c) => Some(decompile_from_chunk(c, key)),
        Err(_) => None,
    }
}

/// True when the blob's leading version byte is within luaur / upstream
/// `LBC_VERSION_MIN..=LBC_VERSION_MAX` (currently 3..=11). Version 0 is an
/// error blob; 12+ is experimental and handled only by native.
#[allow(dead_code)]
fn looks_like_luaur_plain_bytecode(bytecode: &[u8]) -> bool {
    let Some(&version) = bytecode.first() else {
        return false;
    };
    deserializer::loadsafe_ir::is_luaur_version(version)
}

fn decompile_from_chunk(chunk: deserializer::chunk::Chunk, encode_key: u8) -> String {
    const PANIC_MSG: &str =
        "-- Decompiled with Topaz\n-- Error: decompilation panicked\n";
    let job = move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decompile_from_chunk_inner(chunk, encode_key)
        }))
        .unwrap_or_else(|_| PANIC_MSG.to_string())
    };

    // wasm32 has no threads. Native: Android / tokio workers are 1–2 MB
    // and SIGSEGV on 60k-line dumps (1–4 MB compiled, ~16 MB decoded IR);
    // lift on a 64 MB stack instead.
    #[cfg(target_arch = "wasm32")]
    {
        job()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 60k-line scripts compile to 1–4 MB; decoded IR (AUX expanded)
        // is ~16 MB and builds CFGs deep enough that petgraph's recursive
        // dominators blew a 16 MB stack. 64 MB is virtual (overcommit).
        const STACK: usize = 64 * 1024 * 1024;
        match std::thread::Builder::new()
            .name("topaz-lift".into())
            .stack_size(STACK)
            .spawn(job)
        {
            Ok(handle) => handle.join().unwrap_or_else(|_| PANIC_MSG.to_string()),
            Err(_) => PANIC_MSG.to_string(),
        }
    }
}

struct LiftedItem {
    ast_function: Arc<Mutex<ast::Function>>,
    function: Function,
    upvalues_in: Vec<ast::RcLocal>,
    child_asts: Vec<ByAddress<Arc<Mutex<ast::Function>>>>,
}

fn decompile_one(
    item: LiftedItem,
    deadline: Instant,
) -> (ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>) {
    let ast_clone = item.ast_function.clone();
    if Instant::now() >= deadline {
        ast_clone.lock().body.push(
            ast::Comment::new("skipped (time budget)".to_string()).into(),
        );
        return (ByAddress(ast_clone), Vec::new());
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        decompile_function(
            item.ast_function,
            item.function,
            item.upvalues_in,
            deadline,
        )
    })) {
        Ok(r) => r,
        Err(_) => {
            ast_clone.lock().body.push(
                ast::Comment::new("failed to decompile function".to_string()).into(),
            );
            (ByAddress(ast_clone), Vec::new())
        }
    }
}

fn decompile_lifted_waves(
    mut lifted: Vec<LiftedItem>,
    deadline: Instant,
) -> FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>> {
    let mut done: FxHashSet<ByAddress<Arc<Mutex<ast::Function>>>> = FxHashSet::default();
    let mut upvalues = FxHashMap::default();

    #[cfg(not(target_arch = "wasm32"))]
    let pool = {
        let n = std::thread::available_parallelism()
            .map(|p| p.get().clamp(2, 16))
            .unwrap_or(8);
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .stack_size(64 * 1024 * 1024)
            .thread_name(|i| format!("topaz-fn-{i}"))
            .build()
            .ok()
    };

    let run_wave = |ready: Vec<LiftedItem>| -> Vec<(
        ByAddress<Arc<Mutex<ast::Function>>>,
        Vec<ast::RcLocal>,
    )> {
        if ready.len() <= 1 {
            return ready
                .into_iter()
                .map(|it| decompile_one(it, deadline))
                .collect();
        }
        #[cfg(target_arch = "wasm32")]
        {
            ready
                .into_iter()
                .map(|it| decompile_one(it, deadline))
                .collect()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(pool) = pool.as_ref() {
                pool.install(|| {
                    ready
                        .into_par_iter()
                        .map(|it| decompile_one(it, deadline))
                        .collect()
                })
            } else {
                ready
                    .into_iter()
                    .map(|it| decompile_one(it, deadline))
                    .collect()
            }
        }
    };

    while !lifted.is_empty() {
        let (ready, rest): (Vec<_>, Vec<_>) = lifted
            .into_iter()
            .partition(|it| it.child_asts.iter().all(|c| done.contains(c)));
        if ready.is_empty() {
            for item in rest {
                let (k, v) = decompile_one(item, deadline);
                done.insert(k.clone());
                upvalues.insert(k, v);
            }
            break;
        }
        lifted = rest;
        for (k, v) in run_wave(ready) {
            done.insert(k.clone());
            upvalues.insert(k, v);
        }
    }
    upvalues
}

fn decompile_from_chunk_inner(chunk: deserializer::chunk::Chunk, _encode_key: u8) -> String {
    // Budget covers lift + SSA. 60k-line scripts used to hang forever in
    // Lifter::lift / construct before the old deadline was even created.
    // 60k-line body, 16 MB decoded IR. SSA still runs; flatten only if late.
    let deadline = Instant::now() + Duration::from_secs(180);
    cfg::set_decompile_deadline(Some(deadline));
    let mut lifted = Vec::new();
    let mut stack = vec![(Arc::<Mutex<ast::Function>>::default(), chunk.main)];
    while let Some((ast_func, func_id)) = stack.pop() {
        if Instant::now() >= deadline {
            ast_func.lock().body.push(
                ast::Comment::new("skipped lift (time budget)".to_string()).into(),
            );
            lifted.push(LiftedItem {
                ast_function: ast_func,
                function: Function::new(func_id as usize),
                upvalues_in: Vec::new(),
                child_asts: Vec::new(),
            });
            continue;
        }
        let lifted_fn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize)
        }));
        match lifted_fn {
            Ok((function, upvalues, child_functions)) => {
                let child_asts: Vec<_> = child_functions.keys().cloned().collect();
                stack.extend(child_functions.into_iter().map(|(a, f)| (a.0, f as u32)));
                lifted.push(LiftedItem {
                    ast_function: ast_func,
                    function,
                    upvalues_in: upvalues,
                    child_asts,
                });
            }
            Err(_) => {
                ast_func.lock().body.push(
                    ast::Comment::new("failed to lift function".to_string()).into(),
                );
                lifted.push(LiftedItem {
                    ast_function: ast_func,
                    function: Function::new(func_id as usize),
                    upvalues_in: Vec::new(),
                    child_asts: Vec::new(),
                });
            }
        }
    }

    let Some(main) = lifted.first().map(|it| it.ast_function.clone()) else {
        return "-- Decompiled with Topaz\n-- Created by: Andrew and TouseefX\n\n".into();
    };

    // Children before parents (nested-closure mutex). Independent siblings
    // in the same wave run in parallel on 64 MB stacks so a 16 MB decoded
    // dump does not sit on one thread and does not SIGSEGV.
    let mut upvalues = decompile_lifted_waves(lifted, deadline);

    let main = ByAddress(main);
    upvalues.remove(&main);
    let mut body = Arc::try_unwrap(main.0).unwrap().into_inner().body;
    link_upvalues(&mut body, &mut upvalues);
    // 60k+ source lines. Do not drop naming/goto-fold at 8k.
    const QUALITY_STMT_CAP: usize = 250_000;
    if body.0.len() >= 8 && body.0.len() < QUALITY_STMT_CAP {
        ast::context_naming::apply_context_naming(&mut body);
        propagate_names(&mut body);
        inline_short_gotos(&mut body);
        ast::guard_clauses::apply_guard_clauses(&mut body);
    }
    if body.0.len() < QUALITY_STMT_CAP {
        name_locals(&mut body, true);
    }

    format!(
        "-- Decompiled with Topaz\n-- Created by: Andrew and TouseefX\n\n{}",
        body
    )
}



#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Try a small set of keys (plain 1, Roblox 203, then `preferred`) and
/// return the first that fully parses. Key 0 is invalid (everything
/// becomes NOP).
pub fn detect_encode_key(bytecode: &[u8], preferred: u8) -> u8 {
    decode_best(bytecode, preferred)
        .map(|(_, k)| k)
        .unwrap_or(if preferred == 0 { 1 } else { preferred })
}

fn decode_best(
    bytecode: &[u8],
    preferred: u8,
) -> Result<(deserializer::chunk::Chunk, u8), String> {
    let mut candidates = Vec::with_capacity(4);
    // Preferred first so a known Roblox key (203) does not waste a full
    // failed parse on key 1. 227 is inv(203) — LunaUX reports it as the
    // encoder "magic key"; try both so either convention loads.
    for k in [preferred, 1u8, 203, 227] {
        if k != 0 && !candidates.contains(&k) {
            candidates.push(k);
        }
    }
    let mut last = None;
    for candidate in candidates {
        match deserializer::loadsafe_ir::decode_chunk(bytecode, candidate) {
            Ok(c) => return Ok((c, candidate)),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| "failed to deserialize bytecode".into()))
}

fn dump_cfgs_from_chunk(chunk: deserializer::chunk::Chunk) -> Vec<cfg::CfgSnapshot> {
    let mut out = Vec::new();
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![chunk.main];
    while let Some(func_id) = stack.pop() {
        if !visited.insert(func_id) {
            continue;
        }
        let (function, _upvalues, child_functions) =
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize);
        let name = if func_id == chunk.main {
            "main".to_string()
        } else {
            format!("function #{func_id}")
        };
        out.push(cfg::CfgSnapshot::from_function(&function, name));
        stack.extend(child_functions.into_iter().map(|(_, f)| f as u32));
    }
    out
}

/// CFG dump through loadsafe_ir (plain key 1, then detected key).
pub fn dump_cfgs_via_luaur(bytecode: &[u8]) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    match decode_best(bytecode, 1) {
        Ok((c, _)) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

#[deprecated(note = "use dump_cfgs_via_luaur")]
pub fn dump_cfgs_via_ruau(bytecode: &[u8]) -> Vec<cfg::CfgSnapshot> {
    dump_cfgs_via_luaur(bytecode)
}

pub fn dump_cfgs_default(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    match decode_best(bytecode, encode_key) {
        Ok((c, _)) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

pub fn dump_cfgs(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    match decode_best(bytecode, encode_key) {
        Ok((c, _)) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

pub fn decompile_bytecode(bytecode: &[u8], encode_key: u8) -> String {
    ast::reset_local_id_counter();
    match decode_best(bytecode, encode_key) {
        Ok((c, key)) => decompile_from_chunk(c, key),
        Err(e) => format!("failed to deserialize bytecode: {e}"),
    }
}

fn propagate_names(body: &mut ast::Block) {
    let mut captured = FxHashSet::default();
    collect_captured_upvalues(body, &mut captured);
    propagate_names_block(body, &captured);
}

/// Collects every local that is captured (by copy or by reference) as an
/// upvalue of some nested closure, anywhere in the function tree rooted at
/// `block`.
///
/// These locals must never have their display name overwritten by the
/// generic "copy the name from the other side of a plain assignment"
/// heuristic in `propagate_names_block`: a captured variable is shared with
/// (and semantically distinct from) whatever unrelated locals happen to live
/// in the closures that capture it, and blindly renaming it to match a
/// sibling can make two different variables print with the identical name,
/// silently corrupting the decompiled source (e.g. turning
/// `aId = idCounter` into the textually-identical-looking but broken
/// `Id = Id` once both locals are named "Id").
fn collect_captured_upvalues(block: &mut ast::Block, out: &mut FxHashSet<ast::RcLocal>) {
    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                out.extend(closure.upvalues.iter().map(|u| match u {
                    ast::Upvalue::Copy(l) | ast::Upvalue::Ref(l) => l.clone(),
                }));
                collect_captured_upvalues(&mut closure.function.lock().body, out);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                collect_captured_upvalues(&mut r#if.then_block.lock(), out);
                collect_captured_upvalues(&mut r#if.else_block.lock(), out);
            }
            ast::Statement::While(r#while) => {
                collect_captured_upvalues(&mut r#while.block.lock(), out);
            }
            ast::Statement::Repeat(repeat) => {
                collect_captured_upvalues(&mut repeat.block.lock(), out);
            }
            ast::Statement::NumericFor(numeric_for) => {
                collect_captured_upvalues(&mut numeric_for.block.lock(), out);
            }
            ast::Statement::GenericFor(generic_for) => {
                collect_captured_upvalues(&mut generic_for.block.lock(), out);
            }
            _ => {}
        }
    }
}

fn propagate_names_block(block: &mut ast::Block, captured: &FxHashSet<ast::RcLocal>) {
    for _ in 0..2 {
        for stat in block.0.iter() {
            if let ast::Statement::Assign(assign) = stat {
                if assign.left.len() == 1 && assign.right.len() == 1 {
                    if let Some(lhs) = assign.left[0].as_local() {
                        let lhs_name = lhs.0 .0.lock().0.clone();
                        if let Some(lhs_name) = lhs_name {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
                                if !captured.contains(rhs) {
                                    let mut rhs_lock = rhs.0 .0.lock();
                                    if rhs_lock.0.is_none() {
                                        rhs_lock.0 = Some(lhs_name);
                                    }
                                }
                            }
                        } else {
                            if let ast::RValue::Local(rhs) = &assign.right[0] {
                                if !captured.contains(lhs) {
                                    let rhs_name = rhs.0 .0.lock().0.clone();
                                    if let Some(rhs_name) = rhs_name {
                                        lhs.0 .0.lock().0 = Some(rhs_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    block.0.retain(|stat| {
        if let ast::Statement::Assign(assign) = stat {
            if assign.prefix && assign.left.len() == 1 && assign.right.len() == 1 {
                if let Some(lhs) = assign.left[0].as_local() {
                    if let ast::RValue::Local(rhs) = &assign.right[0] {
                        let lhs_name = lhs.0 .0.lock().0.clone();
                        let rhs_name = rhs.0 .0.lock().0.clone();
                        if let (Some(ln), Some(rn)) = (&lhs_name, &rhs_name) {
                            if ln == rn {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    });

    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                propagate_names_block(&mut closure.function.lock().body, captured);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                propagate_names_block(&mut r#if.then_block.lock(), captured);
                propagate_names_block(&mut r#if.else_block.lock(), captured);
            }
            ast::Statement::While(r#while) => {
                propagate_names_block(&mut r#while.block.lock(), captured);
            }
            ast::Statement::Repeat(repeat) => {
                propagate_names_block(&mut repeat.block.lock(), captured);
            }
            ast::Statement::NumericFor(numeric_for) => {
                propagate_names_block(&mut numeric_for.block.lock(), captured);
            }
            ast::Statement::GenericFor(generic_for) => {
                propagate_names_block(&mut generic_for.block.lock(), captured);
            }
            _ => {}
        }
    }
}

fn flatten_cfg(function: &Function) -> ast::Block {
    let mut body = ast::Block::default();
    let mut order = Vec::new();
    if let Some(entry) = *function.entry() {
        let mut dfs = Dfs::new(function.graph(), entry);
        while let Some(n) = dfs.next(function.graph()) {
            order.push(n);
        }
    } else {
        order.extend(function.graph().node_indices());
    }
    for n in order {
        if let Some(b) = function.block(n) {
            if !b.0.is_empty() {
                body.push(ast::Comment::new(format!("block {}", n.index())).into());
                body.extend(b.0.iter().cloned());
            }
        }
    }
    body
}

fn finish_function(
    ast_function: Arc<Mutex<ast::Function>>,
    body: ast::Block,
    params: Vec<ast::RcLocal>,
    is_variadic: bool,
    func_line: Option<usize>,
    upvalues_in: Vec<ast::RcLocal>,
    post: bool,
) -> (ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>) {
    {
        let mut ast_function = ast_function.lock();
        ast_function.body = body;
        if post && !ast_function.body.0.is_empty() && ast_function.body.0.len() < 250_000 {
            post_process::apply_all(&mut ast_function.body);
        }
        ast_function.parameters = params;
        ast_function.is_variadic = is_variadic;
        ast_function.line = func_line;
    }
    (ByAddress(ast_function), upvalues_in)
}

fn decompile_function(
    ast_function: Arc<Mutex<ast::Function>>,
    mut function: Function,
    upvalues_in: Vec<ast::RcLocal>,
    deadline: Instant,
) -> (ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>) {
    let params = function.parameters.clone();
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let node_count = function.graph().node_count();
    let large = node_count > 800;
    if Instant::now() >= deadline {
        let params = std::mem::take(&mut function.parameters);
        let is_variadic = function.is_variadic;
        let func_line = function.line;
        let body = flatten_cfg(&function);
        return finish_function(
            ast_function,
            body,
            params,
            is_variadic,
            func_line,
            upvalues_in,
            false,
        );
    }

    let constructed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cfg::ssa::construct(&mut function, &upvalues_in)
    }));
    let (local_count, local_groups, upvalue_in_groups, upvalue_passed_groups) = match constructed {
        Ok(v) => v,
        Err(_) => {
            let body = flatten_cfg(&function);
            return finish_function(
                ast_function,
                body,
                params,
                is_variadic,
                func_line,
                upvalues_in,
                false,
            );
        }
    };
    let upvalue_to_group = upvalue_in_groups
        .into_iter()
        .chain(
            upvalue_passed_groups
                .into_iter()
                .map(|m| (ast::RcLocal::default(), m)),
        )
        .flat_map(|(i, g)| g.into_iter().map(move |u| (u, i.clone())))
        .collect::<IndexMap<_, _>>();

    let local_to_group = local_groups
        .into_iter()
        .enumerate()
        .flat_map(|(i, g)| g.into_iter().map(move |l| (l, i)))
        .collect::<FxHashMap<_, _>>();

    let mut changed = true;
    let mut iters = 0u32;
    // Tiny scripts (a single require, etc.) must stay near-instant.
    let ssa_cap = if node_count <= 12 {
        1
    } else if large {
        2
    } else {
        4
    };
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        while changed && iters < ssa_cap && Instant::now() < deadline {
            iters += 1;
            changed = false;

            if let Some(entry) = *function.entry() {
                let dominators = simple_fast(function.graph(), entry);
                changed |= structure_jumps(&mut function, &dominators);
            }

            ssa::inline::inline(&mut function, &local_to_group, &upvalue_to_group);

            if node_count > 2 && structure_conditionals(&mut function) {
                changed = true;
            }
            let mut local_map = FxHashMap::default();

            if ssa::construct::remove_unnecessary_params(&mut function, &mut local_map) {
                changed = true;
            }
            ssa::construct::apply_local_map(&mut function, local_map);
        }
    }));

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ssa::Destructor::new(
            &mut function,
            upvalue_to_group,
            upvalues_in.iter().cloned().collect(),
            local_count,
        )
        .destruct();
    }));

    let params = std::mem::take(&mut function.parameters);
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let lifted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        restructure::lift(function)
    }));
    let body = match lifted {
        Ok(b) => b,
        Err(_) => ast::Block::default(),
    };
    let block = Arc::new(body.into());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        LocalDeclarer::default().declare_locals(
            Arc::clone(&block),
            &upvalues_in.iter().chain(params.iter()).cloned().collect(),
        );
    }));

    let body = Arc::try_unwrap(block).unwrap().into_inner();
    finish_function(
        ast_function,
        body,
        params,
        is_variadic,
        func_line,
        upvalues_in,
        Instant::now() < deadline,
    )
}

fn link_upvalues(
    body: &mut ast::Block,
    upvalues: &mut FxHashMap<ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>>,
) {
    for stat in &mut body.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                let old_upvalues = &upvalues[&closure.function];
                let mut function = closure.function.lock();

                let mut local_map =
                    FxHashMap::with_capacity_and_hasher(old_upvalues.len(), Default::default());
                for (old, new) in
                    old_upvalues
                        .iter()
                        .zip(closure.upvalues.iter().map(|u| match u {
                            ast::Upvalue::Copy(l) | ast::Upvalue::Ref(l) => l,
                        }))
                {
                    let old_name = old.0.0.lock().0.clone();
                    if let Some(ref name) = old_name {
                        if !ast::name_locals::is_synthetic_name(name) {
                            let mut new_lock = new.0.0.lock();
                            if new_lock.0.is_none()
                                || new_lock
                                    .0
                                    .as_ref()
                                    .map(|s| ast::name_locals::is_synthetic_name(s))
                                    .unwrap_or(true)
                            {
                                new_lock.0 = Some(name.clone());
                            }
                        }
                    }
                    local_map.insert(old.clone(), new.clone());
                }
                link_upvalues(&mut function.body, upvalues);
                replace_locals(&mut function.body, &local_map);
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                link_upvalues(&mut r#if.then_block.lock(), upvalues);
                link_upvalues(&mut r#if.else_block.lock(), upvalues);
            }
            ast::Statement::While(r#while) => {
                link_upvalues(&mut r#while.block.lock(), upvalues);
            }
            ast::Statement::Repeat(repeat) => {
                link_upvalues(&mut repeat.block.lock(), upvalues);
            }
            ast::Statement::NumericFor(numeric_for) => {
                link_upvalues(&mut numeric_for.block.lock(), upvalues);
            }
            ast::Statement::GenericFor(generic_for) => {
                link_upvalues(&mut generic_for.block.lock(), upvalues);
            }
            _ => {}
        }
    }
}
