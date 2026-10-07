use parking_lot::Mutex;
use triomphe::Arc;

use crate::{
    LocalRw, RcLocal, SideEffects, Traverse, block_has_side_effects, formatter::Formatter,
    shared_blocks_equal,
};

use super::{Block, RValue};

use std::fmt;

#[derive(Debug, Clone)]
pub struct If {
    pub condition: RValue,
    pub then_block: Arc<Mutex<Block>>,
    pub else_block: Arc<Mutex<Block>>,
}

impl PartialEq for If {
    fn eq(&self, other: &Self) -> bool {
        self.condition == other.condition
            && shared_blocks_equal(&self.then_block, &other.then_block)
            && shared_blocks_equal(&self.else_block, &other.else_block)
    }
}

impl If {
    pub fn new(condition: RValue, then_block: Block, else_block: Block) -> Self {
        Self {
            condition,
            then_block: Arc::new(then_block.into()),
            else_block: Arc::new(else_block.into()),
        }
    }
}

impl Traverse for If {
    fn rvalues_mut(&mut self) -> Vec<&mut RValue> {
        vec![&mut self.condition]
    }

    fn rvalues(&self) -> Vec<&RValue> {
        vec![&self.condition]
    }
}

impl SideEffects for If {
    fn has_side_effects(&self) -> bool {
        self.condition.has_side_effects()
            || block_has_side_effects(&self.then_block)
            || block_has_side_effects(&self.else_block)
    }
}

impl LocalRw for If {
    fn values_read(&self) -> Vec<&RcLocal> {
        self.condition.values_read()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        self.condition.values_read_mut()
    }
}

impl fmt::Display for If {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Formatter {
            indentation_level: 0,
            indentation_mode: Default::default(),
            output: f,
        }
        .format_if(self)
    }
}
