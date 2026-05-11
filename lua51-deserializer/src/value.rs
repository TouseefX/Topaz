use enum_as_inner::EnumAsInner;
use nom::{
    bytes::complete::take,
    error::{Error, ErrorKind, ParseError},
    multi::count,
    number::complete::{le_f64, le_u32, le_u64, le_u8},
    Err, IResult,
};

#[derive(Debug, EnumAsInner)]
pub enum Value<'a> {
    Nil,
    Boolean(bool),
    Number(f64),
    String(&'a [u8]),
}

impl<'a> Value<'a> {
    pub fn parse(input: &'a [u8], size_t_width: u8) -> IResult<&'a [u8], Self> {
        let (input, kind) = le_u8(input)?;

        match kind {
            0 => Ok((input, Self::Nil)),
            1 => {
                let (input, value) = le_u8(input)?;

                Ok((input, Self::Boolean(value != 0)))
            }
            3 => {
                let (input, value) = le_f64(input)?;

                Ok((input, Self::Number(value)))
            }
            4 => {
                let (input, value) = parse_string(input, size_t_width)?;

                // TODO: lua bytecode actually allows the string to be completely empty
                // it sets the type to string but gc to NULL
                // this probably causes some weird behavior
                assert!(!value.is_empty());

                // exclude null terminator
                Ok((input, Self::String(&value[..value.len() - 1])))
            }
            _ => Err(Err::Failure(Error::from_error_kind(
                input,
                ErrorKind::Switch,
            ))),
        }
    }
}

pub fn parse_size_t(input: &[u8], size_t_width: u8) -> IResult<&[u8], usize> {
    match size_t_width {
        4 => {
            let (input, val) = le_u32(input)?;
            Ok((input, val as usize))
        }
        8 => {
            let (input, val) = le_u64(input)?;
            Ok((input, val as usize))
        }
        _ => panic!("Unsupported size_t width: {}", size_t_width),
    }
}

pub fn parse_string(input: &[u8], size_t_width: u8) -> IResult<&[u8], &[u8]> {
    let (input, string_length) = parse_size_t(input, size_t_width)?;
    take(string_length)(input)
}

pub fn parse_strings(input: &[u8], size_t_width: u8) -> IResult<&[u8], Vec<&[u8]>> {
    let (input, string_count) = le_u32(input)?;
    let (input, strings) = count(|i| parse_string(i, size_t_width), string_count as usize)(input)?;

    Ok((input, strings))
}
