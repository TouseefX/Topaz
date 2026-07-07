//! Compound Assignment Detection and Transformation
//!
//! Detects patterns like `x = x + 1` and converts to `x += 1`

use crate::{Assign, BinaryOperation, Block, LValue, RValue, Statement};
use crate::assign::CompoundOp;

impl From<BinaryOperation> for Option<CompoundOp> {
    fn from(op: BinaryOperation) -> Self {
        match op {
            BinaryOperation::Add => Some(CompoundOp::Add),
            BinaryOperation::Sub => Some(CompoundOp::Sub),
            BinaryOperation::Mul => Some(CompoundOp::Mul),
            BinaryOperation::Div => Some(CompoundOp::Div),
            BinaryOperation::Mod => Some(CompoundOp::Mod),
            BinaryOperation::Pow => Some(CompoundOp::Pow),
            BinaryOperation::Concat => Some(CompoundOp::Concat),
            BinaryOperation::IDiv => Some(CompoundOp::IDiv),
            _ => None,
        }
    }
}

/// Check if two lvalue/rvalue pairs refer to the same location
fn same_location(left: &LValue, right: &RValue) -> bool {
    match (left, right) {
        (LValue::Local(l), RValue::Local(r)) => l == r,
        (LValue::Global(l), RValue::Global(r)) => l == r,
        (LValue::Index(l), RValue::Index(r)) => l == r,
        _ => false,
    }
}

/// Try to convert an assignment to a compound assignment.
/// Returns true if successful.
pub fn try_convert_assign(assign: &mut Assign) -> bool {
    // Only handle single assignments: `x = x op y`
    if assign.left.len() != 1 || assign.right.len() != 1 {
        return false;
    }

    // Don't convert if already compound
    if assign.compound_op.is_some() {
        return false;
    }

    // Don't convert local declarations
    if assign.prefix {
        return false;
    }

    let binary = match &assign.right[0] {
        RValue::Binary(b) => b,
        _ => return false,
    };

    let compound_op: CompoundOp = match Option::<CompoundOp>::from(binary.operation) {
        Some(op) => op,
        None => return false,
    };

    // Check if left side matches the left operand of the binary expression
    if !same_location(&assign.left[0], &binary.left) {
        return false;
    }

    // Convert: replace right side with just the right operand of the binary
    assign.right = vec![*binary.right.clone()];
    assign.compound_op = Some(compound_op);
    true
}

/// Apply compound assignment detection to an entire block
/// Returns the number of compound assignments converted
pub fn detect_compound_assignments(block: &mut Block) -> usize {
    let mut count = 0;
    
    for statement in block.iter_mut() {
        match statement {
            Statement::Assign(assign) => {
                if try_convert_assign(assign) {
                    count += 1;
                }
            }
            Statement::If(if_stat) => {
                count += detect_compound_assignments(&mut if_stat.then_block.lock());
                count += detect_compound_assignments(&mut if_stat.else_block.lock());
            }
            Statement::While(while_stat) => {
                count += detect_compound_assignments(&mut while_stat.block.lock());
            }
            Statement::Repeat(repeat_stat) => {
                count += detect_compound_assignments(&mut repeat_stat.block.lock());
            }
            Statement::NumericFor(for_stat) => {
                count += detect_compound_assignments(&mut for_stat.block.lock());
            }
            Statement::GenericFor(for_stat) => {
                count += detect_compound_assignments(&mut for_stat.block.lock());
            }
            _ => {}
        }
    }
    
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Binary, BinaryOperation, Literal, LValue, RcLocal, RValue};

    /// Regression test for a critical bug where `x = x + y` was rendered as
    /// the semantically broken `x = y`, silently dropping the accumulator
    /// read. This happened because `formatter::format_assign` checked
    /// `assign.compound_op` inside the `if assign.prefix` branch, but
    /// `try_convert_assign` (below) only ever sets `compound_op` on
    /// non-prefix assignments -- making the `+=`-printing branch dead code,
    /// so execution fell through to the generic printer using
    /// `assign.right[0]`, which by that point had already been rewritten to
    /// just the right-hand operand of the original binary expression.
    ///
    /// This test exercises the full pipeline (convert, then print) so it
    /// would catch either half of that regression: a broken
    /// `try_convert_assign` that mangles `assign.right`, or a broken
    /// formatter that ignores `compound_op`.
    #[test]
    fn compound_assign_round_trips_through_formatter() {
        let cases: &[(BinaryOperation, &str)] = &[
            (BinaryOperation::Add, "+="),
            (BinaryOperation::Sub, "-="),
            (BinaryOperation::Mul, "*="),
            (BinaryOperation::Div, "/="),
            (BinaryOperation::Mod, "%="),
            (BinaryOperation::Pow, "^="),
            (BinaryOperation::Concat, "..="),
            (BinaryOperation::IDiv, "//="),
        ];

        for &(op, expected_op_str) in cases {
            let total = RcLocal::new(crate::Local::new(Some("total".to_string())));
            let step: RValue = if op == BinaryOperation::Concat {
                Literal::String(b"y".to_vec()).into()
            } else {
                Literal::Number(1.0).into()
            };

            let mut assign = crate::Assign::new(
                vec![LValue::Local(total.clone())],
                vec![Binary::new(RValue::Local(total.clone()), step, op).into()],
            );

            assert!(
                try_convert_assign(&mut assign),
                "expected {op:?} pattern to be recognized as a compound assignment"
            );
            assert_eq!(assign.compound_op.is_some(), true);

            // The right-hand side must now be *only* the step operand -- if
            // this regresses to still containing the self-referential
            // binary expression, formatting would double up the read.
            assert!(
                !matches!(&assign.right[0], RValue::Binary(_)),
                "right-hand side should have been reduced to just the step operand"
            );

            let printed = assign.to_string();
            assert!(
                printed.contains(expected_op_str),
                "expected printed compound assignment to contain `{expected_op_str}`, got: {printed:?}"
            );
            // The critical regression check: the accumulator variable must
            // still appear on the right-hand side of the operator (i.e. the
            // read of `total` must not have been silently dropped, turning
            // `total += y` into the broken `total = y`).
            assert!(
                !printed.trim_start().starts_with("total ="),
                "compound assignment must not be printed as a plain `total = ...` \
                 assignment (this is the exact bug where `total = total + i` \
                 silently became `total = i`): got {printed:?}"
            );
        }
    }

    /// A local declaration (`local x = x + y`) can never be a compound
    /// assignment -- there is no prior value of `x` to add to. Verifies
    /// `try_convert_assign` correctly refuses prefix assignments.
    #[test]
    fn compound_assign_never_applies_to_local_declarations() {
        let x = RcLocal::new(crate::Local::new(Some("x".to_string())));
        let mut assign = crate::Assign::new(
            vec![LValue::Local(x.clone())],
            vec![Binary::new(
                RValue::Local(x.clone()),
                Literal::Number(1.0).into(),
                BinaryOperation::Add,
            )
            .into()],
        );
        assign.prefix = true;

        assert!(
            !try_convert_assign(&mut assign),
            "a `local` declaration must never be converted into a compound assignment"
        );
        assert!(assign.compound_op.is_none());
    }
}
