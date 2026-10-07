//! Collapse compiler-shaped if/else into expressions.
//!
//! Luau compiles `assert(type(x) == "number", msg)` as:
//!   if type(x) == "number" then v = true else v = false end
//!   assert(v, msg)
//! and `typeof(x) == "Vector3" and x or default` as a mutating if.
//! Matching those recovers Leitnick-style CameraShaker.

use crate::{
    Assign, Binary, BinaryOperation, Block, LValue, Literal, RValue, RcLocal, Reduce, Statement,
    Unary, UnaryOperation,
};

fn is_fastcall_comment(stmt: &Statement) -> bool {
    matches!(stmt, Statement::Comment(c) if c.text.contains("fastcall"))
}

fn significant<'a>(block: &'a Block) -> Vec<&'a Statement> {
    block
        .0
        .iter()
        .filter(|s| !matches!(s, Statement::Empty(_)) && !is_fastcall_comment(s))
        .collect()
}

fn single_assign(block: &Block) -> Option<(LValue, RValue)> {
    let stmts = significant(block);
    if stmts.len() != 1 {
        return None;
    }
    let Statement::Assign(assign) = stmts[0] else {
        return None;
    };
    if assign.left.len() != 1 || assign.right.len() != 1 {
        return None;
    }
    Some((assign.left[0].clone(), assign.right[0].clone()))
}

fn always_truthy(rv: &RValue) -> bool {
    match rv {
        RValue::Literal(
            Literal::Boolean(true)
            | Literal::Number(_)
            | Literal::Integer(_)
            | Literal::String(_)
            | Literal::Vector(..),
        )
        | RValue::Table(_)
        | RValue::Closure(_) => true,
        RValue::Binary(b) if b.operation == BinaryOperation::Concat => true,
        RValue::Unary(u) if u.operation == UnaryOperation::Length => true,
        _ => false,
    }
}

fn combine(cond: RValue, then_v: RValue, else_v: RValue) -> Option<RValue> {
    match (&then_v, &else_v) {
        (
            RValue::Literal(Literal::Boolean(true)),
            RValue::Literal(Literal::Boolean(false)),
        ) => Some(cond.reduce_condition()),
        (
            RValue::Literal(Literal::Boolean(false)),
            RValue::Literal(Literal::Boolean(true)),
        ) => Some(Unary::new(cond, UnaryOperation::Not).reduce_condition()),
        (_, RValue::Literal(Literal::Boolean(false) | Literal::Nil)) => Some(
            Binary::new(cond.reduce_condition(), then_v, BinaryOperation::And).reduce(),
        ),
        (RValue::Literal(Literal::Boolean(false) | Literal::Nil), _) => Some(
            Binary::new(
                Unary::new(cond, UnaryOperation::Not).reduce_condition(),
                else_v,
                BinaryOperation::And,
            )
            .reduce(),
        ),
        _ if always_truthy(&then_v) => Some(
            Binary::new(
                Binary::new(cond.reduce_condition(), then_v, BinaryOperation::And).into(),
                else_v,
                BinaryOperation::Or,
            )
            .reduce(),
        ),
        _ => None,
    }
}

fn collapse_if_assign(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.0.len() {
        match &mut block.0[i] {
            Statement::If(if_stmt) => {
                changed |= collapse_if_assign(&mut if_stmt.then_block.lock());
                changed |= collapse_if_assign(&mut if_stmt.else_block.lock());
            }
            Statement::While(s) => {
                changed |= collapse_if_assign(&mut s.block.lock());
            }
            Statement::Repeat(s) => {
                changed |= collapse_if_assign(&mut s.block.lock());
            }
            Statement::NumericFor(s) => {
                changed |= collapse_if_assign(&mut s.block.lock());
            }
            Statement::GenericFor(s) => {
                changed |= collapse_if_assign(&mut s.block.lock());
            }
            _ => {}
        }

        let Some((left, then_v, else_v, cond)) = (if let Statement::If(if_stmt) = &block.0[i] {
            let then_a = single_assign(&if_stmt.then_block.lock());
            let else_a = single_assign(&if_stmt.else_block.lock());
            match (then_a, else_a) {
                (Some((tl, tv)), Some((el, ev))) if tl == el => {
                    Some((tl, tv, ev, if_stmt.condition.clone()))
                }
                _ => None,
            }
        } else {
            None
        }) else {
            i += 1;
            continue;
        };

        if let Some(expr) = combine(cond, then_v, else_v) {
            let mut assign = Assign::new(vec![left], vec![expr]);
            let merge_decl = i > 0
                && matches!(
                    &block.0[i - 1],
                    Statement::Assign(prev)
                        if prev.prefix
                            && prev.right.is_empty()
                            && prev.left.len() == 1
                            && prev.left[0] == assign.left[0]
                );
            if merge_decl {
                assign.prefix = true;
                block.0.remove(i - 1);
                i -= 1;
            }
            block.0[i] = assign.into();
            changed = true;
            continue;
        }
        i += 1;
    }
    changed
}

fn is_typeof_or_type(rv: &RValue) -> bool {
    match rv {
        RValue::Global(g) => g.0 == b"typeof" || g.0 == b"type",
        _ => false,
    }
}

/// `typeof(x) ~= "T"` → `typeof(x) == "T"`
fn invert_typeof_neq(cond: &RValue, local: &RcLocal) -> Option<RValue> {
    fn match_neq(rv: &RValue, local: &RcLocal) -> Option<RValue> {
        let RValue::Binary(b) = rv else {
            return None;
        };
        if b.operation != BinaryOperation::NotEqual {
            return None;
        }
        let call_on_left = matches!(
            b.left.as_ref(),
            RValue::Call(c)
                if is_typeof_or_type(&c.value)
                    && c.arguments.len() == 1
                    && matches!(&c.arguments[0], RValue::Local(l) if l == local)
        );
        if call_on_left && matches!(b.right.as_ref(), RValue::Literal(Literal::String(_))) {
            return Some(
                Binary::new(
                    *b.left.clone(),
                    *b.right.clone(),
                    BinaryOperation::Equal,
                )
                .into(),
            );
        }
        None
    }

    if let Some(eq) = match_neq(cond, local) {
        return Some(eq);
    }
    let RValue::Binary(b) = cond else {
        return None;
    };
    if b.operation != BinaryOperation::Or {
        return None;
    }
    match_neq(&b.left, local).or_else(|| match_neq(&b.right, local))
}

fn fold_typeof_default(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.0.len() {
        match &mut block.0[i] {
            Statement::If(if_stmt) => {
                changed |= fold_typeof_default(&mut if_stmt.then_block.lock());
                changed |= fold_typeof_default(&mut if_stmt.else_block.lock());
            }
            Statement::While(s) => {
                changed |= fold_typeof_default(&mut s.block.lock());
            }
            Statement::Repeat(s) => {
                changed |= fold_typeof_default(&mut s.block.lock());
            }
            Statement::NumericFor(s) => {
                changed |= fold_typeof_default(&mut s.block.lock());
            }
            Statement::GenericFor(s) => {
                changed |= fold_typeof_default(&mut s.block.lock());
            }
            _ => {}
        }

        let Some((left, default, cond)) = (if let Statement::If(if_stmt) = &block.0[i] {
            let else_empty = if_stmt
                .else_block
                .lock()
                .0
                .iter()
                .all(|s| matches!(s, Statement::Empty(_)) || is_fastcall_comment(s));
            if !else_empty {
                None
            } else {
                single_assign(&if_stmt.then_block.lock())
                    .map(|(l, v)| (l, v, if_stmt.condition.clone()))
            }
        } else {
            None
        }) else {
            i += 1;
            continue;
        };

        let Some(local) = left.as_local().cloned() else {
            i += 1;
            continue;
        };
        // Vector3.new(...) is a call (not a literal) but always truthy.
        // Only skip known-falsy defaults, which would break `and/or`.
        if matches!(
            default,
            RValue::Literal(Literal::Nil | Literal::Boolean(false))
        ) {
            i += 1;
            continue;
        }
        let Some(type_eq) = invert_typeof_neq(&cond, &local) else {
            i += 1;
            continue;
        };
        let expr = Binary::new(
            Binary::new(type_eq, RValue::Local(local.clone()), BinaryOperation::And).into(),
            default,
            BinaryOperation::Or,
        )
        .reduce();
        block.0[i] = Assign::new(vec![left], vec![expr]).into();
        changed = true;
        i += 1;
    }
    changed
}

fn strip_fastcall_comments(block: &mut Block) {
    block
        .0
        .retain(|s| !matches!(s, Statement::Empty(_)) && !is_fastcall_comment(s));
    for stmt in &mut block.0 {
        match stmt {
            Statement::If(s) => {
                strip_fastcall_comments(&mut s.then_block.lock());
                strip_fastcall_comments(&mut s.else_block.lock());
            }
            Statement::While(s) => strip_fastcall_comments(&mut s.block.lock()),
            Statement::Repeat(s) => strip_fastcall_comments(&mut s.block.lock()),
            Statement::NumericFor(s) => strip_fastcall_comments(&mut s.block.lock()),
            Statement::GenericFor(s) => strip_fastcall_comments(&mut s.block.lock()),
            _ => {}
        }
    }
}

pub fn apply(block: &mut Block) {
    strip_fastcall_comments(block);
    for _ in 0..4 {
        let a = collapse_if_assign(block);
        let b = fold_typeof_default(block);
        if !a && !b {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, Global, If, Local, RcLocal};

    fn named(n: &str) -> RcLocal {
        RcLocal::new(Local::new(Some(n.into())))
    }

    fn type_eq(local: RcLocal, ty: &str) -> RValue {
        Binary::new(
            Call::new(
                Global::new(b"type".to_vec()).into(),
                vec![RValue::Local(local)],
            )
            .into(),
            Literal::String(ty.as_bytes().to_vec()).into(),
            BinaryOperation::Equal,
        )
        .into()
    }

    #[test]
    fn bool_if_collapses_to_condition() {
        let x = named("x");
        let v = named("v");
        let then_b = Block(vec![Assign::new(
            vec![v.clone().into()],
            vec![Literal::Boolean(true).into()],
        )
        .into()]);
        let else_b = Block(vec![Assign::new(
            vec![v.clone().into()],
            vec![Literal::Boolean(false).into()],
        )
        .into()]);
        let mut block = Block(vec![If::new(type_eq(x, "number"), then_b, else_b).into()]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            !printed.contains("if "),
            "boolean if should collapse: {printed}"
        );
        assert!(printed.contains("=="), "expected comparison: {printed}");
    }

    #[test]
    fn table_and_field_collapses() {
        let p = named("p");
        let v = named("v");
        let field = RValue::Index(crate::Index::new(
            RValue::Local(p.clone()),
            Literal::String(b"_camShakeInstance".to_vec()).into(),
        ));
        let then_b = Block(vec![Assign::new(vec![v.clone().into()], vec![field]).into()]);
        let else_b = Block(vec![Assign::new(
            vec![v.clone().into()],
            vec![Literal::Boolean(false).into()],
        )
        .into()]);
        let mut block = Block(vec![If::new(type_eq(p, "table"), then_b, else_b).into()]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            printed.contains("and"),
            "expected `type == \"table\" and x._camShakeInstance`: {printed}"
        );
        assert!(!printed.contains("if "), "if should collapse: {printed}");
    }

    #[test]
    fn typeof_default_becomes_and_or() {
        let x = named("x");
        let cond = Binary::new(
            Binary::new(
                Call::new(
                    Global::new(b"typeof".to_vec()).into(),
                    vec![RValue::Local(x.clone())],
                )
                .into(),
                Literal::String(b"Vector3".to_vec()).into(),
                BinaryOperation::NotEqual,
            )
            .into(),
            Unary::new(RValue::Local(x.clone()), UnaryOperation::Not).into(),
            BinaryOperation::Or,
        )
        .into();
        let default = Call::new(
            RValue::Index(crate::Index::new(
                Global::new(b"Vector3".to_vec()).into(),
                Literal::String(b"new".to_vec()).into(),
            )),
            vec![
                Literal::Number(0.15).into(),
                Literal::Number(0.15).into(),
                Literal::Number(0.15).into(),
            ],
        );
        let then_b = Block(vec![Assign::new(
            vec![x.clone().into()],
            vec![default.into()],
        )
        .into()]);
        let mut block = Block(vec![If::new(cond, then_b, Block::default()).into()]);
        apply(&mut block);
        let printed = block.to_string();
        assert!(
            printed.contains("and") && printed.contains("or"),
            "expected typeof == \"Vector3\" and x or default: {printed}"
        );
        assert!(!printed.contains("if "), "if should collapse: {printed}");
    }
}
