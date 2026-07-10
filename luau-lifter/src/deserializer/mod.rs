pub mod bytecode;
pub mod chunk;
pub mod constant;
pub mod function;
pub mod leb128;

/// Parse a Luau bytecode blob. Returns a `Bytecode::Chunk(Chunk)` on
/// success, or a `Bytecode::Error(String)` if the bytecode itself is
/// an error blob (status byte 0). Any structural parsing failure is
/// returned as `Err(String)`.
pub fn deserialize(bytecode: &[u8], encode_key: u8) -> Result<bytecode::Bytecode, String> {
    bytecode::Bytecode::parse(bytecode, encode_key).map_err(|e| e.to_string())
}
