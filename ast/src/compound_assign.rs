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
