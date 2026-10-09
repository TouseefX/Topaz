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

use petgraph::visit::Dfs;

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Write as _;
use std::sync::mpsc;
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
    begin_decompile_deadline();
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
    begin_decompile_deadline();
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
    // thread_local deadline does not follow us onto the 64 MB lift
    // thread — copy the Instant so decode + lift share one 180s budget.
    let inherited = cfg::decompile_deadline();
    let job = move || {
        if let Some(d) = inherited {
            cfg::set_decompile_deadline(Some(d));
        }
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
    ast_function: ast::SharedFunction,
    function: Function,
    upvalues_in: Vec<ast::RcLocal>,
    child_asts: Vec<ByAddress<ast::SharedFunction>>,
}

fn decompile_one(
    item: LiftedItem,
    deadline: Instant,
) -> (ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>) {
    // Rayon workers do not inherit the parent thread_local deadline.
    cfg::set_decompile_deadline(Some(deadline));
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
) -> FxHashMap<ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>> {
    let mut done: FxHashSet<ByAddress<ast::SharedFunction>> = FxHashSet::default();
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
        ByAddress<ast::SharedFunction>,
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
    // Budget covers decode + lift + SSA. Armed before decode_best so a
    // 21 MB AUX-expanded dump cannot hang in the loader forever.
    let deadline = cfg::decompile_deadline().unwrap_or_else(|| {
        let d = Instant::now() + cfg::decompile_budget();
        cfg::set_decompile_deadline(Some(d));
        d
    });
    let n_funcs = chunk.functions.len().max(1);
    let mut lifted = Vec::new();
    let mut stack = vec![(ast::share_function(ast::Function::default()), chunk.main)];
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
    let body = Arc::try_unwrap(main.0).unwrap().into_inner().body;

    // Post-wave can still spin on aliased elseif Arcs. recv_timeout plus
    // deep-cloned snapshots (closures detached) mean a hang still returns
    // structured, *valid* Luau instead of 58 bytes or a `goto` dump.
    let remaining = deadline.saturating_duration_since(Instant::now());
    let wait = if cfg::decompile_budget_secs() >= 30 {
        remaining.max(Duration::from_secs(15))
    } else {
        remaining
    };
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("topaz-post".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            cfg::set_decompile_deadline(Some(deadline));
            ast::set_post_deadline(Some(deadline));
            run_post_wave(body, upvalues, tx, n_funcs);
        });
    match spawned {
        Ok(_) => recv_post_wave(rx, wait, n_funcs),
        Err(_) => {
            "-- Decompiled with Topaz\n-- TOPAZ_INCOMPLETE\n-- Error: failed to spawn post-process thread\n"
                .into()
        }
    }
}

enum PostEvent {
    Entering(&'static str),
    /// Deep clone of the AST after the named pass completed.
    Snapshot { body: ast::Block, pass: &'static str },
    Done(String),
}

const FALLBACK_BYTE_CAP: usize = 8 * 1024 * 1024;
const COMPLETE_BYTE_CAP: usize = 32 * 1024 * 1024;

struct CappedWriter {
    buf: String,
    cap: usize,
    hit: bool,
}

impl std::fmt::Write for CappedWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.hit {
            return Ok(());
        }
        let room = self.cap.saturating_sub(self.buf.len());
        if s.len() <= room {
            self.buf.push_str(s);
        } else {
            self.buf.push_str(&s[..room]);
            self.hit = true;
        }
        Ok(())
    }
}

fn format_block_capped(body: &ast::Block, cap: usize) -> (String, bool) {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut w = CappedWriter {
            buf: String::new(),
            cap,
            hit: false,
        };
        let _ = write!(&mut w, "{body}");
        (w.buf, w.hit)
    })) {
        Ok(pair) => pair,
        Err(_) => ("-- formatter panicked\n".into(), true),
    }
}

fn looks_time_budget_skipped(s: &str) -> bool {
    s.contains("skipped (time budget)") || s.contains("skipped lift (time budget)")
}

/// Leftover SSA for-loop IR or raw CFG dump. Syntactically we now emit
/// valid Luau for those, but the decompile is not fully structured.
fn looks_unstructured(s: &str) -> bool {
    s.contains("[internal control]")
        || s.contains("-- unstructured generic-for")
        || s.contains("-- unstructured numeric-for")
        || s.contains("-- GenericForInit")
        || s.contains("-- GenericForNext")
        || s.contains("-- NumForInit")
        || s.contains("-- NumForNext")
        || s.contains("-- block ")
        || s.contains("failed to decompile function")
        || s.contains("failed to lift function")
}

fn render_complete(body: &mut ast::Block) -> String {
    let _ = ast::sanitize_for_luau(body);
    let (body_s, capped) = format_block_capped(body, COMPLETE_BYTE_CAP);
    if looks_time_budget_skipped(&body_s) || capped || looks_unstructured(&body_s) {
        let why = if capped {
            "output capped"
        } else if looks_time_budget_skipped(&body_s) {
            "some functions skipped (time budget)"
        } else {
            "unstructured control flow remains"
        };
        format!(
            "-- Decompiled with Topaz\n-- Created by: Andrew and TouseefX\n-- {INCOMPLETE_MARK}\n-- ERROR: {why}\n\n{body_s}"
        )
    } else {
        format!("-- Decompiled with Topaz\n-- Created by: Andrew and TouseefX\n\n{body_s}")
    }
}

fn render_incomplete(body: &mut ast::Block, pass: &str, n_total: usize) -> String {
    let n_printed = ast::sanitize_for_luau(body);
    let budget = cfg::decompile_budget_secs();
    let (body_s, capped) = format_block_capped(body, FALLBACK_BYTE_CAP);
    let cap_note = if capped {
        "\n-- fallback output capped"
    } else {
        ""
    };
    format!(
        "-- Decompiled with Topaz\n-- Created by: Andrew and TouseefX\n-- {INCOMPLETE_MARK}\n-- ERROR: time budget {budget}s expired in pass {pass}\n-- functions printed: {n_printed}/{n_total} (incomplete){cap_note}\n\n{body_s}"
    )
}

fn recv_post_wave(rx: mpsc::Receiver<PostEvent>, wait: Duration, n_funcs: usize) -> String {
    let deadline = Instant::now() + wait;
    let mut fallback: Option<ast::Block> = None;
    let mut current_pass = "post-process";
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(PostEvent::Entering(p)) => current_pass = p,
            Ok(PostEvent::Snapshot { body, pass }) => {
                current_pass = pass;
                fallback = Some(body);
            }
            Ok(PostEvent::Done(s)) => return s,
            Err(_) => break,
        }
    }
    match fallback {
        Some(mut b) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            render_incomplete(&mut b, current_pass, n_funcs)
        }))
        .unwrap_or_else(|_| {
            format!(
                "-- Decompiled with Topaz\n-- {INCOMPLETE_MARK}\n-- Error: fallback emit panicked in pass {current_pass}\n"
            )
        }),
        None => format!(
            "-- Decompiled with Topaz\n-- {INCOMPLETE_MARK}\n-- Error: post-process timed out in pass {current_pass}\n"
        ),
    }
}

fn run_post_wave(
    mut body: ast::Block,
    mut upvalues: FxHashMap<ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>>,
    tx: mpsc::Sender<PostEvent>,
    _n_funcs: usize,
) {
    let dbg = std::env::var("TOPAZ_DEBUG_CYCLES")
        .ok()
        .filter(|s| s != "0" && !s.eq_ignore_ascii_case("false"))
        .is_some();
    let post_start = Instant::now();
    macro_rules! step {
        ($name:expr, $e:expr) => {{
            let _ = tx.send(PostEvent::Entering($name));
            if dbg {
                eprintln!(
                    "[post] entering {} t+{:.1}s",
                    $name,
                    post_start.elapsed().as_secs_f64()
                );
            }
            let t0 = Instant::now();
            $e;
            if dbg {
                eprintln!(
                    "[post] after {} ({:.1}s): {}",
                    $name,
                    t0.elapsed().as_secs_f64(),
                    report_cycles(&mut body)
                );
            }
        }};
    }
    if dbg {
        eprintln!("[post] start t+0.0s: {}", report_cycles(&mut body));
    }
    step!("link_upvalues", link_upvalues(&mut body, &mut upvalues));
    const QUALITY_STMT_CAP: usize = 250_000;
    if body.0.len() >= 8 && body.0.len() < QUALITY_STMT_CAP {
        step!(
            "apply_context_naming",
            ast::context_naming::apply_context_naming(&mut body)
        );
        step!("propagate_names", propagate_names(&mut body));
        if !ast::past_post_deadline() {
            let _ = tx.send(PostEvent::Snapshot {
                body: body.deep_clone(),
                pass: "propagate_names",
            });
        }
        step!("inline_short_gotos", inline_short_gotos(&mut body));
        if !ast::past_post_deadline() {
            let _ = tx.send(PostEvent::Snapshot {
                body: body.deep_clone(),
                pass: "inline_short_gotos",
            });
        }
        step!(
            "apply_guard_clauses",
            ast::guard_clauses::apply_guard_clauses(&mut body)
        );
    }
    if body.0.len() < QUALITY_STMT_CAP {
        step!("name_locals", name_locals(&mut body, true));
    }
    let _ = tx.send(PostEvent::Done(render_complete(&mut body)));
}

/// `TOPAZ_DEBUG_CYCLES=1`: report blocks reachable from themselves
/// (would self-deadlock a non-reentrant mutex) and Arcs shared by more
/// than one parent (shallow `Statement::clone()`).
fn walk_block(
    block: &mut ast::Block,
    on_path: &mut Vec<*const ()>,
    seen: &mut FxHashSet<*const ()>,
    shared: &mut usize,
    trail: &mut Vec<&'static str>,
) -> Option<String> {
    for stat in &mut block.0 {
        let mut found: Option<String> = None;
        stat.traverse_rvalues(&mut |rvalue| {
            if found.is_some() {
                return;
            }
            if let ast::RValue::Closure(c) = rvalue {
                let ptr = Arc::as_ptr(&c.function.0) as *const ();
                if on_path.contains(&ptr) {
                    found = Some("closure function reached again".into());
                } else if seen.insert(ptr) {
                    if let Some(mut g) = c.function.try_lock() {
                        on_path.push(ptr);
                        trail.push("closure");
                        found = walk_block(&mut g.body, on_path, seen, shared, trail);
                        trail.pop();
                        on_path.pop();
                    } else {
                        found = Some("closure function already locked".into());
                    }
                } else {
                    *shared += 1;
                }
            }
        });
        if found.is_some() {
            return found;
        }

        let kids: Vec<(&'static str, ast::SharedBlock)> = match stat {
            ast::Statement::If(i) => vec![
                ("then", i.then_block.clone()),
                ("else", i.else_block.clone()),
            ],
            ast::Statement::While(w) => vec![("while", w.block.clone())],
            ast::Statement::Repeat(r) => vec![("repeat", r.block.clone())],
            ast::Statement::NumericFor(f) => vec![("for", f.block.clone())],
            ast::Statement::GenericFor(f) => vec![("for-in", f.block.clone())],
            _ => continue,
        };
        for (tag, m) in kids {
            let ptr = Arc::as_ptr(&m) as *const ();
            if on_path.contains(&ptr) {
                let start = trail.len().saturating_sub(12);
                return Some(format!(
                    "CYCLE at depth {} via ...{:?} -> {}",
                    trail.len(),
                    &trail[start..],
                    tag
                ));
            }
            if !seen.insert(ptr) {
                *shared += 1;
                continue;
            }
            let Some(mut guard) = m.try_lock() else {
                return Some(format!(
                    "block already locked at depth {} ({})",
                    trail.len(),
                    tag
                ));
            };
            on_path.push(ptr);
            trail.push(tag);
            let r = walk_block(&mut guard, on_path, seen, shared, trail);
            trail.pop();
            on_path.pop();
            if r.is_some() {
                return r;
            }
        }
    }
    None
}

fn report_cycles(body: &mut ast::Block) -> String {
    let mut on_path = Vec::new();
    let mut seen = FxHashSet::default();
    let mut shared = 0usize;
    let mut trail = Vec::new();
    let cyc = walk_block(body, &mut on_path, &mut seen, &mut shared, &mut trail);
    format!(
        "{} blocks, {} shared, {}",
        seen.len(),
        shared,
        cyc.unwrap_or_else(|| "no cycle".into())
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
        if cfg::past_decompile_deadline() {
            return Err("decode timed out".into());
        }
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
    begin_decompile_deadline();
    match decode_best(bytecode, encode_key) {
        Ok((c, key)) => decompile_from_chunk(c, key),
        Err(e) => format!("failed to deserialize bytecode: {e}"),
    }
}

/// Override the wall-clock decompile budget (seconds). `0` expires immediately.
/// CLI `--time-budget` and tests call this; otherwise `TOPAZ_TIME_BUDGET_SECS`
/// or the 180s default applies.
pub fn set_time_budget_secs(secs: u64) {
    cfg::set_decompile_budget_secs(secs);
}

/// Marker grepped by the CLI to exit non-zero on truncated output.
pub const INCOMPLETE_MARK: &str = "TOPAZ_INCOMPLETE";

pub fn output_is_incomplete(out: &str) -> bool {
    out.contains(INCOMPLETE_MARK)
        || out.contains("-- Error: post-process timed out")
        || out.contains("-- Error: decompilation panicked")
        || out.contains("-- Error: failed to spawn post-process thread")
        || out.contains("-- Error: fallback emit panicked")
        || out.contains("skipped (time budget)")
        || out.contains("skipped lift (time budget)")
        || looks_unstructured(out)
        || out.starts_with("failed to deserialize")
}

fn begin_decompile_deadline() {
    // Must start *before* decode_best. A desynced varint used to
    // `with_capacity` hundreds of millions of slots while "reading
    // bytecode" — the budget lived only after decode, so that hang
    // never timed out.
    cfg::set_decompile_deadline(Some(Instant::now() + cfg::decompile_budget()));
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
                if let Some(mut function) = closure.function.try_lock() {
                    collect_captured_upvalues(&mut function.body, out);
                }
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
            }
            ast::Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
            }
            ast::Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
            }
            ast::Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
            }
            ast::Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    collect_captured_upvalues(&mut b, out);
                }
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
                if let Some(mut function) = closure.function.try_lock() {
                    propagate_names_block(&mut function.body, captured);
                }
            }
        });
        match stat {
            ast::Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
            }
            ast::Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
            }
            ast::Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
            }
            ast::Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
            }
            ast::Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    propagate_names_block(&mut b, captured);
                }
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
                body.extend(b.0.iter().cloned());
            }
        }
    }
    body
}

fn finish_function(
    ast_function: ast::SharedFunction,
    body: ast::Block,
    params: Vec<ast::RcLocal>,
    is_variadic: bool,
    func_line: Option<usize>,
    upvalues_in: Vec<ast::RcLocal>,
    post: bool,
) -> (ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>) {
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
    ast_function: ast::SharedFunction,
    mut function: Function,
    upvalues_in: Vec<ast::RcLocal>,
    deadline: Instant,
) -> (ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>) {
    let params = function.parameters.clone();
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let node_count = function.graph().node_count();
    let large = node_count > 800;
    if Instant::now() >= deadline {
        let params = std::mem::take(&mut function.parameters);
        let is_variadic = function.is_variadic;
        let func_line = function.line;
        let mut body = flatten_cfg(&function);
        body.push(ast::Comment::new("skipped (time budget)".to_string()).into());
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
            let mut body = flatten_cfg(&function);
            body.push(ast::Comment::new("failed to decompile function".to_string()).into());
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
    } else if node_count > 2000 {
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
                let idom = cfg::compute_idoms(function.graph(), entry);
                let dom_idx = cfg::DomIndex::build(function.graph().node_indices(), &idom);
                changed |= structure_jumps(&mut function, &dom_idx);
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

    let destruct_ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ssa::Destructor::new(
            &mut function,
            upvalue_to_group,
            upvalues_in.iter().cloned().collect(),
            local_count,
        )
        .destruct();
    }))
    .is_ok();
    if !destruct_ok {
        // Half-destructed CFG: do not restructure or format it. That is
        // what deadlocked parking_lot after `local_defs[&local]` panicked.
        let mut body = flatten_cfg(&function);
        body.push(ast::Comment::new("failed to decompile function".to_string()).into());
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

    let params = std::mem::take(&mut function.parameters);
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let lifted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        restructure::lift(function)
    }));
    let body = match lifted {
        Ok(b) => b,
        Err(_) => ast::Block(vec![
            ast::Comment::new("failed to decompile function".to_string()).into(),
        ]),
    };
    let block = ast::share_block(body);
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
    upvalues: &mut FxHashMap<ByAddress<ast::SharedFunction>, Vec<ast::RcLocal>>,
) {
    if cfg::past_decompile_deadline() {
        return;
    }
    for stat in &mut body.0 {
        stat.traverse_rvalues(&mut |rvalue| {
            if let ast::RValue::Closure(closure) = rvalue {
                let Some(old_upvalues) = upvalues.get(&closure.function).cloned() else {
                    return;
                };
                let Some(mut function) = closure.function.try_lock() else {
                    return;
                };

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
                if let Some(mut b) = r#if.then_block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
            }
            ast::Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
            }
            ast::Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
            }
            ast::Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
            }
            ast::Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    link_upvalues(&mut b, upvalues);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod incomplete_output_tests {
    use super::*;

    fn assert_no_statement_goto(s: &str) {
        for line in s.lines() {
            let t = line.trim_start();
            assert!(!t.starts_with("goto "), "statement goto leaked: {line}");
            assert!(!t.starts_with("::"), "label leaked: {line}");
        }
    }

    #[test]
    fn incomplete_render_never_emits_statement_goto() {
        let mut body = ast::Block(vec![
            ast::Statement::Label(ast::Label("l93".into())),
            ast::Statement::Goto(ast::Goto::new(ast::Label("l95".into()))),
            ast::Statement::Label(ast::Label("l95".into())),
        ]);
        let s = render_incomplete(&mut body, "apply_guard_clauses", 1609);
        assert!(s.contains(INCOMPLETE_MARK));
        assert!(s.contains("apply_guard_clauses"));
        assert!(s.contains("functions printed:"));
        assert!(output_is_incomplete(&s));
        assert_no_statement_goto(&s);
    }

    #[test]
    fn complete_render_strips_goto() {
        let mut body = ast::Block(vec![ast::Statement::Goto(ast::Goto::new(
            ast::Label("l1".into()),
        ))]);
        let s = render_complete(&mut body);
        assert!(!s.contains(INCOMPLETE_MARK));
        assert_no_statement_goto(&s);
    }

    #[test]
    fn skip_budget_comment_marks_complete_path_incomplete() {
        let mut body = ast::Block(vec![ast::Comment::new("skipped (time budget)".into()).into()]);
        let s = render_complete(&mut body);
        assert!(s.contains(INCOMPLETE_MARK));
        assert!(output_is_incomplete(&s));
    }

    #[test]
    fn leftover_generic_for_marks_complete_path_incomplete() {
        let g = ast::RcLocal::new(ast::Local::new(Some("g".into())));
        let s = ast::RcLocal::new(ast::Local::new(Some("s".into())));
        let c = ast::RcLocal::new(ast::Local::new(Some("c".into())));
        let v1 = ast::RcLocal::new(ast::Local::new(Some("v1".into())));
        let mut body = ast::Block(vec![ast::GenericForNext::new(
            vec![v1],
            g.into(),
            s,
            c,
        )
        .into()]);
        let out = render_complete(&mut body);
        assert!(out.contains(INCOMPLETE_MARK), "{out}");
        assert!(output_is_incomplete(&out));
        assert!(!out.contains("[internal control]"), "{out}");
        assert!(out.contains("if v1 ~= nil then"), "{out}");
        for line in out.lines() {
            let t = line.trim_start();
            if t.starts_with("if ") {
                assert!(t.contains(" then"), "if without then: {line}");
            }
            assert!(!t.starts_with("goto "));
            assert!(!t.starts_with("::"));
        }
    }

    #[test]
    fn leftover_cfg_block_comment_marks_incomplete() {
        let mut body = ast::Block(vec![ast::Comment::new("block 102".into()).into()]);
        let out = render_complete(&mut body);
        assert!(out.contains(INCOMPLETE_MARK), "{out}");
        assert!(output_is_incomplete(&out));
    }

    #[test]
    fn caught_ssa_panic_marks_incomplete() {
        let mut body = ast::Block(vec![ast::Comment::new(
            "failed to decompile function".into(),
        )
        .into()]);
        let out = render_complete(&mut body);
        assert!(out.contains(INCOMPLETE_MARK), "{out}");
        assert!(output_is_incomplete(&out));
        assert!(output_is_incomplete("failed to decompile function"));
        assert!(output_is_incomplete("failed to lift function"));
    }

    #[test]
    fn incomplete_render_cyclic_else_terminates() {
        let shared = ast::share_block(ast::Block::default());
        let inner = ast::If {
            condition: ast::Literal::Boolean(true).into(),
            then_block: ast::share_block(ast::Block::default()),
            else_block: shared.clone(),
        };
        shared.lock().0.push(ast::Statement::If(inner));
        let mut body = ast::Block(vec![ast::Statement::If(ast::If {
            condition: ast::Literal::Boolean(false).into(),
            then_block: ast::share_block(ast::Block::default()),
            else_block: shared,
        })]);
        let s = render_incomplete(&mut body, "name_locals", 4);
        assert!(s.contains(INCOMPLETE_MARK));
        assert!(s.contains("name_locals"));
        assert!(s.len() < FALLBACK_BYTE_CAP + 4096);
        assert_no_statement_goto(&s);
    }
}
