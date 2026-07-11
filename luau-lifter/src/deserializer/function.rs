use std::convert::TryInto;

use crate::{instruction::*, op_code::OpCode};

use super::{
    constant::Constant,
    leb128::{read_leb128_u32, read_leb128_u64},
};

// Constants from upstream Luau (LBC_* enum values).
const LBC_CONSTANT_NIL: u8 = 0;
const LBC_CONSTANT_BOOLEAN: u8 = 1;
const LBC_CONSTANT_NUMBER: u8 = 2;
const LBC_CONSTANT_STRING: u8 = 3;
const LBC_CONSTANT_IMPORT: u8 = 4;
const LBC_CONSTANT_TABLE: u8 = 5;
const LBC_CONSTANT_CLOSURE: u8 = 6;
const LBC_CONSTANT_VECTOR: u8 = 7;
const LBC_CONSTANT_TABLE_WITH_CONSTANTS: u8 = 8;
const LBC_CONSTANT_INTEGER: u8 = 9;
const LBC_CONSTANT_CLASS_SHAPE: u8 = 10;

#[derive(Debug, Clone)]
pub struct DebugLocal {
    /// Index into the string table for the local's name.
    pub name_index: usize,
    /// Start PC (logical, post-AUX expansion).
    pub start_pc: usize,
    /// End PC (logical, post-AUX expansion).
    pub end_pc: usize,
    /// Register where the local lives.
    pub register: u8,
}

#[derive(Debug, Clone, Default)]
pub struct DebugInfo {
    pub locals: Vec<DebugLocal>,
    /// Indices into the string table for upvalue names.
    pub upvalue_names: Vec<usize>,
}

const OPCODES_WITH_AUX: &[OpCode] = &[
    OpCode::LOP_GETGLOBAL,
    OpCode::LOP_SETGLOBAL,
    OpCode::LOP_GETIMPORT,
    OpCode::LOP_GETTABLEKS,
    OpCode::LOP_SETTABLEKS,
    OpCode::LOP_NAMECALL,
    OpCode::LOP_JUMPIFEQ,
    OpCode::LOP_JUMPIFLE,
    OpCode::LOP_JUMPIFLT,
    OpCode::LOP_JUMPIFNOTEQ,
    OpCode::LOP_JUMPIFNOTLE,
    OpCode::LOP_JUMPIFNOTLT,
    OpCode::LOP_NEWTABLE,
    OpCode::LOP_SETLIST,
    OpCode::LOP_FORGLOOP,
    OpCode::LOP_LOADKX,
    OpCode::LOP_FASTCALL2,
    OpCode::LOP_FASTCALL2K,
    OpCode::LOP_FASTCALL3,
    OpCode::LOP_JUMPXEQKNIL,
    OpCode::LOP_JUMPXEQKB,
    OpCode::LOP_JUMPXEQKN,
    OpCode::LOP_JUMPXEQKS,
    OpCode::LOP_GETUDATAKS,
    OpCode::LOP_SETUDATAKS,
    OpCode::LOP_NAMECALLUDATA,
    OpCode::LOP_NEWCLASSMEMBER,
    OpCode::LOP_CALLFB,
    OpCode::LOP_CMPPROTO,
];

#[derive(Debug)]
pub struct Function {
    pub max_stack_size: u8,
    pub num_parameters: u8,
    pub num_upvalues: u8,
    pub is_vararg: bool,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Constant>,

    /// Indices into the *function table* of child prototypes.
    pub functions: Vec<u32>,

    pub line_defined: u32,

    /// Index into the string table for the function's name (debug).
    pub function_name: u32,

    /// If present, the per-instruction line info deltas.
    pub line_gap_log2: Option<u8>,
    /// `codesize` bytes, one per instruction, encoding the cumulative
    /// line-offset delta for that instruction relative to the line at
    /// the previous index.
    pub line_info_delta: Option<Vec<u8>>,
    /// `(codesize-1) >> line_gap_log2 + 1` signed i32 deltas; the
    /// base line for each group of instructions.
    pub abs_line_info_delta: Option<Vec<i32>>,

    pub debug_info: Option<DebugInfo>,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at offset {}", self.message, self.position)
    }
}

impl std::error::Error for ParseError {}

impl Function {
    /// Decode a sequence of raw instruction words (with the encode
    /// key applied to the op-code byte) into a Vec of `Instruction`
    /// plus a parallel Vec of NOP placeholders for AUX slots. This
    /// is byte-for-byte the same logic as upstream Luau.
    fn decode_instructions(
        raw: &[u32],
        encode_key: u8,
    ) -> Result<Vec<Instruction>, String> {
        let mut out = Vec::with_capacity(raw.len());
        let mut pc = 0;
        while pc < raw.len() {
            let ins = Instruction::parse(raw[pc], encode_key)
                .map_err(|e| format!("invalid op code: {:?}", e))?;
            let op = match ins {
                Instruction::BC { op_code, .. }
                | Instruction::AD { op_code, .. }
                | Instruction::E { op_code, .. } => op_code,
            };
            if OPCODES_WITH_AUX.contains(&op) {
                let Some(&aux) = raw.get(pc + 1) else {
                    return Err(format!("expected AUX word for op {:?}", op));
                };
                pc += 2;
                match ins {
                    Instruction::BC { op_code, a, b, c, .. } => {
                        out.push(Instruction::BC { op_code, a, b, c, aux });
                    }
                    Instruction::AD { op_code, a, d, .. } => {
                        out.push(Instruction::AD { op_code, a, d, aux });
                    }
                    _ => unreachable!(),
                }
                out.push(Instruction::BC {
                    op_code: OpCode::LOP_NOP,
                    a: 0,
                    b: 0,
                    c: 0,
                    aux: 0,
                });
            } else {
                out.push(ins);
                pc += 1;
            }
        }
        Ok(out)
    }

    /// Parse a single function's body from `data` at `offset`.
    /// `strings` is the chunk-level string table used to resolve
    /// `LBC_CONSTANT_STRING` indices back to the underlying bytes.
    /// `version` is the chunk bytecode version (needed for version-
    /// gated trailing sections such as the feedback vector).
    pub(crate) fn parse(
        data: &[u8],
        offset: &mut usize,
        encode_key: u8,
        _strings: &[Vec<u8>],
        version: u8,
    ) -> Result<Self, ParseError> {
        let start = *offset;

        // Helper: require at least N bytes available.
        macro_rules! need {
            ($n:expr) => {{
                if data.len() - *offset < $n {
                    return Err(ParseError {
                        message: format!(
                            "unexpected EOF: needed {} bytes, have {}",
                            $n,
                            data.len() - *offset
                        ),
                        position: start,
                    });
                }
            }};
        }

        // -- Header --
        // Layout (lvmload.cpp):
        //   u8 maxstacksize
        //   u8 numparams
        //   u8 nups
        //   u8 is_vararg
        //   u8 flags            (only if version >= 4)
        //   typeinfo section    (only if version >= 4)
        need!(4);
        let max_stack_size = data[*offset];
        let num_parameters = data[*offset + 1];
        let num_upvalues = data[*offset + 2];
        let is_vararg = data[*offset + 3] != 0;
        *offset += 4;

        if version >= 4 {
            need!(1);
            let _flags = data[*offset];
            *offset += 1;

            // -- Type info section --
            // Per upstream `BytecodeBuilder::writeFunction`, the format
            // is:
            //
            //   varint(typesize)               -- total size of inner block
            //                                    (== 0 ⇒ no typeinfo)
            //   <typesize bytes of inner>      -- opaque blob
            //
            // The VM in `lvmload.cpp` just `memcpy`s the whole inner
            // blob into the proto, so the upstream reader doesn't care
            // about its contents.
            //
            // In practice, the writer in luau-compile 0.728 (the build
            // we test against) produces an inner block whose internal
            // structure doesn't always match the documented format
            // (which would be 3 varints + data). The `typesize` varint
            // may be 0, or it may be a small number that doesn't
            // account for the full inner block. We try the modern
            // "skip typesize bytes" interpretation first; if it
            // doesn't produce a plausible rest of the function, we
            // fall back to a brute-force search of skip amounts (0 to
            // 32 bytes) and pick the one where the resulting
            // codesize + sizek are both plausible.
            let (types_size, advance) = read_leb128_u32(data, *offset)
                .map_err(|e| ParseError { message: format!("types_size: {e}"), position: start })?;
            *offset += advance;
            if types_size > 0 {
                let after_varint = *offset;
                let modern_end = after_varint + types_size as usize;
                // Check if the modern interpretation produces a
                // plausible rest of the function.
                let modern_ok = if modern_end > data.len() {
                    false
                } else if let Ok((cs, csadv)) = read_leb128_u32(data, modern_end) {
                    if (1..=10000).contains(&cs) {
                        let insns_end = modern_end + csadv + (cs as usize) * 4;
                        insns_end <= data.len()
                            && read_leb128_u32(data, insns_end)
                                .map(|(sk, _)| sk <= 100)
                                .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                };
                if modern_ok {
                    *offset = modern_end;
                } else {
                    // Fall back to brute-force search. Try all
                    // plausible skip amounts and pick the first one
                    // that produces a valid rest of the function
                    // (codesize in 1..=10000, followed by codesize*4
                    // bytes of instructions, followed by a sizek
                    // varint <= 100).
                    let mut best_skip: Option<usize> = None;
                    for skip_delta in 0..=32 {
                        let candidate_end = after_varint + skip_delta;
                        if candidate_end > data.len() {
                            break;
                        }
                        if let Ok((cs, csadv)) = read_leb128_u32(data, candidate_end) {
                            if (1..=10000).contains(&cs) {
                                let insns_end = candidate_end + csadv + (cs as usize) * 4;
                                if insns_end <= data.len() {
                                    if let Ok((sk, _)) = read_leb128_u32(data, insns_end) {
                                        if sk <= 100 {
                                            best_skip = Some(candidate_end);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let chosen = best_skip.unwrap_or(modern_end);
                    if chosen > data.len() {
                        return Err(ParseError {
                            message: format!(
                                "typeinfo section would overflow: end={} > data.len()={}",
                                chosen,
                                data.len()
                            ),
                            position: start,
                        });
                    }
                    *offset = chosen;
                }
            }
        }

        // -- Instruction list --
        let (codesize, advance) = read_leb128_u32(data, *offset)
            .map_err(|e| ParseError { message: format!("codesize: {e}"), position: start })?;
        *offset += advance;
        let codesize = codesize as usize;
        // Each instruction is a 32-bit word; LEB128 varint pre-counts them.
        need!(codesize * 4);
        let mut raw_instructions = Vec::with_capacity(codesize);
        for _ in 0..codesize {
            let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
            raw_instructions.push(u32::from_le_bytes(bytes));
            *offset += 4;
        }
        let instructions = Self::decode_instructions(&raw_instructions, encode_key)
            .map_err(|e| ParseError { message: e, position: start })?;

        // -- Constant list --
        let (sizek, advance) = read_leb128_u32(data, *offset)
            .map_err(|e| ParseError { message: format!("const count: {e}"), position: start })?;
        *offset += advance;
        let mut constants: Vec<Constant> = Vec::with_capacity(sizek as usize);
        for _ in 0..sizek {
            need!(1);
            let tag = data[*offset];
            *offset += 1;
            match tag {
                LBC_CONSTANT_NIL => constants.push(Constant::Nil),
                LBC_CONSTANT_BOOLEAN => {
                    need!(1);
                    let v = data[*offset] != 0;
                    *offset += 1;
                    constants.push(Constant::Boolean(v));
                }
                LBC_CONSTANT_NUMBER => {
                    need!(8);
                    let bytes: [u8; 8] = data[*offset..*offset + 8].try_into().unwrap();
                    let v = f64::from_le_bytes(bytes);
                    *offset += 8;
                    constants.push(Constant::Number(v));
                }
                LBC_CONSTANT_VECTOR => {
                    need!(16);
                    let mut v = [0f32; 4];
                    for slot in v.iter_mut() {
                        let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
                        *slot = f32::from_le_bytes(bytes);
                        *offset += 4;
                    }
                    constants.push(Constant::Vector(v[0], v[1], v[2], v[3]));
                }
                LBC_CONSTANT_STRING => {
                    let (idx, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("string idx: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += adv;
                    // Store the raw 1-based string-table index (0 =
                    // "no string"). The lifter does
                    // `str_list[name_index - 1]` after a `name_index > 0`
                    // check, so this must stay 1-based.
                    constants.push(Constant::String(idx as usize));
                }
                LBC_CONSTANT_IMPORT => {
                    need!(4);
                    let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
                    let v = u32::from_le_bytes(bytes) as usize;
                    *offset += 4;
                    constants.push(Constant::Import(v));
                }
                LBC_CONSTANT_TABLE | LBC_CONSTANT_TABLE_WITH_CONSTANTS => {
                    let (length, advance) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("table length: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += advance;
                    let with_constants = tag == LBC_CONSTANT_TABLE_WITH_CONSTANTS;
                    let mut keys = Vec::with_capacity(length as usize);
                    for _ in 0..length {
                        let (k, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                            ParseError {
                                message: format!("table key: {e}"),
                                position: start,
                            }
                        })?;
                        *offset += adv;
                        keys.push(k as usize);
                    }
                    if with_constants {
                        // Topaz's existing `Constant::TableWithConstants`
                        // is the right type here.
                        let mut entries = Vec::with_capacity(length as usize);
                        for key in &keys {
                            need!(4);
                            let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
                            let v = i32::from_le_bytes(bytes);
                            *offset += 4;
                            entries.push(super::constant::TableConstantEntry {
                                key: *key,
                                value_index: v,
                            });
                        }
                        constants.push(Constant::TableWithConstants(entries));
                    } else {
                        constants.push(Constant::Table(keys));
                    }
                }
                LBC_CONSTANT_CLOSURE => {
                    let (v, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("closure idx: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += adv;
                    constants.push(Constant::Closure(v as usize));
                }
                LBC_CONSTANT_INTEGER => {
                    // Layout (lvmload.cpp / BytecodeBuilder):
                    //   u8  is_negative
                    //   varint64 magnitude
                    // value = is_negative ? (~magnitude + 1) : magnitude
                    need!(1);
                    let is_negative = data[*offset] != 0;
                    *offset += 1;
                    let (magnitude, adv) =
                        read_leb128_u64(data, *offset).map_err(|e| ParseError {
                            message: format!("int magnitude: {e}"),
                            position: start,
                        })?;
                    *offset += adv;
                    let v: i64 = if is_negative {
                        (!magnitude).wrapping_add(1) as i64
                    } else {
                        magnitude as i64
                    };
                    constants.push(Constant::Integer(v));
                }
                LBC_CONSTANT_CLASS_SHAPE => {
                    // Real layout from upstream lvmload.cpp:
                    //   varint(class_name_const_idx)
                    //   varint(num_properties)
                    //   varint(num_methods)
                    //   (num_properties + num_methods) × varint(member_name_const_idx)
                    //
                    // Older Topaz code only read a single field count,
                    // which desynced the stream so the *next* constant's
                    // tag byte was actually part of the class-shape
                    // payload (commonly showing up as "unknown constant
                    // tag 11" when a feedback-slot type or a small
                    // varint byte was misread as a tag).
                    let (_class_name, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("class_shape class_name: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += adv;
                    let (nprops, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("class_shape nprops: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += adv;
                    let (nmethods, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                        ParseError {
                            message: format!("class_shape nmethods: {e}"),
                            position: start,
                        }
                    })?;
                    *offset += adv;
                    let nmembers = (nprops as u64)
                        .saturating_add(nmethods as u64);
                    for _ in 0..nmembers {
                        let (_, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                            ParseError {
                                message: format!("class_shape member: {e}"),
                                position: start,
                            }
                        })?;
                        *offset += adv;
                    }
                    // No decompiler representation for class shapes yet.
                    constants.push(Constant::Nil);
                }
                other => {
                    // Unknown constant tag — we don't know the payload
                    // size, so we can't safely continue. Bail out.
                    return Err(ParseError {
                        message: format!("unknown constant tag {}", other),
                        position: *offset - 1,
                    });
                }
            }
        }

        // -- Protos (child function indices) --
        let (psize, advance) = read_leb128_u32(data, *offset)
            .map_err(|e| ParseError { message: format!("psize: {e}"), position: start })?;
        *offset += advance;
        let mut functions = Vec::with_capacity(psize as usize);
        for _ in 0..psize {
            let (idx, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                ParseError { message: format!("proto idx: {e}"), position: start }
            })?;
            *offset += adv;
            functions.push(idx);
        }

        // -- Debug name (string table index, 0 = anonymous) and
        //    line defined --
        let (line_defined, advance) = read_leb128_u32(data, *offset)
            .map_err(|e| ParseError { message: format!("line_defined: {e}"), position: start })?;
        *offset += advance;
        let (debugname_idx, advance) = read_leb128_u32(data, *offset)
            .map_err(|e| ParseError { message: format!("debugname: {e}"), position: start })?;
        *offset += advance;
        // The bytecode stores function_name as a 1-based string-table
        // index where 0 means "no name" (anonymous). We keep the raw
        // value here so the lifter's `if name_index == 0 { None }`
        // check continues to work — converting to 0-based would lose
        // the distinction between "no name" and "name = strings[0]".
        let function_name = debugname_idx;

        // -- Line info (optional) --
        need!(1);
        let lineinfo = data[*offset];
        *offset += 1;
        let (line_gap_log2, line_info_delta, abs_line_info_delta) = if lineinfo != 0 {
            need!(1);
            let gap = data[*offset];
            *offset += 1;
            // `codesize` u8 cumulative-delta bytes.
            need!(codesize);
            let mut line_info_delta = Vec::with_capacity(codesize);
            for _ in 0..codesize {
                line_info_delta.push(data[*offset]);
                *offset += 1;
            }
            // `((codesize-1) >> gap) + 1` i32 deltas (signed).
            let abs_count = if codesize == 0 {
                0
            } else {
                ((codesize - 1) >> gap) + 1
            };
            need!(abs_count * 4);
            let mut abs_line_info_delta = Vec::with_capacity(abs_count);
            for _ in 0..abs_count {
                let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
                let v = i32::from_le_bytes(bytes);
                *offset += 4;
                abs_line_info_delta.push(v);
            }
            (Some(gap), Some(line_info_delta), Some(abs_line_info_delta))
        } else {
            (None, None, None)
        };

        // -- Debug info (optional) --
        need!(1);
        let debuginfo = data[*offset];
        *offset += 1;
        let debug_info = if debuginfo != 0 {
            let (sizelocvars, advance) = read_leb128_u32(data, *offset).map_err(|e| {
                ParseError {
                    message: format!("sizelocvars: {e}"),
                    position: start,
                }
            })?;
            *offset += advance;
            let mut locals = Vec::with_capacity(sizelocvars as usize);
            for _ in 0..sizelocvars {
                // varname: varint string-table index (1-based, 0 = none).
                let (name_idx, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                    ParseError {
                        message: format!("local name idx: {e}"),
                        position: start,
                    }
                })?;
                *offset += adv;
                let (start_pc, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                    ParseError {
                        message: format!("local startpc: {e}"),
                        position: start,
                    }
                })?;
                *offset += adv;
                let (end_pc, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                    ParseError {
                        message: format!("local endpc: {e}"),
                        position: start,
                    }
                })?;
                *offset += adv;
                need!(1);
                let register = data[*offset];
                *offset += 1;
                // Keep the raw 1-based value so the lifter's
                // `if name_index == 0 { skip }` check works.
                let name_index = name_idx as usize;
                locals.push(DebugLocal {
                    name_index,
                    start_pc: start_pc as usize,
                    end_pc: end_pc as usize,
                    register,
                });
            }
            let (sizeupvalues, advance) = read_leb128_u32(data, *offset).map_err(|e| {
                ParseError {
                    message: format!("sizeupvalues: {e}"),
                    position: start,
                }
            })?;
            *offset += advance;
            let mut upvalue_names = Vec::with_capacity(sizeupvalues as usize);
            for _ in 0..sizeupvalues {
                let (idx, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                    ParseError {
                        message: format!("upval name idx: {e}"),
                        position: start,
                    }
                })?;
                *offset += adv;
                // Keep raw 1-based value (0 = no name).
                upvalue_names.push(idx as usize);
            }
            Some(DebugInfo {
                locals,
                upvalue_names,
            })
        } else {
            None
        };

        // -- Feedback vector (version >= 11) --
        // Layout (lvmload.cpp):
        //   varint(feedbackvecsize)
        //   feedbackvecsize × (u8 slottype + varint pc)
        // We only need to consume the bytes so the next proto (or the
        // main-function index) stays aligned. The decompiler does not
        // currently use feedback data.
        if version >= 11 {
            let (fb_count, advance) = read_leb128_u32(data, *offset).map_err(|e| {
                ParseError {
                    message: format!("feedbackvecsize: {e}"),
                    position: start,
                }
            })?;
            *offset += advance;
            for _ in 0..fb_count {
                need!(1);
                // slottype (LFT_CALLTARGET = 0 today; ignore value)
                *offset += 1;
                let (_, adv) = read_leb128_u32(data, *offset).map_err(|e| {
                    ParseError {
                        message: format!("feedback slot pc: {e}"),
                        position: start,
                    }
                })?;
                *offset += adv;
            }
        }

        // -- Cost model (version >= 12, only if LPF_INLINABLE) --
        // Consumed by the version-12 size prefix skip in Chunk::parse
        // when present; we deliberately do not try to re-derive the
        // flag bit here because the size prefix already lets us jump
        // past unknown trailing data.


        Ok(Function {
            max_stack_size,
            num_parameters,
            num_upvalues,
            is_vararg,
            instructions,
            constants,
            functions,
            line_defined,
            function_name,
            line_gap_log2,
            line_info_delta,
            abs_line_info_delta,
            debug_info,
        })
    }
}
