//! Table Constructor Cleanup
//!
//! Detects and fixes duplicate keys in table constructors by splitting them
//! into separate assignment statements. Also folds `t.key = v` after
//! `t = { key = nil, ... }` back into the constructor (DUPTABLE + SETTABLEKS).

use crate::{
    Assign, Block, Index, LValue, Literal, LocalRw, RValue, SideEffects, Statement, Table, Traverse,
};

fn field_key_matches(key: &RValue, id: &str) -> bool {
    match key {
        RValue::Literal(Literal::String(s)) => {
            format!("str:{}", String::from_utf8_lossy(s)) == id
        }
        RValue::Literal(Literal::Number(n)) => format!("num:{n}") == id,
        RValue::Literal(Literal::Integer(n)) => format!("num:{n}") == id,
        RValue::Literal(Literal::Boolean(b)) => format!("bool:{b}") == id,
        _ => false,
    }
}

/// Fold consecutive `t.key = value` into a preceding `t = { ... }`.
fn fold_field_assigns(block: &mut Block) {
    let mut i = 0;
    while i < block.0.len() {
        let object_local = if let Statement::Assign(assign) = &block.0[i]
            && assign.left.len() == 1
            && assign.right.len() == 1
            && assign.right[0].as_table().is_some()
            && let LValue::Local(object_local) = &assign.left[0]
        {
            Some(object_local.clone())
        } else {
            None
        };
        if let Some(object_local) = object_local {
            let mut j = i + 1;
            while j < block.0.len() {
                let can_fold = matches!(
                    &block.0[j],
                    Statement::Assign(field_assign)
                        if field_assign.left.len() == 1
                            && field_assign.right.len() == 1
                            && matches!(
                                &field_assign.left[0],
                                LValue::Index(Index { left, .. })
                                    if matches!(left.as_ref(), RValue::Local(local) if local == &object_local)
                            )
                            && !field_assign.right[0].values_read().contains(&&object_local)
                );
                if !can_fold {
                    break;
                }
                let field_assign = block.0.remove(j).into_assign().unwrap();
                let key = *field_assign
                    .left
                    .into_iter()
                    .next()
                    .unwrap()
                    .into_index()
                    .unwrap()
                    .right;
                let value = field_assign.right.into_iter().next().unwrap();
                block.0[i]
                    .as_assign_mut()
                    .unwrap()
                    .right[0]
                    .as_table_mut()
                    .unwrap()
                    .put_field(key, value);
            }
            if let Statement::Assign(assign) = &mut block.0[i]
                && let RValue::Table(table) = &mut assign.right[0]
            {
                table.drop_nil_fields();
            }
        }

        match &mut block.0[i] {
            Statement::If(if_stmt) => {
                fold_field_assigns(&mut if_stmt.then_block.lock());
                fold_field_assigns(&mut if_stmt.else_block.lock());
            }
            Statement::While(while_stmt) => {
                fold_field_assigns(&mut while_stmt.block.lock());
            }
            Statement::Repeat(repeat_stmt) => {
                fold_field_assigns(&mut repeat_stmt.block.lock());
            }
            Statement::NumericFor(for_stmt) => {
                fold_field_assigns(&mut for_stmt.block.lock());
            }
            Statement::GenericFor(for_stmt) => {
                fold_field_assigns(&mut for_stmt.block.lock());
            }
            _ => {}
        }
        i += 1;
    }
}

/// Clean up table constructors by removing duplicate keys
pub fn cleanup_table_constructors(block: &mut Block) {
    fold_field_assigns(block);
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

        // Deduplicate any table constructors nested elsewhere in this
        // statement's expressions — e.g. as a call argument
        // (`create({ { ..., BackgroundColor3 = nil, ..., BackgroundColor3 =
        // Color3.new(...) } })`), a binary/unary operand, or nested inside
        // another table's value position. These aren't bound to their own
        // lvalue the way a top-level `x = { ... }` is, so they can't be
        // split into a separate statement like `split_table_with_duplicates`
        // does above. Since they're pure data literals with no side
        // effects, we just keep the last occurrence of each duplicate key
        // and drop the earlier (shadowed) one in place. This walks the
        // full expression tree via `Traverse`, so it reaches any depth of
        // nesting, not just tables directly assigned at the top level.
        block.0[i].traverse_rvalues(&mut |rvalue| {
            if let RValue::Table(table) = rvalue {
                dedupe_table_own_keys(table);
                table.drop_nil_fields();
            }
        });

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

/// Deduplicate a table's own top-level keys in place, keeping the last
/// occurrence of each duplicate and dropping earlier (shadowed) ones. Does
/// not recurse — callers reach nested tables via `Traverse::traverse_rvalues`,
/// which invokes this on every table found at any depth in an expression
/// tree.
fn dedupe_table_own_keys(table: &mut Table) {
    let mut seen_keys = std::collections::HashSet::new();
    let mut keep = vec![true; table.0.len()];
    for i in (0..table.0.len()).rev() {
        if let Some(key) = &table.0[i].0 {
            let key_str = match key {
                RValue::Literal(Literal::String(s)) => {
                    Some(format!("str:{}", String::from_utf8_lossy(s)))
                }
                RValue::Literal(Literal::Number(n)) => Some(format!("num:{}", n)),
                RValue::Literal(Literal::Boolean(b)) => Some(format!("bool:{}", b)),
                _ => None,
            };
            if let Some(key_str) = key_str {
                if !seen_keys.insert(key_str) {
                    keep[i] = false;
                }
            }
        }
    }

    let mut idx = 0;
    table.0.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
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

            if !seen_keys.insert(key_str.clone()) {
                // DUPTABLE placeholder: keep the later real value in the
                // constructor instead of splitting `t.key = value` back out.
                if let Some((_, slot)) = clean_entries.iter_mut().find(|(k, _)| {
                    matches!(k, Some(prev) if field_key_matches(prev, &key_str))
                }) {
                    if matches!(slot, RValue::Literal(Literal::Nil)) || !slot.has_side_effects()
                    {
                        *slot = value;
                        continue;
                    }
                }
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
    let mut cleaned = Table(clean_entries);
    cleaned.drop_nil_fields();
    let mut new_assign = Assign::new(
        vec![table_lvalue],
        vec![RValue::Table(cleaned)],
    );
    new_assign.prefix = assign.prefix; // Preserve the local declaration
    block.0[index] = Statement::Assign(new_assign);

    // Insert the duplicate assignments after the table constructor
    for (i, assignment) in duplicate_assignments.into_iter().enumerate() {
        block.0.insert(index + 1 + i, assignment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Local, RcLocal};

    fn named(n: &str) -> RcLocal {
        RcLocal::new(Local::new(Some(n.into())))
    }

    fn key(s: &str) -> RValue {
        Literal::String(s.as_bytes().to_vec()).into()
    }

    #[test]
    fn folds_settableks_into_duptable_nils() {
        let t = named("t");
        let p = named("p");
        let table = Table(vec![
            (Some(key("_running")), Literal::Boolean(false).into()),
            (Some(key("_renderPriority")), Literal::Nil.into()),
        ]);
        let mut decl = Assign::new(vec![t.clone().into()], vec![table.into()]);
        decl.prefix = true;
        let field = Assign::new(
            vec![LValue::Index(Index::new(
                RValue::Local(t.clone()),
                key("_renderPriority"),
            ))],
            vec![RValue::Local(p)],
        );
        let mut block = Block(vec![decl.into(), field.into()]);
        cleanup_table_constructors(&mut block);
        let printed = block.to_string();
        assert!(
            printed.contains("_renderPriority = p"),
            "expected field folded into constructor: {printed}"
        );
        assert!(
            !printed.contains("t._renderPriority"),
            "should not keep SETTABLEKS: {printed}"
        );
        assert_eq!(block.0.len(), 1, "field assign should be gone: {printed}");
    }
}
