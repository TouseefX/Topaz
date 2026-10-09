//! Unused Variable Detection and Renaming
//!
//! Detects unused local variables and function parameters, renaming them to `_`
//! following Lua conventions.

use crate::{Block, Statement, LValue, RValue, RcLocal, LocalRw, Traverse};
use rustc_hash::FxHashSet;

/// Mark unused variables with `_`
pub fn mark_unused_variables(block: &mut Block) {
    let mut all_locals: FxHashSet<RcLocal> = FxHashSet::default();
    collect_locals(block, &mut all_locals);

    let mut used_locals: FxHashSet<RcLocal> = FxHashSet::default();
    collect_used_locals(block, &mut used_locals);

    let unused_locals: FxHashSet<RcLocal> = all_locals
        .difference(&used_locals)
        .cloned()
        .collect();

    rename_unused(block, &unused_locals);
}

fn for_nested(block: &Block, f: &mut impl FnMut(&Block)) {
    for statement in block.iter() {
        match statement {
            Statement::If(if_stmt) => {
                if let Some(b) = if_stmt.then_block.try_lock() {
                    f(&b);
                }
                if let Some(b) = if_stmt.else_block.try_lock() {
                    f(&b);
                }
            }
            Statement::While(while_stmt) => {
                if let Some(b) = while_stmt.block.try_lock() {
                    f(&b);
                }
            }
            Statement::Repeat(repeat_stmt) => {
                if let Some(b) = repeat_stmt.block.try_lock() {
                    f(&b);
                }
            }
            Statement::NumericFor(for_stmt) => {
                if let Some(b) = for_stmt.block.try_lock() {
                    f(&b);
                }
            }
            Statement::GenericFor(for_stmt) => {
                if let Some(b) = for_stmt.block.try_lock() {
                    f(&b);
                }
            }
            _ => {}
        }
    }
}

fn for_nested_mut(block: &mut Block, f: &mut impl FnMut(&mut Block)) {
    for statement in block.iter_mut() {
        match statement {
            Statement::If(if_stmt) => {
                if let Some(mut b) = if_stmt.then_block.try_lock() {
                    f(&mut b);
                }
                if let Some(mut b) = if_stmt.else_block.try_lock() {
                    f(&mut b);
                }
            }
            Statement::While(while_stmt) => {
                if let Some(mut b) = while_stmt.block.try_lock() {
                    f(&mut b);
                }
            }
            Statement::Repeat(repeat_stmt) => {
                if let Some(mut b) = repeat_stmt.block.try_lock() {
                    f(&mut b);
                }
            }
            Statement::NumericFor(for_stmt) => {
                if let Some(mut b) = for_stmt.block.try_lock() {
                    f(&mut b);
                }
            }
            Statement::GenericFor(for_stmt) => {
                if let Some(mut b) = for_stmt.block.try_lock() {
                    f(&mut b);
                }
            }
            _ => {}
        }
    }
}

fn collect_locals(block: &Block, locals: &mut FxHashSet<RcLocal>) {
    for statement in block.iter() {
        match statement {
            Statement::Assign(assign) => {
                if assign.prefix {
                    for lvalue in &assign.left {
                        if let LValue::Local(local) = lvalue {
                            locals.insert(local.clone());
                        }
                    }
                }
            }
            Statement::NumericFor(for_stmt) => {
                locals.insert(for_stmt.counter.clone());
            }
            Statement::GenericFor(for_stmt) => {
                for local in &for_stmt.res_locals {
                    locals.insert(local.clone());
                }
            }
            _ => {}
        }
        if let Statement::Assign(assign) = statement {
            for rvalue in &assign.right {
                if let RValue::Closure(closure) = rvalue {
                    if let Some(function) = closure.function.try_lock() {
                        for param in &function.parameters {
                            locals.insert(param.clone());
                        }
                        collect_locals(&function.body, locals);
                    }
                }
            }
        }
    }
    for_nested(block, &mut |b| collect_locals(b, locals));
}

fn collect_used_locals(block: &Block, used: &mut FxHashSet<RcLocal>) {
    for statement in block.iter() {
        for local in statement.values_read() {
            used.insert(local.clone());
        }
        if let Statement::Assign(assign) = statement {
            for rvalue in &assign.right {
                if let RValue::Closure(closure) = rvalue {
                    if let Some(function) = closure.function.try_lock() {
                        collect_used_locals(&function.body, used);
                    }
                }
            }
        }
    }
    for_nested(block, &mut |b| collect_used_locals(b, used));
}

fn rename_unused(block: &mut Block, unused: &FxHashSet<RcLocal>) {
    for statement in block.iter_mut() {
        if let Statement::Assign(assign) = statement {
            if assign.prefix {
                for lvalue in &mut assign.left {
                    if let LValue::Local(local) = lvalue {
                        if unused.contains(local) {
                            local.0 .0.lock().0 = Some("_".to_string());
                        }
                    }
                }
            }
        }
        match statement {
            Statement::NumericFor(for_stmt) => {
                if unused.contains(&for_stmt.counter) {
                    for_stmt.counter.0 .0.lock().0 = Some("_".to_string());
                }
            }
            Statement::GenericFor(for_stmt) => {
                for local in &mut for_stmt.res_locals {
                    if unused.contains(local) {
                        local.0 .0.lock().0 = Some("_".to_string());
                    }
                }
            }
            _ => {}
        }
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let itertools::Either::Right(RValue::Closure(closure)) = value {
                if let Some(mut function) = closure.function.try_lock() {
                    for param in &mut function.parameters {
                        if unused.contains(param) {
                            param.0 .0.lock().0 = Some("_".to_string());
                        }
                    }
                    rename_unused(&mut function.body, unused);
                }
            }
            None
        });
    }
    for_nested_mut(block, &mut |b| rename_unused(b, unused));
}
