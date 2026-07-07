//! Integration regression tests that decompile small, known Luau bytecode
//! fixtures and check the output text for specific bug patterns that have
//! previously caused silent miscompilation or runtime crashes.
//!
//! Fixtures were compiled with a real Luau bytecode compiler
//! (`@lune/luau`'s `compile()`, from https://github.com/lune-org/lune) so
//! they match the bytecode format actually produced by Roblox/Luau tooling
//! (as opposed to the standalone `luau-compile` CLI, which at the time of
//! writing emits a newer bytecode version than this deserializer supports).
//! Each fixture's corresponding `.lua` source is kept alongside the
//! `.luau.bin` bytecode for reference/regeneration.
//!
//! The default encode key used by `topaz decompile` (and by
//! `luau_lifter::decompile_bytecode`) is expected to be `0`/no XOR
//! obfuscation for these fixtures, since they were compiled directly
//! without any additional string-encoding step.

/// `topaz decompile`'s default encode key (used to XOR-deobfuscate string
/// constants in the bytecode). These fixtures were compiled without any
/// additional string obfuscation, but the default key must still be
/// supplied since `luau_lifter::decompile_bytecode` uses it to decode the
/// bytecode's string table unconditionally.
const ENCODE_KEY: u8 = 203;

const ACCUMULATOR_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/accumulator_loop.luau.bin");
const CONTINUE_IN_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/continue_in_loop.luau.bin");
const UPVALUE_COLLISION_BYTECODE: &[u8] = include_bytes!("fixtures/luau_upval_collision.luau.bin");

/// Regression test for a critical bug where compound-assignment printing
/// (`x += y`) was dead code in the formatter (nested inside the wrong
/// `if`-branch), so every detected compound assignment silently lost its
/// self-referential read: `total = total + i` was printed as the broken
/// `total = i` instead of `total += i`. This corrupted every
/// counter/accumulator loop pattern in decompiled output.
#[test]
fn accumulator_loop_uses_compound_assignment_not_plain_reassignment() {
    let output = luau_lifter::decompile_bytecode(ACCUMULATOR_LOOP_BYTECODE, ENCODE_KEY);

    assert!(
        output.contains("+="),
        "expected the accumulator loop body (`total = total + i`) to be \
         printed using a compound assignment (`total += i`): \
         got decompiled output:\n{output}"
    );

    // The specific regression: the loop body must not contain a bare
    // reassignment of the accumulator to just the loop variable (which is
    // what `total = total + i` becomes when the `+ total` read is
    // silently dropped).
    for line in output.lines() {
        let trimmed = line.trim();
        // Look for exactly `NAME = NAME2` where NAME2 is a *different*
        // single identifier -- i.e. a plain-copy assignment sitting where
        // a compound assignment (`+=`) was expected. A legitimate `+=`
        // line will contain the `+=` operator instead of a bare `=`.
        if let Some((lhs, rhs)) = trimmed.split_once(" = ") {
            if is_simple_identifier(lhs) && is_simple_identifier(rhs) && lhs != rhs {
                panic!(
                    "found a suspicious plain reassignment `{trimmed}` where \
                     a compound assignment was expected inside the \
                     accumulator loop (this reproduces the bug where \
                     `total = total + i` silently became `total = i`): \
                     got decompiled output:\n{output}"
                );
            }
        }
    }
}

/// Regression test for a bug where `continue`-equivalent control flow
/// inside a loop (an early "skip to next iteration" guard) was
/// incorrectly synthesized as a bare `return`, which exits the entire
/// enclosing function instead of just skipping the current iteration.
#[test]
fn early_exit_inside_loop_uses_continue_not_return() {
    let output = luau_lifter::decompile_bytecode(CONTINUE_IN_LOOP_BYTECODE, ENCODE_KEY);

    assert!(
        output.contains("continue"),
        "expected the loop's early-skip guard (`if seen[item] then ... end`) \
         to use `continue`: got decompiled output:\n{output}"
    );
}

/// Regression test for the NodeSorter-style bug where a captured upvalue
/// (a shared id counter) was renamed to match an unrelated sibling
/// local's heuristically inferred name (both ended up named "Id"),
/// producing the nonsensical, crash-inducing self-assignment `Id = Id`.
#[test]
fn upvalue_is_not_renamed_to_match_unrelated_sibling() {
    let output = luau_lifter::decompile_bytecode(UPVALUE_COLLISION_BYTECODE, ENCODE_KEY);

    for (name, decl) in find_self_referential_assignments(&output) {
        panic!(
            "found self-referential assignment `{decl}` for variable \
             `{name}`, which is never valid decompiled output (this \
             reproduces the NodeSorter `Id = Id` bug): \
             got decompiled output:\n{output}"
        );
    }
}

fn is_simple_identifier(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.chars().next().unwrap().is_ascii_digit()
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
            if is_simple_identifier(lhs) && lhs == rhs {
                hits.push((lhs.to_string(), trimmed.to_string()));
            }
        }
    }
    hits
}
