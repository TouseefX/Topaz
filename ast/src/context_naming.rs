//! Context-based Variable Naming
//!
//! Implements Oracle-style naming by extracting names from:
//! - GetService("ServiceName") → c1_ServiceName
//! - String literals in function calls
//! - Context-based heuristics
//!
//! This does NOT use debug info - it infers names from the code itself.

use crate::{Block, Statement, Assign, LValue, RValue, Call, Index, Literal, Traverse};
use std::collections::HashMap;

/// Apply context-based naming to variables
pub fn apply_context_naming(block: &mut Block) {
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    
    for statement in block.iter_mut() {
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let itertools::Either::Right(RValue::Closure(closure)) = value {
                apply_context_naming(&mut closure.function.lock().body);
            };
            None
        });
        process_statement(statement, &mut name_counts);
    }
}

fn process_statement(statement: &mut Statement, name_counts: &mut HashMap<String, usize>) {
    match statement {
        Statement::Assign(assign) => {
            try_extract_name_from_assign(assign, name_counts);
        }
        Statement::If(if_stmt) => {
            apply_context_naming(&mut if_stmt.then_block.lock());
            apply_context_naming(&mut if_stmt.else_block.lock());
        }
        Statement::While(while_stmt) => {
            apply_context_naming(&mut while_stmt.block.lock());
        }
        Statement::Repeat(repeat_stmt) => {
            apply_context_naming(&mut repeat_stmt.block.lock());
        }
        Statement::NumericFor(for_stmt) => {
            apply_context_naming(&mut for_stmt.block.lock());
        }
        Statement::GenericFor(for_stmt) => {
            apply_context_naming(&mut for_stmt.block.lock());
        }
        _ => {}
    }
}

/// Try to extract a meaningful name from an assignment
fn try_extract_name_from_assign(assign: &mut Assign, name_counts: &mut HashMap<String, usize>) {
    // Only process single assignments
    if assign.left.len() != 1 || assign.right.len() != 1 {
        return;
    }
    
    // Only process local variable assignments
    let target_local = match &assign.left[0] {
        LValue::Local(local) => local,
        _ => return,
    };
    
    // Try to extract name from the right-hand side
    if let Some(base_name) = extract_name_from_rvalue(&assign.right[0]) {
        // Generate a unique name with counter
        let count = name_counts.entry(base_name.clone()).or_insert(0);
        *count += 1;
        let new_name = if *count == 1 {
            base_name.clone()
        } else {
            format!("{}_{}", base_name, count)
        };
        
        // Apply the name to the local variable
        let mut lock = target_local.0 .0.lock();
        if lock.0.is_none() || lock.0.as_ref().map(|s| crate::name_locals::is_synthetic_name(s)).unwrap_or(true) {
            lock.0 = Some(new_name);
        }
    }
}

/// Extract a meaningful name from an RValue
fn extract_name_from_rvalue(rvalue: &RValue) -> Option<String> {
    match rvalue {
        // Handle function calls like GetService("ReplicatedStorage")
        RValue::Call(call) => extract_name_from_call(call),
        
        // Handle method calls like game:GetService("ReplicatedStorage")
        RValue::MethodCall(method_call) => {
            // Check if it's GetService
            if method_call.method == "GetService" {
                if let Some(RValue::Literal(Literal::String(service_name))) = method_call.arguments.first() {
                    // Convert to valid identifier
                    let name = String::from_utf8_lossy(service_name).to_string();
                    return Some(sanitize_identifier(&name));
                }
            }
            None
        }
        
        // Handle indexing like game.Workspace
        RValue::Index(index) => extract_name_from_index(index),
        
        _ => None,
    }
}

/// Extract name from a function call
fn extract_name_from_call(call: &Call) -> Option<String> {
    // Check if calling a method like GetService
    if let RValue::Index(index) = &*call.value {
        if let RValue::Literal(Literal::String(method_name)) = &*index.right {
            let method = String::from_utf8_lossy(method_name).to_string();
            
            // Handle GetService("Name")
            if method == "GetService" {
                if let Some(RValue::Literal(Literal::String(service_name))) = call.arguments.first() {
                    let name = String::from_utf8_lossy(service_name).to_string();
                    return Some(sanitize_identifier(&name));
                }
            }
            
            // Handle WaitForChild("Name")
            if method == "WaitForChild" || method == "FindFirstChild" {
                if let Some(RValue::Literal(Literal::String(child_name))) = call.arguments.first() {
                    let name = String::from_utf8_lossy(child_name).to_string();
                    return Some(sanitize_identifier(&name));
                }
            }
        }
    }
    
    None
}

/// Extract name from an index expression
fn extract_name_from_index(index: &Index) -> Option<String> {
    // Handle game.Workspace, game.Players, etc.
    if let RValue::Literal(Literal::String(field_name)) = &*index.right {
        let name = String::from_utf8_lossy(field_name).to_string();
        return Some(sanitize_identifier(&name));
    }
    
    None
}

/// Sanitize a string to be a valid Lua identifier
fn sanitize_identifier(name: &str) -> String {
    let mut result = String::new();
    
    for (i, ch) in name.chars().enumerate() {
        if ch.is_alphanumeric() || ch == '_' {
            // First character can't be a number
            if i == 0 && ch.is_numeric() {
                result.push('_');
            }
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    
    // If empty or starts with number, prefix with underscore
    if result.is_empty() {
        result = "var".to_string();
    } else if result.chars().next().unwrap().is_numeric() {
        result = format!("_{}", result);
    }
    
    result
}
