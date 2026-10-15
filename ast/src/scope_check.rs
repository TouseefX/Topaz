//! Debug check, enabled by `TOPAZ_SCOPE_CHECK`.
//!
//! Walks a block the way Luau scopes it and counts every read or write of a
//! local that no enclosing scope declares. Such a name is a global at run
//! time, which is `nil` unless something defines it, so each one is a
//! functional break and not a readability problem. A block whose lock is
//! already held cannot be walked, so it is counted separately.

use std::collections::HashSet;

use itertools::Either;

use crate::{Block, LValue, LocalRw, RValue, RcLocal, SharedBlock, Statement, Traverse};

/// What one check found.
#[derive(Default)]
pub struct ScopeReport {
    /// Reads and writes of locals that no enclosing scope declares.
    pub unresolved: usize,
    /// Blocks that could not be locked, so their contents were not checked.
    pub skipped_blocks: usize,
    /// The first few offenders, as `name (function at line N)`.
    pub examples: Vec<String>,
}

const MAX_EXAMPLES: usize = 4;

/// Checks `block` as a whole program: its own locals and every closure in it.
pub fn check(block: &mut Block) -> ScopeReport {
    let mut report = ScopeReport::default();
    let mut scope: HashSet<RcLocal> = HashSet::new();
    check_block(block, &mut scope, None, &mut report);
    report
}

/// Checks one function body. `seed` holds the locals that are in scope before
/// the body starts: its upvalues, parameters and hoisted locals.
pub fn check_function(block: &mut Block, seed: &[RcLocal]) -> ScopeReport {
    let mut report = ScopeReport::default();
    let mut scope: HashSet<RcLocal> = seed.iter().cloned().collect();
    check_block(block, &mut scope, None, &mut report);
    report
}

fn check_use(
    local: &RcLocal,
    scope: &HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
) {
    if scope.contains(local) {
        return;
    }
    report.unresolved += 1;
    if report.examples.len() < MAX_EXAMPLES {
        let name = local
            .0
             .0
            .lock()
            .0
            .clone()
            .unwrap_or_else(|| "?".to_string());
        report
            .examples
            .push(format!("{} (function at line {:?})", name, line));
    }
}

fn check_block(
    block: &mut Block,
    scope: &mut HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
) {
    for statement in block.0.iter_mut() {
        check_statement(statement, scope, line, report);
    }
}

fn check_owned_block(
    block: &SharedBlock,
    mut scope: HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
) {
    match block.try_lock() {
        Some(mut b) => check_block(&mut b, &mut scope, line, report),
        None => report.skipped_blocks += 1,
    }
}

fn check_statement(
    statement: &mut Statement,
    scope: &mut HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
) {
    let declares = matches!(statement, Statement::Assign(assign) if assign.prefix);

    // A repeat's condition sees the locals of its block, so it is checked
    // below, inside that scope.
    if !matches!(statement, Statement::Repeat(_)) {
        let reads: Vec<RcLocal> = statement.values_read().into_iter().cloned().collect();
        for local in &reads {
            check_use(local, scope, line, report);
        }
        if !declares {
            let writes: Vec<RcLocal> = statement.values_written().into_iter().cloned().collect();
            for local in &writes {
                check_use(local, scope, line, report);
            }
        }
    }

    // Closures see the locals in scope at the statement, not the ones it
    // declares, so they are checked before the declarations below.
    statement.post_traverse_values(&mut |value| -> Option<()> {
        if let Either::Right(RValue::Closure(closure)) = value {
            match closure.function.try_lock() {
                Some(mut function) => {
                    let mut inner = scope.clone();
                    for param in &function.parameters {
                        inner.insert(param.clone());
                    }
                    let func_line = function.line;
                    check_block(&mut function.body, &mut inner, func_line, report);
                }
                None => report.skipped_blocks += 1,
            }
        }
        None
    });

    if let Statement::Assign(assign) = &*statement {
        if assign.prefix {
            for lvalue in &assign.left {
                if let LValue::Local(local) = lvalue {
                    scope.insert(local.clone());
                }
            }
        }
    }

    match statement {
        Statement::If(r#if) => {
            check_owned_block(&r#if.then_block, scope.clone(), line, report);
            check_owned_block(&r#if.else_block, scope.clone(), line, report);
        }
        Statement::While(r#while) => {
            check_owned_block(&r#while.block, scope.clone(), line, report);
        }
        Statement::Repeat(repeat) => {
            let mut inner = scope.clone();
            if let Some(b) = repeat.block.try_lock() {
                for s in b.0.iter() {
                    if let Statement::Assign(assign) = s {
                        if assign.prefix {
                            for lvalue in &assign.left {
                                if let LValue::Local(local) = lvalue {
                                    inner.insert(local.clone());
                                }
                            }
                        }
                    }
                }
            }
            for local in repeat.condition.values_read() {
                check_use(local, &inner, line, report);
            }
            check_owned_block(&repeat.block, scope.clone(), line, report);
        }
        Statement::NumericFor(numeric_for) => {
            let mut inner = scope.clone();
            inner.insert(numeric_for.counter.clone());
            check_owned_block(&numeric_for.block, inner, line, report);
        }
        Statement::GenericFor(generic_for) => {
            let mut inner = scope.clone();
            for local in &generic_for.res_locals {
                inner.insert(local.clone());
            }
            check_owned_block(&generic_for.block, inner, line, report);
        }
        _ => {}
    }
}
