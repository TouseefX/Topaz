//! Manual LEB128 (Little-Endian Base 128) varint decoder.
//!
//! We don't use `nom_leb128` here because:
//!   * We want a simple `Result<(u32, usize)>` API that returns the
//!     value AND the number of bytes consumed in one call (so we can
//!     advance our offset).
//!   * We want a single function that does what `nom_leb128::leb128_u32`
//!     does, without the nom combinator machinery.
//!
//! LEB128 encoding: each byte contributes 7 low bits to the value; the
//! high bit indicates whether more bytes follow.

/// Read an unsigned LEB128 varint from `data` at `*offset`.
/// Returns `(value, bytes_consumed)` on success.
pub fn read_leb128_u32(data: &[u8], offset: usize) -> Result<(u32, usize), String> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    let mut i = 0usize;
    loop {
        if offset + i >= data.len() {
            return Err("LEB128: unexpected EOF".into());
        }
        let byte = data[offset + i];
        i += 1;
        if shift >= 32 {
            return Err(format!(
                "LEB128: varint exceeds u32 (shift={}, byte=0x{:02x})",
                shift, byte
            ));
        }
        result |= ((byte & 0x7f) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok((result, i));
        }
        shift += 7;
    }
}

/// Read an unsigned 64-bit LEB128 varint from `data` at `offset`.
///
/// Used for `LBC_CONSTANT_INTEGER` magnitudes (Luau bytecode version 8+)
/// which can exceed 32 bits.
pub fn read_leb128_u64(data: &[u8], offset: usize) -> Result<(u64, usize), String> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut i = 0usize;
    loop {
        if offset + i >= data.len() {
            return Err("LEB128: unexpected EOF".into());
        }
        let byte = data[offset + i];
        i += 1;
        if shift >= 64 {
            return Err(format!(
                "LEB128: varint exceeds u64 (shift={}, byte=0x{:02x})",
                shift, byte
            ));
        }
        result |= ((byte & 0x7f) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok((result, i));
        }
        shift += 7;
    }
}
