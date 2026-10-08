use std::collections::HashMap;

use itertools::Either;

use crate::{Block, LocalRw, RValue, RcLocal, Statement, Traverse};

pub fn replace_locals<H: std::hash::BuildHasher>(
    block: &mut Block,
    map: &HashMap<RcLocal, RcLocal, H>,
) {
    for statement in &mut block.0 {
        for local in statement.values_read_mut() {
            if let Some(new_local) = map.get(local) {
                *local = new_local.clone();
            }
        }
        for local in statement.values_written_mut() {
            if let Some(new_local) = map.get(local) {
                *local = new_local.clone();
            }
        }
        
        statement.post_traverse_values(&mut |value| -> Option<()> {
            if let Either::Right(RValue::Closure(closure)) = value {
                if let Some(mut function) = closure.function.try_lock() {
                    replace_locals(&mut function.body, map)
                }
            };
            None
        });
        match statement {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    replace_locals(&mut b, map);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    replace_locals(&mut b, map);
                }
            }
            Statement::While(r#while) => {
                if let Some(mut b) = r#while.block.try_lock() {
                    replace_locals(&mut b, map);
                }
            }
            Statement::Repeat(repeat) => {
                if let Some(mut b) = repeat.block.try_lock() {
                    replace_locals(&mut b, map);
                }
            }
            Statement::NumericFor(numeric_for) => {
                if let Some(mut b) = numeric_for.block.try_lock() {
                    replace_locals(&mut b, map);
                }
            }
            Statement::GenericFor(generic_for) => {
                if let Some(mut b) = generic_for.block.try_lock() {
                    replace_locals(&mut b, map);
                }
            }
            _ => {}
        }
    }
}
