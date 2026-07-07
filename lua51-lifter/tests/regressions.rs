//! Integration regression tests that decompile small, known Lua 5.1
//! bytecode fixtures and check the output text for specific bug patterns
//! that have previously caused silent miscompilation or runtime crashes.
//!
//! These tests intentionally check *textual* properties of the decompiled
//! output rather than executing it, so they don't require a Lua runtime to
//! be available in CI. Each fixture's corresponding `.lua` source is kept
//! alongside the `.luac` bytecode for reference/regeneration
//! (e.g. `luac5.1 -s -o fixture.luac fixture.lua`).

const UPVALUE_NAMING_COLLISION_BYTECODE: &[u8] =
    include_bytes!("fixtures/upvalue_naming_collision.luac");

/// Regression test for a bug where a captured upvalue (here, a shared id
/// counter) could be renamed to match an unrelated local's heuristically
/// inferred name (here, "Id", derived from a `.Id` field access), because
/// the name-propagation pass didn't know the counter was captured by a
/// nested closure. This made two logically distinct variables print with
/// the exact same name, turning:
///
///   aId = idCounter
///   idCounter = idCounter + 1
///
/// into the textually-plausible but semantically broken:
///
///   Id = Id
///   Id = Id + 1
///
/// which crashes at runtime with "attempt to perform arithmetic on a nil
/// value" the first time the counter is used before ever being
/// initialized (since `Id = Id` is a self-assigning no-op).
#[test]
fn upvalue_is_not_renamed_to_match_unrelated_sibling() {
    let output = lua51_lifter::decompile_bytecode(UPVALUE_NAMING_COLLISION_BYTECODE);

    // Every statement of the form `NAME = NAME` (a bare local/global
    // self-assignment, as opposed to a field assignment like
    // `p2.Id = Id`, which is legitimate) is inherently broken -- it always
    // means a naming collision occurred between two logically distinct
    // variables immediately after declaration. This is exactly the
    // NodeSorter regression, where the captured `idCounter` upvalue got
    // renamed to "Id" to match an unrelated sibling local also named "Id"
    // (derived from an unrelated `.Id` field access), producing the
    // nonsensical and crash-inducing `Id = Id`.
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!(
            "found self-referential assignment `{decl}` for variable \
             `{name}`, which is never valid decompiled output (this \
             reproduces the NodeSorter `Id = Id` bug): \
             got decompiled output:\n{output}"
        );
    }
}

/// Scans decompiled output for any line of the exact form `NAME = NAME`
/// (ignoring surrounding whitespace), which is never valid: assigning a
/// variable to itself with no computation is either dead code the
/// decompiler should have removed, or -- far more likely -- evidence that
/// two logically distinct variables were printed with the same name.
fn find_self_referential_assignments(output: &str) -> Vec<(String, String)> {
    let mut hits = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if !lhs.is_empty()
                && lhs == rhs
                && lhs
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !lhs.chars().next().unwrap().is_ascii_digit()
            {
                hits.push((lhs.to_string(), trimmed.to_string()));
            }
        }
    }
    hits
}
