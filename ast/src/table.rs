use crate::{
    formatter::Formatter, Literal, LocalRw, RValue, RcLocal, Reduce, SideEffects, Traverse,
};

use std::{fmt, iter};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table(pub Vec<(Option<RValue>, RValue)>);

impl Reduce for Table {
    fn reduce(self) -> RValue {
        self.into()
    }

    fn reduce_condition(self) -> RValue {
        if self.has_side_effects() {
            
            self.into()
        } else {
            Literal::Boolean(true).into()
        }
    }
}


impl LocalRw for Table {
    fn values_read(&self) -> Vec<&RcLocal> {
        self.0
            .iter()
            .flat_map(|(k, v)| k.iter().chain(iter::once(v)))
            .flat_map(|v| v.values_read())
            .collect()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        self.0
            .iter_mut()
            .flat_map(|(k, v)| k.iter_mut().chain(iter::once(v)))
            .flat_map(|v| v.values_read_mut())
            .collect()
    }
}

impl Traverse for Table {
    fn rvalues_mut(&mut self) -> Vec<&mut RValue> {
        self.0
            .iter_mut()
            .flat_map(|(k, v)| k.iter_mut().chain(iter::once(v)))
            .collect()
    }

    fn rvalues(&self) -> Vec<&RValue> {
        self.0
            .iter()
            .flat_map(|(k, v)| k.iter().chain(iter::once(v)))
            .collect()
    }
}

impl SideEffects for Table {
    fn has_side_effects(&self) -> bool {
        self.0
            .iter()
            .flat_map(|(k, v)| k.iter().chain(iter::once(v)))
            .any(|r| r.has_side_effects())
    }
}


impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Formatter {
            indentation_level: 0,
            indentation_mode: Default::default(),
            output: f,
        }
        .format_table(self)
    }
}
