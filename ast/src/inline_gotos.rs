//! Post-passes that remove CFG-restructuring `goto`/`::label::` fallbacks.
//!
//! Luau has **no** `goto`. Topaz's restructurer emits `goto lN` / `::lN::` when
//! it cannot fold a control-flow edge into pure if/while/for. These passes turn
//! the common join-point patterns back into structured Luau.
//!
//! Patterns (see real dumps Place_* scripts):
//!
//! 1. **Fallthrough join** (if + tail both jump to same label, then fall through):
//!    ```text
//!    if C then ...; goto L end
//!    ...; goto L
//!    ::L::
//!    rest
//!    ```
//!    → `if C then ... end` + middle + `rest` (no gotos).
//!
//! 2. **Else-arm join label** (diamond collapsed as if/else with label in else):
//!    ```text
//!    if C then goto L else ::L:: body end
//!    ```
//!    → `if not C then body end`
//!
//! 3. **Nested skip if** into else join:
//!    ```text
//!    if C then if D then goto L end else ::L:: body end
//!    ```
//!    → `if not C or D then body end`
//!
//! 4. Existing **short-tail inlining** for gotos into a short terminating tail.

use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;

use crate::{
    deep_clone_statement, Binary, BinaryOperation, Block, If, Literal, RValue, Repeat, Statement,
    Traverse, Unary, UnaryOperation, While,
};

/// Recurse into nested `if`/`while`/`for` bodies without re-entering a
/// mutex this thread already holds, and without walking an `Arc` twice
/// (aliased `else`-chains used to expand forever).
fn for_each_nested(stmt: &Statement, f: &mut impl FnMut(&Block)) {
    match stmt {
        Statement::If(r#if) => {
            crate::visit_shared(&r#if.then_block, f);
            crate::visit_shared(&r#if.else_block, f);
        }
        Statement::While(w) => crate::visit_shared(&w.block, f),
        Statement::Repeat(r) => crate::visit_shared(&r.block, f),
        Statement::NumericFor(n) => crate::visit_shared(&n.block, f),
        Statement::GenericFor(g) => crate::visit_shared(&g.block, f),
        _ => {}
    }
}

fn for_each_nested_mut(stmt: &mut Statement, f: &mut impl FnMut(&mut Block)) {
    match stmt {
        Statement::If(r#if) => {
            crate::visit_shared_mut(&r#if.then_block, f);
            crate::visit_shared_mut(&r#if.else_block, f);
        }
        Statement::While(w) => crate::visit_shared_mut(&w.block, f),
        Statement::Repeat(r) => crate::visit_shared_mut(&r.block, f),
        Statement::NumericFor(n) => crate::visit_shared_mut(&n.block, f),
        Statement::GenericFor(g) => crate::visit_shared_mut(&g.block, f),
        _ => {}
    }
}

fn cycle_debug_level() -> u8 {
    match std::env::var("TOPAZ_DEBUG_CYCLES") {
        Ok(s) if s == "2" || s.eq_ignore_ascii_case("verbose") => 2,
        Ok(s) if s == "0" || s.eq_ignore_ascii_case("false") || s.is_empty() => 0,
        Ok(_) => 1,
        Err(_) => 0,
    }
}

pub fn inline_short_gotos(block: &mut Block) {
    let depth = crate::goto_depth_enter();
    struct DepthGuard(u32);
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            crate::goto_depth_leave(self.0);
        }
    }
    let _depth_guard = DepthGuard(depth);
    if depth == 0 {
        crate::reset_done_funcs();
    }
    if crate::past_post_deadline() {
        return;
    }
    let dbg = cycle_debug_level();
    let t0 = std::time::Instant::now();
    let rounds = if block.0.len() > 20_000 {
        2
    } else if block.0.len() > 4_000 {
        4
    } else {
        16
    };
    let mut last_round = 0usize;
    let mut last_tails = 0usize;
    for _round in 0..rounds {
        if crate::past_post_deadline() {
            break;
        }
        last_round = _round;
        let mut changed = false;
        crate::reset_walk_seen();
        changed |= eliminate_join_gotos(block);
        if crate::past_post_deadline() {
            break;
        }
        crate::reset_walk_seen();
        changed |= rewrite_skip_rest_gotos(block);
        if crate::past_post_deadline() {
            break;
        }
        crate::reset_walk_seen();
        changed |= rewrite_back_edge_loops(block);
        crate::reset_walk_seen();
        let tails = collect_short_tails(block);
        last_tails = tails.len();
        // Level 1: one line per round on the root body only. Nested
        // closures used to dump 569k lines *inside* the 180s budget.
        if dbg >= 2 || (dbg >= 1 && depth == 0) {
            eprintln!("[goto] round {}: {} tails", _round, tails.len());
        }
        if !tails.is_empty() {
            crate::reset_walk_seen();
            replace_gotos(block, &tails, &mut changed);
        }
        if !changed {
            break;
        }
    }
    if dbg >= 1 && depth == 0 {
        eprintln!(
            "[goto] done: last_round={} tails={} {:.1}s",
            last_round,
            last_tails,
            t0.elapsed().as_secs_f64()
        );
    }
    if !crate::past_post_deadline() {
        crate::reset_walk_seen();
        let tails = collect_short_tails(block);
        crate::reset_walk_seen();
        prune_unused_labels(block, &tails);
        crate::reset_walk_seen();
        remove_orphan_labels(block);
        crate::reset_walk_seen();
        drop_unresolved_trailing_gotos(block);
        crate::reset_walk_seen();
        descend_into_closures(block);
    }
}

/// Final cleanup: `goto L` with no matching `::L::` left in the function.
fn drop_unresolved_trailing_gotos(block: &mut Block) {
    let mut labels = std::collections::HashSet::new();
    collect_label_names(block, &mut labels);
    strip_gotos_not_in(block, &labels);
}

fn collect_label_names(block: &Block, labels: &mut std::collections::HashSet<String>) {
    for s in &block.0 {
        if let Statement::Label(l) = s {
            labels.insert(l.0.clone());
        }
        for_each_nested(s, &mut |b| collect_label_names(b, labels));
    }
}

fn strip_gotos_not_in(block: &mut Block, labels: &std::collections::HashSet<String>) {
    block.0.retain(|s| match s {
        Statement::Goto(g) => labels.contains(&g.0 .0),
        _ => true,
    });
    for s in block.0.iter_mut() {
        for_each_nested_mut(s, &mut |b| strip_gotos_not_in(b, labels));
    }
}

fn descend_into_closures(block: &mut Block) {
    if crate::past_post_deadline() {
        return;
    }
    for statement in block.0.iter_mut() {
        statement.traverse_rvalues(&mut |rv: &mut RValue| {
            if crate::past_post_deadline() {
                return;
            }
            if let RValue::Closure(closure) = rv {
                let ptr = triomphe::Arc::as_ptr(&closure.function.0) as *const ();
                if !crate::mark_func_done(ptr) {
                    return;
                }
                if let Some(mut f) = closure.function.try_lock() {
                    inline_short_gotos(&mut f.body);
                }
            }
        });
        for_each_nested_mut(statement, &mut |b| descend_into_closures(b));
    }
}

/// Remove join-point gotos that restructure left as structured control flow.
fn eliminate_join_gotos(block: &mut Block) -> bool {
    if crate::past_post_deadline() {
        return false;
    }
    let mut changed = false;
    // Recurse first for else/fallthrough patterns (nested joins).
    for statement in block.0.iter_mut() {
        for_each_nested_mut(statement, &mut |b| {
            changed |= eliminate_join_gotos(b);
        });
    }

    changed |= rewrite_else_join_labels(block);
    changed |= rewrite_fallthrough_joins(block);
    // skip_rest is parent-before-child and must run as its own walk so nested
    // `goto L` is not stripped before the parent can turn it into else.
    changed
}

fn label_name(s: &Statement) -> Option<&str> {
    match s {
        Statement::Label(l) => Some(l.0.as_str()),
        _ => None,
    }
}

fn goto_name(s: &Statement) -> Option<&str> {
    match s {
        Statement::Goto(g) => Some(g.0 .0.as_str()),
        _ => None,
    }
}

fn ends_with_goto_named(stmts: &[Statement], name: &str) -> bool {
    matches!(stmts.last(), Some(Statement::Goto(g)) if g.0 .0 == name)
}

fn strip_trailing_goto(stmts: &mut Vec<Statement>, name: &str) -> bool {
    if ends_with_goto_named(stmts, name) {
        stmts.pop();
        true
    } else {
        false
    }
}

/// Strip trailing `goto name` at the end of `stmts`, and also trailing
/// `goto name` inside a trailing nested `if` with empty else (early exit to join).
fn strip_trailing_goto_deep(stmts: &mut Vec<Statement>, name: &str) -> bool {
    let mut changed = strip_trailing_goto(stmts, name);
    // Walk from the end: if last stmt is if with empty else ending in goto name, strip there.
    if let Some(Statement::If(r#if)) = stmts.last_mut() {
        let else_empty = r#if
            .else_block
            .try_lock()
            .map(|b| b.0.is_empty())
            .unwrap_or(false);
        if else_empty {
            if let Some(mut then_b) = r#if.then_block.try_lock() {
                if strip_trailing_goto_deep(&mut then_b.0, name) {
                    changed = true;
                }
            }
        }
    }
    changed
}

fn make_not(cond: RValue) -> RValue {
    Unary {
        value: Box::new(cond),
        operation: UnaryOperation::Not,
    }
    .into()
}

fn make_or(left: RValue, right: RValue) -> RValue {
    Binary::new(left, right, BinaryOperation::Or).into()
}

/// Pattern: if C then ... goto L else ::L:: body end
/// and variants with a nested single-if that only goto L.
fn rewrite_else_join_labels(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.0.len() {
        let rewritten = match &block.0[i] {
            Statement::If(r#if) => try_rewrite_if_else_join(r#if),
            _ => None,
        };
        if let Some(stmts) = rewritten {
            let n = stmts.len();
            block.0.splice(i..=i, stmts);
            changed = true;
            i += n;
            continue;
        }
        i += 1;
    }
    changed
}

fn try_rewrite_if_else_join(r#if: &If) -> Option<Vec<Statement>> {
    let then_block = r#if.then_block.try_lock()?;
    let else_block = r#if.else_block.try_lock()?;

    // else must start with ::L::
    let label = label_name(else_block.0.first()?)?;
    let label = label.to_string();

    // Body under label (everything after the label in else).
    let body: Block = else_block.0[1..].to_vec().into();

    // Case 1: then is only `goto L`
    if then_block.0.len() == 1 && goto_name(&then_block.0[0]) == Some(label.as_str()) {
        drop(then_block);
        drop(else_block);
        return Some(vec![Statement::If(If::new(
            make_not(r#if.condition.clone()),
            body,
            Block::default(),
        ))]);
    }

    // Case 2: then is only `if D then goto L end` (empty else on inner)
    if then_block.0.len() == 1 {
        if let Statement::If(inner) = &then_block.0[0] {
            let inner_then = inner.then_block.try_lock()?;
            let inner_else = inner.else_block.try_lock()?;
            let only_goto = inner_then.0.len() == 1
                && goto_name(&inner_then.0[0]) == Some(label.as_str())
                && inner_else.0.is_empty();
            if only_goto {
                let d = inner.condition.clone();
                drop(inner_then);
                drop(inner_else);
                drop(then_block);
                drop(else_block);
                // body when: not C or D
                let cond = make_or(make_not(r#if.condition.clone()), d);
                return Some(vec![Statement::If(If::new(cond, body, Block::default()))]);
            }
        }
    }

    // Case 3: then ends with goto L, with prefix A before the goto.
    // if C then A; goto L else ::L:: body end
    //
    // Control: both sides execute `body`. Side C also runs A first.
    //   if C then A end
    //   body
    //
    // Nested pure skip-if `if D then goto L end` inside A only jumps to the
    // join early (skips rest of A). We leave those as `if D then else body-less
    // continue A` is hard; only fold when A is entirely pure skip-ifs:
    //   if not C or D1 or D2 then body end
    if ends_with_goto_named(&then_block.0, &label) && then_block.0.len() > 1 {
        let has_label = then_block
            .0
            .iter()
            .any(|s| label_name(s) == Some(label.as_str()));
        if !has_label {
            let then_prefix: Vec<Statement> = then_block.0[..then_block.0.len() - 1].to_vec();

            // Collect pure skip conditions; leftover real statements.
            let mut a_stmts = Vec::new();
            let mut skip_conds: Vec<RValue> = Vec::new();
            for stmt in &then_prefix {
                if let Statement::If(inner) = stmt {
                    let Some(inner_then) = inner.then_block.try_lock() else {
                        a_stmts.push(stmt.clone());
                        continue;
                    };
                    let Some(inner_else) = inner.else_block.try_lock() else {
                        a_stmts.push(stmt.clone());
                        continue;
                    };
                    let pure_skip = inner_then.0.len() == 1
                        && goto_name(&inner_then.0[0]) == Some(label.as_str())
                        && inner_else.0.is_empty();
                    if pure_skip {
                        skip_conds.push(inner.condition.clone());
                        continue;
                    }
                }
                a_stmts.push(stmt.clone());
            }

            drop(then_block);
            drop(else_block);

            // All of A were pure skip-ifs (or A was only skip-ifs + final goto).
            if a_stmts.is_empty() {
                let mut cond = make_not(r#if.condition.clone());
                for d in skip_conds {
                    cond = make_or(cond, d);
                }
                return Some(vec![Statement::If(If::new(cond, body, Block::default()))]);
            }

            // Real work in A: if C then A end; body
            // (skip_conds inside A left as-is in a_stmts when not pure — we only
            // stripped pure ones; if pure ones were mixed, they were removed which
            // is OK: they only jumped to body early, equivalent to skipping rest of A)
            let mut out = vec![Statement::If(If::new(
                r#if.condition.clone(),
                a_stmts.into(),
                Block::default(),
            ))];
            out.extend(body.0);
            return Some(out);
        }
    }

    None
}


fn rewrite_fallthrough_joins(block: &mut Block) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < block.0.len() {
        // Patterns:
        //   A) if C then ... goto L end; [middle]; goto L; ::L:: rest
        //   B) if C then ... goto L end; [middle]; ::L:: rest
        //
        // Nested skip-forward gotos to L inside C's then (e.g. early exit
        // past a sibling block) MUST be rewritten to if/else BEFORE we
        // remove ::L::, otherwise drop_unresolved_trailing_gotos deletes
        // them and both blocks run (silent behavior bug).
        if i + 1 >= block.0.len() {
            break;
        }

        let Some((label, label_idx, has_pre_goto)) = find_join_after_if(block, i) else {
            i += 1;
            continue;
        };

        if let Statement::If(r#if) = &mut block.0[i] {
            if let Some(mut then_b) = r#if.then_block.try_lock() {
                // 1) Preserve skip-forward semantics for every remaining goto L.
                while rewrite_one_skip_forward_to_label(&mut then_b.0, &label) {
                    changed = true;
                }
                // 2) Only now remove the trailing join jump at end of then.
                if strip_trailing_goto(&mut then_b.0, &label) {
                    changed = true;
                }
            }
        }

        // 3) Remove label (and optional pre-goto) — safe: no nested goto L left.
        block.0.remove(label_idx);
        if has_pre_goto {
            block.0.remove(label_idx - 1);
        }
        changed = true;
    }
    changed
}

/// Within `stmts` (typically an if-then body whose join label is `label`
/// after the enclosing if), rewrite one skip-forward `goto label`:
///
/// ```text
/// if D then
///   ...
///   goto label   -- skip rest
/// end
/// rest...
/// ```
/// →
/// ```text
/// if D then
///   ...
/// else
///   rest...
/// end
/// ```
///
/// Also handles one nesting level used by real dumps:
/// ```text
/// if U then
///   ...
///   if D then B; goto label end
/// end
/// rest...
/// ```
/// → `if U then ...; if D then B else rest end else rest end`
/// (rest is cloned into both else arms when U's false path must run rest).
fn rewrite_one_skip_forward_to_label(stmts: &mut Vec<Statement>, label: &str) -> bool {
    // Top-level: if D then ... goto L end; rest
    for i in 0..stmts.len() {
        let (else_empty, then_ends, then_len) = {
            let Statement::If(r#if) = &stmts[i] else {
                continue;
            };
            let Some(then_b) = r#if.then_block.try_lock() else {
                continue;
            };
            let Some(else_b) = r#if.else_block.try_lock() else {
                continue;
            };
            (
                else_b.0.is_empty(),
                ends_with_goto_named(&then_b.0, label),
                then_b.0.len(),
            )
        };
        if else_empty && then_ends && then_len >= 1 && i + 1 < stmts.len() {
            let rest: Vec<Statement> = stmts.drain(i + 1..).collect();
            if let Statement::If(r#if) = &mut stmts[i] {
                if let Some(mut then_b) = r#if.then_block.try_lock() {
                    strip_trailing_goto(&mut then_b.0, label);
                }
                if let Some(mut else_b) = r#if.else_block.try_lock() {
                    else_b.0 = rest;
                    return true;
                }
            }
            stmts.extend(rest);
            return false;
        }
    }

    // Nested: if U then [..., if D then B; goto L end] end; rest
    // where the inner if is the last stmt of U's then and ends with goto L.
    for i in 0..stmts.len() {
        let nested = {
            let Statement::If(outer) = &stmts[i] else {
                continue;
            };
            let else_empty = outer
                .else_block
                .try_lock()
                .map(|b| b.0.is_empty())
                .unwrap_or(false);
            if !else_empty {
                continue;
            }
            let Some(then_b) = outer.then_block.try_lock() else {
                continue;
            };
            if then_b.0.is_empty() {
                continue;
            }
            let last = then_b.0.last().unwrap();
            let Statement::If(inner) = last else {
                continue;
            };
            let Some(inner_then) = inner.then_block.try_lock() else {
                continue;
            };
            let Some(inner_else) = inner.else_block.try_lock() else {
                continue;
            };
            inner_else.0.is_empty() && ends_with_goto_named(&inner_then.0, label)
        };
        if !nested || i + 1 >= stmts.len() {
            continue;
        }
        let rest: Vec<Statement> = stmts.drain(i + 1..).collect();
        if let Statement::If(outer) = &mut stmts[i] {
            // Outer false path must still run rest. Detach so the two else
            // arms do not share nested `Arc<Mutex<Block>>`s — a shallow
            // `rest.clone()` aliased them and the next walker self-deadlocked.
            if let Some(mut else_b) = outer.else_block.try_lock() {
                else_b.0 = rest.iter().map(deep_clone_statement).collect();
            }
            if let Some(mut then_b) = outer.then_block.try_lock() {
                // Inner if is last in then: absorb rest into its else, strip goto.
                if let Some(Statement::If(inner)) = then_b.0.last_mut() {
                    if let Some(mut inner_then) = inner.then_block.try_lock() {
                        strip_trailing_goto(&mut inner_then.0, label);
                    }
                    if let Some(mut inner_else) = inner.else_block.try_lock() {
                        inner_else.0 = rest;
                        return true;
                    }
                }
            }
        }
        stmts.extend(rest);
        return false;
    }

    // Recurse into nested if bodies for deeper skip-forward gotos.
    for s in stmts.iter_mut() {
        if let Statement::If(r#if) = s {
            if let Some(mut then_b) = r#if.then_block.try_lock() {
                if rewrite_one_skip_forward_to_label(&mut then_b.0, label) {
                    return true;
                }
            }
            if let Some(mut else_b) = r#if.else_block.try_lock() {
                if rewrite_one_skip_forward_to_label(&mut else_b.0, label) {
                    return true;
                }
            }
        }
    }
    false
}

/// Returns (label_name, label_index, pre_goto_before_label).
///
/// Accepts:
/// - `goto L; ::L::` after the if (with optional middle stmts)
/// - bare `::L::` after the if when the if's then ends with `goto L`
///   and no other goto L exists in the middle (then is skipping middle).
fn find_join_after_if(block: &Block, if_idx: usize) -> Option<(String, usize, bool)> {
    let Statement::If(r#if) = &block.0[if_idx] else {
        return None;
    };
    // Never hold then and else together: aliased branches deadlock.
    let else_empty = r#if
        .else_block
        .try_lock()
        .map(|b| b.0.is_empty())
        .unwrap_or(false);
    if !else_empty {
        return None;
    }
    let then_b = r#if.then_block.try_lock()?;
    let label = goto_name(then_b.0.last()?)?.to_string();

    // then must end with goto L; optionally allow nested structure that ends that way.
    if !ends_with_goto_named(&then_b.0, &label) {
        return None;
    }
    drop(then_b);

    // Find ::L:: after if_idx. Prefer the first label L that is only targeted
    // by the then-arm and optional single goto L just before the label.
    let mut j = if_idx + 1;
    while j < block.0.len() {
        if label_name(&block.0[j]) == Some(label.as_str()) {
            let has_pre_goto =
                j > if_idx + 0 && goto_name(&block.0[j - 1]) == Some(label.as_str());
            // Middle must not define the same label earlier (we're at first L).
            // Middle may contain other gotos to L only if has_pre_goto (the one we remove).
            // If middle has additional goto L not immediately before label, bail.
            let mut k = if_idx + 1;
            while k < j {
                if k == j - 1 && has_pre_goto {
                    k += 1;
                    continue;
                }
                if goto_name(&block.0[k]) == Some(label.as_str()) {
                    return None;
                }
                if label_name(&block.0[k]) == Some(label.as_str()) {
                    return None;
                }
                k += 1;
            }
            return Some((label, j, has_pre_goto));
        }
        j += 1;
    }
    None
}

/// Convert mid-block `goto L` that only skips the rest of the *same* block
/// into structured if/else, when `::L::` is no longer present (already removed
/// by a join pass) or when L labels the immediate fallthrough after an if.
///
/// Example (after outer join label removed):
/// ```text
/// if C then
///   A
///   if D then
///     B
///     goto L   -- skip E
///   end
///   E
/// end
/// ```
/// →
/// ```text
/// if C then
///   A
///   if D then
///     B
///   else
///     E
///   end
/// end
/// ```
fn rewrite_skip_rest_gotos(block: &mut Block) -> bool {
    if crate::past_post_deadline() {
        return false;
    }
    let mut changed = false;

    // Handle this block first so patterns like
    //   if D then B; goto L end; E
    // are rewritten before recursion strips the inner goto.
    let mut i = 0;
    while i < block.0.len() {
        // Case: if D then ...; goto L end; rest...
        // where L is not defined later in this block → goto meant "skip rest".
        if let Statement::If(r#if) = &block.0[i] {
            let else_empty = r#if
                .else_block
                .try_lock()
                .map(|b| b.0.is_empty())
                .unwrap_or(false);
            let (label, then_len) = match r#if.then_block.try_lock() {
                Some(then_b) => (
                    then_b.0.last().and_then(goto_name).map(|s| s.to_string()),
                    then_b.0.len(),
                ),
                None => (None, 0),
            };

            if else_empty {
                if let Some(label) = label {
                    // Label must not appear later in this block (already cleaned join).
                    let label_later = block.0[i + 1..]
                        .iter()
                        .any(|s| label_name(s) == Some(label.as_str()));
                    let goto_later = block.0[i + 1..]
                        .iter()
                        .any(|s| goto_name(s) == Some(label.as_str()));
                    // A label *before* this if is a back-edge (loop), not
                    // "skip the rest". Leave it for `rewrite_back_edge_loops`.
                    let label_earlier = block.0[..i]
                        .iter()
                        .any(|s| label_name(s) == Some(label.as_str()));

                    if !label_earlier && !label_later && !goto_later && then_len >= 1 {
                        // Move statements after this if into the else-arm; strip goto from then.
                        let rest: Vec<Statement> = block.0.drain(i + 1..).collect();
                        if let Statement::If(r#if) = &mut block.0[i] {
                            if let Some(mut then_b) = r#if.then_block.try_lock() {
                                strip_trailing_goto(&mut then_b.0, &label);
                            }
                            if let Some(mut else_b) = r#if.else_block.try_lock() {
                                else_b.0 = rest;
                                changed = true;
                                break;
                            }
                        }
                        block.0.extend(rest);
                        break;
                    }
                }
            }
        }

        // Do not strip bare trailing gotos here — parent skip-rest needs them.
        // Orphan trailing gotos (last stmt, no label) are dropped after joins.
        i += 1;
    }

    // Then recurse into nested blocks.
    for statement in block.0.iter_mut() {
        for_each_nested_mut(statement, &mut |b| {
            changed |= rewrite_skip_rest_gotos(b);
        });
    }

    changed
}

/// Turn remaining back-edges into `while`/`repeat` so we never inline a
/// loop tail into itself (the shallow-clone deadlock) and so the output
/// is valid Luau (no `goto`).
///
/// ```text
/// ::L:: body; goto L                 →  while true do body end
/// ::L:: if c then B; goto L end      →  while c do B end
/// ::L:: A; if c then goto L end      →  repeat A until not c
/// ```
fn rewrite_back_edge_loops(block: &mut Block) -> bool {
    if crate::past_post_deadline() {
        return false;
    }
    let mut changed = false;
    for statement in block.0.iter_mut() {
        for_each_nested_mut(statement, &mut |b| {
            changed |= rewrite_back_edge_loops(b);
        });
    }

    let mut i = 0;
    while i < block.0.len() {
        let Some(label) = label_name(&block.0[i]).map(str::to_owned) else {
            i += 1;
            continue;
        };
        let mut region_end = i + 1;
        while region_end < block.0.len() && label_name(&block.0[region_end]).is_none() {
            region_end += 1;
        }

        let mut edge: Option<(usize, bool)> = None;
        for j in (i + 1..region_end).rev() {
            if goto_name(&block.0[j]) == Some(label.as_str()) {
                edge = Some((j, true));
                break;
            }
            if trailing_if_back_edge(&block.0[j], &label) {
                edge = Some((j, false));
                break;
            }
        }
        let Some((j, bare_goto)) = edge else {
            i += 1;
            continue;
        };

        let prefix_has_goto = block.0[i + 1..j].iter().any(|s| stmt_contains_goto(s, &label));
        if prefix_has_goto {
            i += 1;
            continue;
        }

        let loop_stmts: Vec<Statement> = if bare_goto {
            let body: Vec<Statement> = block.0[i + 1..j].iter().map(deep_clone_statement).collect();
            vec![Statement::While(While::new(
                Literal::Boolean(true).into(),
                body.into(),
            ))]
        } else {
            let prefix: Vec<Statement> =
                block.0[i + 1..j].iter().map(deep_clone_statement).collect();
            match if_back_edge_to_loop(&block.0[j], &label, &prefix) {
                Some(stmts) => stmts,
                None => {
                    i += 1;
                    continue;
                }
            }
        };
        let n = loop_stmts.len();
        block.0.splice(i + 1..=j, loop_stmts);
        changed = true;
        i += 1 + n;
    }
    changed
}

fn trailing_if_back_edge(stmt: &Statement, label: &str) -> bool {
    let Statement::If(r#if) = stmt else {
        return false;
    };
    let else_empty = r#if
        .else_block
        .try_lock()
        .map(|b| b.0.is_empty())
        .unwrap_or(false);
    if !else_empty {
        return false;
    }
    r#if.then_block
        .try_lock()
        .map(|b| ends_with_goto_named(&b.0, label))
        .unwrap_or(false)
}

fn if_back_edge_to_loop(
    stmt: &Statement,
    label: &str,
    prefix: &[Statement],
) -> Option<Vec<Statement>> {
    let Statement::If(r#if) = stmt else {
        return None;
    };
    let else_empty = r#if
        .else_block
        .try_lock()
        .map(|b| b.0.is_empty())
        .unwrap_or(false);
    if !else_empty {
        return None;
    }
    let then_b = r#if.then_block.try_lock()?;
    if !ends_with_goto_named(&then_b.0, label) {
        return None;
    }
    let then_prefix = &then_b.0[..then_b.0.len() - 1];
    if then_prefix.iter().any(|s| stmt_contains_goto(s, label)) {
        return None;
    }
    let cond = r#if.condition.clone();
    let then_body: Vec<Statement> = then_prefix.iter().map(deep_clone_statement).collect();
    drop(then_b);

    if prefix.is_empty() {
        Some(vec![Statement::While(While::new(cond, then_body.into()))])
    } else if then_body.is_empty() {
        Some(vec![Statement::Repeat(Repeat::new(
            make_not(cond),
            prefix.to_vec().into(),
        ))])
    } else {
        // ::L:: A; if c then B; goto L end  →  A; while c do B; A end
        let mut while_body = then_body;
        while_body.extend(prefix.iter().map(deep_clone_statement));
        let mut out = prefix.to_vec();
        out.push(Statement::While(While::new(cond, while_body.into())));
        Some(out)
    }
}

fn remove_orphan_labels(block: &mut Block) {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    count_remaining_gotos(block, &mut counts);
    block.0.retain(|s| match s {
        Statement::Label(l) => counts.get(&l.0).copied().unwrap_or(0) > 0,
        _ => true,
    });
    for statement in block.0.iter_mut() {
        for_each_nested_mut(statement, &mut |b| remove_orphan_labels(b));
    }
}

// ----- short-tail inlining (existing) -----

fn collect_short_tails(block: &Block) -> FxHashMap<String, Vec<Statement>> {
    let mut out = FxHashMap::default();
    walk_for_tails(block, &mut out);
    out
}

fn walk_for_tails(block: &Block, out: &mut FxHashMap<String, Vec<Statement>>) {
    for (idx, statement) in block.0.iter().enumerate() {
        if let Statement::Label(label) = statement {
            if let Some(tail) = extract_short_tail(&block.0, idx + 1) {
                // A tail that jumps back to its own label is a loop, not a
                // terminating copy. Inlining it splices the `If` into its
                // own `then_block` (shallow `Statement::clone()`) and the
                // next walker deadlocks on parking_lot.
                if !stmts_contain_goto(&tail, &label.0) {
                    out.entry(label.0.clone()).or_insert(tail);
                }
            }
        }
        for_each_nested(statement, &mut |b| walk_for_tails(b, out));
    }
}

fn stmts_contain_goto(stmts: &[Statement], name: &str) -> bool {
    stmts.iter().any(|s| stmt_contains_goto(s, name))
}

fn nested_has_goto(block: &crate::SharedBlock, name: &str) -> bool {
    match block.try_lock() {
        Some(b) => stmts_contain_goto(&b.0, name),
        // Already locked: assume a back-edge so we refuse the tail.
        None => true,
    }
}

fn stmt_contains_goto(stmt: &Statement, name: &str) -> bool {
    if goto_name(stmt) == Some(name) {
        return true;
    }
    match stmt {
        Statement::If(r#if) => {
            nested_has_goto(&r#if.then_block, name) || nested_has_goto(&r#if.else_block, name)
        }
        Statement::While(w) => nested_has_goto(&w.block, name),
        Statement::Repeat(r) => nested_has_goto(&r.block, name),
        Statement::NumericFor(n) => nested_has_goto(&n.block, name),
        Statement::GenericFor(g) => nested_has_goto(&g.block, name),
        _ => false,
    }
}

const MAX_TAIL_LEN: usize = 64;

/// Nested statement count. Shallow `If` clones of a 50k-line else were
/// "one statement" under `MAX_TAIL_LEN` and, when spliced, aliased that
/// else into a cycle (v12 ClientRenderer hang).
fn stmt_weight(stmt: &Statement, seen: &mut FxHashSet<*const ()>) -> usize {
    match stmt {
        Statement::If(r#if) => 1
            .saturating_add(shared_weight(&r#if.then_block, seen))
            .saturating_add(shared_weight(&r#if.else_block, seen)),
        Statement::While(w) => 1usize.saturating_add(shared_weight(&w.block, seen)),
        Statement::Repeat(r) => 1usize.saturating_add(shared_weight(&r.block, seen)),
        Statement::NumericFor(n) => 1usize.saturating_add(shared_weight(&n.block, seen)),
        Statement::GenericFor(g) => 1usize.saturating_add(shared_weight(&g.block, seen)),
        _ => 1,
    }
}

fn shared_weight(block: &crate::SharedBlock, seen: &mut FxHashSet<*const ()>) -> usize {
    let ptr = Arc::as_ptr(block) as *const ();
    if !seen.insert(ptr) {
        return 0;
    }
    let Some(g) = block.try_lock() else {
        return MAX_TAIL_LEN + 1;
    };
    let mut n = 0usize;
    for s in &g.0 {
        n = n.saturating_add(stmt_weight(s, seen));
        if n > MAX_TAIL_LEN {
            return MAX_TAIL_LEN + 1;
        }
    }
    n
}

fn extract_short_tail(stmts: &[Statement], from: usize) -> Option<Vec<Statement>> {
    if from >= stmts.len() {
        return None;
    }
    let mut tail = Vec::with_capacity(2);
    let mut i = from;
    let mut weight = 0usize;
    let mut seen = FxHashSet::default();
    while i < stmts.len() && tail.len() < MAX_TAIL_LEN {
        let stmt = &stmts[i];
        let w = stmt_weight(stmt, &mut seen);
        if w > MAX_TAIL_LEN || weight.saturating_add(w) > MAX_TAIL_LEN {
            return None;
        }
        weight += w;
        match stmt {
            Statement::Return(_)
            | Statement::Break(_)
            | Statement::Continue(_)
            | Statement::Goto(_) => {
                tail.push(stmt.clone());
                return Some(tail);
            }
            Statement::Label(_) => return None,
            _ => {
                tail.push(stmt.clone());
            }
        }
        i += 1;
    }
    if !tail.is_empty() && i == stmts.len() {
        Some(tail)
    } else {
        None
    }
}

fn replace_gotos(block: &mut Block, tails: &FxHashMap<String, Vec<Statement>>, changed: &mut bool) {
    if crate::past_post_deadline() {
        return;
    }
    let mut i = 0;
    while i < block.0.len() {
        let replacement = match &block.0[i] {
            // Deep clone: shallow `Statement::clone()` of an `If` shares
            // then/else Arcs with the labeled tail. Splicing that into
            // another if's else built the v12 else→else cycle.
            Statement::Goto(goto) => tails.get(&goto.0 .0).map(|v| {
                v.iter().map(deep_clone_statement).collect::<Vec<_>>()
            }),
            _ => None,
        };
        if let Some(repl) = replacement {
            let n = repl.len();
            block.0.splice(i..=i, repl);
            i += n;
            *changed = true;
            continue;
        }
        for_each_nested_mut(&mut block.0[i], &mut |b| {
            replace_gotos(b, tails, changed);
        });
        i += 1;
    }
}

fn prune_unused_labels(block: &mut Block, tails: &FxHashMap<String, Vec<Statement>>) {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    count_remaining_gotos(block, &mut counts);

    block.0.retain(|s| match s {
        Statement::Label(l) => {
            if !tails.contains_key(&l.0) {
                return true;
            }
            counts.get(&l.0).copied().unwrap_or(0) > 0
        }
        _ => true,
    });
    for statement in block.0.iter_mut() {
        for_each_nested_mut(statement, &mut |b| prune_unused_labels(b, tails));
    }
}

fn count_remaining_gotos(block: &Block, counts: &mut FxHashMap<String, usize>) {
    for statement in &block.0 {
        match statement {
            Statement::Goto(goto) => {
                *counts.entry(goto.0 .0.clone()).or_insert(0) += 1;
            }
            other => for_each_nested(other, &mut |b| count_remaining_gotos(b, counts)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Goto, Label, Literal};
    use rustc_hash::FxHashSet;
    use triomphe::Arc;

    fn lit_true() -> RValue {
        Literal::Boolean(true).into()
    }

    #[test]
    fn fallthrough_join_removes_gotos() {
        // if true then goto l1 end; goto l1; ::l1::; return
        let mut body = Block(vec![
            Statement::If(If::new(
                lit_true(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::Goto(Goto::new(Label("l1".into()))),
            Statement::Label(Label("l1".into())),
            Statement::Return(crate::Return {
                values: vec![],
            }),
        ]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l1::"), "label remains: {s}");
    }

    #[test]
    fn else_join_simple_goto_inverts_condition() {
        // if C then goto l1 else ::l1:: x = 1 end
        let mut body = Block(vec![Statement::If(If::new(
            Literal::Boolean(false).into(),
            Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
            Block(vec![
                Statement::Label(Label("l1".into())),
                Statement::Return(crate::Return::new(vec![])),
            ]),
        ))]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l1::"), "label remains: {s}");
        assert!(
            s.contains("not") || s.contains("else"),
            "expected inverted condition or else-form: {s}",
        );
    }

    #[test]
    fn else_join_nested_skip_if() {
        // if C then if D then goto l1 end else ::l1:: return end
        let inner = If::new(
            Literal::Boolean(true).into(),
            Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
            Block::default(),
        );
        let mut body = Block(vec![Statement::If(If::new(
            Literal::Boolean(false).into(),
            Block(vec![Statement::If(inner)]),
            Block(vec![
                Statement::Label(Label("l1".into())),
                Statement::Return(crate::Return::new(vec![])),
            ]),
        ))]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l1::"), "label remains: {s}");
    }


    #[test]
    fn else_join_with_prefix_then_body() {
        // if C then local work; if D then goto L end; goto L else ::L:: return end
        // → if C then local work end; return  (approx; skip-if stripped)
        use crate::Assign;
        let work = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(1.0).into()],
        ));
        let skip = If::new(
            Literal::Boolean(true).into(),
            Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
            Block::default(),
        );
        let mut body = Block(vec![Statement::If(If::new(
            Literal::Boolean(false).into(),
            Block(vec![
                work,
                Statement::If(skip),
                Statement::Goto(Goto::new(Label("l1".into()))),
            ]),
            Block(vec![
                Statement::Label(Label("l1".into())),
                Statement::Return(crate::Return::new(vec![])),
            ]),
        ))]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l1::"), "label remains: {s}");
    }



    #[test]
    fn fallthrough_join_without_second_goto() {
        // if true then x; goto l1 end
        // y
        // ::l1::
        // return
        // → if true then x end; y; return
        use crate::Assign;
        let work = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(1.0).into()],
        ));
        let mid = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(2.0).into()],
        ));
        let mut body = Block(vec![
            Statement::If(If::new(
                lit_true(),
                Block(vec![
                    work,
                    Statement::Goto(Goto::new(Label("l1".into()))),
                ]),
                Block::default(),
            )),
            mid,
            Statement::Label(Label("l1".into())),
            Statement::Return(crate::Return::new(vec![])),
        ]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l1::"), "label remains: {s}");
    }



    #[test]
    fn skip_rest_goto_becomes_else() {
        // if C then
        //   A
        //   if D then B; goto L end
        //   E
        // end
        // (no ::L::)  →  if D then B else E end under C
        use crate::Assign;
        let a = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(1.0).into()],
        ));
        let b = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(2.0).into()],
        ));
        let e = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(3.0).into()],
        ));
        let inner = If::new(
            Literal::Boolean(true).into(),
            Block(vec![b, Statement::Goto(Goto::new(Label("l81".into())))]),
            Block::default(),
        );
        let mut body = Block(vec![Statement::If(If::new(
            lit_true(),
            Block(vec![a, Statement::If(inner), e]),
            Block::default(),
        ))]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l81::"), "label remains: {s}");
        assert!(s.contains("else"), "expected else arm for skipped rest: {s}");
    }



    #[test]
    fn nested_skip_forward_before_join_not_double_run() {
        // if C then
        //   if U then
        //     if D then B; goto L end
        //   end
        //   E
        //   goto L
        // end
        // ::L::
        // rest
        //
        // When U and D: only B must run (not E). Deleting goto without
        // restructuring would run B then E (silent speed double-apply bug).
        use crate::Assign;
        let b = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(1.0).into()],
        ));
        let e = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(crate::RcLocal::default())],
            vec![Literal::Number(2.0).into()],
        ));
        let inner = If::new(
            Literal::Boolean(true).into(), // D
            Block(vec![b, Statement::Goto(Goto::new(Label("l81".into())))]),
            Block::default(),
        );
        let ulted = If::new(
            Literal::Boolean(true).into(), // U
            Block(vec![Statement::If(inner)]),
            Block::default(),
        );
        let mut body = Block(vec![
            Statement::If(If::new(
                lit_true(), // C
                Block(vec![
                    Statement::If(ulted),
                    e,
                    Statement::Goto(Goto::new(Label("l81".into()))),
                ]),
                Block::default(),
            )),
            Statement::Label(Label("l81".into())),
            Statement::Return(crate::Return::new(vec![])),
        ]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(!s.contains("::l81::"), "label remains: {s}");
        // Must have else so E is not unconditional after B
        assert!(s.contains("else"), "expected else for skipped E: {s}");
    }

    #[test]
    fn back_edge_if_becomes_while() {
        // ::L:: if c then goto L end; return  →  while c do end; return
        let mut body = Block(vec![
            Statement::Label(Label("l1".into())),
            Statement::If(If::new(
                lit_true(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::Return(crate::Return::new(vec![])),
        ]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
        assert!(s.contains("while"), "expected while from back-edge: {s}");
    }

    #[test]
    fn inlined_goto_tails_do_not_share_else_arcs() {
        // if C then goto L end
        // if D then goto L end
        // ::L::
        // if E then return else x = 1 end
        //
        // Both gotos inline the same if-tail. Shallow clone would make
        // the three `else` Arcs identical; mutating one mutates all.
        use crate::{Assign, RcLocal};
        let work = Statement::Assign(Assign::new(
            vec![crate::LValue::Local(RcLocal::default())],
            vec![Literal::Number(1.0).into()],
        ));
        let tail_if = If::new(
            lit_true(),
            Block(vec![Statement::Return(crate::Return::new(vec![]))]),
            Block(vec![work]),
        );
        let mut body = Block(vec![
            Statement::If(If::new(
                lit_true(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::If(If::new(
                Literal::Boolean(false).into(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::Label(Label("l1".into())),
            Statement::If(tail_if),
        ]);
        inline_short_gotos(&mut body);
        let mut else_ptrs = Vec::new();
        let mut seen = FxHashSet::default();
        fn collect(
            block: &Block,
            out: &mut Vec<*const ()>,
            seen: &mut FxHashSet<*const ()>,
        ) {
            for s in &block.0 {
                if let Statement::If(i) = s {
                    let p = Arc::as_ptr(&i.else_block) as *const ();
                    out.push(p);
                    let then_p = Arc::as_ptr(&i.then_block) as *const ();
                    if seen.insert(then_p) {
                        if let Some(b) = i.then_block.try_lock() {
                            collect(&b, out, seen);
                        }
                    }
                    if seen.insert(p) {
                        if let Some(b) = i.else_block.try_lock() {
                            collect(&b, out, seen);
                        }
                    }
                }
            }
        }
        collect(&body, &mut else_ptrs, &mut seen);
        let unique = else_ptrs.iter().copied().collect::<FxHashSet<_>>();
        assert_eq!(
            unique.len(),
            else_ptrs.len(),
            "inlined if-tails still share else Arcs: {} ptrs {} unique",
            else_ptrs.len(),
            unique.len()
        );
    }

    #[test]
    fn inlining_back_edge_tail_does_not_deadlock() {
        // goto L before ::L:: if c then goto L end; return
        // Without skipping self-tails, replace_gotos splices the If into
        // its own then_block (shallow clone) and parking_lot hangs.
        let mut body = Block(vec![
            Statement::If(If::new(
                lit_true(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::Label(Label("l1".into())),
            Statement::If(If::new(
                lit_true(),
                Block(vec![Statement::Goto(Goto::new(Label("l1".into())))]),
                Block::default(),
            )),
            Statement::Return(crate::Return::new(vec![])),
        ]);
        inline_short_gotos(&mut body);
        let s = body.to_string();
        assert!(!s.contains("goto"), "gotos remain: {s}");
    }

}
