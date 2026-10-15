//! Moves top-level `local` declarations down to their first use.
//!
//! A local is active from its declaration to the end of its block. Topaz puts
//! every top-level declaration at the start of the function, so a value that is
//! used once, early, stays active for the whole body. Moving the declaration to
//! just before its first mention shortens that lifetime, and it is the only way
//! to lower the active-local peak without moving code.
//!
//! A declaration `local x = e` moves from `d` to `f`, the first top-level
//! statement after `d` that mentions `x`, only when this does not change what
//! the program does:
//!
//! * `e` only reads locals, literals, closures and table constructors (or,
//!   with a flag, arithmetic, comparisons and indexing, which can run a
//!   metamethod). Calls and globals are never moved.
//! * No statement between `d` and `f` writes a local that `e` reads.
//! * If `e` reads a local a closure captures, no statement between calls
//!   anything, because the callee could write that local.
//! * If `e` may run a metamethod, no statement between has side effects, and
//!   none mentions a captured local.
//!
//! Local names are unique within a function, so a moved declaration cannot
//! change which variable a name refers to.

use std::collections::HashSet;

use itertools::Either;

use crate::{Block, LValue, LocalRw, RValue, RcLocal, Statement, Traverse};

/// Counts for one call to [`sink_declarations`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SinkStats {
    /// Top-level single-local declarations considered.
    pub candidates: usize,
    /// Declarations moved to their first use.
    pub moved: usize,
    /// Declarations whose value is a call, global or vararg, so never moved.
    pub not_movable: usize,
    /// Declarations the block never mentions again.
    pub unused: usize,
    /// First use is a plain assignment to the local.
    pub first_use_writes: usize,
    /// Blocked because a statement in between writes an input of the value.
    pub blocked_writes: usize,
    /// Blocked because a call in between could change a captured input.
    pub blocked_calls: usize,
    /// Blocked because the value may run a metamethod and the statements in
    /// between have side effects or mention captured locals.
    pub blocked_metamethod: usize,
}

/// What a statement reads, writes, and may do, including everything nested in
/// its blocks and closure bodies.
#[derive(Default)]
struct Effects {
    /// Every local the statement reads or writes.
    mentions: HashSet<RcLocal>,
    /// Every local the statement writes.
    writes: HashSet<RcLocal>,
    /// Every local mentioned inside a closure body.
    captured: HashSet<RcLocal>,
    /// The statement may call a function.
    calls: bool,
    /// The statement may change a table, a global, or run a call.
    side_effects: bool,
}

impl Effects {
    fn absorb(&mut self, other: Effects) {
        self.mentions.extend(other.mentions);
        self.writes.extend(other.writes);
        self.captured.extend(other.captured);
        self.calls |= other.calls;
        self.side_effects |= other.side_effects;
    }

    /// A construct that could not be inspected is assumed to do everything.
    fn unknown(&mut self) {
        self.calls = true;
        self.side_effects = true;
    }
}

/// Moves the movable top-level declarations of `block` down to their first
/// use. Returns the counts. Nested blocks are not changed; each function body
/// is handled by its own call.
pub fn sink_declarations(block: &mut Block) -> SinkStats {
    let mut stats = SinkStats::default();
    let n = block.0.len();

    let summaries: Vec<Effects> = block
        .0
        .iter_mut()
        .map(|statement| {
            let mut fx = Effects::default();
            summarize(statement, &mut fx);
            fx
        })
        .collect();
    let captured: HashSet<RcLocal> = summaries
        .iter()
        .flat_map(|s| s.captured.iter().cloned())
        .collect();

    let mut moves: Vec<(usize, usize)> = Vec::new();
    for d in 0..n {
        let Statement::Assign(assign) = &block.0[d] else {
            continue;
        };
        if !assign.prefix
            || assign.compound_op.is_some()
            || assign.left.len() != 1
            || assign.right.len() > 1
        {
            continue;
        }
        let LValue::Local(local) = &assign.left[0] else {
            continue;
        };
        stats.candidates += 1;

        let (reads, may_call): (Vec<RcLocal>, bool) = match assign.right.first() {
            None => (Vec::new(), false),
            Some(value) => match movable(value) {
                Some(may_call) => (
                    value.values_read().into_iter().cloned().collect(),
                    may_call,
                ),
                None => {
                    stats.not_movable += 1;
                    continue;
                }
            },
        };
        if reads.contains(local) {
            stats.not_movable += 1;
            continue;
        }

        let Some(f) = (d + 1..n).find(|&i| summaries[i].mentions.contains(local)) else {
            stats.unused += 1;
            continue;
        };
        let assigns_local = matches!(
            &block.0[f],
            Statement::Assign(a) if a.left.iter().any(|lv| matches!(lv, LValue::Local(x) if x == local))
        );
        if assigns_local {
            stats.first_use_writes += 1;
            continue;
        }

        let reads_captured = reads.iter().any(|r| captured.contains(r));
        let mut blocked: Option<&mut usize> = None;
        for i in d + 1..f {
            let s = &summaries[i];
            if reads.iter().any(|r| s.writes.contains(r)) {
                blocked = Some(&mut stats.blocked_writes);
                break;
            }
            if reads_captured && s.calls {
                blocked = Some(&mut stats.blocked_calls);
                break;
            }
            if may_call
                && (s.calls
                    || s.side_effects
                    || s.mentions.iter().any(|m| captured.contains(m)))
            {
                blocked = Some(&mut stats.blocked_metamethod);
                break;
            }
        }
        match blocked {
            Some(counter) => *counter += 1,
            None => {
                moves.push((d, f));
                stats.moved += 1;
            }
        }
    }

    if moves.is_empty() {
        return stats;
    }

    // Rebuild the block. A moved declaration is emitted right before its first
    // use, in its original order when several land on the same statement.
    let mut moved = vec![false; n];
    let mut before: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(d, f) in &moves {
        moved[d] = true;
        before[f].push(d);
    }
    let mut slots: Vec<Option<Statement>> = std::mem::take(&mut block.0)
        .into_iter()
        .map(Some)
        .collect();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        for &d in &before[i] {
            out.push(slots[d].take().expect("a moved declaration is emitted once"));
        }
        if !moved[i] {
            out.push(slots[i].take().expect("a kept statement is emitted once"));
        }
    }
    block.0 = out;
    stats
}

/// Whether a declared value can move. `Some(true)` means it may run a
/// metamethod. `None` means it must stay where it is.
fn movable(value: &RValue) -> Option<bool> {
    match value {
        RValue::Literal(_) | RValue::Local(_) | RValue::Closure(_) => Some(false),
        RValue::Table(table) => {
            let mut may_call = false;
            for (key, item) in &table.0 {
                for part in key.iter().chain(std::iter::once(item)) {
                    may_call |= movable(part)?;
                }
            }
            Some(may_call)
        }
        RValue::Unary(unary) => {
            movable(&unary.value)?;
            Some(true)
        }
        RValue::Binary(binary) => {
            movable(&binary.left)?;
            movable(&binary.right)?;
            Some(true)
        }
        RValue::Index(index) => {
            movable(&index.left)?;
            movable(&index.right)?;
            Some(true)
        }
        _ => None,
    }
}

fn summarize_block(block: &mut Block, fx: &mut Effects) {
    for statement in &mut block.0 {
        summarize(statement, fx);
    }
}

fn summarize(statement: &mut Statement, fx: &mut Effects) {
    for local in statement.values_read_mut() {
        fx.mentions.insert(local.clone());
    }
    for local in statement.values_written_mut() {
        fx.mentions.insert(local.clone());
        fx.writes.insert(local.clone());
    }

    match statement {
        Statement::Call(_) | Statement::MethodCall(_) => {
            fx.calls = true;
            fx.side_effects = true;
        }
        Statement::Assign(assign) => {
            if assign.left.iter().any(|lv| !matches!(lv, LValue::Local(_))) {
                fx.side_effects = true;
            }
        }
        Statement::If(_)
        | Statement::Empty(_)
        | Statement::Comment(_)
        | Statement::Return(_)
        | Statement::Break(_)
        | Statement::Continue(_) => {}
        // Loops, loop set-up, `close` and `set list` are not inspected here.
        _ => fx.unknown(),
    }

    statement.post_traverse_values(&mut |value| -> Option<()> {
        if let Either::Right(value) = value {
            match value {
                RValue::Call(_) | RValue::MethodCall(_) | RValue::Select(_) => {
                    fx.calls = true;
                    fx.side_effects = true;
                }
                RValue::Closure(closure) => match closure.function.try_lock() {
                    Some(mut function) => {
                        let mut inner = Effects::default();
                        summarize_block(&mut function.body, &mut inner);
                        fx.captured.extend(inner.mentions.iter().cloned());
                        fx.absorb(inner);
                    }
                    None => fx.unknown(),
                },
                _ => {}
            }
        }
        None
    });

    match statement {
        Statement::If(r#if) => {
            match r#if.then_block.try_lock() {
                Some(mut b) => summarize_block(&mut b, fx),
                None => fx.unknown(),
            }
            match r#if.else_block.try_lock() {
                Some(mut b) => summarize_block(&mut b, fx),
                None => fx.unknown(),
            }
        }
        Statement::While(r#while) => match r#while.block.try_lock() {
            Some(mut b) => summarize_block(&mut b, fx),
            None => fx.unknown(),
        },
        Statement::Repeat(repeat) => match repeat.block.try_lock() {
            Some(mut b) => summarize_block(&mut b, fx),
            None => fx.unknown(),
        },
        Statement::NumericFor(numeric_for) => match numeric_for.block.try_lock() {
            Some(mut b) => summarize_block(&mut b, fx),
            None => fx.unknown(),
        },
        Statement::GenericFor(generic_for) => match generic_for.block.try_lock() {
            Some(mut b) => summarize_block(&mut b, fx),
            None => fx.unknown(),
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Call, Global, Local};

    fn local(name: &str) -> RcLocal {
        RcLocal::new(Local::new(Some(name.into())))
    }

    fn decl(l: &RcLocal, value: RValue) -> Statement {
        let mut a = Assign::new(vec![l.clone().into()], vec![value]);
        a.prefix = true;
        a.into()
    }

    fn set(l: &RcLocal, value: RValue) -> Statement {
        Assign::new(vec![l.clone().into()], vec![value]).into()
    }

    fn global(name: &str) -> RValue {
        Global::new(name.as_bytes().to_vec()).into()
    }

    fn call(name: &str, args: Vec<RValue>) -> Statement {
        Statement::Call(Call::new(Global::new(name.as_bytes().to_vec()).into(), args))
    }

    /// Index of the statement that declares `l`, if any.
    fn decl_index(block: &Block, l: &RcLocal) -> Option<usize> {
        block.0.iter().position(|s| match s {
            Statement::Assign(a) if a.prefix => a
                .left
                .iter()
                .any(|lv| matches!(lv, LValue::Local(x) if x == l)),
            _ => false,
        })
    }

    #[test]
    fn moves_a_pure_declaration_past_an_unrelated_call() {
        // a = 1; g(); m(a)  ->  g(); a = 1; m(a)
        let a = local("a");
        let mut block = Block(vec![
            decl(&a, RValue::Literal(crate::Literal::Integer(1))),
            call("g", vec![]),
            call("m", vec![RValue::Local(a.clone())]),
        ]);
        let stats = sink_declarations(&mut block);
        assert_eq!(stats.moved, 1);
        assert_eq!(decl_index(&block, &a), Some(1));
    }

    #[test]
    fn keeps_a_declaration_whose_input_is_written_before_its_use() {
        // a = 1; b = a; a = 2; m(b)  must keep b = a before a = 2
        let a = local("a");
        let b = local("b");
        let mut block = Block(vec![
            decl(&a, RValue::Literal(crate::Literal::Integer(1))),
            decl(&b, RValue::Local(a.clone())),
            set(&a, RValue::Literal(crate::Literal::Integer(2))),
            call("m", vec![RValue::Local(b.clone())]),
        ]);
        let stats = sink_declarations(&mut block);
        assert_eq!(stats.blocked_writes, 1);
        assert_eq!(decl_index(&block, &b), Some(1));
    }

    #[test]
    fn never_moves_a_call_or_global() {
        // c = f(); g(); m(c)  stays as it is: the call must run first
        let c = local("c");
        let mut block = Block(vec![
            decl(&c, global("f")),
            call("g", vec![]),
            call("m", vec![RValue::Local(c.clone())]),
        ]);
        let stats = sink_declarations(&mut block);
        assert_eq!(stats.moved, 0);
        assert_eq!(stats.not_movable, 1);
        assert_eq!(decl_index(&block, &c), Some(0));
    }
}
