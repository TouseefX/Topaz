use super::list::parse_list;
use nom::{
    number::complete::{le_f32, le_f64, le_i32, le_i64, le_u32, le_u8},
    IResult,
};
use nom_leb128::leb128_usize;

// Constant type tags — must match LuauBytecodeTag in Luau's Bytecode.h.
const CONSTANT_NIL: u8 = 0;
const CONSTANT_BOOLEAN: u8 = 1;
const CONSTANT_NUMBER: u8 = 2;
const CONSTANT_STRING: u8 = 3;
const CONSTANT_IMPORT: u8 = 4;
const CONSTANT_TABLE: u8 = 5;
const CONSTANT_CLOSURE: u8 = 6;
const CONSTANT_VECTOR: u8 = 7;
const CONSTANT_TABLE_WITH_CONSTANTS: u8 = 8; // Added in bytecode v7
const CONSTANT_INTEGER: u8 = 9; // Added in bytecode v8

/// A single entry in a `TableWithConstants`: a key index paired with an
/// optional constant-table index for a pre-filled value.
///
/// `value_index == 0` means "no pre-filled value for this key".
/// Otherwise it is a 1-based index into the constant table.
#[derive(Debug)]
pub struct TableConstantEntry {
    pub key: usize,
    pub value_index: i32,
}

#[derive(Debug)]
pub enum Constant {
    Nil,
    Boolean(bool),
    Number(f64),
    String(usize),
    Import(usize),
    /// Keys-only table template (used by DUPTABLE in bytecode <= v6).
    Table(Vec<usize>),
    Closure(usize),
    Vector(f32, f32, f32, f32),
    /// Table template with pre-filled constant values (bytecode v7+).
    ///
    /// Each entry pairs a string-table key index with an optional constant
    /// index for the value. The decompiler currently treats this identically
    /// to a plain `Table` — only the keys matter for DUPTABLE lifting.
    TableWithConstants(Vec<TableConstantEntry>),
    /// 64-bit integer constant (bytecode v8+).
    Integer(i64),
}

impl Constant {
    pub(crate) fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, tag) = le_u8(input)?;
        match tag {
            CONSTANT_NIL => Ok((input, Constant::Nil)),
            CONSTANT_BOOLEAN => {
                let (input, value) = le_u8(input)?;
                Ok((input, Constant::Boolean(value != 0u8)))
            }
            CONSTANT_NUMBER => {
                let (input, value) = le_f64(input)?;
                Ok((input, Constant::Number(value)))
            }
            CONSTANT_STRING => {
                let (input, string_index) = leb128_usize(input)?;
                Ok((input, Constant::String(string_index)))
            }
            CONSTANT_IMPORT => {
                let (input, import_index) = le_u32(input)?;
                Ok((input, Constant::Import(import_index as usize)))
            }
            CONSTANT_TABLE => {
                let (input, keys) = parse_list(input, leb128_usize)?;
                Ok((input, Constant::Table(keys)))
            }
            CONSTANT_CLOSURE => {
                let (input, f_id) = leb128_usize(input)?;
                Ok((input, Constant::Closure(f_id)))
            }
            CONSTANT_VECTOR => {
                let (input, x) = le_f32(input)?;
                let (input, y) = le_f32(input)?;
                let (input, z) = le_f32(input)?;
                let (input, w) = le_f32(input)?;
                Ok((input, Constant::Vector(x, y, z, w)))
            }
            CONSTANT_TABLE_WITH_CONSTANTS => {
                // Format: varint count, then for each entry: varint key + i32 value_index.
                let (mut input, count) = leb128_usize(input)?;
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let (rest, key) = leb128_usize(input)?;
                    let (rest, value_index) = le_i32(rest)?;
                    input = rest;
                    entries.push(TableConstantEntry { key, value_index });
                }
                Ok((input, Constant::TableWithConstants(entries)))
            }
            CONSTANT_INTEGER => {
                let (input, value) = le_i64(input)?;
                Ok((input, Constant::Integer(value)))
            }
            _ => panic!("unknown constant tag: {}", tag),
        }
    }
}
