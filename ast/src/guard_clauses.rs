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
/// - Nested `if`/`else` blocks only inherit the enclosing block's exit kind
///   when the `if` statement itself sits in *tail position* of that block
///   (i.e. it's the last statement, or the second-to-last followed only by
///   a statement that already matches the exit kind) -- only then does
///   "falling off the end of a branch" actually reach the enclosing block's
///   own exit. If the `if` is *not* in tail position (more statements
///   follow it in the same block), falling off the end of its branches
///   just falls through to those sibling statements instead -- which is
///   not something a single synthetic statement can represent, so no exit
///   kind is threaded through in that case (`ExitKind::None`), which
///   disables guard-clause synthesis inside those branches entirely.
///   Getting this wrong is a real correctness bug: it silently rewrites
///   "keep executing the rest of the enclosing block" into "exit the
///   enclosing loop/function early", dropping any code that was supposed
///   to run afterward (e.g. a `while true do ... break end`-derived `if`
///   block followed by more statements after the loop).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ExitKind {
    Return,
    Continue,
    /// This position is not in tail position of any enclosing loop/
    /// function exit, so no single statement can correctly represent
    /// "falling off the end here" -- guard-clause synthesis must not be
    /// applied.
    None,
}

impl ExitKind {
    fn make_statement(self) -> Option<Statement> {
        match self {
            ExitKind::Return => Some(Statement::Return(crate::Return { values: vec![] })),
            ExitKind::Continue => Some(Statement::Continue(crate::Continue {})),
            ExitKind::None => None,
        }
    }

    fn matches_existing(self, stmt: &Statement) -> bool {
        match self {
            ExitKind::Return => is_void_return(stmt),
            ExitKind::Continue => matches!(stmt, Statement::Continue(_)),
            ExitKind::None => false,
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

        // `if`/`else` branches only inherit the current exit kind when this
        // `if` statement is itself in tail position of `block` (see
        // `ExitKind`'s doc comment for why) -- otherwise, falling off the
        // end of a branch here just falls through to sibling statements
        // that follow in `block`, which no single synthetic statement can
        // represent, so we must not synthesize guard-clause exits there.
        let is_tail_position = i == block.0.len() - 1
            || (i == block.0.len() - 2 && exit_kind.matches_existing(&block.0[i + 1]));
        let branch_exit_kind = if is_tail_position {
            exit_kind
        } else {
            ExitKind::None
        };

        // Recurse into nested blocks first (post-order processing).
        // Loop bodies reset the exit kind to `Continue` regardless of tail
        // position, since falling off the end of a loop body always moves
        // to the next iteration of *that* loop, never out of the
        // enclosing function or into sibling statements after the loop.
        match &mut block.0[i] {
            Statement::If(r#if) => {
                apply_guard_clauses_ctx(&mut r#if.then_block.lock(), branch_exit_kind);
                apply_guard_clauses_ctx(&mut r#if.else_block.lock(), branch_exit_kind);
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

        // Case 2: Guard clause inversion at the end of a block. Only
        // applicable when this position is in tail position *and* that
        // tail position actually has a representable exit statement
        // (`ExitKind::None` means "falls through to sibling statements",
        // which can't be synthesized as a single statement, so we must
        // leave the code as nested ifs rather than risk turning a
        // fallthrough into an incorrect early exit).
        if is_tail_position {
            if let Some(exit_stmt) = exit_kind.make_statement() {
                let mut did_case2 = false;
                if let Statement::If(r#if) = &mut block.0[i] {
                    let else_empty = r#if.else_block.lock().0.is_empty();
                    let then_len = r#if.then_block.lock().0.len();
                    let is_negated = match &r#if.condition {
                        RValue::Unary(u) => u.operation == UnaryOperation::Not,
                        _ => false,
                    };
                    // Never synthesize a guard-clause exit out of a
                    // `then`-block that already ends in its own terminator
                    // (return/break/continue/goto) -- in that case the
                    // block doesn't actually fall through, so inverting it
                    // and appending a synthetic exit would silently
                    // duplicate/alter control flow.
                    let then_already_terminates = ends_in_terminator(&r#if.then_block.lock().0);
                    if !then_already_terminates
                        && else_empty
                        && (then_len >= 2 || (then_len >= 1 && is_negated))
                    {
                        let new_cond =
                            Unary::new(r#if.condition.clone(), UnaryOperation::Not).reduce_condition();
                        let stmts = std::mem::take(&mut r#if.then_block.lock().0);
                        r#if.then_block.lock().0.push(exit_stmt);
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

    /// Regression test for a critical control-flow bug where an `if`
    /// statement that is *not* in tail position of its enclosing block
    /// (i.e. more statements follow it) still had a guard-clause exit
    /// synthesized inside its branches, incorrectly turning "fall through
    /// to the sibling statements that follow" into "exit the enclosing
    /// loop/function early" -- silently dropping the sibling statements
    /// for whichever branch took the synthesized-exit path. This is
    /// exactly the shape produced by a `while true do ... break end` loop
    /// followed by more code after the loop (e.g. a binary-search
    /// insertion-position loop followed by `table.insert(...)`): one
    /// branch of the post-loop `if` must fall through to the
    /// `table.insert` call, not return out of the function early.
    #[test]
    fn if_not_in_tail_position_does_not_synthesize_an_exit() {
        // Builds:
        //   if cond() then
        //       doA()
        //       doB()
        //   end
        //   doAfter()  -- must always run, regardless of which branch
        //                 of the `if` above was taken
        let if_stat = crate::If::new(
            RValue::Call(crate::Call {
                value: Box::new(RValue::Global(Global(b"cond".to_vec()))),
                arguments: vec![],
            }),
            vec![call_stmt("doA"), call_stmt("doB")].into(),
            Block::default(),
        );
        let mut block: Block = vec![if_stat.into(), call_stmt("doAfter")].into();

        apply_guard_clauses(&mut block);

        let printed = block.to_string();
        assert!(
            !printed.contains("return") && !printed.contains("continue"),
            "an `if` with more statements following it in the same block \
             must never have a guard-clause exit synthesized inside its \
             branches, since falling off the end of either branch must \
             fall through to the sibling statements that follow (not \
             exit the enclosing loop/function): got {printed:?}"
        );
        assert!(
            printed.contains("doAfter"),
            "the statement following the `if` must be preserved and must \
             run regardless of which branch of the `if` was taken: got {printed:?}"
        );
    }

    /// Same bug, but nested one level deeper inside a loop, to make sure
    /// the fix doesn't just special-case the top level: an `if` inside a
    /// loop body that is not itself in tail position of that loop body
    /// must not get a `continue` synthesized inside its branches either,
    /// since that would skip sibling statements later in the same loop
    /// iteration instead of just falling through to them.
    #[test]
    fn if_not_in_tail_position_inside_loop_does_not_synthesize_continue() {
        // Builds:
        //   for i = 1, 10 do
        //       if cond() then
        //           doA()
        //           doB()
        //       end
        //       doAfter()
        //   end
        let if_stat = crate::If::new(
            RValue::Call(crate::Call {
                value: Box::new(RValue::Global(Global(b"cond".to_vec()))),
                arguments: vec![],
            }),
            vec![call_stmt("doA"), call_stmt("doB")].into(),
            Block::default(),
        );
        let loop_body: Block = vec![if_stat.into(), call_stmt("doAfter")].into();
        let mut block: Block = vec![NumericFor::new(
            Literal::Number(1.0).into(),
            Literal::Number(10.0).into(),
            Literal::Number(1.0).into(),
            RcLocal::default(),
            loop_body,
        )
        .into()]
        .into();

        apply_guard_clauses(&mut block);

        let printed = block.to_string();
        assert!(
            !printed.contains("continue") && !printed.contains("return"),
            "an `if` with more statements following it in the same loop \
             iteration must never have a guard-clause exit synthesized \
             inside its branches: got {printed:?}"
        );
        assert!(
            printed.contains("doAfter"),
            "the statement following the `if` inside the loop body must \
             be preserved and must run regardless of which branch of the \
             `if` was taken: got {printed:?}"
        );
    }
}

