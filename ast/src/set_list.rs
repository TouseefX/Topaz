use crate::{formatter, LocalRw, RValue, RcLocal, SideEffects, Traverse};

#[derive(Debug, Clone, PartialEq)]
pub struct SetList {
    pub object_local: RcLocal,
    pub index: usize,
    pub values: Vec<RValue>,
    pub tail: Option<RValue>,
}

impl SetList {
    pub fn new(
        object_local: RcLocal,
        index: usize,
        values: Vec<RValue>,
        tail: Option<RValue>,
    ) -> Self {
        Self {
            object_local,
            index,
            values,
            tail,
        }
    }
}

impl LocalRw for SetList {
    fn values_read(&self) -> Vec<&RcLocal> {
        let tail_locals = self
            .tail
            .as_ref()
            .map(|t| t.values_read())
            .unwrap_or_default();
        std::iter::once(&self.object_local)
            .chain(self.values.iter().flat_map(|rvalue| rvalue.values_read()))
            .chain(tail_locals)
            .collect()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        let tail_locals = self
            .tail
            .as_mut()
            .map(|t| t.values_read_mut())
            .unwrap_or_default();
        std::iter::once(&mut self.object_local)
            .chain(
                self.values
                    .iter_mut()
                    .flat_map(|rvalue| rvalue.values_read_mut()),
            )
            .chain(tail_locals)
            .collect()
    }
}

impl SideEffects for SetList {
    fn has_side_effects(&self) -> bool {
        self.values
            .iter()
            .chain(self.tail.as_ref())
            .any(|rvalue| rvalue.has_side_effects())
    }
}

impl Traverse for SetList {
    fn rvalues(&self) -> Vec<&RValue> {
        self.values.iter().chain(self.tail.as_ref()).collect()
    }

    fn rvalues_mut(&mut self) -> Vec<&mut RValue> {
        self.values.iter_mut().chain(self.tail.as_mut()).collect()
    }
}

impl std::fmt::Display for SetList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.index == 1 && self.tail.is_none() {
            if self.values.is_empty() {
                write!(f, "{} = {{}}", self.object_local)
            } else if self.values.len() == 1 {
                write!(f, "{} = {{{}}}", self.object_local, self.values[0])
            } else {
                write!(
                    f,
                    "{} = {{\n\t{}\n}}",
                    self.object_local,
                    self.values
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",\n\t")
                )
            }
        } else if self.index == 1 && self.tail.is_some() {
            let mut all_values = self.values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>();
            
            if let Some(tail) = &self.tail {
                all_values.push(tail.to_string());
            }
            
            if all_values.len() == 1 {
                write!(f, "{} = {{{}}}", self.object_local, all_values[0])
            } else {
                write!(
                    f,
                    "{} = {{\n\t{}\n}}",
                    self.object_local,
                    all_values.join(",\n\t")
                )
            }
        } else {
            let mut result = String::new();
            for (i, value) in self.values.iter().enumerate() {
                if i > 0 {
                    result.push_str("\n");
                }
                result.push_str(&format!(
                    "{}[{}] = {}",
                    self.object_local,
                    self.index + i,
                    value
                ));
            }
            if let Some(tail) = &self.tail {
                if !self.values.is_empty() {
                    result.push_str("\n");
                }
                result.push_str(&format!(
                    "{}[{}] = {}",
                    self.object_local,
                    self.index + self.values.len(),
                    tail
                ));
            }
            write!(f, "{}", result)
        }
    }
}
