//! Fold single-assignment copies `a = b` so Luau does not spend a
//! register on each SSA clone. ClientRenderer overflowed the 200-local
//! cap because destruct invented a fresh name per phi.

use std::collections::HashMap;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    replace_locals::replace_locals, Block, LocalRw, RValue, RcLocal, Statement, Traverse,
};

pub fn fold_copy_locals(block: &mut Block) {
    fold_copy_locals_with(block, &FxHashSet::default());
}

/// `upvalues` are this function's captured outer locals. Never fold a
/// copy into them — CameraShaker `Update` initializes two accumulators
/// from a module-level zero; folding both into that upvalue made
/// `v27` nil and `v_u32 +=` mutate the shared zero.
pub fn fold_copy_locals_with(block: &mut Block, upvalues: &FxHashSet<RcLocal>) {
    fold_nested_closures(block);
    fold_one_function(block, upvalues);
}

fn fold_nested_closures(block: &mut Block) {
    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rv| {
            if let RValue::Closure(c) = rv {
                let ups: FxHashSet<_> = c
                    .upvalues
                    .iter()
                    .map(|u| match u {
                        crate::Upvalue::Copy(l) | crate::Upvalue::Ref(l) => l.clone(),
                    })
                    .collect();
                if let Some(mut f) = c.function.try_lock() {
                    fold_copy_locals_with(&mut f.body, &ups);
                }
            }
        });
        match stat {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    fold_nested_closures(&mut b);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    fold_nested_closures(&mut b);
                }
            }
            Statement::While(w) => {
                if let Some(mut b) = w.block.try_lock() {
                    fold_nested_closures(&mut b);
                }
            }
            Statement::Repeat(r) => {
                if let Some(mut b) = r.block.try_lock() {
                    fold_nested_closures(&mut b);
                }
            }
            Statement::NumericFor(n) => {
                if let Some(mut b) = n.block.try_lock() {
                    fold_nested_closures(&mut b);
                }
            }
            Statement::GenericFor(g) => {
                if let Some(mut b) = g.block.try_lock() {
                    fold_nested_closures(&mut b);
                }
            }
            _ => {}
        }
    }
}

fn fold_one_function(block: &mut Block, upvalues: &FxHashSet<RcLocal>) {
    let mut captured = collect_captured(block);
    captured.extend(upvalues.iter().cloned());
    let mut writes = FxHashMap::default();
    let mut write_at = FxHashMap::default();
    let mut index = 0u32;
    count_writes(block, &mut writes, &mut write_at, &mut index);

    let mut map: HashMap<RcLocal, RcLocal> = HashMap::new();
    index = 0;
    collect_copies(block, &writes, &write_at, &captured, &mut map, &mut index);
    if map.is_empty() {
        return;
    }
    follow_map(&mut map);
    replace_locals(block, &map);
    strip_identity_assigns(block);
}

fn collect_captured(block: &mut Block) -> FxHashSet<RcLocal> {
    let mut out = FxHashSet::default();
    collect_captured_in(block, &mut out);
    out
}

fn collect_captured_in(block: &mut Block, out: &mut FxHashSet<RcLocal>) {
    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rv| {
            if let RValue::Closure(c) = rv {
                for u in &c.upvalues {
                    match u {
                        crate::Upvalue::Copy(l) | crate::Upvalue::Ref(l) => {
                            out.insert(l.clone());
                        }
                    }
                }
            }
        });
        match stat {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
            }
            Statement::While(w) => {
                if let Some(mut b) = w.block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
            }
            Statement::Repeat(r) => {
                if let Some(mut b) = r.block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
            }
            Statement::NumericFor(n) => {
                if let Some(mut b) = n.block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
            }
            Statement::GenericFor(g) => {
                if let Some(mut b) = g.block.try_lock() {
                    collect_captured_in(&mut b, out);
                }
            }
            _ => {}
        }
    }
}

fn count_writes(
    block: &Block,
    writes: &mut FxHashMap<RcLocal, u32>,
    write_at: &mut FxHashMap<RcLocal, u32>,
    index: &mut u32,
) {
    for stat in &block.0 {
        let i = *index;
        *index += 1;
        for l in stat.values_written() {
            *writes.entry(l.clone()).or_insert(0) += 1;
            write_at.entry(l.clone()).or_insert(i);
        }
        match stat {
            Statement::If(r#if) => {
                if let Some(b) = r#if.then_block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
                if let Some(b) = r#if.else_block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
            }
            Statement::While(w) => {
                if let Some(b) = w.block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
            }
            Statement::Repeat(r) => {
                if let Some(b) = r.block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
            }
            Statement::NumericFor(n) => {
                if let Some(b) = n.block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
            }
            Statement::GenericFor(g) => {
                if let Some(b) = g.block.try_lock() {
                    count_writes(&b, writes, write_at, index);
                }
            }
            _ => {}
        }
    }
}

fn collect_copies(
    block: &Block,
    writes: &FxHashMap<RcLocal, u32>,
    write_at: &FxHashMap<RcLocal, u32>,
    captured: &FxHashSet<RcLocal>,
    map: &mut HashMap<RcLocal, RcLocal>,
    index: &mut u32,
) {
    for stat in &block.0 {
        let i = *index;
        *index += 1;
        if let Statement::Assign(a) = stat {
            if a.compound_op.is_none() && a.left.len() == 1 && a.right.len() == 1 {
                if let (Some(dst), Some(src)) = (a.left[0].as_local(), a.right[0].as_local()) {
                    if dst != src
                        && !captured.contains(dst)
                        && !captured.contains(src)
                        && writes.get(dst).copied().unwrap_or(0) == 1
                    {
                        let src_w = writes.get(src).copied().unwrap_or(0);
                        let src_ok = src_w == 0
                            || (src_w == 1 && write_at.get(src).copied().unwrap_or(u32::MAX) < i);
                        if src_ok {
                            map.insert(dst.clone(), src.clone());
                        }
                    }
                }
            }
        }
        match stat {
            Statement::If(r#if) => {
                if let Some(b) = r#if.then_block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
                if let Some(b) = r#if.else_block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
            }
            Statement::While(w) => {
                if let Some(b) = w.block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
            }
            Statement::Repeat(r) => {
                if let Some(b) = r.block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
            }
            Statement::NumericFor(n) => {
                if let Some(b) = n.block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
            }
            Statement::GenericFor(g) => {
                if let Some(b) = g.block.try_lock() {
                    collect_copies(&b, writes, write_at, captured, map, index);
                }
            }
            _ => {}
        }
    }
}

fn follow_map(map: &mut HashMap<RcLocal, RcLocal>) {
    let keys: Vec<_> = map.keys().cloned().collect();
    for k in keys {
        let mut hops = 0u32;
        while let Some(n) = map.get(&k).cloned() {
            hops += 1;
            if hops > 64 {
                break;
            }
            if let Some(n2) = map.get(&n).cloned() {
                if n2 == k {
                    break;
                }
                map.insert(k.clone(), n2);
            } else {
                break;
            }
        }
    }
}

fn strip_identity_assigns(block: &mut Block) {
    block.0.retain(|s| {
        if let Statement::Assign(a) = s {
            if a.compound_op.is_none() && a.left.len() == 1 && a.right.len() == 1 {
                if let (Some(l), Some(r)) = (a.left[0].as_local(), a.right[0].as_local()) {
                    if l == r {
                        return false;
                    }
                }
            }
        }
        true
    });
    for stat in &mut block.0 {
        match stat {
            Statement::If(r#if) => {
                if let Some(mut b) = r#if.then_block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
                if let Some(mut b) = r#if.else_block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
            }
            Statement::While(w) => {
                if let Some(mut b) = w.block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
            }
            Statement::Repeat(r) => {
                if let Some(mut b) = r.block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
            }
            Statement::NumericFor(n) => {
                if let Some(mut b) = n.block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
            }
            Statement::GenericFor(g) => {
                if let Some(mut b) = g.block.try_lock() {
                    strip_identity_assigns(&mut b);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Binary, BinaryOperation, If, Literal, Local};

    #[test]
    fn folds_ssa_copy_into_source() {
        let request2 = RcLocal::new(Local::new(Some("request2".into())));
        let v1779 = RcLocal::new(Local::new(Some("v1779".into())));
        let mut body = Block(vec![
            Assign::new(
                vec![request2.clone().into()],
                vec![Literal::String(b"x".to_vec()).into()],
            )
            .into(),
            Assign::new(
                vec![v1779.clone().into()],
                vec![request2.clone().into()],
            )
            .into(),
            Statement::If(If::new(
                Binary::new(
                    v1779.clone().into(),
                    Literal::String(b"DL1Flip".to_vec()).into(),
                    BinaryOperation::Equal,
                )
                .into(),
                Block::default(),
                Block::default(),
            )),
        ]);
        fold_copy_locals(&mut body);
        let s = body.to_string();
        assert!(s.contains("request2"), "{s}");
        assert!(!s.contains("v1779"), "copy should be folded:\n{s}");
    }

    #[test]
    fn does_not_fold_across_later_source_assign() {
        let b = RcLocal::new(Local::new(Some("b".into())));
        let a = RcLocal::new(Local::new(Some("a".into())));
        let mut body = Block(vec![
            Assign::new(vec![a.clone().into()], vec![b.clone().into()]).into(),
            Assign::new(
                vec![b.clone().into()],
                vec![Literal::Number(1.0).into()],
            )
            .into(),
            crate::Return::new(vec![a.clone().into()]).into(),
        ]);
        fold_copy_locals(&mut body);
        let s = body.to_string();
        assert!(
            s.contains("a") && s.contains("return a"),
            "must keep a holding the old b:\n{s}"
        );
    }

    #[test]
    fn folds_inside_if_body() {
        let request2 = RcLocal::new(Local::new(Some("request2".into())));
        let v = RcLocal::new(Local::new(Some("v".into())));
        let inner = Block(vec![
            Assign::new(vec![v.clone().into()], vec![request2.clone().into()]).into(),
            crate::Return::new(vec![v.clone().into()]).into(),
        ]);
        let mut body = Block(vec![
            Assign::new(
                vec![request2.clone().into()],
                vec![Literal::String(b"x".to_vec()).into()],
            )
            .into(),
            Statement::If(If::new(
                Literal::Boolean(true).into(),
                inner,
                Block::default(),
            )),
        ]);
        fold_copy_locals(&mut body);
        let s = body.to_string();
        assert!(s.contains("return request2"), "{s}");
        assert!(!s.contains("return v"), "{s}");
    }

    #[test]
    fn does_not_fold_copy_of_upvalue() {
        let zero = RcLocal::new(Local::new(Some("v_u32".into())));
        let rot = RcLocal::new(Local::new(Some("v27".into())));
        let mut body = Block(vec![
            Assign::new(vec![rot.clone().into()], vec![zero.clone().into()]).into(),
            crate::Return::new(vec![rot.clone().into()]).into(),
        ]);
        let mut ups = FxHashSet::default();
        ups.insert(zero);
        fold_copy_locals_with(&mut body, &ups);
        let s = body.to_string();
        assert!(
            s.contains("v27"),
            "accumulator must stay distinct from module zero:\n{s}"
        );
    }
}
