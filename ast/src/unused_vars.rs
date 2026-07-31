//! Unused Variable Detection and Renaming
//!
//! Detects unused local variables and function parameters, renaming them to `_`
//! following Lua conventions.

use crate::{Block, Statement, LValue, RValue, RcLocal, LocalRw, Traverse};
use rustc_hash::FxHashSet;

/// Mark unused variables with `_`
pub fn mark_unused_variables(block: &mut Block) {
    // Collect all local variables
    let mut all_locals: FxHashSet<RcLocal> = FxHashSet::default();
    collect_locals(block, &mut all_locals);
    
    // Collect all used locals (those that are read)
    let mut used_locals: FxHashSet<RcLocal> = FxHashSet::default();
    collect_used_locals(block, &mut used_locals);
    
    // Find unused locals
    let unused_locals: FxHashSet<RcLocal> = all_locals
        .difference(&used_locals)
        .cloned()
        .collect();
    
    // Rename unused locals to "_"
    rename_unused(block, &unused_locals);
}

/// Collect all local variables defined in the block
fn collect_locals(block: &Block, locals: &mut FxHashSet<RcLocal>) {
    for statement in block.iter() {
        match statement {
            Statement::Assign(assign) => {
                if assign.prefix {
                    // Local declaration
                    for lvalue in &assign.left {
                        if let LValue::Local(local) = lvalue {
                            locals.insert(local.clone());
                        }
                    }
                }
            }
            Statement::If(if_stmt) => {
                collect_locals(&if_stmt.then_block.lock(), locals);
                collect_locals(&if_stmt.else_block.lock(), locals);
            }
            Statement::While(while_stmt) => {
                collect_locals(&while_stmt.block.lock(), locals);
            }
            Statement::Repeat(repeat_stmt) => {
                collect_locals(&repeat_stmt.block.lock(), locals);
            }
            Statement::NumericFor(for_stmt) => {
                locals.insert(for_stmt.counter.clone());
                collect_locals(&for_stmt.block.lock(), locals);
            }
            Statement::GenericFor(for_stmt) => {
                for local in &for_stmt.res_locals {
                    locals.insert(local.clone());
                }
                collect_locals(&for_stmt.block.lock(), locals);
            }
            _ => {}
        }
        
        // Also collect from closures - need to use interior mutability
        if let Statement::Assign(assign) = statement {
            for rvalue in &assign.right {
                if let RValue::Closure(closure) = rvalue {
                    let function = closure.function.lock();
                    for param in &function.parameters {
                        locals.insert(param.clone());
                    }
                    collect_locals(&function.body, locals);
                }
            }
        }
    }
}

/// Collect all local variables that are actually used (read)
fn collect_used_locals(block: &Block, used: &mut FxHashSet<RcLocal>) {
    for statement in block.iter() {
        // Collect all locals that are read
        for local in statement.values_read() {
            used.insert(local.clone());
        }
        
        // Recursively process nested blocks
        match statement {
            Statement::If(if_stmt) => {
                collect_used_locals(&if_stmt.then_block.lock(), used);
                collect_used_locals(&if_stmt.else_block.lock(), used);
            }
            Statement::While(while_stmt) => {
                collect_used_locals(&while_stmt.block.lock(), used);
            }
            Statement::Repeat(repeat_stmt) => {
                collect_used_locals(&repeat_stmt.block.lock(), used);
            }
            Statement::NumericFor(for_stmt) => {
                collect_used_locals(&for_stmt.block.lock(), used);
            }
            Statement::GenericFor(for_stmt) => {
                collect_used_locals(&for_stmt.block.lock(), used);
            }
            _ => {}
        }
        
        // Also collect from closures - need to use interior mutability
        if let Statement::Assign(assign) = statement {
            for rvalue in &assign.right {
                if let RValue::Closure(closure) = rvalue {
                    let function = closure.function.lock();
                    collect_used_locals(&function.body, used);
                }
            }
        }
    }
}

/// Rename unused locals to "_"
fn rename_unused(block: &mut Block, unused: &FxHashSet<RcLocal>) {
    for statement in block.iter_mut() {
        // Rename in local declarations
        if let Statement::Assign(assign) = statement {
            if assign.prefix {
                for lvalue in &mut assign.left {
                    if let LValue::Local(local) = lvalue {
                        if unused.contains(local) {
                            let mut lock = local.0 .0.lock();
                            lock.0 = Some("_".to_string());
                        }
                    }
                }
            }
        }
        
        // Recursively process nested blocks
        match statement {
            Statement::If(if_stmt) => {
                rename_unused(&mut if_stmt.then_block.lock(), unused);
                rename_unused(&mut if_stmt.else_block.lock(), unused);
            }
            Statement::While(while_stmt) => {
                rename_unused(&mut while_stmt.block.lock(), unused);
            }
            Statement::Repeat(repeat_stmt) => {
                rename_unused(&mut repeat_stmt.block.lock(), unused);
            }
            Statement::NumericFor(for_stmt) => {
                if unused.contains(&for_stmt.counter) {
                    let mut lock = for_stmt.counter.0 .0.lock();
                    lock.0 = Some("_".to_string());
                }
                rename_unused(&mut for_stmt.block.lock(), unused);
            }
            Statement::GenericFor(for_stmt) => {
                for local in &mut for_stmt.res_locals {
                    if unused.contains(local) {
                        let mut lock = local.0 .0.lock();
                        lock.0 = Some("_".to_string());
                    }
                }
                rename_unused(&mut for_stmt.block.lock(), unused);
            }
            _ => {}
        }
        
        // Also rename in closures
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let itertools::Either::Right(RValue::Closure(closure)) = value {
                let mut function = closure.function.lock();
                for param in &mut function.parameters {
                    if unused.contains(param) {
                        let mut lock = param.0 .0.lock();
                        lock.0 = Some("_".to_string());
                    }
                }
                rename_unused(&mut function.body, unused);
            }
            None
        });
    }
}
