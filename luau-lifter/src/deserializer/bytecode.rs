use super::chunk::Chunk;
use super::function::ParseError;

#[derive(Debug)]
pub enum Bytecode {
    Error(String),
    Chunk(Chunk),
}

impl Bytecode {
    /// Parse a Luau bytecode blob.
    ///
    /// Format: `[u8 status][...]`. If status is 0, the rest is an
    /// error string. If status is in `3..=12`, it's a valid chunk
    /// encoded with the corresponding bytecode version.
    pub fn parse(data: &[u8], encode_key: u8) -> Result<Self, ParseError> {
        if data.is_empty() {
            return Err(ParseError {
                message: "empty bytecode".into(),
                position: 0,
            });
        }
        let status = data[0];
        match status {
            0 => {
                // Error blob: the rest is the message.
                Ok(Bytecode::Error(
                    String::from_utf8_lossy(&data[1..]).into_owned(),
                ))
            }
            // LBC_VERSION_MAX is currently 11 publicly, but version 12
            // (proto size prefix + cost model) is already shipping in
            // experimental / Roblox builds. Accept it so we can skip
            // unknown trailing fields via the size prefix.
            3..=12 => Chunk::parse(data, encode_key).map(Bytecode::Chunk),
            other => Err(ParseError {
                message: format!("unsupported bytecode version {}", other),
                position: 0,
            }),
        }
    }
}
