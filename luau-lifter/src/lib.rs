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

use clap::Parser;
use parking_lot::Mutex;
use petgraph::algo::dominators::simple_fast;
use rayon::prelude::*;

use anyhow::anyhow;
use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;
use walkdir::WalkDir;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Instant,
};

use deserializer::bytecode::Bytecode;

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
    // 1) Plain path (luaur-compatible).
    if let Ok(c) = deserializer::loadsafe_ir::decode_chunk(bytecode, 1) {
        return decompile_from_chunk(c, 1);
    }

    // 2) Same IR decoder with a detected Roblox / custom encode key.
    let key = detect_encode_key(bytecode, encode_key);
    match deserializer::loadsafe_ir::decode_chunk(bytecode, key) {
        Ok(c) => decompile_from_chunk(c, key),
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
    // Wrap the entire decompilation in catch_unwind to prevent
    // panics in the AST/restructure pipeline from killing the process.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        decompile_from_chunk_inner(chunk, encode_key)
    }))
    .unwrap_or_else(|_| {
        "-- Decompiled with Topaz\n-- Error: decompilation panicked\n".to_string()
    })
}

fn decompile_from_chunk_inner(chunk: deserializer::chunk::Chunk, encode_key: u8) -> String {
    let mut lifted = Vec::new();
    let mut stack = vec![(Arc::<Mutex<ast::Function>>::default(), chunk.main)];
    while let Some((ast_func, func_id)) = stack.pop() {
        let (function, upvalues, child_functions) =
            Lifter::lift(&chunk.functions, &chunk.string_table, func_id as usize);
        lifted.push((ast_func, function, upvalues));
        stack.extend(child_functions.into_iter().map(|(a, f)| (a.0, f as u32)));
    }

    let (main, ..) = lifted.first().unwrap().clone();

    // Process all functions in parallel using rayon.
    // Each function is independent (owns its own Function CFG).
    let mut upvalues: FxHashMap<_, _> = lifted
        .into_par_iter()
        .map(|(ast_function, function, upvalues_in)| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decompile_function(ast_function.clone(), function, upvalues_in)
            }));

            match result {
                Ok(r) => r,
                Err(e) => {
                    let panic_information = match e.downcast::<String>() {
                        Ok(v) => *v,
                        Err(e) => match e.downcast::<&str>() {
                            Ok(v) => v.to_string(),
                            _ => "Unknown Source of Error".to_owned(),
                        },
                    };

                    let mut message = String::new();
                    use std::fmt::Write;
                    writeln!(message, "failed to decompile: {panic_information}").unwrap();

                    ast_function.lock().body.extend(
                        message
                            .trim_end()
                            .split('\n')
                            .map(|s| ast::Comment::new(s.to_string()).into()),
                    );
                    (ByAddress(ast_function), Vec::new())
                }
            }
        })
        .collect();

    let main = ByAddress(main);
    upvalues.remove(&main);
    let mut body = Arc::try_unwrap(main.0).unwrap().into_inner().body;
    link_upvalues(&mut body, &mut upvalues);
    ast::context_naming::apply_context_naming(&mut body);
    propagate_names(&mut body);
    inline_short_gotos(&mut body);
    ast::guard_clauses::apply_guard_clauses(&mut body);
    name_locals(&mut body, true);

    format!(
        "-- Decomplied with Topaz\n-- Created by: Andrew & TouseefX\n-- Key: {}\n\n{}",
        encode_key,
        body.to_string()
    )
}



#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    paths: Vec<String>,

    #[clap(short, long, default_value_t = 0)]
    threads: usize,

    #[clap(short, long, default_value_t = 1)]
    key: u8,
    #[clap(short, long)]
    recursive: bool,
    #[clap(short, long)]
    verbose: bool,
}


pub fn detect_encode_key(bytecode: &[u8], preferred: u8) -> u8 {
    // Prefer a key that fully parses through loadsafe_ir (same IR as decompile).
    // Key 0 is invalid (wrapping_mul maps every op to NOP).
    // Order: plain 1 first is handled by the caller; here we try preferred
    // then Roblox 203 then 1.
    let mut candidates = vec![preferred, 203u8, 1u8];
    candidates.dedup();
    candidates.retain(|&k| k != 0);
    if candidates.is_empty() {
        candidates.push(1);
    }
    for candidate in candidates {
        if deserializer::loadsafe_ir::decode_chunk(bytecode, candidate).is_ok() {
            return candidate;
        }
    }
    if preferred == 0 { 1 } else { preferred }
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
    if let Ok(c) = deserializer::loadsafe_ir::decode_chunk(bytecode, 1) {
        return dump_cfgs_from_chunk(c);
    }
    let key = detect_encode_key(bytecode, 1);
    match deserializer::loadsafe_ir::decode_chunk(bytecode, key) {
        Ok(c) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

#[deprecated(note = "use dump_cfgs_via_luaur")]
pub fn dump_cfgs_via_ruau(bytecode: &[u8]) -> Vec<cfg::CfgSnapshot> {
    dump_cfgs_via_luaur(bytecode)
}

pub fn dump_cfgs_default(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    if let Ok(c) = deserializer::loadsafe_ir::decode_chunk(bytecode, 1) {
        return dump_cfgs_from_chunk(c);
    }
    let key = detect_encode_key(bytecode, encode_key);
    match deserializer::loadsafe_ir::decode_chunk(bytecode, key) {
        Ok(c) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

pub fn dump_cfgs(bytecode: &[u8], encode_key: u8) -> Vec<cfg::CfgSnapshot> {
    ast::reset_local_id_counter();
    let encode_key = detect_encode_key(bytecode, encode_key);
    match deserializer::loadsafe_ir::decode_chunk(bytecode, encode_key) {
        Ok(c) => dump_cfgs_from_chunk(c),
        Err(_) => Vec::new(),
    }
}

pub fn decompile_bytecode(bytecode: &[u8], encode_key: u8) -> String {
    ast::reset_local_id_counter();
    let encode_key = detect_encode_key(bytecode, encode_key);
    match deserializer::loadsafe_ir::decode_chunk(bytecode, encode_key) {
        Ok(c) => decompile_from_chunk(c, encode_key),
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

fn decompile_function(
    ast_function: Arc<Mutex<ast::Function>>,
    mut function: Function,
    upvalues_in: Vec<ast::RcLocal>,
) -> (ByAddress<Arc<Mutex<ast::Function>>>, Vec<ast::RcLocal>) {
    let (local_count, local_groups, upvalue_in_groups, upvalue_passed_groups) =
        cfg::ssa::construct(&mut function, &upvalues_in);
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
    while changed {
        changed = false;

        let dominators = simple_fast(function.graph(), function.entry().unwrap());
        changed |= structure_jumps(&mut function, &dominators);

        ssa::inline::inline(&mut function, &local_to_group, &upvalue_to_group);

        if structure_conditionals(&mut function)

        {
            changed = true;
        }
        let mut local_map = FxHashMap::default();

        if ssa::construct::remove_unnecessary_params(&mut function, &mut local_map) {
            changed = true;
        }
        ssa::construct::apply_local_map(&mut function, local_map);
    }

    ssa::Destructor::new(
        &mut function,
        upvalue_to_group,
        upvalues_in.iter().cloned().collect(),
        local_count,
    )
    .destruct();

    let params = std::mem::take(&mut function.parameters);
    let is_variadic = function.is_variadic;
    let func_line = function.line;
    let block = Arc::new(restructure::lift(function).into());
    LocalDeclarer::default().declare_locals(

        Arc::clone(&block),
        &upvalues_in.iter().chain(params.iter()).cloned().collect(),
    );

    {
        let mut ast_function = ast_function.lock();
        ast_function.body = Arc::try_unwrap(block).unwrap().into_inner();
        
        // Apply post-processing to this function's body
        post_process::apply_all(&mut ast_function.body);
        ast_function.parameters = params;
        ast_function.is_variadic = is_variadic;
        ast_function.line = func_line;
    }
    (ByAddress(ast_function), upvalues_in)
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
