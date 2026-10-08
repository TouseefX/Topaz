use std::convert::TryInto;

use super::function::{Function, ParseError};
use super::leb128::{read_count, read_leb128_u32};

const MAX_STRINGS: u32 = 200_000;
const MAX_FUNCTIONS: u32 = 50_000;
const MAX_USERDATA: u32 = 512;

#[derive(Debug)]
pub struct Chunk {
    pub string_table: Vec<Vec<u8>>,
    pub functions: Vec<Function>,
    /// Index into `functions` of the main function prototype.
    pub main: u32,
}

impl Chunk {
    /// Parse a complete Luau bytecode blob into a `Chunk`.
    ///
    /// Format (from upstream `BytecodeBuilder::finalize`):
    /// ```text
    ///   u8 version             // LBC_VERSION (3..=11)
    ///   u8 types_version       // LBC_TYPE_VERSION (1..=3) if version >= 4
    ///   varint(string_count) + string_count × (varint length + bytes)
    ///   userdata type mapping:
    ///     (u8 idx + varint nameRef) * terminated by u8(0)
    ///   varint(function_count) + function_count × Function
    ///   varint(main_function_index)
    /// ```
    pub fn parse(data: &[u8], encode_key: u8) -> Result<Self, ParseError> {
        let start = 0usize;
        let mut offset = 0usize;

        macro_rules! need {
            ($n:expr) => {{
                if data.len() - offset < $n {
                    return Err(ParseError {
                        message: format!(
                            "unexpected EOF: needed {} bytes at offset {}",
                            $n,
                            offset
                        ),
                        position: start,
                    });
                }
            }};
        }

        // -- 1-byte version --
        need!(1);
        let version = data[offset];
        offset += 1;

        // -- 1-byte types version (only if version >= 4) --
        let types_version = if version >= 4 {
            need!(1);
            let v = data[offset];
            offset += 1;
            v
        } else {
            0
        };
        // Studio always writes types_version 3. Unknown future values
        // still have a typesize payload we skip, so don't abort the
        // whole blob — a wrong guess here used to reject real scripts.

        // -- String table --
        let (string_count, advance) = read_count(data, offset, MAX_STRINGS, "string_count").map_err(|e| {
            ParseError {
                message: e,
                position: start,
            }
        })?;
        offset += advance;
        let mut string_table = Vec::with_capacity(string_count as usize);
        for i in 0..string_count {
            if i & 4095 == 0 && cfg::past_decompile_deadline() {
                return Err(ParseError {
                    message: "decode timed out (string table)".into(),
                    position: start,
                });
            }
            let (length, advance) = read_leb128_u32(data, offset).map_err(|e| {
                ParseError {
                    message: format!("string length: {e}"),
                    position: start,
                }
            })?;
            offset += advance;
            need!(length as usize);
            string_table.push(data[offset..offset + length as usize].to_vec());
            offset += length as usize;
        }

        // -- Userdata type mapping (only if types_version >= 3) --
        if types_version >= 3 {
            let mut userdata_n = 0u32;
            loop {
                need!(1);
                let idx = data[offset];
                offset += 1;
                if idx == 0 {
                    break;
                }
                userdata_n += 1;
                if userdata_n > MAX_USERDATA {
                    return Err(ParseError {
                        message: format!("userdata map exceeds cap {MAX_USERDATA}"),
                        position: start,
                    });
                }
                // Skip the varint nameRef.
                let (_, advance) = read_leb128_u32(data, offset).map_err(|e| {
                    ParseError {
                        message: format!("userdata nameRef: {e}"),
                        position: start,
                    }
                })?;
                offset += advance;
            }
        }

        // -- Function table --
        let (function_count, advance) = read_count(data, offset, MAX_FUNCTIONS, "function_count").map_err(|e| {
            ParseError {
                message: e,
                position: start,
            }
        })?;
        offset += advance;
        let mut functions = Vec::with_capacity(function_count as usize);
        for i in 0..function_count {
            if i & 15 == 0 && cfg::past_decompile_deadline() {
                return Err(ParseError {
                    message: "decode timed out (function table)".into(),
                    position: start,
                });
            }
            // Version 12+ prefixes each proto with a size varint so loaders
            // can skip unknown trailing fields. Consume it and pass the
            // remaining budget into Function::parse.
            let proto_size_limit = if version >= 12 {
                let (proto_size, adv) = read_leb128_u32(data, offset).map_err(|e| {
                    ParseError {
                        message: format!("proto size: {e}"),
                        position: start,
                    }
                })?;
                offset += adv;
                Some(proto_size as usize)
            } else {
                None
            };
            let proto_start = offset;
            let f = Function::parse(data, &mut offset, encode_key, &string_table, version)?;
            if let Some(proto_size) = proto_size_limit {
                // Skip any unknown trailing bytes the current parser
                // doesn't understand (cost model, future fields, …).
                let consumed = offset.saturating_sub(proto_start);
                if consumed > proto_size {
                    return Err(ParseError {
                        message: format!(
                            "proto overran declared size: consumed {} > size {}",
                            consumed, proto_size
                        ),
                        position: proto_start,
                    });
                }
                offset = proto_start + proto_size;
            }
            functions.push(f);
        }

        // -- Main function index --
        let (main, advance) = read_leb128_u32(data, offset).map_err(|e| {
            ParseError {
                message: format!("main idx: {e}"),
                position: start,
            }
        })?;
        offset += advance;

        Ok(Chunk {
            string_table,
            functions,
            main,
        })
    }
}

// `ParseError` is used by callers; the `TryInto` import is kept to
// silence unused-import warnings if we later need it.
#[allow(dead_code)]
fn _unused() {
    let _: Option<[u8; 4]> = [0u8; 4].try_into().ok();
}
