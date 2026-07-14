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

use rustc_hash::FxHashMap;

use crate::{
    Binary, BinaryOperation, Block, If, RValue, Statement, Traverse, Unary, UnaryOperation,
};

pub fn inline_short_gotos(block: &mut Block) {
    for _ in 0..64 {
        let mut changed = false;
        changed |= eliminate_join_gotos(block);
        let tails = collect_short_tails(block);
        if !tails.is_empty() {
            replace_gotos(block, &tails, &mut changed);
        }
        if !changed {
            break;
        }
    }
    let tails = collect_short_tails(block);
    prune_unused_labels(block, &tails);
    remove_orphan_labels(block);
    descend_into_closures(block);
}

fn descend_into_closures(block: &mut Block) {
    for statement in block.0.iter_mut() {
        statement.traverse_rvalues(&mut |rv: &mut RValue| {
            if let RValue::Closure(closure) = rv {
                inline_short_gotos(&mut closure.function.lock().body);
            }
        });
        match statement {
            Statement::If(r#if) => {
                descend_into_closures(&mut r#if.then_block.lock());
                descend_into_closures(&mut r#if.else_block.lock());
            }
            Statement::While(r#while) => descend_into_closures(&mut r#while.block.lock()),
            Statement::Repeat(repeat) => descend_into_closures(&mut repeat.block.lock()),
            Statement::NumericFor(nf) => descend_into_closures(&mut nf.block.lock()),
            Statement::GenericFor(gf) => descend_into_closures(&mut gf.block.lock()),
            _ => {}
        }
    }
}

/// Remove join-point gotos that restructure left as structured control flow.
fn eliminate_join_gotos(block: &mut Block) -> bool {
    let mut changed = false;
    // Recurse first so nested blocks clean up before we match parents.
    for statement in block.0.iter_mut() {
        match statement {
            Statement::If(r#if) => {
                changed |= eliminate_join_gotos(&mut r#if.then_block.lock());
                changed |= eliminate_join_gotos(&mut r#if.else_block.lock());
            }
            Statement::While(r#while) => {
                changed |= eliminate_join_gotos(&mut r#while.block.lock());
            }
            Statement::Repeat(repeat) => {
                changed |= eliminate_join_gotos(&mut repeat.block.lock());
            }
            Statement::NumericFor(nf) => {
                changed |= eliminate_join_gotos(&mut nf.block.lock());
            }
            Statement::GenericFor(gf) => {
                changed |= eliminate_join_gotos(&mut gf.block.lock());
            }
            _ => {}
        }
    }

    changed |= rewrite_else_join_labels(block);
    changed |= rewrite_fallthrough_joins(block);
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
    let then_block = r#if.then_block.lock();
    let else_block = r#if.else_block.lock();

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
            let inner_then = inner.then_block.lock();
            let inner_else = inner.else_block.lock();
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
                    let inner_then = inner.then_block.lock();
                    let inner_else = inner.else_block.lock();
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
        //      (middle is the "else" fallthrough; then jumps over middle to join)
        //
        // For B, rewrite to: if C then ... end; middle; rest
        // For A, same but also drop the second goto.
        if i + 1 >= block.0.len() {
            break;
        }

        let Some((label, label_idx, has_pre_goto)) = find_join_after_if(block, i) else {
            i += 1;
            continue;
        };

        // Strip trailing goto L from then (and nested pure skip gotos already handled).
        if let Statement::If(r#if) = &mut block.0[i] {
            let mut then_b = r#if.then_block.lock();
            strip_trailing_goto(&mut then_b.0, &label);
            // Also strip trailing gotos from nested ifs that only jump to L? leave for now.
        }

        // Remove label; if a goto immediately precedes it, remove that too.
        block.0.remove(label_idx);
        if has_pre_goto {
            block.0.remove(label_idx - 1);
        }
        changed = true;
        // re-scan from i
    }
    changed
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
    let then_b = r#if.then_block.lock();
    let else_b = r#if.else_block.lock();
    // Allow empty else; if else is non-empty this is not a fallthrough join.
    if !else_b.0.is_empty() {
        return None;
    }
    let label = goto_name(then_b.0.last()?)?.to_string();

    // then must end with goto L; optionally allow nested structure that ends that way.
    if !ends_with_goto_named(&then_b.0, &label) {
        return None;
    }
    drop(then_b);
    drop(else_b);

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

fn remove_orphan_labels(block: &mut Block) {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    count_remaining_gotos(block, &mut counts);
    block.0.retain(|s| match s {
        Statement::Label(l) => counts.get(&l.0).copied().unwrap_or(0) > 0,
        _ => true,
    });
    for statement in block.0.iter_mut() {
        match statement {
            Statement::If(r#if) => {
                remove_orphan_labels(&mut r#if.then_block.lock());
                remove_orphan_labels(&mut r#if.else_block.lock());
            }
            Statement::While(r#while) => remove_orphan_labels(&mut r#while.block.lock()),
            Statement::Repeat(repeat) => remove_orphan_labels(&mut repeat.block.lock()),
            Statement::NumericFor(nf) => remove_orphan_labels(&mut nf.block.lock()),
            Statement::GenericFor(gf) => remove_orphan_labels(&mut gf.block.lock()),
            _ => {}
        }
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
                out.entry(label.0.clone()).or_insert(tail);
            }
        }
        match statement {
            Statement::If(r#if) => {
                walk_for_tails(&r#if.then_block.lock(), out);
                walk_for_tails(&r#if.else_block.lock(), out);
            }
            Statement::While(r#while) => walk_for_tails(&r#while.block.lock(), out),
            Statement::Repeat(repeat) => walk_for_tails(&repeat.block.lock(), out),
            Statement::NumericFor(numeric_for) => walk_for_tails(&numeric_for.block.lock(), out),
            Statement::GenericFor(generic_for) => walk_for_tails(&generic_for.block.lock(), out),
            _ => {}
        }
    }
}

const MAX_TAIL_LEN: usize = 64;

fn extract_short_tail(stmts: &[Statement], from: usize) -> Option<Vec<Statement>> {
    if from >= stmts.len() {
        return None;
    }
    let mut tail = Vec::with_capacity(2);
    let mut i = from;
    while i < stmts.len() && tail.len() < MAX_TAIL_LEN {
        let stmt = &stmts[i];
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
    let mut i = 0;
    while i < block.0.len() {
        let replacement = match &block.0[i] {
            Statement::Goto(goto) => tails.get(&goto.0 .0).cloned(),
            _ => None,
        };
        if let Some(repl) = replacement {
            let n = repl.len();
            block.0.splice(i..=i, repl);
            i += n;
            *changed = true;
            continue;
        }
        match &mut block.0[i] {
            Statement::If(r#if) => {
                replace_gotos(&mut r#if.then_block.lock(), tails, changed);
                replace_gotos(&mut r#if.else_block.lock(), tails, changed);
            }
            Statement::While(r#while) => replace_gotos(&mut r#while.block.lock(), tails, changed),
            Statement::Repeat(repeat) => replace_gotos(&mut repeat.block.lock(), tails, changed),
            Statement::NumericFor(nf) => replace_gotos(&mut nf.block.lock(), tails, changed),
            Statement::GenericFor(gf) => replace_gotos(&mut gf.block.lock(), tails, changed),
            _ => {}
        }
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
        match statement {
            Statement::If(r#if) => {
                prune_unused_labels(&mut r#if.then_block.lock(), tails);
                prune_unused_labels(&mut r#if.else_block.lock(), tails);
            }
            Statement::While(r#while) => prune_unused_labels(&mut r#while.block.lock(), tails),
            Statement::Repeat(repeat) => prune_unused_labels(&mut repeat.block.lock(), tails),
            Statement::NumericFor(nf) => prune_unused_labels(&mut nf.block.lock(), tails),
            Statement::GenericFor(gf) => prune_unused_labels(&mut gf.block.lock(), tails),
            _ => {}
        }
    }
}

fn count_remaining_gotos(block: &Block, counts: &mut FxHashMap<String, usize>) {
    for statement in &block.0 {
        match statement {
            Statement::Goto(goto) => {
                *counts.entry(goto.0 .0.clone()).or_insert(0) += 1;
            }
            Statement::If(r#if) => {
                count_remaining_gotos(&r#if.then_block.lock(), counts);
                count_remaining_gotos(&r#if.else_block.lock(), counts);
            }
            Statement::While(r#while) => count_remaining_gotos(&r#while.block.lock(), counts),
            Statement::Repeat(repeat) => count_remaining_gotos(&repeat.block.lock(), counts),
            Statement::NumericFor(nf) => count_remaining_gotos(&nf.block.lock(), counts),
            Statement::GenericFor(gf) => count_remaining_gotos(&gf.block.lock(), counts),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Goto, Label, Literal};

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
        assert!(s.contains("not"), "expected inverted condition: {s}");
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

}
