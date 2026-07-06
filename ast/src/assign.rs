use std::fmt;

use crate::{formatter::Formatter, RcLocal, SideEffects, Traverse};

use super::{LValue, LocalRw, RValue};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    IDiv,
}

impl fmt::Display for CompoundOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompoundOp::Add => write!(f, "+="),
            CompoundOp::Sub => write!(f, "-="),
            CompoundOp::Mul => write!(f, "*="),
            CompoundOp::Div => write!(f, "/="),
            CompoundOp::Mod => write!(f, "%="),
            CompoundOp::Pow => write!(f, "^="),
            CompoundOp::Concat => write!(f, "..="),
            CompoundOp::IDiv => write!(f, "//="),
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub left: Vec<LValue>,
    pub right: Vec<RValue>,
    pub prefix: bool,
    pub parallel: bool,
    pub compound_op: Option<CompoundOp>,
}

impl Assign {
    pub fn new(left: Vec<LValue>, right: Vec<RValue>) -> Self {
        Self {
            left,
            right,
            prefix: false,
            parallel: false,
            compound_op: None,
        }
    }
}

impl Traverse for Assign {
    fn lvalues_mut(&mut self) -> Vec<&mut LValue> {
        self.left.iter_mut().collect()
    }

    fn rvalues_mut(&mut self) -> Vec<&mut RValue> {
        self.right.iter_mut().collect()
    }

    fn rvalues(&self) -> Vec<&RValue> {
        self.right.iter().collect()
    }
}

impl SideEffects for Assign {
    fn has_side_effects(&self) -> bool {
        self.right.iter().any(|r| r.has_side_effects())
            || self.left.iter().any(|l| l.has_side_effects())
    }
}

impl LocalRw for Assign {
    fn values_read(&self) -> Vec<&RcLocal> {
        self.left
            .iter()
            .flat_map(|l| l.values_read())
            .chain(self.right.iter().flat_map(|r| r.values_read()))
            .collect()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        self.left
            .iter_mut()
            .flat_map(|l| l.values_read_mut())
            .chain(self.right.iter_mut().flat_map(|r| r.values_read_mut()))
            .collect()
    }

    fn values_written(&self) -> Vec<&RcLocal> {
        self.left.iter().flat_map(|l| l.values_written()).collect()
    }

    fn values_written_mut(&mut self) -> Vec<&mut RcLocal> {
        self.left
            .iter_mut()
            .flat_map(|l| l.values_written_mut())
            .collect()
    }
}

impl fmt::Display for Assign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Formatter {
            indentation_level: 0,
            indentation_mode: Default::default(),
            output: f,
        }
        .format_assign(self)
    }
}
