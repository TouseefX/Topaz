use crate::{Block, RValue, Reduce, Statement, Traverse, Unary, UnaryOperation};

pub fn apply_guard_clauses(block: &mut Block) {
    let mut i = 0;
    while i < block.0.len() {
        // Recurse into closures first
        block.0[i].traverse_rvalues(&mut |rv| {
            if let RValue::Closure(closure) = rv {
                apply_guard_clauses(&mut closure.function.lock().body);
            }
        });

        // Recurse into nested blocks first (post-order processing)
        match &mut block.0[i] {
            Statement::If(r#if) => {
                apply_guard_clauses(&mut r#if.then_block.lock());
                apply_guard_clauses(&mut r#if.else_block.lock());
            }
            Statement::While(r#while) => apply_guard_clauses(&mut r#while.block.lock()),
            Statement::Repeat(repeat) => apply_guard_clauses(&mut repeat.block.lock()),
            Statement::NumericFor(nf) => apply_guard_clauses(&mut nf.block.lock()),
            Statement::GenericFor(gf) => apply_guard_clauses(&mut gf.block.lock()),
            _ => {}
        }

        // Case 1: Redundant else removal after terminator
        let mut did_case1 = false;
        if let Statement::If(r#if) = &mut block.0[i] {
            let ends_term = ends_in_terminator(&r#if.then_block.lock().0);
            let else_not_empty = !r#if.else_block.lock().0.is_empty();
            if ends_term && else_not_empty {
                let stmts = std::mem::take(&mut r#if.else_block.lock().0);
                block.0.splice(i + 1..i + 1, stmts);
                did_case1 = true;
            }
        }
        if did_case1 {
            i += 1;
            continue;
        }

        // Case 2: Guard clause inversion at the end of a block
        let is_at_end = i == block.0.len() - 1
            || (i == block.0.len() - 2 && is_void_return(&block.0[i + 1]));

        if is_at_end {
            let mut did_case2 = false;
            if let Statement::If(r#if) = &mut block.0[i] {
                let else_empty = r#if.else_block.lock().0.is_empty();
                let then_len = r#if.then_block.lock().0.len();
                let is_negated = match &r#if.condition {
                    RValue::Unary(u) => u.operation == UnaryOperation::Not,
                    _ => false,
                };
                if else_empty && (then_len >= 2 || (then_len >= 1 && is_negated)) {
                    let new_cond = Unary::new(r#if.condition.clone(), UnaryOperation::Not).reduce_condition();
                    let stmts = std::mem::take(&mut r#if.then_block.lock().0);
                    r#if.then_block.lock().0.push(Statement::Return(crate::Return { values: vec![] }));
                    r#if.condition = new_cond;
                    block.0.splice(i + 1..i + 1, stmts);
                    did_case2 = true;
                }
            }
            if did_case2 {
                i += 1;
                continue;
            }
        }

        i += 1;
    }
}

fn ends_in_terminator(stmts: &[Statement]) -> bool {
    if let Some(last) = stmts.last() {
        matches!(
            last,
            Statement::Return(_) | Statement::Break(_) | Statement::Continue(_) | Statement::Goto(_)
        )
    } else {
        false
    }
}

fn is_void_return(stmt: &Statement) -> bool {
    if let Statement::Return(r) = stmt {
        r.values.is_empty()
    } else {
        false
    }
}
