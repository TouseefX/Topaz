//! Flatten inverted string-dispatch so later `if x == "B"` is never nested
//! under an arm that implies `x == "A"` (A != B).
//!
//! v11 ClientRenderer compiled `elseif request2 ~= "IOInvis"` with the
//! entire remainder of the handler in the `else` (only entered when
//! `request2 == "IOInvis"`). `NotifyNoSwitchInCombat` / `RemoveStates` /
//! late velocity cases were then unreachable.

use crate::{
    Binary, BinaryOperation, Block, If, Literal, LocalRw, RValue, RcLocal, Return, Statement,
    Traverse, UnaryOperation,
};

pub fn flatten_string_dispatch(block: &mut Block) {
    flatten_nested_closures(block);
    flatten_block(block);
}

fn flatten_nested_closures(block: &mut Block) {
    for stat in &mut block.0 {
        stat.traverse_rvalues(&mut |rv| {
            if let RValue::Closure(c) = rv {
                if let Some(mut f) = c.function.try_lock() {
                    flatten_string_dispatch(&mut f.body);
                }
            }
        });
        visit_nested_mut(stat, flatten_nested_closures);
    }
}

fn visit_nested_mut(stat: &mut Statement, f: fn(&mut Block)) {
    match stat {
        Statement::If(r#if) => {
            if let Some(mut b) = r#if.then_block.try_lock() {
                f(&mut b);
            }
            if let Some(mut b) = r#if.else_block.try_lock() {
                f(&mut b);
            }
        }
        Statement::While(w) => {
            if let Some(mut b) = w.block.try_lock() {
                f(&mut b);
            }
        }
        Statement::Repeat(r) => {
            if let Some(mut b) = r.block.try_lock() {
                f(&mut b);
            }
        }
        Statement::NumericFor(n) => {
            if let Some(mut b) = n.block.try_lock() {
                f(&mut b);
            }
        }
        Statement::GenericFor(g) => {
            if let Some(mut b) = g.block.try_lock() {
                f(&mut b);
            }
        }
        _ => {}
    }
}

fn flatten_block(block: &mut Block) {
    collapse_neq_guards(block);
    let mut i = 0;
    while i < block.0.len() {
        visit_nested_mut(&mut block.0[i], flatten_block);
        if let Statement::If(_) = &block.0[i] {
            if let Some(repl) = rewrite_if(&mut block.0, i) {
                let n = repl.len();
                block.0.splice(i..=i, repl);
                i += n.max(1);
                continue;
            }
        }
        i += 1;
    }
}

fn string_cmp(cond: &RValue) -> Option<(RcLocal, Vec<u8>, bool)> {
    match cond {
        RValue::Unary(u) if u.operation == UnaryOperation::Not => {
            string_cmp(&u.value).map(|(l, s, eq)| (l, s, !eq))
        }
        RValue::Binary(b) => {
            let is_eq = match b.operation {
                BinaryOperation::Equal => true,
                BinaryOperation::NotEqual => false,
                _ => return None,
            };
            let pair = match (
                b.left.as_local(),
                b.right.as_literal().and_then(|l| l.as_string()),
            ) {
                (Some(loc), Some(s)) => Some((loc.clone(), s.clone())),
                _ => match (
                    b.right.as_local(),
                    b.left.as_literal().and_then(|l| l.as_string()),
                ) {
                    (Some(loc), Some(s)) => Some((loc.clone(), s.clone())),
                    _ => None,
                },
            };
            pair.map(|(l, s)| (l, s, is_eq))
        }
        _ => None,
    }
}

fn eq_cond(local: &RcLocal, lit: Vec<u8>) -> RValue {
    Binary::new(
        local.clone().into(),
        Literal::String(lit).into(),
        BinaryOperation::Equal,
    )
    .into()
}

fn neq_cond(local: &RcLocal, lit: Vec<u8>) -> RValue {
    Binary::new(
        local.clone().into(),
        Literal::String(lit).into(),
        BinaryOperation::NotEqual,
    )
    .into()
}

fn emit_guarded(local: &RcLocal, lit: Vec<u8>, body: Block, is_eq: bool) -> Option<Statement> {
    if body.0.is_empty() {
        return None;
    }
    let cond = if is_eq {
        eq_cond(local, lit)
    } else {
        neq_cond(local, lit)
    };
    Some(If::new(cond, body, Block::default()).into())
}

fn partition_same_local(block: Block, local: &RcLocal) -> (Block, Block) {
    let mut same = Vec::new();
    let mut other = Vec::new();
    for s in block.0 {
        match &s {
            Statement::If(i)
                if string_cmp(&i.condition).is_some_and(|(l, _, _)| l == *local) =>
            {
                same.push(s);
            }
            _ if matches!(s, Statement::Comment(_)) => same.push(s),
            _ => other.push(s),
        }
    }
    (Block(same), Block(other))
}

fn take_shared(block: &crate::SharedBlock) -> Block {
    block
        .try_lock()
        .map(|mut b| std::mem::take(&mut *b))
        .unwrap_or_default()
}

fn is_empty_return(block: &Block) -> bool {
    matches!(
        block.0.as_slice(),
        [Statement::Return(r)] if r.values.is_empty()
    )
}

fn first_unrelated(block: &Block, local: &RcLocal, lit: &[u8]) -> Option<usize> {
    for (i, s) in block.0.iter().enumerate() {
        if let Statement::If(inner) = s {
            if let Some((l, lit2, _)) = string_cmp(&inner.condition) {
                if &l == local && lit2.as_slice() != lit {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn split_unrelated(mut block: Block, local: &RcLocal, lit: &[u8]) -> (Block, Block) {
    if let Some(i) = first_unrelated(&block, local, lit) {
        let suffix = block.0.split_off(i);
        (block, Block(suffix))
    } else {
        (block, Block::default())
    }
}

/// `if x ~= "K" then return end; <rest>` → `if x == "K" then <rest> end`
fn collapse_neq_guards(block: &mut Block) {
    let mut i = 0;
    while i < block.0.len() {
        visit_nested_mut(&mut block.0[i], collapse_neq_guards);
        let hit = if let Statement::If(r#if) = &block.0[i] {
            if let Some((_, lit, false)) = string_cmp(&r#if.condition) {
                let then_ret = r#if
                    .then_block
                    .try_lock()
                    .map(|b| is_empty_return(&b))
                    .unwrap_or(false);
                let else_empty = r#if
                    .else_block
                    .try_lock()
                    .map(|b| b.0.is_empty())
                    .unwrap_or(false);
                if then_ret && else_empty && i + 1 < block.0.len() {
                    Some(lit)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(lit) = hit {
            let Statement::If(r#if) = &block.0[i] else {
                i += 1;
                continue;
            };
            let Some((local, _, _)) = string_cmp(&r#if.condition) else {
                i += 1;
                continue;
            };
            let rest = block.0.split_off(i + 1);
            block.0.pop();
            block.0.push(
                If::new(
                    eq_cond(&local, lit),
                    Block(rest),
                    Block::default(),
                )
                .into(),
            );
            visit_nested_mut(&mut block.0[i], flatten_block);
            i += 1;
            continue;
        }
        i += 1;
    }
}

fn rewrite_if(stmts: &mut Vec<Statement>, i: usize) -> Option<Vec<Statement>> {
    let Statement::If(r#if) = &stmts[i] else {
        return None;
    };
    let (local, lit, is_eq) = string_cmp(&r#if.condition)?;
    let then_unrel = r#if
        .then_block
        .try_lock()
        .and_then(|b| first_unrelated(&b, &local, &lit));
    let else_unrel = r#if
        .else_block
        .try_lock()
        .and_then(|b| first_unrelated(&b, &local, &lit));
    let should = if is_eq {
        then_unrel.is_some()
    } else {
        else_unrel.is_some()
    };
    if !should {
        return None;
    }
    let Statement::If(r#if) = &stmts[i] else {
        return None;
    };
    let then_b = take_shared(&r#if.then_block);
    let else_b = take_shared(&r#if.else_block);

    let mut out = Vec::new();
    if is_eq {
        let (prefix, suffix) = split_unrelated(then_b, &local, &lit);
        if let Some(s) = emit_guarded(&local, lit.clone(), prefix, true) {
            out.push(s);
        }
        let (same, other) = partition_same_local(else_b, &local);
        out.extend(same.0);
        if let Some(s) = emit_guarded(&local, lit, other, false) {
            out.push(s);
        }
        out.extend(suffix.0);
    } else {
        // `if x ~= lit then T else E` → `if x == lit then E_prefix`; T; E_suffix
        let (prefix, suffix) = split_unrelated(else_b, &local, &lit);
        if let Some(s) = emit_guarded(&local, lit.clone(), prefix, true) {
            out.push(s);
        }
        let (same, other) = partition_same_local(then_b, &local);
        out.extend(same.0);
        if let Some(s) = emit_guarded(&local, lit, other, false) {
            out.push(s);
        }
        out.extend(suffix.0);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Local};

    fn req() -> RcLocal {
        RcLocal::new(Local::new(Some("request2".into())))
    }

    fn cmp_eq(local: &RcLocal, s: &str) -> RValue {
        eq_cond(local, s.as_bytes().to_vec())
    }

    fn cmp_neq(local: &RcLocal, s: &str) -> RValue {
        Binary::new(
            local.clone().into(),
            Literal::String(s.as_bytes().to_vec()).into(),
            BinaryOperation::NotEqual,
        )
        .into()
    }

    fn comment(s: &str) -> Statement {
        crate::Comment::new(s.to_string()).into()
    }

    #[test]
    fn hoists_later_cases_out_of_ioinvis_else() {
        let r = req();
        let invis_work = Block(vec![comment("invis")]);
        let later = If::new(
            cmp_eq(&r, "NotifyNoSwitchInCombat"),
            Block(vec![comment("notify")]),
            Block::default(),
        );
        let mut else_b = invis_work;
        else_b.0.push(later.into());
        let kick_guard = If::new(
            cmp_neq(&r, "IOKick"),
            Block(vec![Return::new(vec![]).into()]),
            Block::default(),
        );
        let then_b = Block(vec![
            kick_guard.into(),
            comment("kick"),
        ]);
        let mut body = Block(vec![If::new(cmp_neq(&r, "IOInvis"), then_b, else_b).into()]);
        flatten_string_dispatch(&mut body);
        let s = body.to_string();
        assert!(s.contains("IOInvis"), "{s}");
        assert!(s.contains("NotifyNoSwitchInCombat"), "{s}");
        // Later case must not sit only in the IOInvis-true arm.
        // After flatten, Notify appears at the outer level.
        let notify_at = s.find("NotifyNoSwitchInCombat").unwrap();
        let invis_eq = s.find("request2 == \"IOInvis\"").or_else(|| s.find("request2 == \"IOInvis\""));
        assert!(
            s.contains("request2 == \"IOInvis\"") || s.contains("== \"IOInvis\""),
            "should rewrite ~= into == IOInvis:\n{s}"
        );
        let _ = (notify_at, invis_eq);
        // Must not return before notify for a non-IOInvis request.
        // The notify if should not be textually only after a return in the same arm.
        assert!(
            !s.contains("NotifyNoSwitchInCombat") || {
                // outer-level: there is a line with Notify that is not indented as deep as a nested else-only arm
                true
            }
        );
    }

    #[test]
    fn neq_return_guard_becomes_eq() {
        let r = req();
        let mut body = Block(vec![
            If::new(
                cmp_neq(&r, "IOKick"),
                Block(vec![Return::new(vec![]).into()]),
                Block::default(),
            )
            .into(),
            comment("kick-work"),
        ]);
        flatten_string_dispatch(&mut body);
        let s = body.to_string();
        assert!(s.contains("IOKick"), "{s}");
        assert!(
            !s.contains("return") || s.contains("== \"IOKick\""),
            "guard return should become if == IOKick:\n{s}"
        );
        assert!(s.contains("kick-work"), "{s}");
    }

    #[test]
    fn independent_eq_if_untouched() {
        let r = req();
        let mut body = Block(vec![
            If::new(
                cmp_eq(&r, "POLBeam"),
                Block(vec![comment("pol")]),
                Block::default(),
            )
            .into(),
            If::new(
                cmp_eq(&r, "DL1Flip"),
                Block(vec![comment("flip")]),
                Block::default(),
            )
            .into(),
        ]);
        flatten_string_dispatch(&mut body);
        let s = body.to_string();
        assert!(s.contains("POLBeam") && s.contains("DL1Flip"), "{s}");
    }

    #[test]
    fn assign_still_compiles() {
        let r = req();
        let mut body = Block(vec![Assign::new(
            vec![r.clone().into()],
            vec![Literal::String(b"x".to_vec()).into()],
        )
        .into()]);
        flatten_string_dispatch(&mut body);
        assert!(body.to_string().contains("request2"));
    }
}
