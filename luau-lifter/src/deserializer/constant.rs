//! Type definitions for Luau constant table entries.
//!
//! These mirror the upstream `LBC_CONSTANT_*` enum values. The actual
//! parsing of constants happens in `function.rs` (because the
//! constant table is interleaved with other function-level sections
//! and we parse it manually for better control over the exact byte
//! layout).
//!
//! Tag 10 (`CLASS_SHAPE`) is consumed by the loadsafe IR decoder and
//! lowered to `Nil` (no first-class AST form yet).
//!
//! See the Luau source for the authoritative definition:
//! <https://github.com/luau-lang/luau/blob/master/Common/include/Luau/Bytecode.h>

/// A key/value pair in a `LBC_CONSTANT_TABLE_WITH_CONSTANTS` entry.
#[derive(Debug, Clone, Copy)]
pub struct TableConstantEntry {
    /// Index into the constant table for the key.
    pub key: usize,
    /// Value of the entry (as a 32-bit signed integer; usually an
    /// index into the constant table too).
    pub value_index: i32,
}

/// A single constant from a function's constant table.
#[derive(Debug, Clone)]
pub enum Constant {
    /// Tag 0.
    Nil,
    /// Tag 1, followed by a `u8` (0 = false, anything else = true).
    Boolean(bool),
    /// Tag 2, followed by a little-endian `f64`.
    Number(f64),
    /// Tag 3, followed by a LEB128 varint that is a 1-based index into
    /// the chunk-level string table. 0 means "no string".
    String(usize),
    /// Tag 4, followed by a 4-byte import descriptor.
    /// `import_index` is opaque; the upstream VM interprets it as
    /// `(count << 8) | id` where `count` is the number of dots in
    /// the import path.
    Import(usize),
    /// Tag 5: a table shape with just keys.
    Table(Vec<usize>),
    /// Tag 6: a closure (an index into the function table).
    Closure(usize),
    /// Tag 7: a 4-component float vector.
    Vector(f32, f32, f32, f32),
    /// Tag 8: a table shape with keys and packed constant values.
    TableWithConstants(Vec<TableConstantEntry>),
    /// Tag 9: a 64-bit signed integer.
    Integer(i64),
}
