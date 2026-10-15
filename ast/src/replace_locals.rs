use std::collections::HashMap;

use itertools::Either;

use crate::{Block, LocalRw, RValue, RcLocal, Statement, Traverse};

/// Closure bodies `replace_locals` could not lock and so did not visit.
/// A skipped body keeps its old names, so it is a debug signal (`[ab]`).
pub static LOCKED_SKIPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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
                match closure.function.try_lock() {
                    Some(mut function) => replace_locals(&mut function.body, map),
                    None => {
                        LOCKED_SKIPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
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
