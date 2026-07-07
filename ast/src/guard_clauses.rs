use crate::{Block, RValue, Reduce, Statement, Traverse, Unary, UnaryOperation};

/// What statement is semantically equivalent to "falling off the end" of the
/// block currently being processed.
///
/// This matters because the guard-clause transform below turns:
///     if COND then A end
/// (when it's the last statement of a block) into:
///     if not COND then <exit> end
///     A
/// `<exit>` must behave exactly like reaching the end of the block would have.
/// - At the top of a function body (or any block whose fallthrough leaves the
///   function), that's `return` (with no values).
/// - Inside the body of a loop (while/repeat/numeric-for/generic-for), falling
///   off the end of the body just moves on to the next iteration, so `<exit>`
///   must be `continue`, never `return`. Emitting `return` there is a
///   correctness bug: it silently aborts the *entire enclosing function*
///   instead of skipping to the next loop iteration.
/// - Nested `if`/`else` blocks inherit whatever the enclosing block would do,
///   since falling off the end of an if-branch just continues on to whatever
///   follows the `if` in its parent block.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExitKind {
    Return,
    Continue,
}

impl ExitKind {
    fn make_statement(self) -> Statement {
        match self {
            ExitKind::Return => Statement::Return(crate::Return { values: vec![] }),
            ExitKind::Continue => Statement::Continue(crate::Continue {}),
        }
    }

    fn matches_existing(self, stmt: &Statement) -> bool {
        match self {
            ExitKind::Return => is_void_return(stmt),
            ExitKind::Continue => matches!(stmt, Statement::Continue(_)),
        }
    }
}

pub fn apply_guard_clauses(block: &mut Block) {
    apply_guard_clauses_ctx(block, ExitKind::Return);
}

fn apply_guard_clauses_ctx(block: &mut Block, exit_kind: ExitKind) {
    let mut i = 0;
    while i < block.0.len() {
        // Recurse into closures first. Closures start a brand new function,
        // so their body's fallthrough always means `return`, regardless of
        // whatever loop we might currently be nested inside of.
        block.0[i].traverse_rvalues(&mut |rv| {
            if let RValue::Closure(closure) = rv {
                apply_guard_clauses_ctx(&mut closure.function.lock().body, ExitKind::Return);
            }
        });

        // Recurse into nested blocks first (post-order processing).
        // `if`/`else` branches inherit the current exit kind (falling off
        // their end just continues on to whatever follows the `if`).
        // Loop bodies reset the exit kind to `Continue`, since falling off
        // the end of a loop body moves to the next iteration, not out of
        // the enclosing function.
        match &mut block.0[i] {
            Statement::If(r#if) => {
                apply_guard_clauses_ctx(&mut r#if.then_block.lock(), exit_kind);
                apply_guard_clauses_ctx(&mut r#if.else_block.lock(), exit_kind);
            }
            Statement::While(r#while) => {
                apply_guard_clauses_ctx(&mut r#while.block.lock(), ExitKind::Continue)
            }
            Statement::Repeat(repeat) => {
                apply_guard_clauses_ctx(&mut repeat.block.lock(), ExitKind::Continue)
            }
            Statement::NumericFor(nf) => {
                apply_guard_clauses_ctx(&mut nf.block.lock(), ExitKind::Continue)
            }
            Statement::GenericFor(gf) => {
                apply_guard_clauses_ctx(&mut gf.block.lock(), ExitKind::Continue)
            }
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
            || (i == block.0.len() - 2 && exit_kind.matches_existing(&block.0[i + 1]));

        if is_at_end {
            let mut did_case2 = false;
            if let Statement::If(r#if) = &mut block.0[i] {
                let else_empty = r#if.else_block.lock().0.is_empty();
                let then_len = r#if.then_block.lock().0.len();
                let is_negated = match &r#if.condition {
                    RValue::Unary(u) => u.operation == UnaryOperation::Not,
                    _ => false,
                };
                // Never synthesize a guard-clause exit out of a `then`-block
                // that already ends in its own terminator (return/break/
                // continue/goto) -- in that case the block doesn't actually
                // fall through, so inverting it and appending a synthetic
                // exit would silently duplicate/alter control flow.
                let then_already_terminates = ends_in_terminator(&r#if.then_block.lock().0);
                if !then_already_terminates
                    && else_empty
                    && (then_len >= 2 || (then_len >= 1 && is_negated))
                {
                    let new_cond = Unary::new(r#if.condition.clone(), UnaryOperation::Not).reduce_condition();
                    let stmts = std::mem::take(&mut r#if.then_block.lock().0);
                    r#if.then_block.lock().0.push(exit_kind.make_statement());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Block, Global, LValue, Literal, NumericFor, RValue, RcLocal};

    fn call_stmt(name: &str) -> Statement {
        Assign::new(
            vec![LValue::Local(RcLocal::default())],
            vec![RValue::Call(crate::Call {
                value: Box::new(RValue::Global(Global(name.as_bytes().to_vec()))),
                arguments: vec![],
            })],
        )
        .into()
    }

    /// Regression test for a critical control-flow bug where the guard
    /// clause inversion pass always synthesized a bare `return` when
    /// flattening a trailing `if COND then A end` -- even when that `if`
    /// lived inside a loop body, where "fall off the end of the block"
    /// actually means "move on to the next iteration" (`continue`), not
    /// "exit the enclosing function" (`return`). Emitting `return` there
    /// silently aborted the whole function instead of just skipping the
    /// current iteration (e.g. Dex Explorer's descendant-processing loop
    /// lost most children of newly added objects because of this exact
    /// pattern).
    #[test]
    fn guard_clause_inside_loop_uses_continue_not_return() {
        // Builds:
        //   for i = 1, 10 do
        //       if cond() then
        //           doA()
        //           doB()
        //       end
        //   end
        let counter = RcLocal::default();
        let if_stat = crate::If::new(
            RValue::Call(crate::Call {
                value: Box::new(RValue::Global(Global(b"cond".to_vec()))),
                arguments: vec![],
            }),
            vec![call_stmt("doA"), call_stmt("doB")].into(),
            Block::default(),
        );
        let loop_body: Block = vec![if_stat.into()].into();
        let mut block: Block = vec![NumericFor::new(
            Literal::Number(1.0).into(),
            Literal::Number(10.0).into(),
            Literal::Number(1.0).into(),
            counter,
            loop_body,
        )
        .into()]
        .into();

        apply_guard_clauses(&mut block);

        let printed = block.to_string();
        assert!(
            printed.contains("continue"),
            "expected the loop-body guard clause to use `continue`, got: {printed:?}"
        );
        assert!(
            !printed.contains("return"),
            "guard clause inside a loop body must never synthesize a bare `return` \
             (that exits the whole enclosing function instead of just skipping \
             the current iteration): got {printed:?}"
        );
    }

    /// The same guard-clause pattern at the top level of a function body (not
    /// inside any loop) must still correctly use `return`, since that's what
    /// falling off the end of a function body means.
    #[test]
    fn guard_clause_outside_loop_uses_return() {
        // Builds:
        //   if cond() then
        //       doA()
        //       doB()
        //   end
        let if_stat = crate::If::new(
            RValue::Call(crate::Call {
                value: Box::new(RValue::Global(Global(b"cond".to_vec()))),
                arguments: vec![],
            }),
            vec![call_stmt("doA"), call_stmt("doB")].into(),
            Block::default(),
        );
        let mut block: Block = vec![if_stat.into()].into();

        apply_guard_clauses(&mut block);

        let printed = block.to_string();
        assert!(
            printed.contains("return"),
            "expected the top-level guard clause to use `return`, got: {printed:?}"
        );
        assert!(
            !printed.contains("continue"),
            "top-level (non-loop) guard clause must not use `continue`: got {printed:?}"
        );
    }
}
