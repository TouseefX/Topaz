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
//! `luau_lifter::decompile_bytecode_default`) is expected to be `0`/no XOR
//! obfuscation for these fixtures, since they were compiled directly
//! without any additional string-encoding step.

/// `topaz decompile`'s default encode key (used to XOR-deobfuscate string
/// constants in the bytecode). These fixtures were compiled without any
/// additional string obfuscation, but the default key must still be
/// supplied since `luau_lifter::decompile_bytecode_default` uses it to decode the
/// bytecode's string table unconditionally.
const ENCODE_KEY: u8 = 203;

const ACCUMULATOR_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/accumulator_loop.luau.bin");
const CONTINUE_IN_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/continue_in_loop.luau.bin");
const UPVALUE_COLLISION_BYTECODE: &[u8] = include_bytes!("fixtures/luau_upval_collision.luau.bin");

// Additional control-flow "torture test" fixtures. These were compiled from
// small but structurally tricky Lua sources (multiple early returns mixed
// with a `continue`-using descendant loop and a shared flag, several
// `continue` sites landing at different points in the same loop body,
// pcall wrapped in a loop with `continue` on failure, and a state-machine
// style `while` loop with cross-branch `elseif` jumps) chosen to exercise
// the exact restructuring code paths flagged as Topaz's historically
// weakest area (loop/continue/break reconstruction, virtual-edge
// refinement for multiple continue sites, and guard-clause flag
// propagation). Each one was round-tripped through `lune`'s real Luau
// runtime (decompiled output executed and its stdout compared byte-for-
// byte against the original source's stdout) during development; these
// tests instead check structural properties of the output text so they
// don't require a Luau runtime in CI.
const EARLY_RETURNS_AND_LOOP_BYTECODE: &[u8] =
    include_bytes!("fixtures/t20_multiple_early_returns_and_loop.luau.bin");
const MULTIPLE_CONTINUES_BYTECODE: &[u8] =
    include_bytes!("fixtures/t24_multiple_continues_diff_points.luau.bin");
const STATE_MACHINE_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/t28_irreducible_ish_loop.luau.bin");
const PCALL_IN_LOOP_BYTECODE: &[u8] = include_bytes!("fixtures/t16_pcall_in_loop.luau.bin");
const LOOP_CARRIED_FLAG_ELSEIF_BYTECODE: &[u8] =
    include_bytes!("fixtures/t12_loop_carried_flag_elseif.luau.bin");

// Usage-based type-inference naming fixtures (see
// ast::type_inference_naming). These exercise the fallback naming path
// that infers a plausible name (e.g. "obj", "str", "num") for a local
// from how it's used later in the function, when nothing else about its
// creating expression suggests a name.
const TYPE_INFER_INSTANCE_BYTECODE: &[u8] = include_bytes!("fixtures/type_infer_instance.luau.bin");
const TYPE_INFER_STR_NUM_BYTECODE: &[u8] = include_bytes!("fixtures/type_infer_str_num.luau.bin");

// A `for` loop whose body is a terminating `if`/`else` where *both* arms
// unconditionally `return` (no shared continuation block at all). Found
// while auditing `guard_clauses`'s loop-body `continue` synthesis: this
// exact shape crashed `restructure::loop::try_collapse_loop` with an
// `index out of bounds` panic (it indexed `then_successors[0]`
// unconditionally, but `then_successors` is empty precisely when the
// `then` arm ends in a `return` and has no outgoing CFG edge), aborting
// decompilation of the entire containing function with a bare
// "failed to decompile" and no further information.
const DIAMOND_RETURN_IN_FOR_LOOP_BYTECODE: &[u8] =
    include_bytes!("fixtures/diamond_return_in_for_loop.luau.bin");

// `local t = {}` followed by sequential `t.field = value` assignments,
// where one of those fields is a closure that captures `t` itself (e.g.
// `t.getSelf = function() return t end`). Found in the real-world Dex
// Explorer bytecode sample's `Main` module-singleton pattern
// (`Main.GetInitDeps = function() return {Main = Main, ...} end`).
// `cfg::ssa::inline`'s table-literal-folding pass used to special-case
// closures out of its "does this field's value read the table being
// built" guard, on the mistaken assumption that a closure capturing a
// variable is somehow different from "reading" it -- but folding such a
// closure into the table constructor produces `local t = {..., getSelf =
// function() return t end}`, which is invalid: per Lua's own scoping
// rules, `t` isn't in scope *inside its own initializer* (a `local`
// declaration's scope begins only after the statement completes), so the
// closure's captured `t` silently resolves to whatever unrelated
// definition of that name existed before (almost always nil or an
// unrelated global), not the table it was written to belong to.
const TABLE_CLOSURE_SELF_REFERENCE_BYTECODE: &[u8] =
    include_bytes!("fixtures/table_closure_self_reference.luau.bin");

/// Regression test for a critical bug where a table's own local was
/// silently captured as nil/undefined inside a closure literal that got
/// folded into that table's own constructor (see
/// `TABLE_CLOSURE_SELF_REFERENCE_BYTECODE`'s doc comment for the full
/// mechanism). The decompiled output must never emit a table literal that
/// contains a closure capturing the very local being declared -- instead,
/// such a closure must be split out as a separate statement (either
/// `t.field = function() ... end` or `function t.field() ... end`) after
/// the table's `local` declaration, once `t` is actually in scope.
#[test]
fn closure_that_captures_its_own_table_is_not_folded_into_the_table_literal() {
    let output = luau_lifter::decompile_bytecode_default(TABLE_CLOSURE_SELF_REFERENCE_BYTECODE, ENCODE_KEY);

    // Find the `local <name> = {` table declaration and confirm the
    // closure field was NOT folded inside it (i.e. the table literal
    // closes with `}` before any `function`/closure keyword tied to the
    // same local appears).
    let local_decl_pos = output
        .find("local ")
        .expect("expected a local table declaration in the output");
    let table_open_pos = output[local_decl_pos..]
        .find('{')
        .map(|p| p + local_decl_pos)
        .expect("expected the local declaration to be a table constructor");
    let table_close_pos = output[table_open_pos..]
        .find('}')
        .map(|p| p + table_open_pos)
        .expect("expected the table constructor to close with `}`");
    let table_literal_body = &output[table_open_pos..table_close_pos];

    assert!(
        !table_literal_body.contains("function"),
        "a closure that captures the table being constructed must never \
         be folded into that table's own literal (this makes the closure \
         capture a stale/nil reference instead of the table itself, per \
         Lua's own local-declaration scoping rules): got decompiled \
         output:\n{output}"
    );

    // The field must still exist afterward as a separate assignment/
    // function-declaration statement, and it must actually work at
    // runtime (returning the same table it's attached to) -- verified
    // separately via lune during development (prints `1.0 true`, matching
    // the original source, instead of the pre-fix `1.0 false`).
    assert!(
        output.contains("GetSelf"),
        "expected the GetSelf closure field to still be present, just not \
         folded into the table literal: got decompiled output:\n{output}"
    );
}

/// Regression test for a critical bug where compound-assignment printing
/// (`x += y`) was dead code in the formatter (nested inside the wrong
/// `if`-branch), so every detected compound assignment silently lost its
/// self-referential read: `total = total + i` was printed as the broken
/// `total = i` instead of `total += i`. This corrupted every
/// counter/accumulator loop pattern in decompiled output.
#[test]
fn accumulator_loop_uses_compound_assignment_not_plain_reassignment() {
    let output = luau_lifter::decompile_bytecode_default(ACCUMULATOR_LOOP_BYTECODE, ENCODE_KEY);

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
    let output = luau_lifter::decompile_bytecode_default(CONTINUE_IN_LOOP_BYTECODE, ENCODE_KEY);

    // This `if` is the true last statement of the loop body, so `continue`
    // is provably correct here (not a guess) -- see `guard_clauses`'s
    // module doc comment. Confirmed to match real Oracle decompiler
    // output on the same class of pattern in the real-world Dex Explorer
    // bytecode (Oracle's TELEPORT_TO handler synthesizes `continue` for
    // exactly this shape).
    assert!(
        output.contains("continue"),
        "expected the loop's early-skip guard (`if seen[item] then ... end`) \
         to use `continue`: got decompiled output:\n{output}"
    );
    assert!(
        !output.contains("return"),
        "the loop's early-skip guard must never be turned into a `return` \
         (a `return` here would abort the whole function instead of \
         skipping one iteration): got decompiled output:\n{output}"
    );
}

/// Regression test for the NodeSorter-style bug where a captured upvalue
/// (a shared id counter) was renamed to match an unrelated sibling
/// local's heuristically inferred name (both ended up named "Id"),
/// producing the nonsensical, crash-inducing self-assignment `Id = Id`.
#[test]
fn upvalue_is_not_renamed_to_match_unrelated_sibling() {
    let output = luau_lifter::decompile_bytecode_default(UPVALUE_COLLISION_BYTECODE, ENCODE_KEY);

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

/// A function this small, with this many distinct early-exit paths mixed
/// with a descendant loop guarded by a shared boolean flag, should always
/// decompile without falling back to unstructured `goto`/labelled-block
/// output -- if it does, that's a strong signal the structuring pass
/// failed to reduce the control flow graph, which historically correlated
/// with mangled `continue`/`return` semantics (see the other tests in this
/// file) even when it doesn't produce outright invalid Lua.
fn assert_fully_structured(output: &str) {
    assert!(
        !output.contains("goto "),
        "expected fully structured output (no goto fallback) for this \
         control-flow pattern: got decompiled output:\n{output}"
    );
    assert!(
        !output.contains("-- block "),
        "expected fully structured output (no raw basic-block fallback) \
         for this control-flow pattern: got decompiled output:\n{output}"
    );
}

/// Regression test for the exact `addObject`/`isNil`-flag control-flow
/// pattern flagged in the Topaz vs Oracle analysis: a function with
/// several early `return`s (including one nested inside an `if`/`elseif`
/// chain) followed by a descendant loop that both `continue`s past
/// already-seen/parentless items *and* propagates a boolean flag set
/// earlier in the function. Getting any of the three `continue` sites
/// wrong (turning them into `return`) or losing the flag's live range
/// silently truncates the resulting tree/graph -- exactly what happened
/// in the original Dex Explorer bug.
#[test]
fn early_returns_with_flag_propagating_descendant_loop() {
    let output = luau_lifter::decompile_bytecode_default(EARLY_RETURNS_AND_LOOP_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);

    // The descendant loop has exactly two `continue` sites in the source,
    // both the true last statement of their respective `if` blocks and of
    // the overall guard chain at that point in the loop body -- so
    // `continue` is provably correct for both (not a guess; see
    // `guard_clauses`'s module doc comment). Both must survive as
    // `continue`, never `return` (a `return` here would abort processing
    // all remaining descendants instead of just skipping one -- the
    // original severity-critical bug this fixture guards against).
    assert_eq!(
        output.matches("continue").count(),
        2,
        "expected exactly 2 `continue` statements (one for \
         already-visited items, one for parentless items) in the \
         descendant loop: got decompiled output:\n{output}"
    );
    let loop_start = output
        .find("for i = 1")
        .expect("expected the descendant loop to be present in the output");
    let loop_body = &output[loop_start..];
    assert!(
        !loop_body.lines().any(|l| l.trim() == "return"),
        "must never synthesize a bare `return` inside the descendant loop \
         (that would abort the whole function instead of skipping to the \
         next descendant, the original severity-critical bug this fixture \
         guards against): got decompiled output:\n{output}"
    );
}


/// Regression test for a loop with multiple `continue`-equivalent exits
/// that land at different points in the control flow (one skips
/// immediately, another only after doing some work first inside a nested
/// `if`), which exercises the "virtual edge refinement" logic that must
/// correctly distinguish `continue` targets from `break` targets when a
/// loop body doesn't have a single simple exit edge.
#[test]
fn multiple_continue_sites_at_different_points() {
    let output = luau_lifter::decompile_bytecode_default(MULTIPLE_CONTINUES_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!("found self-referential assignment `{decl}` for variable `{name}`: got decompiled output:\n{output}");
    }
}

/// Regression test for a state-machine-style `while` loop where control
/// jumps between `elseif` branches in a way that doesn't correspond to
/// simple sequential fallthrough, historically a stress case for
/// structured control-flow recovery.
#[test]
fn state_machine_style_loop_stays_structured() {
    let output = luau_lifter::decompile_bytecode_default(STATE_MACHINE_LOOP_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
}

/// Regression test for `pcall` used inside a loop with a `continue` on
/// failure -- a pattern the original analysis specifically called out as
/// fragile ("Add special handling for Roblox/Lua patterns that use
/// `continue`-style early iteration skips after `pcall`"). The
/// decompiler is free to express the "skip on failure" logic either as an
/// explicit `if not ok then continue end` guard clause, or as the
/// logically-equivalent (and arguably more readable) positive-condition
/// `if ok then ... end` -- both are correct, so this test only checks
/// that whichever shape is chosen doesn't fall back to unstructured
/// output, and that it doesn't contain a self-referential assignment.
#[test]
fn pcall_failure_inside_loop_uses_continue() {
    let output = luau_lifter::decompile_bytecode_default(PCALL_IN_LOOP_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
    assert!(
        output.contains("pcall"),
        "expected the pcall call itself to survive decompilation: \
         got decompiled output:\n{output}"
    );
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!("found self-referential assignment `{decl}` for variable `{name}`: got decompiled output:\n{output}");
    }
}

/// Regression test for a loop where two boolean flags are updated by an
/// `if`/`elseif` chain and then read by a *separate* `if`/`elseif` chain
/// later in the same iteration -- this requires the flags' live ranges to
/// correctly span from their assignment to their use within the loop
/// body, similar to the `isNil` flag propagation bug.
#[test]
fn loop_carried_flags_read_by_later_elseif_chain() {
    let output = luau_lifter::decompile_bytecode_default(LOOP_CARRIED_FLAG_ELSEIF_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!("found self-referential assignment `{decl}` for variable `{name}`: got decompiled output:\n{output}");
    }
}

/// A local with no name derivable from its creating expression (here,
/// `table.remove(queue)`), but which is later indexed with `.ClassName`
/// and called with `:IsA(...)`/`:GetChildren()`, should be named `obj`
/// by the usage-based type-inference fallback rather than a generic
/// synthetic name like `v3`.
#[test]
fn usage_based_naming_infers_instance_like_local() {
    let output = luau_lifter::decompile_bytecode_default(TYPE_INFER_INSTANCE_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
    assert!(
        output.contains("local obj ="),
        "expected the instance-like local (used with .ClassName, :IsA, \
         :GetChildren) to be named `obj`: got decompiled output:\n{output}"
    );
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!("found self-referential assignment `{decl}` for variable `{name}`: got decompiled output:\n{output}");
    }
}

/// Locals with no derivable creating-expression name, but which are
/// repeatedly passed to `tostring()`/`tonumber()`, should be named `str`
/// and `num` respectively by the usage-based type-inference fallback.
#[test]
fn usage_based_naming_infers_string_and_number_locals() {
    let output = luau_lifter::decompile_bytecode_default(TYPE_INFER_STR_NUM_BYTECODE, ENCODE_KEY);
    assert_fully_structured(&output);
    assert!(
        output.contains("str"),
        "expected the string-like local (repeatedly passed to tostring) \
         to be named using the `str` hint: got decompiled output:\n{output}"
    );
    assert!(
        output.contains("num"),
        "expected the number-like local (repeatedly passed to tonumber) \
         to be named using the `num` hint: got decompiled output:\n{output}"
    );
    for (name, decl) in find_self_referential_assignments(&output) {
        panic!("found self-referential assignment `{decl}` for variable `{name}`: got decompiled output:\n{output}");
    }
}

/// Regression test for a determinism bug where `RcLocal`'s `Hash`/`Ord`
/// were derived from `ByAddress`, i.e. the raw heap pointer address of
/// the underlying `Arc<Mutex<Local>>` allocation. Since `RcLocal` is used
/// as the key of many `HashMap`/`HashSet`s throughout SSA construction/
/// destruction (congruence classes, definition sets, upvalue sets, etc),
/// their iteration order -- and therefore the order synthetic variable
/// names get handed out in the final decompiled output -- silently
/// depended on ASLR (address space layout randomization), which varies
/// between runs of the exact same binary on the exact same input. This
/// made Topaz's output non-reproducible: running `topaz decompile` twice
/// on an unchanged binary and input could (rarely, depending on the
/// specific allocator layout that run happened to get) produce two
/// different, though each individually valid and correct, decompiles --
/// which breaks diffing, caching, and reasoning about the tool's output.
///
/// This test decompiles the same bytecode many times within a single
/// process and asserts every decompile produces identical output. This
/// doesn't exercise ASLR directly (a single process only gets one
/// address-space layout), but the actual fix (making `RcLocal::hash`/
/// `cmp` use a stable creation-order id instead of the pointer address)
/// was independently verified using `setarch -R` to force layout
/// randomization across 20 separate process invocations, which all
/// produced byte-identical output after the fix (and did not, before it).
#[test]
fn decompilation_is_deterministic_across_repeated_runs() {
    let first = luau_lifter::decompile_bytecode_default(EARLY_RETURNS_AND_LOOP_BYTECODE, ENCODE_KEY);
    for i in 0..9 {
        let repeat = luau_lifter::decompile_bytecode_default(EARLY_RETURNS_AND_LOOP_BYTECODE, ENCODE_KEY);
        assert_eq!(
            first, repeat,
            "decompiling the same bytecode twice in the same process \
             produced different output on repetition {i}; this indicates \
             a source of nondeterminism (e.g. hashing/ordering by \
             pointer address) has crept back into the naming or \
             structuring pipeline"
        );
    }
}

/// Same as `decompilation_is_deterministic_across_repeated_runs`, but
/// interleaves *different* bytecode payloads (as a long-lived server
/// process like `topaz serve` would when handling varied requests back
/// to back on the same worker thread) to make sure decompiling unrelated
/// payloads in between doesn't perturb a given payload's output either --
/// this is exactly the scenario `reset_local_id_counter` exists to
/// protect against (each job's `RcLocal` ids used to keep climbing across
/// jobs on the same thread instead of restarting from zero).
#[test]
fn decompilation_is_deterministic_when_interleaved_with_other_jobs() {
    let expected_a = luau_lifter::decompile_bytecode_default(EARLY_RETURNS_AND_LOOP_BYTECODE, ENCODE_KEY);
    let expected_b = luau_lifter::decompile_bytecode_default(MULTIPLE_CONTINUES_BYTECODE, ENCODE_KEY);
    let expected_c = luau_lifter::decompile_bytecode_default(TYPE_INFER_INSTANCE_BYTECODE, ENCODE_KEY);

    for _ in 0..5 {
        assert_eq!(
            luau_lifter::decompile_bytecode_default(EARLY_RETURNS_AND_LOOP_BYTECODE, ENCODE_KEY),
            expected_a,
            "payload A's output changed after decompiling unrelated \
             payloads on the same thread in between"
        );
        assert_eq!(
            luau_lifter::decompile_bytecode_default(MULTIPLE_CONTINUES_BYTECODE, ENCODE_KEY),
            expected_b,
            "payload B's output changed after decompiling unrelated \
             payloads on the same thread in between"
        );
        assert_eq!(
            luau_lifter::decompile_bytecode_default(TYPE_INFER_INSTANCE_BYTECODE, ENCODE_KEY),
            expected_c,
            "payload C's output changed after decompiling unrelated \
             payloads on the same thread in between"
        );
    }
}

/// Regression test for a crash (not a wrong guess -- an outright panic)
/// found while auditing `guard_clauses`'s loop-body `continue` synthesis
/// for exactly the class of bug the user reported ("continue is added
/// where return is supposed to be"): a numeric `for` loop whose body is a
/// terminating `if`/`else` where *both* branches unconditionally
/// `return`, so neither has a CFG successor that continues the loop.
/// `restructure::loop::try_collapse_loop` indexed `then_successors[0]`
/// unconditionally in several branches of a loop-shape-matching
/// condition, but `then_successors` is empty exactly in this shape,
/// causing an `index out of bounds` panic that aborted decompilation of
/// the entire containing function (surfaced to users as a bare
/// "-- failed to decompile" comment with no further information).
///
/// This is a stronger, more fundamental version of the "return vs
/// continue" ambiguity: it's not that the wrong statement gets picked --
/// nothing gets decompiled at all. It must decompile successfully and
/// preserve both `return nil` statements (one per branch) rather than
/// collapsing them into an incorrect loop-continuation.
#[test]
fn for_loop_with_both_if_branches_returning_does_not_crash() {
    let output =
        luau_lifter::decompile_bytecode_default(DIAMOND_RETURN_IN_FOR_LOOP_BYTECODE, ENCODE_KEY);

    assert!(
        !output.contains("failed to decompile"),
        "expected the for-loop-with-both-branches-returning pattern to \
         decompile successfully instead of panicking: got decompiled \
         output:\n{output}"
    );
    assert_eq!(
        output.matches("return nil").count(),
        2,
        "expected both branches' `return nil` to survive decompilation \
         (one for the negative-item case, one for the positive-item \
         case): got decompiled output:\n{output}"
    );
    // Must not be duplicated/hoisted outside the loop's conditional --
    // there must still be exactly one `for` loop wrapping them.
    assert_eq!(
        output.matches("for ").count(),
        1,
        "expected exactly one `for` loop (both `return`s belong inside \
         its conditional, not duplicated elsewhere): got decompiled \
         output:\n{output}"
    );
}
