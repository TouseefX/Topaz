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

fn format_number(f: &mut fmt::Formatter, value: f64) -> fmt::Result {
    if value.is_infinite() {
        if value.is_sign_positive() {
            write!(f, "math.huge")
        } else {
            write!(f, "-math.huge")
        }
    } else if value.is_nan() {
        write!(f, "(0 / 0)")
    } else if (value - std::f64::consts::PI).abs() <= 1e-12 {
        write!(f, "math.pi")
    } else if (value + std::f64::consts::PI).abs() <= 1e-12 {
        write!(f, "-math.pi")
    } else {
        let mut buffer = ryu::Buffer::new();
        let printed = buffer.format_finite(value);
        write!(f, "{}", printed.strip_suffix(".0").unwrap_or(printed))
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Nil => write!(f, "nil"),
            Literal::Boolean(value) => write!(f, "{}", value),
            &Literal::Number(value) => format_number(f, value),
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
                write!(f, "Vector3.new(")?;
                format_number(f, *x as f64)?;
                write!(f, ", ")?;
                format_number(f, *y as f64)?;
                write!(f, ", ")?;
                format_number(f, *z as f64)?;
                if *w != 0.0 {
                    write!(f, ", ")?;
                    format_number(f, *w as f64)?;
                }
                write!(f, ")")
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

    #[test]
    fn number_literals_print_pi_and_huge() {
        assert_eq!(Literal::Number(std::f64::consts::PI).to_string(), "math.pi");
        assert_eq!(
            Literal::Number(-std::f64::consts::PI).to_string(),
            "-math.pi"
        );
        assert_eq!(Literal::Number(f64::INFINITY).to_string(), "math.huge");
        assert_eq!(Literal::Number(f64::NEG_INFINITY).to_string(), "-math.huge");
        assert_eq!(Literal::Number(1.0).to_string(), "1");
    }

    #[test]
    fn vector_literals_strip_dot_zero() {
        assert_eq!(
            Literal::Vector(0.0, 1.0, 2.5, 0.0).to_string(),
            "Vector3.new(0, 1, 2.5)"
        );
    }
}
