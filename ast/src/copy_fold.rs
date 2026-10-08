//! Copy coalescing and single-use inlining after SSA destruct.
//!
//! Recovers `return setmetatable({ ... }, CameraShaker)` from
//! `local t = { ... }; return setmetatable(t, mt)`, and drops the
//! loop-carried `v21 = v19` / `v19 = v21` dance in CameraShaker:Update.

use std::collections::{HashMap, HashSet};

use crate::{
    replace_locals::replace_locals, Block, LValue, LocalRw, RValue, RcLocal, Select, SideEffects,
    Statement, Traverse, Upvalue,
};

fn local_copy(stmt: &Statement) -> Option<(RcLocal, RcLocal)> {
    let Statement::Assign(a) = stmt else {
        return None;
    };
    if a.left.len() != 1 || a.right.len() != 1 {
        return None;
    }
    let left = a.left[0].as_local()?.clone();
    let right = a.right[0].as_local()?.clone();
    if left == right {
        return None;
    }
    Some((left, right))
}

fn is_identity(stmt: &Statement) -> bool {
    let Statement::Assign(a) = stmt else {
        return false;
    };
    if a.prefix || a.left.len() != 1 || a.right.len() != 1 {
        return false;
    }
    match (a.left[0].as_local(), a.right[0].as_local()) {
        (Some(l), Some(r)) => l == r,
        _ => false,
    }
}

fn loop_body_arc(stmt: &Statement) -> Option<crate::SharedBlock> {
    match stmt {
        Statement::NumericFor(s) => Some(s.block.clone()),
        Statement::GenericFor(s) => Some(s.block.clone()),
        Statement::While(s) => Some(s.block.clone()),
        Statement::Repeat(s) => Some(s.block.clone()),
        _ => None,
    }
}

fn for_nested_blocks(block: &mut Block, f: &mut impl FnMut(&mut Block)) {
    for stmt in &mut block.0 {
        match stmt {
            Statement::If(s) => {
                f(&mut s.then_block.lock());
                f(&mut s.else_block.lock());
            }
            Statement::While(s) => f(&mut s.block.lock()),
            Statement::Repeat(s) => f(&mut s.block.lock()),
            Statement::NumericFor(s) => f(&mut s.block.lock()),
            Statement::GenericFor(s) => f(&mut s.block.lock()),
            _ => {}
        }
    }
}

fn stmt_writes_temp(stmt: &Statement, temps: &HashSet<RcLocal>) -> bool {
    if stmt.values_written().iter().any(|l| temps.contains(l)) {
        return true;
    }
    match stmt {
        Statement::If(s) => {
            s.then_block
                .lock()
                .iter()
                .any(|t| stmt_writes_temp(t, temps))
                || s.else_block
                    .lock()
                    .iter()
                    .any(|t| stmt_writes_temp(t, temps))
        }
        Statement::While(s) => s.block.lock().iter().any(|t| stmt_writes_temp(t, temps)),
        Statement::Repeat(s) => s.block.lock().iter().any(|t| stmt_writes_temp(t, temps)),
        Statement::NumericFor(s) => s.block.lock().iter().any(|t| stmt_writes_temp(t, temps)),
        Statement::GenericFor(s) => s.block.lock().iter().any(|t| stmt_writes_temp(t, temps)),
        _ => false,
    }
}

fn trailing_copy_count(body: &Block, copies: &[(RcLocal, RcLocal)]) -> usize {
    let mut matched = 0;
    for s in body.0.iter().rev() {
        if matched >= copies.len() {
            break;
        }
        match local_copy(s) {
            Some((a, b)) if copies.iter().any(|(ca, cb)| ca == &a && cb == &b) => {
                matched += 1;
            }
            _ => break,
        }
    }
    matched
}

fn remove_identity(block: &mut Block) {
    for_nested_blocks(block, &mut remove_identity);
    block.0.retain(|s| !is_identity(s));
}

fn fold_loop_carried(block: &mut Block) {
    for_nested_blocks(block, &mut fold_loop_carried);

    let mut i = 0;
    while i < block.0.len() {
        if loop_body_arc(&block.0[i]).is_none() {
            i += 1;
            continue;
        }
        let mut copies = Vec::new();
        let mut k = i;
        while k > 0 {
            if let Some(pair) = local_copy(&block.0[k - 1]) {
                copies.push(pair);
                k -= 1;
            } else {
                break;
            }
        }
        if copies.is_empty() {
            i += 1;
            continue;
        }

        let matched = {
            let body = loop_body_arc(&block.0[i]).unwrap();
            trailing_copy_count(&body.lock(), &copies)
        };
        if matched != copies.len() {
            i += 1;
            continue;
        }

        let temps: HashSet<RcLocal> = copies.iter().map(|(a, _)| a.clone()).collect();
        let extra_write = {
            let body = loop_body_arc(&block.0[i]).unwrap();
            let body = body.lock();
            let n = body.0.len().saturating_sub(matched);
            body.0[..n]
                .iter()
                .any(|s| stmt_writes_temp(s, &temps))
        };
        if extra_write {
            i += 1;
            continue;
        }

        let map: HashMap<RcLocal, RcLocal> = copies.iter().cloned().collect();
        let copies_len = i - k;
        let mut tail = Block(block.0.split_off(i + 1));
        replace_locals(&mut tail, &map);
        if let Some(body) = loop_body_arc(block.0.last().unwrap()) {
            let mut body = body.lock();
            replace_locals(&mut body, &map);
            let new_len = body.0.len().saturating_sub(matched);
            body.0.truncate(new_len);
        }
        block.0.append(&mut tail.0);
        block.0.drain(k..k + copies_len);
        i = k + 1;
    }
}

fn lvalue_object_is(stmt: &Statement, local: &RcLocal) -> bool {
    let Statement::Assign(a) = stmt else {
        return false;
    };
    a.left.iter().any(|lv| match lv {
        LValue::Local(l) => l == local,
        LValue::Index(idx) => idx.left.values_read().iter().any(|l| *l == local),
        _ => false,
    })
}

fn replace_local_in_stmt(stmt: &mut Statement, from: &RcLocal, to: RValue) {
    stmt.traverse_rvalues(&mut |rv| {
        if let RValue::Local(l) = rv {
            if l == from {
                *rv = to.clone();
            }
        }
    });
}

/// `Closure::traverse_rvalues` is empty, so replacing a `Local` rvalue never
/// rewrites a captured upvalue. Inlining `_callback = p._callback` into
/// `BindToRenderStep(..., function() _callback(...) end)` then deleting the
/// assign left CameraShaker printing `UNNAMED_120(v7)`.
fn rvalue_captures_upvalue(rv: &RValue, local: &RcLocal) -> bool {
    if let RValue::Closure(c) = rv {
        if c.upvalues.iter().any(|u| match u {
            Upvalue::Copy(l) | Upvalue::Ref(l) => l == local,
        }) {
            return true;
        }
    }
    rv.rvalues()
        .iter()
        .any(|inner| rvalue_captures_upvalue(inner, local))
}

fn stmt_captures_upvalue(stmt: &Statement, local: &RcLocal) -> bool {
    stmt.rvalues()
        .iter()
        .any(|rv| rvalue_captures_upvalue(rv, local))
}

fn assigned_local(stmt: &Statement) -> Option<(RcLocal, RValue)> {
    let Statement::Assign(a) = stmt else {
        return None;
    };
    if a.left.len() != 1 || a.right.len() != 1 {
        return None;
    }
    let local = a.left[0].as_local()?.clone();
    Some((local, a.right[0].clone()))
}

fn collect_read_counts(block: &Block, counts: &mut HashMap<RcLocal, usize>) {
    for s in &block.0 {
        for l in s.values_read() {
            *counts.entry(l.clone()).or_insert(0) += 1;
        }
        match s {
            Statement::If(st) => {
                collect_read_counts(&st.then_block.lock(), counts);
                collect_read_counts(&st.else_block.lock(), counts);
            }
            Statement::While(st) => collect_read_counts(&st.block.lock(), counts),
            Statement::Repeat(st) => collect_read_counts(&st.block.lock(), counts),
            Statement::NumericFor(st) => collect_read_counts(&st.block.lock(), counts),
            Statement::GenericFor(st) => collect_read_counts(&st.block.lock(), counts),
            _ => {}
        }
    }
}

/// O(n) single-use inlining. The previous “scan every other statement
/// for each assign” was O(n²) and made 60k-line dumps take minutes
/// (Oracle finishes those in ~2s).
fn inline_consecutive(block: &mut Block, counts: &HashMap<RcLocal, usize>) -> bool {
    let mut changed = false;
    for_nested_blocks(block, &mut |b| {
        changed |= inline_consecutive(b, counts);
    });
    let mut i = 0;
    while i + 1 < block.0.len() {
        let Some((local, expr)) = assigned_local(&block.0[i]) else {
            i += 1;
            continue;
        };
        if counts.get(&local).copied().unwrap_or(0) != 1 {
            i += 1;
            continue;
        }
        if lvalue_object_is(&block.0[i + 1], &local) {
            i += 1;
            continue;
        }
        let reads_next = block.0[i + 1]
            .values_read()
            .iter()
            .any(|l| *l == &local);
        if !reads_next {
            i += 1;
            continue;
        }
        if stmt_captures_upvalue(&block.0[i + 1], &local) {
            i += 1;
            continue;
        }
        replace_local_in_stmt(&mut block.0[i + 1], &local, expr);
        block.0.remove(i);
        changed = true;
    }
    changed
}

fn merge_decl_assign(block: &mut Block) {
    for_nested_blocks(block, &mut merge_decl_assign);
    let mut i = 0;
    while i + 1 < block.0.len() {
        let merge = match (&block.0[i], &block.0[i + 1]) {
            (Statement::Assign(decl), Statement::Assign(assign))
                if decl.prefix
                    && decl.right.is_empty()
                    && decl.left.len() == 1
                    && assign.left.len() == 1
                    && assign.right.len() == 1
                    && decl.left[0] == assign.left[0] =>
            {
                true
            }
            _ => false,
        };
        if merge {
            let mut assign = block.0.remove(i + 1).into_assign().unwrap();
            assign.prefix = true;
            block.0[i] = assign.into();
        }
        i += 1;
    }
}

fn collect_reads(block: &Block, reads: &mut HashSet<RcLocal>) {
    for s in &block.0 {
        for l in s.values_read() {
            reads.insert(l.clone());
        }
        match s {
            Statement::If(st) => {
                collect_reads(&st.then_block.lock(), reads);
                collect_reads(&st.else_block.lock(), reads);
            }
            Statement::While(st) => collect_reads(&st.block.lock(), reads),
            Statement::Repeat(st) => collect_reads(&st.block.lock(), reads),
            Statement::NumericFor(st) => collect_reads(&st.block.lock(), reads),
            Statement::GenericFor(st) => collect_reads(&st.block.lock(), reads),
            _ => {}
        }
    }
}

fn dce_unused(block: &mut Block, reads: &HashSet<RcLocal>) {
    for_nested_blocks(block, &mut |b| dce_unused(b, reads));
    let mut i = 0;
    while i < block.0.len() {
        let remove_or_replace = if let Statement::Assign(a) = &block.0[i] {
            let all_local = a.left.iter().all(|lv| lv.as_local().is_some());
            let all_unused = all_local
                && a.left
                    .iter()
                    .all(|lv| !reads.contains(lv.as_local().unwrap()));
            if all_unused {
                if a.right.iter().any(|r| r.has_side_effects()) {
                    if a.right.len() == 1 {
                        match &a.right[0] {
                            RValue::Call(c) | RValue::Select(Select::Call(c)) => {
                                Some(Some(c.clone()))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    Some(None)
                }
            } else {
                None
            }
        } else {
            None
        };
        match remove_or_replace {
            Some(Some(call)) => {
                block.0[i] = Statement::Call(call);
                i += 1;
            }
            Some(None) => {
                block.0.remove(i);
            }
            None => i += 1,
        }
    }
}

pub fn apply(block: &mut Block) {
    merge_decl_assign(block);
    fold_loop_carried(block);
    remove_identity(block);
    for _ in 0..8 {
        let mut counts = HashMap::new();
        collect_read_counts(block, &mut counts);
        if !inline_consecutive(block, &counts) {
            break;
        }
        merge_decl_assign(block);
        remove_identity(block);
    }
    let mut reads = HashSet::new();
    collect_reads(block, &mut reads);
    dce_unused(block, &reads);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Block, Call, Literal, Local, NumericFor, RValue, Statement};

    fn named(n: &str) -> RcLocal {
        RcLocal::new(Local::new(Some(n.into())))
    }

    #[test]
    fn inlines_temp_into_assert() {
        let v = named("v");
        let cond = RValue::Literal(Literal::Boolean(true));
        let assert_call = Call::new(
            crate::Global::new(b"assert".to_vec()).into(),
            vec![RValue::Local(v.clone()), Literal::String(b"msg".to_vec()).into()],
        );
        let mut decl = Assign::new(vec![v.into()], vec![cond]);
        decl.prefix = true;
        let mut block = Block(vec![decl.into(), Statement::Call(assert_call)]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            printed.contains("assert(true"),
            "expected inlined assert: {printed}"
        );
        assert!(
            !printed.contains("local v"),
            "temp should be gone: {printed}"
        );
    }

    #[test]
    fn drops_loop_carried_copies() {
        let v19 = named("v19");
        let v21 = named("v21");
        let i = named("i");
        let body = Block(vec![
            Assign::new(
                vec![v19.clone().into()],
                vec![RValue::Local(v21.clone())],
            )
            .into(),
            Assign::new(
                vec![v21.clone().into()],
                vec![RValue::Local(v19.clone())],
            )
            .into(),
        ]);
        let for_stmt = NumericFor::new(
            Literal::Number(1.0).into(),
            Literal::Number(10.0).into(),
            Literal::Number(1.0).into(),
            i,
            body,
        );
        let mut block = Block(vec![
            Assign::new(
                vec![v21.clone().into()],
                vec![RValue::Local(v19.clone())],
            )
            .into(),
            for_stmt.into(),
            crate::Return::new(vec![RValue::Local(v21)]).into(),
        ]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            !printed.contains("v21"),
            "loop-carried tmp should coalesce: {printed}"
        );
    }

    #[test]
    fn does_not_inline_local_captured_as_upvalue() {
        let cb = named("_callback");
        let owner = named("p_u5");
        let mut decl = Assign::new(
            vec![cb.clone().into()],
            vec![RValue::Local(owner.clone())],
        );
        decl.prefix = true;
        let inner = crate::Function {
            name: None,
            line: None,
            parameters: vec![named("v7")],
            is_variadic: false,
            body: Block::default(),
        };
        let closure = crate::Closure {
            function: by_address::ByAddress(crate::share_function(inner)),
            upvalues: vec![crate::Upvalue::Ref(cb.clone())],
        };
        let call = Call::new(
            crate::Global::new(b"BindToRenderStep".to_vec()).into(),
            vec![RValue::Closure(closure)],
        );
        let mut block = Block(vec![decl.into(), Statement::Call(call)]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            printed.contains("_callback"),
            "upvalue local must not be inlined away: {printed}"
        );
    }
}
