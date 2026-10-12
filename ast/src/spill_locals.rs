//! Keeps every function under Luau's 200-active-local limit.
//!
//! Luau rejects a function whose scope holds more than 200 active locals
//! (`cFrame652` in compile output). The first [`KEEP_TOP_LEVEL_LOCALS`]
//! locals a function declares at its own top level stay real locals. Every
//! later top-level local moves into one table, `_spilled`, created at the
//! start of the function, so each use becomes `_spilled.<name>`.
//!
//! A top-level local lives for the whole call, so the table gives it the same
//! lifetime, and closures that capture it still share one variable. Locals
//! declared inside loops or nested blocks are never spilled, which keeps their
//! per-iteration semantics.

use std::collections::{HashMap, HashSet};

use itertools::Either;

use crate::{
    Assign, Block, Index, LValue, Literal, Local, RValue, RcLocal, Statement, Table, Traverse,
};

/// Number of top-level locals per function that stay real locals. The rest
/// spill. The margin covers locals declared inside nested blocks.
pub const KEEP_TOP_LEVEL_LOCALS: usize = 120;

const SPILL_TABLE: &str = "_spilled";

/// Lua keywords cannot be used as `_spilled.<name>` fields.
const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Spills excess top-level locals in `block` and in every function nested
/// inside it. Each function body is handled on its own, so the 200-local
/// limit is checked per function.
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

/// Handles one function body. Only the body's own top-level declarations are
/// considered. References inside nested closures are rewritten too, because
/// those closures capture the same locals.
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

    // Top-level `local` declarations in source order.
    let mut declared: Vec<RcLocal> = Vec::new();
    let mut seen: HashSet<RcLocal> = HashSet::new();
    for statement in &block.0 {
        if let Statement::Assign(assign) = statement {
            if !assign.prefix {
                continue;
            }
            for lvalue in &assign.left {
                if let LValue::Local(local) = lvalue {
                    if seen.insert(local.clone()) {
                        declared.push(local.clone());
                    }
                }
            }
        }
    }
    if declared.len() <= KEEP_TOP_LEVEL_LOCALS {
        return;
    }

    let spilled: Vec<RcLocal> = declared.split_off(KEEP_TOP_LEVEL_LOCALS);
    if std::env::var_os("TOPAZ_DEBUG_SPILL").is_some() {
        eprintln!(
            "[spill] body declares {} top-level locals; keeping {}, spilling {}",
            KEEP_TOP_LEVEL_LOCALS + spilled.len(),
            KEEP_TOP_LEVEL_LOCALS,
            spilled.len()
        );
    }
    let table = RcLocal::new(Local::new(Some(SPILL_TABLE.to_string())));

    // Field name per spilled local: its source name, sanitized and made unique.
    let mut keys: HashMap<RcLocal, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    for (i, local) in spilled.iter().enumerate() {
        let base = local
            .0
            .0
            .lock()
            .0
            .clone()
            .unwrap_or_else(|| format!("v{}", KEEP_TOP_LEVEL_LOCALS + i));
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

    rewrite_block(block, &table, &keys);
    fix_declarations(block);

    let mut decl = Assign::new(
        vec![LValue::Local(table.clone())],
        vec![RValue::Table(Table(Vec::new()))],
    );
    decl.prefix = true;
    block.0.insert(0, Statement::Assign(decl));
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

/// Replaces every use of a spilled local with `_spilled.<key>`, in this block,
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

/// After rewriting, a top-level `local` declaration can mix kept locals with
/// `_spilled.<key>` targets, and `_spilled.<key>` is not valid after `local`.
/// Such a declaration becomes: `local <kept locals>` on their own, then a plain
/// assignment to all targets. An empty right side becomes `nil` values.
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
}
