//! Raw-constant IR decoder mirrored from luaur-vm `loadsafe` / C++ `lvmload.cpp`.
//!
//! # Why this exists
//!
//! luaur's `luau_load` → `loadsafe` is a faithful engine loader. After it runs,
//! constant table entries are **live `TValue`s**:
//! - `LBC_CONSTANT_IMPORT` has already been resolved via `resolveImportSafe`
//! - table templates are real tables, closures are real closures, etc.
//!
//! A decompiler needs the **serialized** form instead:
//! - `Import(iid)` with the packed 10-10-10-2 descriptor intact
//! - `Table(keys)` / `TableWithConstants(entries)` as shape metadata
//! - `String(1-based index)` into the chunk string table
//! - `Closure(proto_index)` as a child-proto index
//!
//! This module walks the **same byte layout** as `loadsafe`, but stores raw
//! [`Constant`] tags. Instruction words are still decoded with an optional
//! Roblox encode key (`op' = op * key` on the low byte).
//!
//! # Layout (chunk)
//!
//! ```text
//! u8  version                    // LBC_VERSION (3..=11 in luaur; we also allow 12)
//! u8  types_version              // if version >= 4
//! varint string_count + strings
//! userdata type map              // if types_version == 3: (u8 idx + varint name)* + 0
//! varint proto_count
//!   [version >= 12: varint proto_size]
//!   proto body...
//! varint main_proto_index
//! ```
//!
//! # Layout (proto body) — same order as loadsafe
//!
//! ```text
//! u8 maxstack, numparams, nups, is_vararg
//! u8 flags                       // if version >= 4
//! typeinfo: varint typesize + typesize bytes   // if version >= 4
//! varint sizecode + sizecode × u32 instructions
//! varint sizek + sizek × (tag + payload)       // RAW constants (no resolveImport)
//! varint sizep + sizep × varint child_proto
//! varint linedefined, debugname
//! lineinfo flag + optional line data
//! debuginfo flag + optional locals/upvalues
//! feedback vector                // if version >= 11
//! cost varint                    // if version >= 12 && LPF_INLINABLE
//! ```
//!
//! # Constant tags (keep raw — do NOT resolve)
//!
//! | Tag | Name                    | Payload                                      |
//! |-----|-------------------------|----------------------------------------------|
//! | 0   | NIL                     | —                                            |
//! | 1   | BOOLEAN                 | u8                                           |
//! | 2   | NUMBER                  | f64 LE                                       |
//! | 3   | STRING                  | varint string_table_index (1-based, 0=none)  |
//! | 4   | IMPORT                  | u32 import_id  **← kept as-is for decompile**|
//! | 5   | TABLE                   | varint n; n × varint key_idx                 |
//! | 6   | CLOSURE                 | varint proto_idx                             |
//! | 7   | VECTOR                  | 4 × f32 LE                                   |
//! | 8   | TABLE_WITH_CONSTANTS    | varint n; n × (varint key + i32 value_idx)   |
//! | 9   | INTEGER                 | u8 sign + varint64 magnitude                 |
//! | 10  | CLASS_SHAPE             | varint class; nprops; nmethods; members…     |
//!
//! Reference: `luaur/crates/luaur-vm/src/functions/loadsafe.rs` and
//! `luau/VM/src/lvmload.cpp`.

use super::bytecode::Bytecode;
use super::chunk::Chunk;
use super::function::ParseError;

/// Decode a bytecode blob into Topaz IR using the loadsafe layout.
///
/// `encode_key` is applied only when expanding instruction op-bytes
/// (Roblox client dumps use 203; plain Luau / luaur-compile use 1).
/// Constant payloads are never encode-keyed.
///
/// # Combine with encode key (luaur + Roblox)
///
/// A plain-only loader (including stock luaur `luau_load`) treats keyed
/// instruction streams as garbage and rejects them. Topaz keeps **one**
/// IR decoder (this module) and only varies `encode_key`:
/// - `1` — plain Luau / luaur-compile / Studio-style dumps
/// - `203` — common Roblox client encode key
/// - other — custom executor keys
///
/// Default decompile tries key `1` first, then `detect_encode_key`.
pub fn decode(data: &[u8], encode_key: u8) -> Result<Bytecode, ParseError> {
    // Implementation currently shares the hardened Chunk/Function parsers,
    // which follow the loadsafe section order and keep raw Constant tags
    // (including Import(iid)). This entry point exists so the decompiler
    // default path and docs name the contract explicitly.
    Bytecode::parse(data, encode_key)
}

/// Same as [`decode`], but returns a `Chunk` or a human-readable error string.
pub fn decode_chunk(data: &[u8], encode_key: u8) -> Result<Chunk, String> {
    match decode(data, encode_key).map_err(|e| e.to_string())? {
        Bytecode::Chunk(c) => Ok(c),
        Bytecode::Error(msg) => Err(msg),
    }
}

/// Whether `version` is within luaur's supported open-source range
/// (`LBC_VERSION_MIN..=LBC_VERSION_MAX`, currently 3..=11).
pub fn is_luaur_version(version: u8) -> bool {
    use luaur::common::enums::luau_bytecode_tag::{LBC_VERSION_MAX, LBC_VERSION_MIN};
    version >= LBC_VERSION_MIN.0 as u8 && version <= LBC_VERSION_MAX.0 as u8
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::deserializer::constant::Constant;

    fn write_varint(out: &mut Vec<u8>, mut v: u32) {
        loop {
            let mut byte = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    fn minimal_proto_prefix(out: &mut Vec<u8>, version: u8) {
        out.push(version);
        if version >= 4 {
            out.push(1); // types_version
        }
        write_varint(out, 0); // strings
        write_varint(out, 1); // one function
        out.extend_from_slice(&[1, 0, 0, 0]); // header
        if version >= 4 {
            out.push(0); // flags
            write_varint(out, 0); // typesize
        }
    }

    fn finish_empty_proto(out: &mut Vec<u8>, insn: u32) {
        write_varint(out, 1); // codesize
        out.extend_from_slice(&insn.to_le_bytes());
        write_varint(out, 0); // sizek
        write_varint(out, 0); // children
        write_varint(out, 0); // line
        write_varint(out, 0); // name
        out.push(0); // no lineinfo
        out.push(0); // no debuginfo
        write_varint(out, 0); // main
    }

    #[test]
    fn import_constant_stays_raw_not_resolved() {
        let mut out = Vec::new();
        out.push(6);
        out.push(1);
        write_varint(&mut out, 1);
        write_varint(&mut out, 4);
        out.extend_from_slice(b"game");
        write_varint(&mut out, 1);
        out.extend_from_slice(&[1, 0, 0, 0]);
        out.push(0);
        write_varint(&mut out, 0);
        write_varint(&mut out, 1);
        out.extend_from_slice(&(0x16u32 | (1 << 16)).to_le_bytes());
        write_varint(&mut out, 2);
        out.push(4); // IMPORT
        out.extend_from_slice(&(1u32 << 30).to_le_bytes());
        out.push(0); // NIL
        write_varint(&mut out, 0);
        write_varint(&mut out, 0);
        write_varint(&mut out, 0);
        out.push(0);
        out.push(0);
        write_varint(&mut out, 0);

        let chunk = decode_chunk(&out, 1).expect("decode");
        match chunk.functions[0].constants[0] {
            Constant::Import(iid) => assert_eq!(iid, (1u32 << 30) as usize),
            ref o => panic!("expected Import, got {o:?}"),
        }
    }

    #[test]
    fn keyed_instructions_need_encode_key() {
        // stored_op * 203 == plain_op  =>  stored = plain * inv(203), inv=227
        let inv: u8 = 227;
        let plain_ret: u8 = 0x16;
        let stored_op = plain_ret.wrapping_mul(inv);
        assert_eq!(stored_op.wrapping_mul(203), plain_ret);

        let mut out = Vec::new();
        minimal_proto_prefix(&mut out, 6);
        let insn = (stored_op as u32) | (1u32 << 16);
        finish_empty_proto(&mut out, insn);

        assert!(
            decode_chunk(&out, 1).is_err(),
            "plain key must reject Roblox-keyed opcodes"
        );
        let chunk = decode_chunk(&out, 203).expect("key 203 must accept");
        assert_eq!(chunk.functions.len(), 1);
        assert!(!chunk.functions[0].instructions.is_empty());
    }

    #[test]
    fn luaur_version_range_matches_common() {
        assert!(is_luaur_version(3));
        assert!(is_luaur_version(6));
        assert!(is_luaur_version(11));
        assert!(!is_luaur_version(0));
        assert!(!is_luaur_version(2));
        assert!(!is_luaur_version(12));
    }
}
