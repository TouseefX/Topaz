use crate::{
    formatter::Formatter, Literal, LocalRw, RValue, RcLocal, Reduce, SideEffects, Traverse,
};

use std::{fmt, iter};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table(pub Vec<(Option<RValue>, RValue)>);

fn field_key_id(key: &RValue) -> Option<String> {
    match key {
        RValue::Literal(Literal::String(s)) => {
            Some(format!("s:{}", String::from_utf8_lossy(s)))
        }
        RValue::Literal(Literal::Number(n)) => Some(format!("n:{n}")),
        RValue::Literal(Literal::Integer(n)) => Some(format!("i:{n}")),
        RValue::Literal(Literal::Boolean(b)) => Some(format!("b:{b}")),
        _ => None,
    }
}

impl Table {
    /// Insert `key = value`, replacing a previous entry with the same
    /// literal key when that entry was `nil` or otherwise side-effect free.
    ///
    /// DUPTABLE materialises keys as `nil`; SETTABLEKS then fills them.
    /// Pushing a second copy of the key made table-cleanup split the
    /// constructor back into `t.key = value` (CameraShaker.new).
    pub fn put_field(&mut self, key: RValue, value: RValue) {
        if let Some(id) = field_key_id(&key) {
            if let Some((_, slot)) = self.0.iter_mut().find(|(k, _)| {
                k.as_ref().and_then(field_key_id).as_deref() == Some(id.as_str())
            }) {
                if matches!(slot, RValue::Literal(Literal::Nil)) || !slot.has_side_effects()
                {
                    *slot = value;
                    return;
                }
            }
        }
        self.0.push((Some(key), value));
    }

    /// `{ k = nil }` does not create `k` in Lua. Drop leftover DUPTABLE
    /// placeholders that were never filled.
    pub fn drop_nil_fields(&mut self) {
        self.0.retain(|(k, v)| {
            !(k.is_some() && matches!(v, RValue::Literal(Literal::Nil)))
        });
    }
}

impl Reduce for Table {
    fn reduce(self) -> RValue {
        self.into()
    }

    fn reduce_condition(mut self) -> RValue {
        if self.has_side_effects() {
            self.0.retain(|(key, value)| {
                key.as_ref().is_some_and(SideEffects::has_side_effects) || value.has_side_effects()
            });
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
