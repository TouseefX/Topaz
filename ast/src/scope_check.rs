//! Debug check, enabled by `TOPAZ_SCOPE_CHECK`.
//!
//! Walks a block the way Luau scopes it and counts reads and writes of locals
//! that are not in scope where they are used. Two kinds are reported apart:
//!
//! - *out of scope*: the local is declared in this body, but not in an
//!   enclosing scope at the use. This is a real scoping bug, and the name is a
//!   global at run time.
//! - *never declared*: no declaration is visible from this body. Before link
//!   time, closure upvalues still carry their pre-link identity, so this kind
//!   is mostly noise per function and is only meaningful in the whole-tree
//!   check.
//!
//! A block whose lock is already held cannot be walked, and is counted apart.

use std::cell::RefCell;
use std::collections::HashSet;

use itertools::Either;

use crate::{Block, LValue, LocalRw, RValue, RcLocal, SharedBlock, Statement, Traverse};

/// What one check found.
#[derive(Default)]
pub struct ScopeReport {
    /// Reads and writes of locals not in scope at the use.
    pub unresolved: usize,
    /// Of those, the locals that no declaration in the checked part declares.
    pub never_declared: usize,
    /// Blocks that could not be locked, so their contents were not checked.
    pub skipped_blocks: usize,
    /// The first few offenders, as `name (kind, function at line N)`.
    pub examples: Vec<String>,
    /// The first few out-of-scope uses, in the same form.
    pub oos_examples: Vec<String>,
    /// The first few statements that contain an out-of-scope use, as text.
    pub oos_snippets: Vec<String>,
    /// The first few statements that contain an offender, as text.
    pub snippets: Vec<String>,
    /// Every local declared in the checked part.
    declared: HashSet<RcLocal>,
}

const MAX_EXAMPLES: usize = 4;
const MAX_OOS_EXAMPLES: usize = 12;
const MAX_OOS_SNIPPETS: usize = 12;
const MAX_SNIPPETS: usize = 3;

thread_local! {
    static SEED: RefCell<Vec<RcLocal>> = RefCell::new(Vec::new());
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

pub fn enabled() -> bool {
    std::env::var_os("TOPAZ_SCOPE_CHECK").is_some()
}

/// Sets the locals in scope on entry to the function whose body the post
/// passes are about to change (its upvalues and parameters).
pub fn set_seed(seed: Vec<RcLocal>) {
    SEED.with(|s| *s.borrow_mut() = seed);
}

/// Records the per-function check of `block` after one pass.
pub fn note(stage: &str, block: &mut Block) {
    if !enabled() {
        return;
    }
    let seed = SEED.with(|s| s.borrow().clone());
    let r = check_function(block, &seed, false);
    let line = format!(
        "{}={}/{}/{}",
        stage, r.unresolved, r.never_declared, r.skipped_blocks
    );
    LOG.with(|l| l.borrow_mut().push(line));
}

/// Returns and clears the records made by [`note`] on this thread.
pub fn take_log() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// Checks `block` as a whole program: its own locals and every closure in it.
pub fn check(block: &mut Block) -> ScopeReport {
    let mut report = ScopeReport::default();
    collect_declared(block, &mut report.declared, true);
    let mut scope: HashSet<RcLocal> = HashSet::new();
    check_block(block, &mut scope, None, &mut report, true);
    report
}

/// Checks one function body. `seed` holds the locals in scope before the body
/// starts. With `closures` false, nested closures are not entered; their own
/// bodies are checked when they are finished.
pub fn check_function(block: &mut Block, seed: &[RcLocal], closures: bool) -> ScopeReport {
    let mut report = ScopeReport::default();
    collect_declared(block, &mut report.declared, closures);
    report.declared.extend(seed.iter().cloned());
    let mut scope: HashSet<RcLocal> = seed.iter().cloned().collect();
    check_block(block, &mut scope, None, &mut report, closures);
    report
}

/// Every local a `local` statement, loop or parameter declares in `block`,
/// its nested blocks and, when `closures`, the closures inside it.
fn collect_declared(block: &mut Block, out: &mut HashSet<RcLocal>, closures: bool) {
    for statement in block.0.iter_mut() {
        if let Statement::Assign(assign) = &*statement {
            if assign.prefix {
                for lvalue in &assign.left {
                    if let LValue::Local(local) = lvalue {
                        out.insert(local.clone());
                    }
                }
            }
        }
        if closures {
            statement.post_traverse_values(&mut |value| -> Option<()> {
                if let Either::Right(RValue::Closure(closure)) = value {
                    if let Some(mut function) = closure.function.try_lock() {
                        for param in &function.parameters {
                            out.insert(param.clone());
                        }
                        collect_declared(&mut function.body, out, true);
                    }
                }
                None
            });
        }
        match statement {
            Statement::If(r#if) => {
                collect_declared_shared(&r#if.then_block, out, closures);
                collect_declared_shared(&r#if.else_block, out, closures);
            }
            Statement::While(r#while) => collect_declared_shared(&r#while.block, out, closures),
            Statement::Repeat(repeat) => collect_declared_shared(&repeat.block, out, closures),
            Statement::NumericFor(numeric_for) => {
                out.insert(numeric_for.counter.clone());
                collect_declared_shared(&numeric_for.block, out, closures);
            }
            Statement::GenericFor(generic_for) => {
                for local in &generic_for.res_locals {
                    out.insert(local.clone());
                }
                collect_declared_shared(&generic_for.block, out, closures);
            }
            _ => {}
        }
    }
}

fn collect_declared_shared(block: &SharedBlock, out: &mut HashSet<RcLocal>, closures: bool) {
    if let Some(mut b) = block.try_lock() {
        collect_declared(&mut b, out, closures);
    }
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
    let declared_somewhere = report.declared.contains(local);
    if !declared_somewhere {
        report.never_declared += 1;
    }
    // Out-of-scope uses get their own list, so the never-declared noise
    // before link time cannot fill the examples first.
    let want = if declared_somewhere {
        report.oos_examples.len() < MAX_OOS_EXAMPLES
    } else {
        report.examples.len() < MAX_EXAMPLES
    };
    if want {
        // Display prints the name, or UNNAMED_<id> before naming, which is
        // the same text the statements show.
        let name = format!("{}", local);
        if declared_somewhere {
            report
                .oos_examples
                .push(format!("{} (out of scope, function at line {:?})", name, line));
        } else {
            report
                .examples
                .push(format!("{} (never declared, function at line {:?})", name, line));
        }
    }
}

fn check_block(
    block: &mut Block,
    scope: &mut HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
    closures: bool,
) {
    for statement in block.0.iter_mut() {
        check_statement(statement, scope, line, report, closures);
    }
}

fn check_owned_block(
    block: &SharedBlock,
    mut scope: HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
    closures: bool,
) {
    match block.try_lock() {
        Some(mut b) => check_block(&mut b, &mut scope, line, report, closures),
        None => report.skipped_blocks += 1,
    }
}

fn check_statement(
    statement: &mut Statement,
    scope: &mut HashSet<RcLocal>,
    line: Option<usize>,
    report: &mut ScopeReport,
    closures: bool,
) {
    let declares = matches!(statement, Statement::Assign(assign) if assign.prefix);

    // A repeat's condition sees the locals of its block, so it is checked
    // below, inside that scope.
    let before = report.unresolved;
    let before_oos = report.unresolved - report.never_declared;
    if !matches!(statement, Statement::Repeat(_)) {
        // Read the expression tree itself. A closure's `values_read` also lists
        // its upvalues, which are not printed and may still name a local that
        // was moved elsewhere, so they are left out here.
        let mut reads: Vec<RcLocal> = Vec::new();
        let mut writes: Vec<RcLocal> = Vec::new();
        statement.post_traverse_values(&mut |value| -> Option<()> {
            match value {
                Either::Left(LValue::Local(local)) => writes.push(local.clone()),
                Either::Right(RValue::Local(local)) => reads.push(local.clone()),
                _ => {}
            }
            None
        });
        for local in &reads {
            check_use(local, scope, line, report);
        }
        if !declares {
            for local in &writes {
                check_use(local, scope, line, report);
            }
        }
    }
    if report.unresolved > before && report.snippets.len() < MAX_SNIPPETS {
        let text = format!("{}", statement).replace('\n', " ");
        report.snippets.push(text.chars().take(220).collect());
    }
    if report.unresolved - report.never_declared > before_oos
        && report.oos_snippets.len() < MAX_OOS_SNIPPETS
    {
        let text = format!("{}", statement).replace('\n', " ");
        report.oos_snippets.push(text.chars().take(260).collect());
    }

    // Closures see the locals in scope at the statement, not the ones it
    // declares, so they are checked before the declarations below.
    if closures {
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let Either::Right(RValue::Closure(closure)) = value {
                match closure.function.try_lock() {
                    Some(mut function) => {
                        let mut inner = scope.clone();
                        for param in &function.parameters {
                            inner.insert(param.clone());
                        }
                        let func_line = function.line;
                        check_block(&mut function.body, &mut inner, func_line, report, true);
                    }
                    None => report.skipped_blocks += 1,
                }
            }
            None
        });
    }

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
            check_owned_block(&r#if.then_block, scope.clone(), line, report, closures);
            check_owned_block(&r#if.else_block, scope.clone(), line, report, closures);
        }
        Statement::While(r#while) => {
            check_owned_block(&r#while.block, scope.clone(), line, report, closures);
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
            check_owned_block(&repeat.block, scope.clone(), line, report, closures);
        }
        Statement::NumericFor(numeric_for) => {
            let mut inner = scope.clone();
            inner.insert(numeric_for.counter.clone());
            check_owned_block(&numeric_for.block, inner, line, report, closures);
        }
        Statement::GenericFor(generic_for) => {
            let mut inner = scope.clone();
            for local in &generic_for.res_locals {
                inner.insert(local.clone());
            }
            check_owned_block(&generic_for.block, inner, line, report, closures);
        }
        _ => {}
    }
}
