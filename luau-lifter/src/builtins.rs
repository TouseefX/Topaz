//! Lookup table for Luau built-in function IDs used by `LOP_FASTCALL*`.
//!
//! `LuauBuiltinFunction` is an enum in Luau's `Common/include/Luau/Bytecode.h`
//! whose numeric values are baked into compiled bytecode. The numbers are
//! append-only — the Luau project adds new builtins at the end of the list —
//! so the table below must be kept in lock-step with upstream
//! `LBF_*` constants.
//!
//! Each entry maps the builtin id to a `(module, function_name)` pair that
//! the lifter emits as a real call expression. For builtins that are called
//! as methods (e.g. `string.byte(s, i)` is `s:byte(i)`) we additionally store
//! `is_method = true` and the first *real* argument register is taken as the
//! receiver; the lifter rewrites the call accordingly.
//!
//! Format: the enum value is the 1-based index into the `BUILTIN_TABLE`
//! (LBF_NONE = 0 is omitted). Keeping the table as a slice indexed by
//! `(builtin_id - 1)` makes it impossible to introduce an off-by-one and
//! trivial to diff against upstream when new entries appear.

/// A built-in that can be fast-called from Luau bytecode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinInfo {
    /// The "module" portion of the call (e.g. `"math"` for `math.abs`). Empty
    /// for global builtins like `assert`, `type`, `tostring`, `tonumber`,
    /// `rawget`, `rawset`, `rawequal`, `rawlen`, `pcall`, `select`, `typeof`,
    /// `unpack` (table.unpack is also exposed), `getmetatable`, `setmetatable`,
    /// `vector`. Topaz's `ast::Global::new`/`ast::Index` pipeline handles
    /// both forms uniformly — empty module + a single name produces
    /// `name(...)`, and a non-empty module produces `module.name(...)`.
    pub module: &'static str,
    /// The function name. Concatenated with `.` after `module` to form the
    /// call target.
    pub name: &'static str,
}

const BUILTIN_TABLE: &[BuiltinInfo] = &[
    // 0  LBF_NONE
    BuiltinInfo { module: "", name: "" },
    // 1  LBF_ASSERT
    BuiltinInfo { module: "", name: "assert" },
    // 2  LBF_MATH_ABS
    BuiltinInfo { module: "math", name: "abs" },
    // 3  LBF_MATH_ACOS
    BuiltinInfo { module: "math", name: "acos" },
    // 4  LBF_MATH_ASIN
    BuiltinInfo { module: "math", name: "asin" },
    // 5  LBF_MATH_ATAN2
    BuiltinInfo { module: "math", name: "atan2" },
    // 6  LBF_MATH_ATAN
    BuiltinInfo { module: "math", name: "atan" },
    // 7  LBF_MATH_CEIL
    BuiltinInfo { module: "math", name: "ceil" },
    // 8  LBF_MATH_COSH
    BuiltinInfo { module: "math", name: "cosh" },
    // 9  LBF_MATH_COS
    BuiltinInfo { module: "math", name: "cos" },
    // 10 LBF_MATH_DEG
    BuiltinInfo { module: "math", name: "deg" },
    // 11 LBF_MATH_EXP
    BuiltinInfo { module: "math", name: "exp" },
    // 12 LBF_MATH_FLOOR
    BuiltinInfo { module: "math", name: "floor" },
    // 13 LBF_MATH_FMOD
    BuiltinInfo { module: "math", name: "fmod" },
    // 14 LBF_MATH_FREXP
    BuiltinInfo { module: "math", name: "frexp" },
    // 15 LBF_MATH_LDEXP
    BuiltinInfo { module: "math", name: "ldexp" },
    // 16 LBF_MATH_LOG10
    BuiltinInfo { module: "math", name: "log10" },
    // 17 LBF_MATH_LOG
    BuiltinInfo { module: "math", name: "log" },
    // 18 LBF_MATH_MAX
    BuiltinInfo { module: "math", name: "max" },
    // 19 LBF_MATH_MIN
    BuiltinInfo { module: "math", name: "min" },
    // 20 LBF_MATH_MODF
    BuiltinInfo { module: "math", name: "modf" },
    // 21 LBF_MATH_POW
    BuiltinInfo { module: "math", name: "pow" },
    // 22 LBF_MATH_RAD
    BuiltinInfo { module: "math", name: "rad" },
    // 23 LBF_MATH_SINH
    BuiltinInfo { module: "math", name: "sinh" },
    // 24 LBF_MATH_SIN
    BuiltinInfo { module: "math", name: "sin" },
    // 25 LBF_MATH_SQRT
    BuiltinInfo { module: "math", name: "sqrt" },
    // 26 LBF_MATH_TANH
    BuiltinInfo { module: "math", name: "tanh" },
    // 27 LBF_MATH_TAN
    BuiltinInfo { module: "math", name: "tan" },
    // 28 LBF_BIT32_ARSHIFT
    BuiltinInfo { module: "bit32", name: "arshift" },
    // 29 LBF_BIT32_BAND
    BuiltinInfo { module: "bit32", name: "band" },
    // 30 LBF_BIT32_BNOT
    BuiltinInfo { module: "bit32", name: "bnot" },
    // 31 LBF_BIT32_BOR
    BuiltinInfo { module: "bit32", name: "bor" },
    // 32 LBF_BIT32_BXOR
    BuiltinInfo { module: "bit32", name: "bxor" },
    // 33 LBF_BIT32_BTEST
    BuiltinInfo { module: "bit32", name: "btest" },
    // 34 LBF_BIT32_EXTRACT
    BuiltinInfo { module: "bit32", name: "extract" },
    // 35 LBF_BIT32_LROTATE
    BuiltinInfo { module: "bit32", name: "lrotate" },
    // 36 LBF_BIT32_LSHIFT
    BuiltinInfo { module: "bit32", name: "lshift" },
    // 37 LBF_BIT32_REPLACE
    BuiltinInfo { module: "bit32", name: "replace" },
    // 38 LBF_BIT32_RROTATE
    BuiltinInfo { module: "bit32", name: "rrotate" },
    // 39 LBF_BIT32_RSHIFT
    BuiltinInfo { module: "bit32", name: "rshift" },
    // 40 LBF_TYPE
    BuiltinInfo { module: "", name: "type" },
    // 41 LBF_STRING_BYTE
    BuiltinInfo { module: "string", name: "byte" },
    // 42 LBF_STRING_CHAR
    BuiltinInfo { module: "string", name: "char" },
    // 43 LBF_STRING_LEN
    BuiltinInfo { module: "string", name: "len" },
    // 44 LBF_TYPEOF
    BuiltinInfo { module: "", name: "typeof" },
    // 45 LBF_STRING_SUB
    BuiltinInfo { module: "string", name: "sub" },
    // 46 LBF_MATH_CLAMP
    BuiltinInfo { module: "math", name: "clamp" },
    // 47 LBF_MATH_SIGN
    BuiltinInfo { module: "math", name: "sign" },
    // 48 LBF_MATH_ROUND
    BuiltinInfo { module: "math", name: "round" },
    // 49 LBF_RAWSET
    BuiltinInfo { module: "", name: "rawset" },
    // 50 LBF_RAWGET
    BuiltinInfo { module: "", name: "rawget" },
    // 51 LBF_RAWEQUAL
    BuiltinInfo { module: "", name: "rawequal" },
    // 52 LBF_TABLE_INSERT
    BuiltinInfo { module: "table", name: "insert" },
    // 53 LBF_TABLE_UNPACK
    BuiltinInfo { module: "table", name: "unpack" },
    // 54 LBF_VECTOR  (vector ctor)
    BuiltinInfo { module: "vector", name: "create" },
    // 55 LBF_BIT32_COUNTLZ
    BuiltinInfo { module: "bit32", name: "countlz" },
    // 56 LBF_BIT32_COUNTRZ
    BuiltinInfo { module: "bit32", name: "countrz" },
    // 57 LBF_SELECT_VARARG
    BuiltinInfo { module: "", name: "select" },
    // 58 LBF_RAWLEN
    BuiltinInfo { module: "", name: "rawlen" },
    // 59 LBF_BIT32_EXTRACTK
    BuiltinInfo { module: "bit32", name: "extract" },
    // 60 LBF_GETMETATABLE
    BuiltinInfo { module: "", name: "getmetatable" },
    // 61 LBF_SETMETATABLE
    BuiltinInfo { module: "", name: "setmetatable" },
    // 62 LBF_TONUMBER
    BuiltinInfo { module: "", name: "tonumber" },
    // 63 LBF_TOSTRING
    BuiltinInfo { module: "", name: "tostring" },
    // 64 LBF_BIT32_BYTESWAP
    BuiltinInfo { module: "bit32", name: "byteswap" },
    // 65..78  BUFFER_READ*/WRITE*
    BuiltinInfo { module: "buffer", name: "readi8" },
    BuiltinInfo { module: "buffer", name: "readu8" },
    BuiltinInfo { module: "buffer", name: "writeu8" },
    BuiltinInfo { module: "buffer", name: "readi16" },
    BuiltinInfo { module: "buffer", name: "readu16" },
    BuiltinInfo { module: "buffer", name: "writeu16" },
    BuiltinInfo { module: "buffer", name: "readi32" },
    BuiltinInfo { module: "buffer", name: "readu32" },
    BuiltinInfo { module: "buffer", name: "writeu32" },
    BuiltinInfo { module: "buffer", name: "readf32" },
    BuiltinInfo { module: "buffer", name: "writef32" },
    BuiltinInfo { module: "buffer", name: "readf64" },
    BuiltinInfo { module: "buffer", name: "writef64" },
    // 79..89  VECTOR_*
    BuiltinInfo { module: "vector", name: "magnitude" },
    BuiltinInfo { module: "vector", name: "normalize" },
    BuiltinInfo { module: "vector", name: "cross" },
    BuiltinInfo { module: "vector", name: "dot" },
    BuiltinInfo { module: "vector", name: "floor" },
    BuiltinInfo { module: "vector", name: "ceil" },
    BuiltinInfo { module: "vector", name: "abs" },
    BuiltinInfo { module: "vector", name: "sign" },
    BuiltinInfo { module: "vector", name: "clamp" },
    BuiltinInfo { module: "vector", name: "min" },
    BuiltinInfo { module: "vector", name: "max" },
    // Note: the ids below are lune 0.10.5-compatible (off by -1
    // from upstream Luau's LBF_ enum starting at LBF_MATH_LERP). The
    // decompiler uses these ids directly when matching against
    // FASTCALL opcodes in lune-compiled bytecode.
    // 89 LBF_MATH_LERP
    BuiltinInfo { module: "math", name: "lerp" },
    // 90 LBF_VECTOR_LERP
    BuiltinInfo { module: "vector", name: "lerp" },
    // 91..93  MATH_IS*
    BuiltinInfo { module: "math", name: "isnan" },
    BuiltinInfo { module: "math", name: "isinf" },
    BuiltinInfo { module: "math", name: "isfinite" },
    // 94..126  INTEGER_*
    BuiltinInfo { module: "integer", name: "create" },
    BuiltinInfo { module: "integer", name: "tonumber" },
    BuiltinInfo { module: "integer", name: "neg" },
    BuiltinInfo { module: "integer", name: "add" },
    BuiltinInfo { module: "integer", name: "sub" },
    BuiltinInfo { module: "integer", name: "mul" },
    BuiltinInfo { module: "integer", name: "div" },
    BuiltinInfo { module: "integer", name: "min" },
    BuiltinInfo { module: "integer", name: "max" },
    BuiltinInfo { module: "integer", name: "rem" },
    BuiltinInfo { module: "integer", name: "idiv" },
    BuiltinInfo { module: "integer", name: "udiv" },
    BuiltinInfo { module: "integer", name: "urem" },
    BuiltinInfo { module: "integer", name: "mod" },
    BuiltinInfo { module: "integer", name: "clamp" },
    BuiltinInfo { module: "integer", name: "band" },
    BuiltinInfo { module: "integer", name: "bor" },
    BuiltinInfo { module: "integer", name: "bnot" },
    BuiltinInfo { module: "integer", name: "bxor" },
    BuiltinInfo { module: "integer", name: "lt" },
    BuiltinInfo { module: "integer", name: "le" },
    BuiltinInfo { module: "integer", name: "ult" },
    BuiltinInfo { module: "integer", name: "ule" },
    BuiltinInfo { module: "integer", name: "gt" },
    BuiltinInfo { module: "integer", name: "ge" },
    BuiltinInfo { module: "integer", name: "ugt" },
    BuiltinInfo { module: "integer", name: "uge" },
    BuiltinInfo { module: "integer", name: "lshift" },
    BuiltinInfo { module: "integer", name: "rshift" },
    BuiltinInfo { module: "integer", name: "arshift" },
    BuiltinInfo { module: "integer", name: "lrotate" },
    BuiltinInfo { module: "integer", name: "rrotate" },
    BuiltinInfo { module: "integer", name: "extract" },
    BuiltinInfo { module: "integer", name: "btest" },
    BuiltinInfo { module: "integer", name: "countrz" },
    BuiltinInfo { module: "integer", name: "countlz" },
    BuiltinInfo { module: "integer", name: "bswap" },
    // 127..128  BUFFER_READINTEGER / WRITEINTEGER
    BuiltinInfo { module: "buffer", name: "readinteger" },
    BuiltinInfo { module: "buffer", name: "writeinteger" },
];

/// Look up a builtin by its Luau id. Returns `None` for `LBF_NONE` (id 0)
/// or for any id past the end of the table (which would mean Luau added
/// a new builtin that this decompiler doesn't recognize — the lifter
/// should fall back to emitting a comment for those).
pub fn lookup(id: u8) -> Option<BuiltinInfo> {
    let info = *BUILTIN_TABLE.get(id as usize)?;
    if info.module.is_empty() && info.name.is_empty() {
        None
    } else {
        Some(info)
    }
}

/// Construct the AST `RValue` for calling a builtin: either a bare
/// `Global(name)` (for module-less builtins) or `Index(Global(module),
/// String(name))` (for module ones). Returns `None` if the builtin
/// should not be lifted to a real call.
pub fn build_call_target(info: BuiltinInfo) -> Option<ast::RValue> {
    use ast::{Global, Index, Literal, RValue};

    if info.module.is_empty() {
        Some(RValue::Global(Global::new(info.name.as_bytes().to_vec())))
    } else {
        Some(RValue::Index(Index::new(
            RValue::Global(Global::new(info.module.as_bytes().to_vec())),
            RValue::Literal(Literal::String(info.name.as_bytes().to_vec())),
        )))
    }
}
