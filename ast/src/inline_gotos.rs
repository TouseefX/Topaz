use rustc_hash::FxHashMap;

use crate::{Block, RValue, Statement, Traverse};

pub fn inline_short_gotos(block: &mut Block) {
    for _ in 0..10 {
        let tails = collect_short_tails(block);
        if tails.is_empty() {
            break;
        }
        let mut changed = false;
        replace_gotos(block, &tails, &mut changed);
        if !changed {
            break;
        }
    }
    let tails = collect_short_tails(block);
    prune_unused_labels(block, &tails);
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

const MAX_TAIL_LEN: usize = 16;

fn extract_short_tail(stmts: &[Statement], from: usize) -> Option<Vec<Statement>> {
    if from >= stmts.len() {
        return None;
    }
    let mut tail = Vec::with_capacity(2);
    let mut i = from;
    while i < stmts.len() && tail.len() < MAX_TAIL_LEN {
        let stmt = &stmts[i];
        match stmt {
            Statement::Return(_) | Statement::Break(_) | Statement::Continue(_) | Statement::Goto(_) => {
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

