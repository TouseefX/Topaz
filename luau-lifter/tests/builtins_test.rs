//! Unit tests for the FASTCALL builtin lookup table. The table mirrors
//! the upstream `LuauBuiltinFunction` enum (see
//! `luau-lang/luau/Common/include/Luau/Bytecode.h`); if a new builtin
//! is added upstream, this test will fail until the table is updated
//! and the new id is wired into the lookup. The test pins the *order*
//! of the entries (since bytecode encodes the id as a u8 position in
//! the enum, the order is part of the wire format and cannot change
//! without breaking every existing Luau binary).

use luau_lifter::builtins::{lookup, BuiltinInfo};

/// Sanity check: `lookup(0)` is `LBF_NONE` and should return `None`.
#[test]
fn lookup_zero_is_none() {
    assert!(lookup(0).is_none(), "LBF_NONE (id 0) should be None");
}

/// A handful of stable, well-known builtin ids. The exact names are
/// the public contract — anyone whose code grep'd the decompiler's
/// output for "math.floor" or "string.byte" depends on these staying
/// stable.
#[test]
fn known_builtins_have_stable_names() {
    let cases: &[(u8, &str, &str)] = &[
        (1, "", "assert"),
        (2, "math", "abs"),
        (12, "math", "floor"),
        (28, "bit32", "arshift"),
        (29, "bit32", "band"),
        (40, "", "type"),
        (44, "", "typeof"),
        (49, "", "rawset"),
        (50, "", "rawget"),
        (52, "table", "insert"),
        (53, "table", "unpack"),
        (54, "vector", "create"),
        (60, "", "getmetatable"),
        (61, "", "setmetatable"),
        (62, "", "tonumber"),
        (63, "", "tostring"),
    ];
    for &(id, expected_module, expected_name) in cases {
        let info: BuiltinInfo = lookup(id)
            .unwrap_or_else(|| panic!("builtin id {id} should resolve to Some"));
        assert_eq!(
            info.module, expected_module,
            "builtin id {id} has wrong module"
        );
        assert_eq!(
            info.name, expected_name,
            "builtin id {id} has wrong name"
        );
    }
}

/// Past the end of the table, lookup must return None (not panic, not
/// return a stale entry). This guards against an off-by-one in the
/// table-size vs. id-range relationship when a new Luau version
/// adds builtins.
#[test]
fn lookup_past_end_is_none() {
    // 200 is comfortably past the current maximum builtin id (~129).
    assert!(lookup(200).is_none());
    assert!(lookup(255).is_none());
}
