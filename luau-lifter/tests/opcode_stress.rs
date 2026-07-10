//! Opcode stress test for the Luau decompiler.
//!
//! This test exists to catch three classes of regression in a single shot:
//!
//! 1. **Lifter panics on edge-case opcodes.** Each fixture in
//!    `tests/fixtures/` was originally compiled from real Luau source by
//!    `@lune/luau`, so it covers a different subset of the ~70 opcodes
//!    in the Luau bytecode instruction set. Running the full decompiler
//!    on all of them and asserting that *none* of them panic
//!    (`decompile_bytecode` returns successfully) is a strong smoke
//!    test that the lifter, SSA, structuring, and AST post-processing
//!    passes all handle the opcode combinations we care about.
//!
//! 2. **Silent miscompilation** in the form of "unhandled instruction"
//!    comments appearing in the output for opcodes that the lifter
//!    claims to support. If such a comment shows up, the decompiler
//!    silently dropped a real instruction, producing source that won't
//!    run the same way as the original. We assert the output does NOT
//!    contain any such comment for the *handled* opcodes.
//!
//! 3. **Total decompiler failures** — the panic hook in
//!    `decompile_bytecode` turns panics into `failed to decompile`
//!    comments. If a function that previously decompiled successfully
//!    starts emitting that comment, this test catches it.
//!
//! New fixtures can be added to `tests/fixtures/` and they'll be
//! automatically picked up — this test enumerates every `*.luau.bin`
//! file at runtime.
//!
//! See `AUDIT.md` in the project root for the full list of bugs this
//! test was originally written to guard against (notably the
//! `unimplemented!()` panic in `Lifter::constant`, the LOADNIL
//! `b < a` underflow, and the unchecked jump-target arithmetic).

use std::fs;
use std::path::Path;

const FIXTURES_DIR: &str = "tests/fixtures";
const ENCODE_KEY: u8 = 203;

/// Run the decompiler on every `*.luau.bin` fixture in `tests/fixtures/`
/// and verify it (a) doesn't panic, (b) doesn't emit a "failed to
/// decompile" comment, and (c) doesn't silently drop any handled
/// instruction as an "unhandled instruction" comment.
#[test]
fn every_fixture_decompiles_cleanly() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES_DIR);
    let mut fixtures: Vec<_> = fs::read_dir(&dir)
        .expect("could not read fixtures dir")
        .filter_map(|e| {
            let e = e.ok()?;
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    fixtures.sort();

    assert!(
        !fixtures.is_empty(),
        "no fixtures found in {FIXTURES_DIR}; the stress test has nothing to exercise"
    );

    let mut failures: Vec<(String, String)> = Vec::new();
    for path in fixtures {
        let bytecode = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                failures.push((path.display().to_string(), format!("read failed: {e}")));
                continue;
            }
        };
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let output = luau_lifter::decompile_bytecode(&bytecode, ENCODE_KEY);

        // (a) The decompiler must not have abandoned the whole function.
        if output.contains("failed to decompile") {
            failures.push((name.clone(), "output contains 'failed to decompile'".into()));
        }
        // (c) The decompiler must not have silently dropped any handled
        //     instruction. These are the "fallback" comments the lifter
        //     emits when it doesn't recognize a particular opcode or
        //     constant kind.
        if output.contains("unhandled instruction") {
            failures.push((name, "output contains 'unhandled instruction'".into()));
        }
    }

    if !failures.is_empty() {
        let mut msg = String::from("opcode stress test failed for fixtures:\n");
        for (name, reason) in &failures {
            msg.push_str(&format!("  - {name}: {reason}\n"));
        }
        panic!("{msg}");
    }
}

/// Verify that the lifter doesn't panic when handed a constant kind it
/// doesn't recognize. The pre-fix code had `unimplemented!()` in
/// `Lifter::constant` which aborted the whole decompilation; the fix
/// substitutes `nil` and continues. We can't easily synthesize a real
/// bytecode blob with an unknown constant kind without going through
/// the deserializer's internal encoding, so we just check that the
/// current fixtures don't crash — and document the case via this
/// test's name. If a future Luau version adds a new constant variant
/// that we don't know about, this test won't catch the regression
/// directly (since we can't fabricate the input), but the
/// `every_fixture_decompiles_cleanly` test above will catch the
/// "failed to decompile" symptom.
#[test]
fn decompile_does_not_panic_on_handled_fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES_DIR);
    let mut fixtures: Vec<_> = fs::read_dir(&dir)
        .expect("could not read fixtures dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("bin"))
        .collect();
    fixtures.sort();

    // Just calling `decompile_bytecode` and checking the result is
    // non-empty is enough to catch a panic. (If the lifter panics,
    // `decompile_bytecode` catches it and returns a string starting
    // with "-- Decomplied with Topaz" but containing the "failed to
    // decompile" comment instead of real source — caught by the test
    // above. Here we additionally check that the *outer* scaffolding
    // is present, which would catch a more catastrophic failure mode
    // where the panic hook itself is broken.)
    for path in fixtures {
        let bytecode = fs::read(&path).expect("read fixture");
        let output = luau_lifter::decompile_bytecode(&bytecode, ENCODE_KEY);
        assert!(
            output.starts_with("-- Decomplied with Topaz"),
            "fixture {} produced output without the standard header: {:?}",
            path.display(),
            &output[..output.len().min(200)]
        );
    }
}
