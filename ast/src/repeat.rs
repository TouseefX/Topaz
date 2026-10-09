use crate::{
    formatter::Formatter, has_side_effects, share_block, shared_blocks_equal, Block, LocalRw,
    RValue, RcLocal, SharedBlock, Traverse,
};
use std::fmt;


#[derive(Debug, Clone)]
pub struct Repeat {
    pub condition: RValue,
    pub block: SharedBlock,
}

impl PartialEq for Repeat {
    fn eq(&self, other: &Self) -> bool {
        self.condition == other.condition && shared_blocks_equal(&self.block, &other.block)
    }
}

has_side_effects!(Repeat);

impl Repeat {
    pub fn new(condition: RValue, block: Block) -> Self {
        Self {
            condition,
            block: share_block(block),
        }
    }
}

impl Traverse for Repeat {
    fn rvalues_mut(&mut self) -> Vec<&mut RValue> {
        vec![&mut self.condition]
    }

    fn rvalues(&self) -> Vec<&RValue> {
        vec![&self.condition]
    }
}

impl LocalRw for Repeat {
    fn values_read(&self) -> Vec<&RcLocal> {
        self.condition.values_read()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        self.condition.values_read_mut()
    }
}

impl fmt::Display for Repeat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Formatter {
            indentation_level: 0,
            indentation_mode: Default::default(),
            output: f,
        }
        .format_repeat(self)
    }
}
