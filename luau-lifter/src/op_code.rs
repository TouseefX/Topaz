use num_enum::TryFromPrimitive;

/// Luau / Roblox Studio 0.735 opcode numbers.
///
/// Taken from the 2026 Mac Studio decompile (`BytecodeBuilder.c`
/// `dumpInstruction` switch) and matching open-source Luau 0.735
/// `Common/include/Luau/Bytecode.h`. Roblox does **not** ship extra
/// bitwise opcodes; `0x59` is `FASTPCALL` and `0x5A` is `NEWCLASS`.
/// Client dumps only shuffle the op *byte* (`stored * encode_key`).
#[repr(u8)]
#[derive(Debug, TryFromPrimitive, Eq, PartialEq, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum OpCode {
    LOP_NOP = 0x00,
    LOP_BREAK = 0x01,
    LOP_LOADNIL = 0x02,
    LOP_LOADB = 0x03,
    LOP_LOADN = 0x04,
    LOP_LOADK = 0x05,
    LOP_MOVE = 0x06,
    LOP_GETGLOBAL = 0x07,
    LOP_SETGLOBAL = 0x08,
    LOP_GETUPVAL = 0x09,
    LOP_SETUPVAL = 0x0A,
    LOP_CLOSEUPVALS = 0x0B,
    LOP_GETIMPORT = 0x0C,
    LOP_GETTABLE = 0x0D,
    LOP_SETTABLE = 0x0E,
    LOP_GETTABLEKS = 0x0F,
    LOP_SETTABLEKS = 0x10,
    LOP_GETTABLEN = 0x11,
    LOP_SETTABLEN = 0x12,
    LOP_NEWCLOSURE = 0x13,
    LOP_NAMECALL = 0x14,
    LOP_CALL = 0x15,
    LOP_RETURN = 0x16,
    LOP_JUMP = 0x17,
    LOP_JUMPBACK = 0x18,
    LOP_JUMPIF = 0x19,
    LOP_JUMPIFNOT = 0x1A,
    LOP_JUMPIFEQ = 0x1B,
    LOP_JUMPIFLE = 0x1C,
    LOP_JUMPIFLT = 0x1D,
    LOP_JUMPIFNOTEQ = 0x1E,
    LOP_JUMPIFNOTLE = 0x1F,
    LOP_JUMPIFNOTLT = 0x20,
    LOP_ADD = 0x21,
    LOP_SUB = 0x22,
    LOP_MUL = 0x23,
    LOP_DIV = 0x24,
    LOP_MOD = 0x25,
    LOP_POW = 0x26,
    LOP_ADDK = 0x27,
    LOP_SUBK = 0x28,
    LOP_MULK = 0x29,
    LOP_DIVK = 0x2A,
    LOP_MODK = 0x2B,
    LOP_POWK = 0x2C,
    LOP_AND = 0x2D,
    LOP_OR = 0x2E,
    LOP_ANDK = 0x2F,
    LOP_ORK = 0x30,
    LOP_CONCAT = 0x31,
    LOP_NOT = 0x32,
    LOP_MINUS = 0x33,
    LOP_LENGTH = 0x34,
    LOP_NEWTABLE = 0x35,
    LOP_DUPTABLE = 0x36,
    LOP_SETLIST = 0x37,
    LOP_FORNPREP = 0x38,
    LOP_FORNLOOP = 0x39,
    LOP_FORGLOOP = 0x3A,
    LOP_FORGPREP_INEXT = 0x3B,
    LOP_FASTCALL3 = 0x3C,
    LOP_FORGPREP_NEXT = 0x3D,
    LOP_NATIVECALL = 0x3E,
    LOP_GETVARARGS = 0x3F,
    LOP_DUPCLOSURE = 0x40,
    LOP_PREPVARARGS = 0x41,
    LOP_LOADKX = 0x42,
    LOP_JUMPX = 0x43,
    LOP_FASTCALL = 0x44,
    LOP_COVERAGE = 0x45,
    LOP_CAPTURE = 0x46,
    LOP_SUBRK = 0x47,
    LOP_DIVRK = 0x48,
    LOP_FASTCALL1 = 0x49,
    LOP_FASTCALL2 = 0x4A,
    LOP_FASTCALL2K = 0x4B,
    LOP_FORGPREP = 0x4C,
    LOP_JUMPXEQKNIL = 0x4D,
    LOP_JUMPXEQKB = 0x4E,
    LOP_JUMPXEQKN = 0x4F,
    LOP_JUMPXEQKS = 0x50,
    LOP_IDIV = 0x51,
    LOP_IDIVK = 0x52,
    LOP_GETUDATAKS = 0x53,
    LOP_SETUDATAKS = 0x54,
    LOP_NAMECALLUDATA = 0x55,
    LOP_NEWCLASSMEMBER = 0x56,
    LOP_CALLFB = 0x57,
    LOP_CMPPROTO = 0x58,
    /// Studio 0.735: `FASTPCALL` (A=0 pcall / A=1 xpcall). Not a bitwise op.
    LOP_FASTPCALL = 0x59,
    /// Studio 0.735: `NEWCLASS` (AUX = class constant, length 2).
    LOP_NEWCLASS = 0x5A,

    LOP__COUNT,
}

impl OpCode {
    /// Word length of this opcode, matching Studio `Luau::getOpLength`
    /// / `BytecodeUtils.h`. 2 means a following AUX word.
    #[inline]
    pub fn word_length(self) -> u32 {
        match self {
            OpCode::LOP_GETGLOBAL
            | OpCode::LOP_SETGLOBAL
            | OpCode::LOP_GETIMPORT
            | OpCode::LOP_GETTABLEKS
            | OpCode::LOP_SETTABLEKS
            | OpCode::LOP_NAMECALL
            | OpCode::LOP_JUMPIFEQ
            | OpCode::LOP_JUMPIFLE
            | OpCode::LOP_JUMPIFLT
            | OpCode::LOP_JUMPIFNOTEQ
            | OpCode::LOP_JUMPIFNOTLE
            | OpCode::LOP_JUMPIFNOTLT
            | OpCode::LOP_NEWTABLE
            | OpCode::LOP_SETLIST
            | OpCode::LOP_FORGLOOP
            | OpCode::LOP_LOADKX
            | OpCode::LOP_FASTCALL2
            | OpCode::LOP_FASTCALL2K
            | OpCode::LOP_FASTCALL3
            | OpCode::LOP_JUMPXEQKNIL
            | OpCode::LOP_JUMPXEQKB
            | OpCode::LOP_JUMPXEQKN
            | OpCode::LOP_JUMPXEQKS
            | OpCode::LOP_GETUDATAKS
            | OpCode::LOP_SETUDATAKS
            | OpCode::LOP_NAMECALLUDATA
            | OpCode::LOP_NEWCLASSMEMBER
            | OpCode::LOP_CALLFB
            | OpCode::LOP_CMPPROTO
            | OpCode::LOP_NEWCLASS => 2,
            _ => 1,
        }
    }
}
