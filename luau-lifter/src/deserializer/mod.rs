use nom::{bytes::complete::take, IResult};
use nom_leb128::leb128_usize;

pub mod bytecode;
pub mod chunk;
pub mod constant;
pub mod function;
mod list;

fn parse_string(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (input, length) = leb128_usize(input)?;
    let (input, bytes) = take(length)(input)?;
    Ok((input, bytes.to_owned()))
}

pub fn deserialize(bytecode: &[u8], encode_key: u8) -> Result<bytecode::Bytecode, String> {
    match bytecode::Bytecode::parse(bytecode, encode_key) {
        Ok((_, deserialized_bytecode)) => Ok(deserialized_bytecode),
        Err(err) => Err(err.to_string()),
    }
}

