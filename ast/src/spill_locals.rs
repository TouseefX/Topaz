//! Keeps every function under Luau's 200-register limit.
//!
//! Luau rejects a function whose body needs more than 200 registers at once
//! (`Out of local registers when trying to allocate <name>: exceeded limit
//! 200`). Locals are registers, so the limit is on how many locals are active
//! at the same point. The count includes locals in nested blocks, and the
//! hidden registers of loops.
//!
//! For each function body this module measures that peak. If the peak is above
//! [`TARGET_ACTIVE_LOCALS`], it moves locals into one table, `_spilled`,
//! created at the start of the function, so each use becomes `_spilled.<name>`.
//! The table is only used when needed, and in two steps:
//!
//! 1. Move the tail of the function's top-level locals. Top-level locals live
//!    for the whole call, so this never changes behaviour.
//! 2. If that is not enough, also move nested locals. A local declared inside
//!    a loop and captured by a closure is never moved, because a single table
//!    field would then be shared between iterations.

use std::collections::{HashMap, HashSet};

use itertools::Either;

use crate::{
    Assign, Block, Index, LValue, Literal, Local, LocalRw, RValue, RcLocal, Statement, Table,
    Traverse,
};

/// Peak number of simultaneously active locals a function may use. This is
/// Luau's own limit (`exceeded limit 200`): only the number of active locals
/// counts against it, not expression temporaries, so no extra margin is kept.
/// Functions that fit are left alone, so only the functions Luau would reject
/// get a `_spilled` table.
pub const TARGET_ACTIVE_LOCALS: usize = 200;

/// Hidden registers a Luau `for` loop keeps for its iteration state.
const LOOP_HIDDEN_REGISTERS: usize = 3;

const SPILL_TABLE: &str = "_spilled";

/// Lua keywords cannot be used as `_spilled.<name>` fields.
const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Spills locals in `block` and in every function nested inside it, when a
/// function needs more active locals than Luau allows. Each function body is
/// measured on its own.
pub fn spill_locals(block: &mut Block) {
    spill_function(block);
    visit_closures(block, &mut |body| spill_locals(body));
}

/// Calls `f` on the body of every closure reachable from `block`, including
/// closures inside nested `if`/loop bodies. `f` decides whether to recurse.
fn visit_closures<G: FnMut(&mut Block)>(block: &mut Block, f: &mut G) {
    for statement in &mut block.0 {
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let Either::Right(RValue::Closure(closure)) = value {
                if let Some(mut function) = closure.function.try_lock() {
                    f(&mut function.body);
                }
            }
            None
        });
        match statement {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    visit_closures(&mut b, f);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    visit_closures(&mut b, f);
                }
            }
            Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    visit_closures(&mut b, f);
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    visit_closures(&mut b, f);
                }
            }
            Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    visit_closures(&mut b, f);
                }
            }
            Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    visit_closures(&mut b, f);
                }
            }
            _ => {}
        }
    }
}

/// A local declared by a `local` statement inside one function body.
struct Decl {
    local: RcLocal,
    /// Declared at the body's top level (not inside any nested block).
    top_level: bool,
    /// Declared inside a loop body, at any depth.
    in_loop: bool,
}

/// Handles one function body. Nested closures are not measured here, because
/// each closure is its own Luau function. Their references to moved locals are
/// still rewritten.
fn spill_function(block: &mut Block) {
    // A body that already has its spill table (a shared body reached twice)
    // is done. A second table with the same name would shadow the first.
    let already_spilled = block.0.iter().any(|s| match s {
        Statement::Assign(assign) if assign.prefix => assign
            .left
            .iter()
            .any(|lv| matches!(lv, LValue::Local(l) if is_spill_table(l))),
        _ => false,
    });
    if already_spilled {
        return;
    }

    let none: HashSet<RcLocal> = HashSet::new();
    let mut decls: Vec<Decl> = Vec::new();
    let mut seen: HashSet<RcLocal> = HashSet::new();
    let peak = scan_block(block, 0, false, 0, &none, &mut decls, &mut seen);
    if peak <= TARGET_ACTIVE_LOCALS {
        return;
    }

    // Locals a closure references, so a closure may observe their value.
    let mut captured: HashSet<RcLocal> = HashSet::new();
    collect_captured(block, false, &mut captured);

    // A declaration inside a loop that a closure captures keeps its own
    // variable per iteration, so it cannot move into the shared table.
    let eligible = |d: &Decl| !d.in_loop || !captured.contains(&d.local);

    let mut top: Vec<RcLocal> = Vec::new();
    let mut nested: Vec<RcLocal> = Vec::new();
    for d in &decls {
        if !eligible(d) {
            continue;
        }
        if d.top_level {
            top.push(d.local.clone());
        } else {
            nested.push(d.local.clone());
        }
    }
    // Creation order, so field names are the same on every run.
    top.sort();
    nested.sort();

    // Move as few locals as possible. Nested locals go first, because they
    // are the ones that keep many scopes busy at once, then top-level locals.
    let mut candidates: Vec<RcLocal> = nested;
    candidates.extend(top);
    let mut spilled = choose_minimal(block, &candidates);
    spilled.sort();
    if spilled.is_empty() {
        return;
    }
    if std::env::var_os("TOPAZ_DEBUG_SPILL").is_some() {
        eprintln!(
            "[spill] body needs {} active locals (target {}); moving {} of {} declared locals",
            peak,
            TARGET_ACTIVE_LOCALS,
            spilled.len(),
            decls.len()
        );
    }

    let table = RcLocal::new(Local::new(Some(SPILL_TABLE.to_string())));
    let keys = field_keys(&spilled);

    rewrite_block(block, &table, &keys);
    fix_declarations(block);

    let mut decl = Assign::new(
        vec![LValue::Local(table.clone())],
        vec![RValue::Table(Table(Vec::new()))],
    );
    decl.prefix = true;
    block.0.insert(0, Statement::Assign(decl));
}

/// Returns the smallest prefix of `candidates` whose move brings the peak
/// active-local count to the target. Moving more locals never raises the peak,
/// so the peak is non-increasing in the prefix length and a binary search
/// finds the cut. If even moving every candidate is not enough, all of them are
/// returned.
fn choose_minimal(block: &Block, candidates: &[RcLocal]) -> Vec<RcLocal> {
    let peak_moving_first = |k: usize| -> usize {
        let excluded: HashSet<RcLocal> = candidates[..k].iter().cloned().collect();
        let mut decls: Vec<Decl> = Vec::new();
        let mut seen: HashSet<RcLocal> = HashSet::new();
        scan_block(block, 0, false, 0, &excluded, &mut decls, &mut seen)
    };
    let (mut lo, mut hi) = (0usize, candidates.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if peak_moving_first(mid) <= TARGET_ACTIVE_LOCALS {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    if peak_moving_first(lo) > TARGET_ACTIVE_LOCALS {
        return candidates.to_vec();
    }
    candidates[..lo].to_vec()
}

/// Measures the peak number of simultaneously active locals in one function
/// body, not counting closures. `excluded` locals are treated as moved to the
/// table, so they do not count. Every declaration is appended to `decls`.
fn scan_block(
    block: &Block,
    depth: usize,
    in_loop: bool,
    base: usize,
    excluded: &HashSet<RcLocal>,
    decls: &mut Vec<Decl>,
    seen: &mut HashSet<RcLocal>,
) -> usize {
    let mut active = base;
    let mut peak = base;
    for statement in &block.0 {
        match statement {
            Statement::Assign(assign) if assign.prefix => {
                for lvalue in &assign.left {
                    if let LValue::Local(local) = lvalue {
                        if seen.insert(local.clone()) {
                            decls.push(Decl {
                                local: local.clone(),
                                top_level: depth == 0,
                                in_loop,
                            });
                            if !excluded.contains(local) {
                                active += 1;
                            }
                        }
                    }
                }
                peak = peak.max(active);
            }
            Statement::If(r#if) => {
                if let Some(b) = r#if.then_block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        in_loop,
                        active,
                        excluded,
                        decls,
                        seen,
                    ));
                }
                if let Some(b) = r#if.else_block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        in_loop,
                        active,
                        excluded,
                        decls,
                        seen,
                    ));
                }
            }
            Statement::While(r#while) => {
                if let Some(b) = r#while.block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        true,
                        active,
                        excluded,
                        decls,
                        seen,
                    ));
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(b) = repeat.block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        true,
                        active,
                        excluded,
                        decls,
                        seen,
                    ));
                }
            }
            Statement::NumericFor(numeric_for) => {
                // The control variable is always a real local, and the loop
                // keeps hidden registers while the body runs.
                let inner = active + LOOP_HIDDEN_REGISTERS + 1;
                peak = peak.max(inner);
                if let Some(b) = numeric_for.block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        true,
                        inner,
                        excluded,
                        decls,
                        seen,
                    ));
                }
            }
            Statement::GenericFor(generic_for) => {
                let inner = active + LOOP_HIDDEN_REGISTERS + generic_for.res_locals.len();
                peak = peak.max(inner);
                if let Some(b) = generic_for.block.try_lock() {
                    peak = peak.max(scan_block(
                        &b,
                        depth + 1,
                        true,
                        inner,
                        excluded,
                        decls,
                        seen,
                    ));
                }
            }
            _ => {}
        }
    }
    peak
}

/// Adds every local that a closure references, at any depth, to `out`.
fn collect_captured(block: &mut Block, in_closure: bool, out: &mut HashSet<RcLocal>) {
    for statement in &mut block.0 {
        if in_closure {
            for local in statement.values_read_mut() {
                out.insert(local.clone());
            }
            for local in statement.values_written_mut() {
                out.insert(local.clone());
            }
        }
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let Either::Right(RValue::Closure(closure)) = value {
                if let Some(mut function) = closure.function.try_lock() {
                    collect_captured(&mut function.body, true, out);
                }
            }
            None
        });
        match statement {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
            }
            Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
            }
            Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
            }
            Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    collect_captured(&mut b, in_closure, out);
                }
            }
            _ => {}
        }
    }
}

/// Field name per moved local: its source name, sanitized and made unique.
/// `locals` must be sorted so the names are deterministic.
fn field_keys(locals: &[RcLocal]) -> HashMap<RcLocal, String> {
    let mut keys: HashMap<RcLocal, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    for (i, local) in locals.iter().enumerate() {
        let base = local
            .0
            .0
            .lock()
            .0
            .clone()
            .unwrap_or_else(|| format!("v{}", i));
        let mut key: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if key.is_empty() || key.starts_with(|c: char| c.is_ascii_digit()) {
            key.insert(0, '_');
        }
        if KEYWORDS.contains(&key.as_str()) {
            key.push('_');
        }
        let mut unique = key.clone();
        let mut n = 2;
        while used.contains(&unique) {
            unique = format!("{key}_{n}");
            n += 1;
        }
        used.insert(unique.clone());
        keys.insert(local.clone(), unique);
    }
    keys
}

fn is_spill_table(local: &RcLocal) -> bool {
    local.0 .0.lock().0.as_deref() == Some(SPILL_TABLE)
}

fn field(table: &RcLocal, key: &str) -> Index {
    Index::new(
        RValue::Local(table.clone()),
        RValue::Literal(Literal::String(key.as_bytes().to_vec())),
    )
}

/// Replaces every use of a moved local with `_spilled.<key>`, in this block,
/// its nested blocks, and nested closures.
fn rewrite_block(block: &mut Block, table: &RcLocal, keys: &HashMap<RcLocal, String>) {
    for statement in &mut block.0 {
        statement.post_traverse_values(&mut |value| -> Option<()> {
            match value {
                Either::Left(lvalue) => {
                    if let LValue::Local(local) = lvalue {
                        if let Some(key) = keys.get(local) {
                            let replacement = LValue::Index(field(table, key));
                            *lvalue = replacement;
                        }
                    }
                }
                Either::Right(rvalue) => match rvalue {
                    RValue::Local(local) => {
                        if let Some(key) = keys.get(local) {
                            let replacement = RValue::Index(field(table, key));
                            *rvalue = replacement;
                        }
                    }
                    RValue::Closure(closure) => {
                        if let Some(mut function) = closure.function.try_lock() {
                            rewrite_block(&mut function.body, table, keys);
                        }
                    }
                    _ => {}
                },
            }
            None
        });
        match statement {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
            }
            Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
            }
            Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
            }
            Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    rewrite_block(&mut b, table, keys);
                }
            }
            _ => {}
        }
    }
}

/// After rewriting, a `local` declaration can mix kept locals with
/// `_spilled.<key>` targets, and `_spilled.<key>` is not valid after `local`.
/// Such a declaration becomes: `local <kept locals>` on their own, then a plain
/// assignment to all targets. An empty right side becomes `nil` values.
/// Applies to this block and its nested blocks, not to closures, which have
/// their own declarations.
fn fix_declarations(block: &mut Block) {
    let old = std::mem::take(&mut block.0);
    let mut out = Vec::with_capacity(old.len() + 8);
    for statement in old {
        match statement {
            Statement::Assign(mut assign)
                if assign.prefix
                    && assign.left.iter().any(|lv| matches!(lv, LValue::Index(_))) =>
            {
                let kept: Vec<LValue> = assign
                    .left
                    .iter()
                    .filter(|lv| matches!(lv, LValue::Local(_)))
                    .cloned()
                    .collect();
                if !kept.is_empty() {
                    let mut pre = Assign::new(kept, Vec::new());
                    pre.prefix = true;
                    out.push(Statement::Assign(pre));
                }
                assign.prefix = false;
                if assign.right.is_empty() {
                    let count = assign.left.len();
                    assign.right = (0..count)
                        .map(|_| RValue::Literal(Literal::Nil))
                        .collect();
                }
                out.push(Statement::Assign(assign));
            }
            other => out.push(other),
        }
    }
    block.0 = out;

    for statement in &mut block.0 {
        match statement {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    fix_declarations(&mut b);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    fix_declarations(&mut b);
                }
            }
            Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    fix_declarations(&mut b);
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    fix_declarations(&mut b);
                }
            }
            Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    fix_declarations(&mut b);
                }
            }
            Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    fix_declarations(&mut b);
                }
            }
            _ => {}
        }
    }
}
