use derive_more::From;
use enum_as_inner::EnumAsInner;
use std::fmt;

use crate::{
    formatter::Formatter, type_system::Infer, LocalRw, Reduce, SideEffects, Traverse, Type,
    TypeSystem,
};

#[derive(Debug, From, Clone, PartialEq, PartialOrd, EnumAsInner)]
pub enum Literal {
    Nil,
    Boolean(bool),
    Number(f64),
    Integer(i64),
    String(Vec<u8>),
    Vector(f32, f32, f32, f32),
}

impl Reduce for Literal {
    fn reduce(self) -> crate::RValue {
        self.into()
    }

    fn reduce_condition(self) -> crate::RValue {
        Literal::Boolean(match self {
            Literal::Boolean(false) | Literal::Nil => false,
            Literal::Boolean(true)
            | Literal::Number(_)
            | Literal::Integer(_)
            | Literal::String(_)
            | Literal::Vector(..) => true,
        })
        .into()
    }
}

impl Infer for Literal {
    fn infer<'a: 'b, 'b>(&'a mut self, _: &mut TypeSystem<'b>) -> Type {
        match self {
            Literal::Nil => Type::Nil,
            Literal::Boolean(_) => Type::Boolean,
            Literal::Number(_) => Type::Number,
            Literal::Integer(_) => Type::Number,
            Literal::String(_) => Type::String,
            Literal::Vector(..) => Type::Vector,
        }
    }
}

impl From<&str> for Literal {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl LocalRw for Literal {}

impl SideEffects for Literal {}

impl Traverse for Literal {}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Nil => write!(f, "nil"),
            Literal::Boolean(value) => write!(f, "{}", value),
            &Literal::Number(value) => {
                
                
                debug_assert!(value.is_finite());
                
                let mut buffer = ryu::Buffer::new();
                let printed = buffer.format_finite(value);
                write!(f, "{}", printed.strip_suffix(".0").unwrap_or(printed))
            }
            // Integer constants come from LBC_CONSTANT_INTEGER. Luau source
            // has no distinct integer literal syntax (numbers are just
            // numbers), so emit a plain decimal without a type suffix.
            // The previous `Ni` form produced invalid Luau (`30i`) and
            // confused readers of decompiled tables/rank values.
            &Literal::Integer(value) => write!(f, "{}", value),
            Literal::String(value) => {
                write!(
                    f,
                    "\"{}\"",
                    Formatter::<fmt::Formatter>::escape_string(value)
                )
            }
            Literal::Vector(x, y, z, w) => {
                if *w == 0.0 {
                    write!(f, "Vector3.new({}, {}, {})", x, y, z)
                } else {
                    write!(f, "Vector3.new({}, {}, {}, {})", x, y, z, w)
                }
            }
        }
    }
}


#[cfg(test)]
mod integer_display_tests {
    use super::Literal;

    #[test]
    fn integer_literals_print_without_suffix() {
        assert_eq!(Literal::Integer(30).to_string(), "30");
        assert_eq!(Literal::Integer(-7).to_string(), "-7");
        assert_eq!(Literal::Integer(0).to_string(), "0");
    }
}
