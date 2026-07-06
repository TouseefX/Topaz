//! Table Constructor Cleanup
//!
//! Detects and fixes duplicate keys in table constructors by splitting them
//! into separate assignment statements.

use crate::{Assign, Block, Index, LValue, Literal, RValue, Statement, Table};

/// Clean up table constructors by removing duplicate keys
pub fn cleanup_table_constructors(block: &mut Block) {
    let mut i = 0;
    while i < block.0.len() {
        let should_split = if let Statement::Assign(assign) = &block.0[i] {
            // Check if this is a table constructor assignment
            if assign.left.len() == 1 && assign.right.len() == 1 {
                if let RValue::Table(table) = &assign.right[0] {
                    has_duplicate_keys(table)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_split {
            split_table_with_duplicates(block, i);
        }

        // Recursively process nested blocks
        match &mut block.0[i] {
            Statement::If(if_stmt) => {
                cleanup_table_constructors(&mut if_stmt.then_block.lock());
                cleanup_table_constructors(&mut if_stmt.else_block.lock());
            }
            Statement::While(while_stmt) => {
                cleanup_table_constructors(&mut while_stmt.block.lock());
            }
            Statement::Repeat(repeat_stmt) => {
                cleanup_table_constructors(&mut repeat_stmt.block.lock());
            }
            Statement::NumericFor(for_stmt) => {
                cleanup_table_constructors(&mut for_stmt.block.lock());
            }
            Statement::GenericFor(for_stmt) => {
                cleanup_table_constructors(&mut for_stmt.block.lock());
            }
            _ => {}
        }

        i += 1;
    }
}

/// Check if a table has duplicate keys
fn has_duplicate_keys(table: &Table) -> bool {
    use std::collections::HashSet;
    let mut seen_keys = HashSet::new();
    
    for (key, _) in &table.0 {
        if let Some(key) = key {
            // Create a string representation of the key for comparison
            let key_str = match key {
                RValue::Literal(Literal::String(s)) => format!("str:{}", String::from_utf8_lossy(s)),
                RValue::Literal(Literal::Number(n)) => format!("num:{}", n),
                RValue::Literal(Literal::Boolean(b)) => format!("bool:{}", b),
                _ => continue, // Skip non-literal keys
            };
            
            if !seen_keys.insert(key_str) {
                return true; // Duplicate found
            }
        }
    }
    
    false
}

/// Split a table constructor with duplicate keys into separate statements
fn split_table_with_duplicates(block: &mut Block, index: usize) {
    let assign = if let Statement::Assign(assign) = block.0[index].clone() {
        assign
    } else {
        return;
    };

    let table_lvalue = assign.left[0].clone();
    let table = if let RValue::Table(table) = &assign.right[0] {
        table.clone()
    } else {
        return;
    };

    // Track which keys we've seen
    use std::collections::HashSet;
    let mut seen_keys = HashSet::new();
    let mut clean_entries: Vec<(Option<RValue>, RValue)> = Vec::new();
    let mut duplicate_assignments = Vec::new();

    for (key, value) in table.0 {
        if let Some(key) = &key {
            let key_str = match key {
                RValue::Literal(Literal::String(s)) => format!("str:{}", String::from_utf8_lossy(s)),
                RValue::Literal(Literal::Number(n)) => format!("num:{}", n),
                RValue::Literal(Literal::Boolean(b)) => format!("bool:{}", b),
                _ => {
                    clean_entries.push((Some(key.clone()), value.clone()));
                    continue;
                }
            };

            if !seen_keys.insert(key_str) {
                // This is a duplicate - create a separate assignment
                let index_expr = Index::new(
                    match &table_lvalue {
                        LValue::Local(l) => RValue::Local(l.clone()),
                        LValue::Global(g) => RValue::Global(g.clone()),
                        LValue::Index(i) => RValue::Index(i.clone()),
                    },
                    key.clone(),
                );
                
                duplicate_assignments.push(Statement::Assign(Assign::new(
                    vec![LValue::Index(index_expr)],
                    vec![value.clone()],
                )));
            } else {
                clean_entries.push((Some(key.clone()), value.clone()));
            }
        } else {
            clean_entries.push((None, value.clone()));
        }
    }

    // Replace the original statement with the cleaned table
    let mut new_assign = Assign::new(
        vec![table_lvalue],
        vec![RValue::Table(Table(clean_entries))],
    );
    new_assign.prefix = assign.prefix; // Preserve the local declaration
    block.0[index] = Statement::Assign(new_assign);

    // Insert the duplicate assignments after the table constructor
    for (i, assignment) in duplicate_assignments.into_iter().enumerate() {
        block.0.insert(index + 1 + i, assignment);
    }
}
