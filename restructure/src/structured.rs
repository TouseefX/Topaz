//! Goto-free structured recovery (structurer v2).
//!
//! Runs on the final CFG (after SSA destruction, edge copies still on
//! `BlockEdge::arguments`) and emits `if` / `while` / `NumericFor` /
//! `GenericFor`, plus `break` / `continue` / `return`. Multi-exit loops use a
//! dispatch flag instead of `goto`.
//!
//! Strategy:
//! * Natural loops come from dominator back edges. A loop body is emitted
//!   inside `while` (or `for`) with transitions to its header as `continue`
//!   and transitions to its exits as `break`.
//! * Two-way branches merge at the immediate post-dominator computed on a
//!   *region view* where paths that end in `return` (or leave the region via
//!   break/continue) are dropped. An `if x then return end` therefore does
//!   not push the rest of the function into the `then` arm.
//! * Anything the structurer cannot prove (irreducible flow, unknown for-loop
//!   shape, emitted-statement budget exceeded, deadline) returns `None`; the
//!   caller then runs the legacy goto-based `GraphStructurer`.
//!
//! The output is validated: no `goto`/labels, no leftover for-loop IR.

use ast::{
    Assign, Binary, BinaryOperation, Block, Break, Continue, GenericFor, If, LValue, Literal,
    NumericFor, RValue, RcLocal, Reduce, Statement, Unary, UnaryOperation, While,
};
use cfg::{block::BranchType, function::Function};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::parallel_assign_from_args;

const NONE: usize = usize::MAX;
const MAX_DEPTH: usize = 1000;

/// `TOPAZ_STRUCTURED_DEBUG=1` prints why the structurer gave up on a
/// function (and what it produced when it succeeded) to stderr.
fn note(why: impl FnOnce() -> String) {
    if std::env::var_os("TOPAZ_STRUCTURED_DEBUG").is_some() {
        let tag = DEBUG_TAG.with(|t| t.borrow().clone());
        eprintln!("structured[{tag}]: {}", why());
    }
}

thread_local! {
    static DEBUG_TAG: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

/// Names the function the next structurer debug lines belong to (the
/// caller sets it, e.g. to the source line). Debug output only.
pub fn set_debug_tag(tag: String) {
    DEBUG_TAG.with(|t| *t.borrow_mut() = tag);
}

#[derive(Clone)]
struct Succ {
    target: usize,
    branch: BranchType,
    args: Vec<(RcLocal, RValue)>,
}

/// Where a statement sequence is being emitted.
#[derive(Clone)]
struct Ctx {
    /// Innermost loop whose body we are inside (`None` = function top level).
    region: Option<usize>,
    /// Reaching this node falls through (end of an `if` arm).
    stop: Option<usize>,
    /// Dispatch flag of `region` when it has several flagged exits.
    flag: Option<RcLocal>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// Emission of this path ended with `break` / `continue` / `return` /
    /// fallthrough. Nothing more may be emitted on it.
    Stop,
    /// The target node must be emitted inline here.
    Inline,
}

struct Loop {
    header: usize,
    body: FxHashSet<usize>,
    parent: Option<usize>,
    /// Every distinct target outside the body, header exit first.
    exits: Vec<usize>,
    /// The subset of `exits` that needs real code (not a `return` block).
    fexits: Vec<usize>,
    /// `NumForInit` / `GenericForInit` taken out of the loop's init block.
    for_init: Option<Statement>,
}

struct Structurer {
    entry: usize,
    reach: Vec<usize>,
    is_reach: Vec<bool>,
    emitted: Vec<bool>,
    active: Vec<bool>,
    succs: Vec<Vec<Succ>>,
    preds: Vec<Vec<usize>>,
    stmts: Vec<Vec<Statement>>,
    conds: Vec<Option<RValue>>,
    loops: Vec<Loop>,
    loop_of_header: Vec<Option<usize>>,
    owner: Vec<Option<usize>>,
    /// Index `loops.len()` is the top-level region.
    ipdom_cache: Vec<Option<FxHashMap<usize, usize>>>,
    budget: usize,
    spent: usize,
    /// How often each node was emitted; the most visited nodes are printed
    /// when the budget trips (debug only).
    visits: Vec<u32>,
}

/// Immediate dominators (or post-dominators) with Cooper–Harvey–Kennedy.
/// `succ`/`pred` are the graph in the direction being analysed. Returns
/// `(idom, post_order)`; unreachable nodes have `idom == NONE`.
fn chk_idom(succ: &[Vec<usize>], pred: &[Vec<usize>], root: usize) -> (Vec<usize>, Vec<usize>) {
    let n = succ.len();
    let mut post: Vec<usize> = Vec::with_capacity(n);
    let mut visited = vec![false; n];
    let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
    visited[root] = true;
    while !stack.is_empty() {
        let top = stack.len() - 1;
        let (node, idx) = stack[top];
        if idx < succ[node].len() {
            stack[top].1 += 1;
            let next = succ[node][idx];
            if !visited[next] {
                visited[next] = true;
                stack.push((next, 0));
            }
        } else {
            post.push(node);
            stack.pop();
        }
    }
    let mut po = vec![NONE; n];
    for (i, &node) in post.iter().enumerate() {
        po[node] = i;
    }
    let mut idom = vec![NONE; n];
    idom[root] = root;
    let mut changed = true;
    while changed {
        changed = false;
        for &b in post.iter().rev() {
            if b == root {
                continue;
            }
            let mut new_idom = NONE;
            for &p in &pred[b] {
                if po[p] == NONE || idom[p] == NONE {
                    continue;
                }
                if new_idom == NONE {
                    new_idom = p;
                } else {
                    new_idom = intersect(p, new_idom, &po, &idom);
                }
            }
            if new_idom != NONE && idom[b] != new_idom {
                idom[b] = new_idom;
                changed = true;
            }
        }
    }
    (idom, post)
}

fn intersect(mut a: usize, mut b: usize, po: &[usize], idom: &[usize]) -> usize {
    while a != b {
        while po[a] < po[b] {
            a = idom[a];
        }
        while po[b] < po[a] {
            b = idom[b];
        }
    }
    a
}

fn dominates(idom: &[usize], a: usize, mut b: usize, root: usize) -> bool {
    loop {
        if b == a {
            return true;
        }
        if b == root || idom[b] == NONE {
            return false;
        }
        b = idom[b];
    }
}

fn assign_stmt(lhs: RcLocal, rhs: RValue) -> Statement {
    Assign::new(vec![LValue::Local(lhs)], vec![rhs]).into()
}

fn number(v: usize) -> RValue {
    RValue::Literal(Literal::Number(v as f64))
}

impl Structurer {
    fn build(function: &Function) -> Option<Self> {
        let graph = function.graph();
        let entry = (*function.entry())?.index();
        // Upper bound on node indices. Computed directly: `node_bound` is a
        // `NodeIndexable` trait method, not an inherent `StableGraph` method.
        let n = graph
            .node_indices()
            .map(|i| i.index() + 1)
            .max()
            .unwrap_or(0);
        let mut present = vec![false; n];
        let mut stmts: Vec<Vec<Statement>> = vec![Vec::new(); n];
        let mut conds: Vec<Option<RValue>> = vec![None; n];
        let mut succs: Vec<Vec<Succ>> = vec![Vec::new(); n];
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut total_stmts = 0usize;
        for ni in graph.node_indices() {
            let i = ni.index();
            present[i] = true;
            let mut v = graph.node_weight(ni)?.0.clone();
            let cond = match v.last() {
                Some(Statement::If(ifs)) => Some(ifs.condition.clone()),
                _ => None,
            };
            if cond.is_some() {
                v.pop();
            }
            total_stmts += v.len();
            stmts[i] = v;
            conds[i] = cond;
        }
        for ei in graph.edge_indices() {
            let (a, b) = graph.edge_endpoints(ei)?;
            let w = graph.edge_weight(ei)?;
            succs[a.index()].push(Succ {
                target: b.index(),
                branch: w.branch_type.clone(),
                args: w.arguments.clone(),
            });
            preds[b.index()].push(a.index());
        }
        for (i, s) in succs.iter().enumerate() {
            if !present[i] {
                continue;
            }
            match s.len() {
                0 | 1 => {}
                2 => {
                    let then_n = s.iter().filter(|x| x.branch == BranchType::Then).count();
                    let else_n = s.iter().filter(|x| x.branch == BranchType::Else).count();
                    if then_n != 1 || else_n != 1 {
                        note(|| format!("node {i}: 2-way branch then={then_n} else={else_n}"));
                        return None;
                    }
                }
                n => {
                    note(|| format!("node {i}: {n} successors"));
                    return None;
                }
            }
            if s.len() == 1 && conds[i].is_some() {
                note(|| format!("node {i}: 1-way node carries a condition"));
                return None;
            }
        }

        let succ_ids: Vec<Vec<usize>> = succs
            .iter()
            .map(|s| s.iter().map(|x| x.target).collect())
            .collect();
        let (idom, post) = chk_idom(&succ_ids, &preds, entry);
        let mut is_reach = vec![false; n];
        for &v in &post {
            is_reach[v] = true;
        }
        let reach: Vec<usize> = post.iter().rev().copied().collect();

        // Natural loops: an edge v -> h where h dominates v is a back edge.
        let mut latches: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for &u in &reach {
            for s in &succs[u] {
                if dominates(&idom, s.target, u, entry) {
                    latches.entry(s.target).or_default().push(u);
                }
            }
        }
        let mut headers: Vec<usize> = latches.keys().copied().collect();
        headers.sort_unstable();
        let mut loops: Vec<Loop> = Vec::with_capacity(headers.len());
        for h in headers {
            let mut body: FxHashSet<usize> = FxHashSet::default();
            body.insert(h);
            let mut stack = latches.get(&h).cloned().unwrap_or_default();
            while let Some(x) = stack.pop() {
                if body.insert(x) {
                    for &p in &preds[x] {
                        if is_reach[p] {
                            stack.push(p);
                        }
                    }
                }
            }
            // Single-entry loops only: every edge into the body from outside
            // must enter at the header.
            for &x in &body {
                if x == h {
                    continue;
                }
                for &p in &preds[x] {
                    if is_reach[p] && !body.contains(&p) {
                        note(|| format!("loop header {h}: multi-entry body at {x}"));
                        return None;
                    }
                }
            }
            loops.push(Loop {
                header: h,
                body,
                parent: None,
                exits: Vec::new(),
                fexits: Vec::new(),
                for_init: None,
            });
        }
        let nl = loops.len();

        let mut parent: Vec<Option<usize>> = vec![None; nl];
        for a in 0..nl {
            let ha = loops[a].header;
            let mut best: Option<usize> = None;
            for b in 0..nl {
                if b == a || !loops[b].body.contains(&ha) {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some(c) => loops[b].body.len() < loops[c].body.len(),
                };
                if better {
                    best = Some(b);
                }
            }
            parent[a] = best;
        }
        for (a, p) in parent.into_iter().enumerate() {
            loops[a].parent = p;
        }

        let mut loop_of_header: Vec<Option<usize>> = vec![None; n];
        for (l, lp) in loops.iter().enumerate() {
            loop_of_header[lp.header] = Some(l);
        }
        let mut owner: Vec<Option<usize>> = vec![None; n];
        let mut by_size: Vec<usize> = (0..nl).collect();
        by_size.sort_by_key(|&l| std::cmp::Reverse(loops[l].body.len()));
        for &l in &by_size {
            for &v in &loops[l].body {
                owner[v] = Some(l);
            }
        }

        // Exits, header exit first, then the rest in node order.
        for l in 0..nl {
            let header = loops[l].header;
            let mut body_sorted: Vec<usize> = loops[l].body.iter().copied().collect();
            body_sorted.sort_unstable();
            let mut order: Vec<usize> = succs[header].iter().map(|s| s.target).collect();
            for &x in &body_sorted {
                order.extend(succs[x].iter().map(|s| s.target));
            }
            let mut exits: Vec<usize> = Vec::new();
            for t in order {
                if !loops[l].body.contains(&t) && !exits.contains(&t) {
                    exits.push(t);
                }
            }
            let fexits: Vec<usize> = exits
                .iter()
                .copied()
                .filter(|&t| !succs[t].is_empty())
                .collect();
            loops[l].exits = exits;
            loops[l].fexits = fexits;
        }

        let mut this = Structurer {
            entry,
            reach,
            is_reach,
            emitted: vec![false; n],
            active: vec![false; n],
            succs,
            preds,
            stmts,
            conds,
            loops,
            loop_of_header,
            owner,
            ipdom_cache: vec![None; nl + 1],
            budget: total_stmts.saturating_mul(8).saturating_add(4096),
            spent: 0,
            visits: vec![0; n],
        };

        // For-loops: take NumForInit / GenericForInit out of the single
        // outside predecessor of the header.
        for l in 0..nl {
            let header = this.loops[l].header;
            let is_for_next = this.stmts[header].len() == 1
                && matches!(
                    this.stmts[header][0],
                    Statement::NumForNext(_) | Statement::GenericForNext(_)
                );
            if !is_for_next {
                continue;
            }
            let mut outside: Vec<usize> = this.preds[header]
                .iter()
                .copied()
                .filter(|p| !this.loops[l].body.contains(p) && this.is_reach[*p])
                .collect();
            outside.sort_unstable();
            outside.dedup();
            if outside.len() != 1 {
                continue;
            }
            let p = outside[0];
            let pos = this.stmts[p].iter().rposition(|s| {
                matches!(
                    s,
                    Statement::NumForInit(_) | Statement::GenericForInit(_)
                )
            });
            if let Some(pos) = pos {
                let init = this.stmts[p].remove(pos);
                this.loops[l].for_init = Some(init);
            }
        }

        Some(this)
    }

    fn in_region(&self, v: usize, region: Option<usize>) -> bool {
        match region {
            Some(r) => {
                if self.owner[v] == Some(r) {
                    return true;
                }
                if let Some(hl) = self.loop_of_header[v] {
                    return self.loops[hl].parent == Some(r);
                }
                false
            }
            None => {
                if self.owner[v].is_none() {
                    return true;
                }
                if let Some(hl) = self.loop_of_header[v] {
                    return self.loops[hl].parent.is_none();
                }
                false
            }
        }
    }

    /// Immediate post-dominator of each region member, computed on the
    /// region view: loop headers are the region entry, transitions back to
    /// the header or out of the region are sinks, and `return` blocks are
    /// not in the view at all.
    fn compute_ipdom(&self, region: Option<usize>) -> FxHashMap<usize, usize> {
        let header = region.map(|r| self.loops[r].header);
        // A single-predecessor `return` block is a pure early exit: leave it
        // out of the view so the other arm can merge past the `if`. A shared
        // `return` block is a real merge point and stays in.
        let members: Vec<usize> = self
            .reach
            .iter()
            .copied()
            .filter(|&v| {
                self.in_region(v, region)
                    && (!self.succs[v].is_empty() || self.preds[v].len() >= 2)
            })
            .collect();
        let m = members.len();
        let mut local: FxHashMap<usize, usize> = FxHashMap::default();
        for (i, &v) in members.iter().enumerate() {
            local.insert(v, i);
        }
        let exit = m;
        let mut rsucc: Vec<Vec<usize>> = vec![Vec::new(); m + 1];
        let mut rpred: Vec<Vec<usize>> = vec![Vec::new(); m + 1];
        for (i, &v) in members.iter().enumerate() {
            let mut vs: Vec<usize> = Vec::new();
            for s in &self.succs[v] {
                if Some(s.target) == header {
                    continue;
                }
                if let Some(&j) = local.get(&s.target) {
                    if !vs.contains(&j) {
                        vs.push(j);
                    }
                }
            }
            if vs.is_empty() {
                rsucc[exit].push(i);
                rpred[i].push(exit);
            }
            for &j in &vs {
                rsucc[j].push(i);
                rpred[i].push(j);
            }
        }
        let (idom, _) = chk_idom(&rsucc, &rpred, exit);
        let mut out = FxHashMap::default();
        for i in 0..m {
            let d = idom[i];
            if d != NONE && d != exit {
                out.insert(members[i], members[d]);
            }
        }
        out
    }

    /// Nearest node reachable from both `a` and `b` inside `region` (not the
    /// region header, not `cur`). Used when there is no post-dominator.
    fn join_of(&self, a: usize, b: usize, region: Option<usize>, cur: usize) -> Option<usize> {
        self.join_all(&[a, b], region, cur)
    }

    /// Nearest node reachable from every target (the first target's walk
    /// order decides ties).
    fn join_all(&self, targets: &[usize], region: Option<usize>, cur: usize) -> Option<usize> {
        // Large functions have thousands of region nodes, and the shared node
        // can sit past a small cap in breadth-first order. A capped walk can
        // miss the join, which then copies the tail into every arm.
        const LIMIT: usize = 16384;
        let header = region.and_then(|r| self.loops.get(r)).map(|l| l.header);
        let walk = |start: usize| -> (Vec<usize>, bool) {
            let mut order: Vec<usize> = Vec::new();
            let mut truncated = false;
            let mut seen: FxHashSet<usize> = FxHashSet::default();
            let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
            seen.insert(start);
            queue.push_back(start);
            while let Some(v) = queue.pop_front() {
                if order.len() >= LIMIT {
                    truncated = true;
                    break;
                }
                if Some(v) == header || !self.in_region(v, region) {
                    continue;
                }
                order.push(v);
                for s in &self.succs[v] {
                    if seen.insert(s.target) {
                        queue.push_back(s.target);
                    }
                }
            }
            (order, truncated)
        };
        let (&first, rest) = targets.split_first()?;
        let (mut order, mut truncated) = walk(first);
        let mut common: FxHashSet<usize> = order.iter().copied().collect();
        for &t in rest {
            let (w_order, w_truncated) = walk(t);
            truncated |= w_truncated;
            let w: FxHashSet<usize> = w_order.into_iter().collect();
            common.retain(|v| w.contains(v));
        }
        if truncated {
            note(|| format!("join walk truncated at {LIMIT} nodes (cur={cur})"));
        }
        order.retain(|v| *v != cur && common.contains(v));
        order.into_iter().next()
    }

    fn merge_of(&mut self, region: Option<usize>, u: usize) -> Option<usize> {
        let key = region.unwrap_or(self.loops.len());
        if self.ipdom_cache[key].is_none() {
            let m = self.compute_ipdom(region);
            self.ipdom_cache[key] = Some(m);
        }
        self.ipdom_cache[key].as_ref().and_then(|m| m.get(&u).copied())
    }

    fn cond_arms(&self, u: usize) -> Option<(usize, usize)> {
        let s = &self.succs[u];
        let t = s.iter().position(|x| x.branch == BranchType::Then)?;
        let e = s.iter().position(|x| x.branch == BranchType::Else)?;
        Some((t, e))
    }

    /// Decide what reaching `t` means in `ctx`: fall through, `continue`,
    /// `break` (with a dispatch flag if needed), inline a `return` block, or
    /// inline `t` itself. Only ever pushes statements when it returns `Stop`.
    fn classify(&mut self, t: usize, ctx: &Ctx, out: &mut Vec<Statement>) -> Option<Flow> {
        if ctx.stop == Some(t) {
            return Some(Flow::Stop);
        }
        let Some(r) = ctx.region else {
            return Some(Flow::Inline);
        };
        if self.loops[r].header == t {
            out.push(Continue {}.into());
            return Some(Flow::Stop);
        }
        if self.loops[r].body.contains(&t) {
            return Some(Flow::Inline);
        }
        if self.succs[t].is_empty() {
            // A return block outside the loop: copy it here.
            self.emitted[t] = true;
            out.extend(self.stmts[t].iter().cloned());
            return Some(Flow::Stop);
        }
        if let Some(flag) = &ctx.flag {
            let k = self.loops[r].fexits.iter().position(|&e| e == t)?;
            out.push(assign_stmt(flag.clone(), number(k + 1)));
        }
        out.push(Break {}.into());
        Some(Flow::Stop)
    }

    /// Apply the SSA copies on an edge, then classify its target.
    fn arrive(&mut self, s: &Succ, ctx: &Ctx, out: &mut Vec<Statement>) -> Option<Flow> {
        if let Some(a) = parallel_assign_from_args(&s.args) {
            out.push(a.into());
        }
        self.classify(s.target, ctx, out)
    }

    /// One arm of a two-way branch: copies, then the target chain.
    fn arm(&mut self, s: &Succ, ctx: &Ctx, depth: usize) -> Option<Vec<Statement>> {
        let mut out = Vec::new();
        if self.arrive(s, ctx, &mut out)? == Flow::Stop {
            return Some(out);
        }
        let rest = self.seq(s.target, ctx, depth + 1, false)?;
        out.extend(rest);
        Some(out)
    }

    /// Branch of a multi-exit dispatch: target only, no copies (applied at
    /// the break sites).
    fn dispatch_arm(&mut self, t: usize, ctx: &Ctx, depth: usize) -> Option<Vec<Statement>> {
        let mut out = Vec::new();
        if self.classify(t, ctx, &mut out)? == Flow::Stop {
            return Some(out);
        }
        let rest = self.seq(t, ctx, depth + 1, false)?;
        out.extend(rest);
        Some(out)
    }

    fn seq(
        &mut self,
        start: usize,
        ctx: &Ctx,
        depth: usize,
        skip_first_checks: bool,
    ) -> Option<Vec<Statement>> {
        if depth > MAX_DEPTH || cfg::past_decompile_deadline() {
            note(|| format!("depth {depth} or deadline reached"));
            return None;
        }
        let mut marked: Vec<usize> = Vec::new();
        let result = self.seq_inner(start, ctx, depth, skip_first_checks, &mut marked);
        for v in marked {
            self.active[v] = false;
        }
        result
    }

    fn seq_inner(
        &mut self,
        start: usize,
        ctx: &Ctx,
        depth: usize,
        mut skip_checks: bool,
        marked: &mut Vec<usize>,
    ) -> Option<Vec<Statement>> {
        let mut out: Vec<Statement> = Vec::new();
        let mut cur = start;
        loop {
            if !skip_checks {
                if self.classify(cur, ctx, &mut out)? == Flow::Stop {
                    return Some(out);
                }
                if let Some(l) = self.loop_of_header[cur] {
                    let (stmts, cont) = self.emit_loop(l, ctx, depth + 1)?;
                    out.extend(stmts);
                    match cont {
                        Some(c) => {
                            cur = c;
                            continue;
                        }
                        None => return Some(out),
                    }
                }
            }
            skip_checks = false;

            // Reaching a node that is still on the emission path means the
            // graph has a cycle that is not a natural loop (irreducible).
            if self.active[cur] {
                note(|| format!("cycle through node {cur} that is not a natural loop"));
                return None;
            }
            // Diagnostic only: a node emitted again is code copied into
            // another path (tail duplication). Counted by context.
            if self.emitted[cur] {
                note(|| {
                    format!(
                        "duplicate emit of node {cur}: region={:?} stop={:?} preds={} succs={}",
                        ctx.region,
                        ctx.stop,
                        self.preds[cur].len(),
                        self.succs[cur].len()
                    )
                });
            }
            self.active[cur] = true;
            marked.push(cur);
            self.emitted[cur] = true;
            self.spent += self.stmts[cur].len() + 1;
            self.visits[cur] = self.visits[cur].saturating_add(1);
            if self.spent > self.budget {
                note(|| {
                    let mut top: Vec<(u32, usize)> = self
                        .visits
                        .iter()
                        .enumerate()
                        .map(|(i, &c)| (c, i))
                        .collect();
                    top.sort_unstable_by(|a, b| b.cmp(a));
                    let top: Vec<String> = top
                        .iter()
                        .take(6)
                        .map(|&(c, i)| format!("node {i} x{c} ({} stmts)", self.stmts[i].len()))
                        .collect();
                    format!(
                        "statement budget exceeded ({} > {}); most emitted: {}",
                        self.spent,
                        self.budget,
                        top.join(", ")
                    )
                });
                return None;
            }
            out.extend(self.stmts[cur].iter().cloned());

            match self.succs[cur].len() {
                0 => return Some(out),
                1 => {
                    let s = self.succs[cur][0].clone();
                    if self.arrive(&s, ctx, &mut out)? == Flow::Stop {
                        return Some(out);
                    }
                    cur = s.target;
                }
                2 => {
                    let Some((ti, ei)) = self.cond_arms(cur) else {
                        note(|| format!("node {cur}: 2-way node without Then/Else edges"));
                        return None;
                    };
                    let Some(cond) = self.conds[cur].clone() else {
                        note(|| format!("node {cur}: 2-way node without a trailing If"));
                        return None;
                    };
                    let then_s = self.succs[cur][ti].clone();
                    let else_s = self.succs[cur][ei].clone();
                    // Post-dominator first. If the arms only meet through
                    // breaks or returns (no post-dominator), stop both arms at
                    // the nearest node they share: copying the rest of the
                    // function into each arm is what blew up `a and b`
                    // chains. An arm without a merge of its own keeps the
                    // enclosing stop, so it never runs past it.
                    let merge = self
                        .merge_of(ctx.region, cur)
                        .or_else(|| self.join_of(then_s.target, else_s.target, ctx.region, cur));
                    // Diagnostic only: a branch with no merge copies its
                    // continuation into both arms. Counted by arm kind.
                    if merge.is_none() {
                        note(|| {
                            format!(
                                "2-way no-merge: then_in_region={} else_in_region={} then_exit={} else_exit={} then_is_stop={} else_is_stop={} stop_set={}",
                                self.in_region(then_s.target, ctx.region),
                                self.in_region(else_s.target, ctx.region),
                                self.succs[then_s.target].is_empty(),
                                self.succs[else_s.target].is_empty(),
                                ctx.stop == Some(then_s.target),
                                ctx.stop == Some(else_s.target),
                                ctx.stop.is_some()
                            )
                        });
                    }
                    let arm_ctx = Ctx {
                        region: ctx.region,
                        stop: merge.or(ctx.stop),
                        flag: ctx.flag.clone(),
                    };
                    let then_b = self.arm(&then_s, &arm_ctx, depth)?;
                    let else_b = self.arm(&else_s, &arm_ctx, depth)?;
                    if !then_b.is_empty() || !else_b.is_empty() {
                        out.push(If::new(cond, Block(then_b), Block(else_b)).into());
                    }
                    match merge {
                        Some(m) => cur = m,
                        None => return Some(out),
                    }
                }
                _ => return None,
            }
        }
    }

    /// Emit loop `l` as a statement list plus the node control continues at
    /// afterwards (`None` = no normal continuation).
    fn emit_loop(
        &mut self,
        l: usize,
        outer: &Ctx,
        depth: usize,
    ) -> Option<(Vec<Statement>, Option<usize>)> {
        let header = self.loops[l].header;
        // The `while` / `for` forms consume the header without going through
        // `seq_inner`, so mark it here; `emit()` checks that every reachable
        // node was emitted.
        self.emitted[header] = true;
        let exits = self.loops[l].exits.clone();
        let fexits = self.loops[l].fexits.clone();
        let hsuccs = self.succs[header].clone();
        let hstmts = self.stmts[header].clone();

        let body_idx: Vec<usize> = (0..hsuccs.len())
            .filter(|&i| self.loops[l].body.contains(&hsuccs[i].target))
            .collect();
        let exit_idx: Vec<usize> = (0..hsuccs.len())
            .filter(|i| !body_idx.contains(i))
            .collect();
        let for_like = hstmts.len() == 1
            && matches!(
                hstmts[0],
                Statement::NumForNext(_) | Statement::GenericForNext(_)
            );
        // `return` exits are copied inline by `classify`, so only real exits
        // count. The `for` / `while` forms need the header's own exit to be
        // the one real exit (or no real exit at all).
        let single_exit = match fexits.as_slice() {
            [] => true,
            [only] => exit_idx.len() == 1 && hsuccs[exit_idx[0]].target == *only,
            _ => false,
        };
        // The header's own exit when the header is a plain two-way node.
        let header_exit = if hsuccs.len() == 2 && body_idx.len() == 1 && exit_idx.len() == 1 {
            Some(hsuccs[exit_idx[0]].target)
        } else {
            None
        };
        // A `for` loop needs the flag when another real exit exists beside
        // its header exit (a return-block header exit is copied inline).
        let flagged = if for_like {
            match fexits.as_slice() {
                [] => false,
                [only] => header_exit != Some(*only),
                _ => true,
            }
        } else {
            fexits.len() > 1
        };
        let flag = if flagged {
            Some(RcLocal::default())
        } else {
            None
        };
        let mut pre: Vec<Statement> = Vec::new();
        if let Some(f) = &flag {
            let mut decl = Assign::new(vec![LValue::Local(f.clone())], vec![]);
            decl.prefix = true;
            pre.push(decl.into());
        }
        let inner = Ctx {
            region: Some(l),
            stop: None,
            flag: flag.clone(),
        };

        if for_like {
            // Only the plain shape `for` recovers; anything else is unknown IR.
            // Extra exits (a `break` to another block) are allowed when the
            // header's own exit is one of them: the body sets the flag and
            // breaks (see `classify`), and the exits are dispatched after the
            // loop, the same way the generic form does it.
            let shape_ok = header_exit.is_some();
            let exits_ok = flagged || fexits.is_empty() || header_exit == fexits.first().copied();
            if !(shape_ok && exits_ok) {
                note(|| format!("for header {header}: unexpected shape (exits={}, fexits={}, succs={}, body={}, outside={})", exits.len(), fexits.len(), hsuccs.len(), body_idx.len(), exit_idx.len()));
                return None;
            }
            let Some(init) = self.loops[l].for_init.clone() else {
                note(|| format!("for header {header}: no NumForInit/GenericForInit in the preheader"));
                return None;
            };
            let bs = hsuccs[body_idx[0]].clone();
            let es = hsuccs[exit_idx[0]].clone();
            let body = self.arm(&bs, &inner, depth)?;
            let stmt: Statement = match (&hstmts[0], &init) {
                (Statement::NumForNext(next), Statement::NumForInit(ni)) => {
                    let counter = next.counter.0.as_local()?.clone();
                    NumericFor::new(
                        ni.counter.1.clone(),
                        ni.limit.1.clone(),
                        ni.step.1.clone(),
                        counter,
                        Block(body),
                    )
                    .into()
                }
                (Statement::GenericForNext(next), Statement::GenericForInit(gi)) => {
                    let mut locals: Vec<RcLocal> = Vec::with_capacity(next.res_locals.len());
                    for lv in &next.res_locals {
                        locals.push(lv.as_local()?.clone());
                    }
                    GenericFor::new(locals, gi.0.right.clone(), Block(body)).into()
                }
                _ => return None,
            };
            let mut out = pre;
            out.push(stmt);
            if !flagged {
                if let Some(a) = parallel_assign_from_args(&es.args) {
                    out.push(a.into());
                }
                return Some((out, Some(es.target)));
            }
            // Several exits: the loop's natural end is the default arm (with
            // its copies); each other exit is selected by the flag set at its
            // break site.
            let merge = self
                .merge_of(outer.region, header)
                .or_else(|| self.join_all(&fexits, outer.region, usize::MAX));
            let dctx = Ctx {
                region: outer.region,
                stop: merge,
                flag: outer.flag.clone(),
            };
            let fl = flag.clone()?;
            let mut natural: Vec<Statement> = Vec::new();
            if let Some(a) = parallel_assign_from_args(&es.args) {
                natural.push(a.into());
            }
            natural.extend(self.dispatch_arm(es.target, &dctx, depth)?);
            let mut acc = natural;
            for (k, &e) in fexits.iter().enumerate().rev() {
                if e == es.target {
                    continue;
                }
                let arm = self.dispatch_arm(e, &dctx, depth)?;
                let cond = Binary::new(
                    RValue::Local(fl.clone()),
                    number(k + 1),
                    BinaryOperation::Equal,
                );
                acc = vec![If::new(cond.into(), Block(arm), Block(acc)).into()];
            }
            out.extend(acc);
            return Some((out, merge));
        }

        let while_like = hstmts.is_empty()
            && self.conds[header].is_some()
            && hsuccs.len() == 2
            && body_idx.len() == 1
            && exit_idx.len() == 1;
        if while_like && single_exit {
            let stay = hsuccs[body_idx[0]].clone();
            let ex = hsuccs[exit_idx[0]].clone();
            let c = self.conds[header].clone()?;
            let cond = if stay.branch == BranchType::Then {
                c
            } else {
                Unary::new(c, UnaryOperation::Not).reduce_condition()
            };
            let body = self.arm(&stay, &inner, depth)?;
            let mut out = pre;
            out.push(While::new(cond, Block(body)).into());
            if let Some(a) = parallel_assign_from_args(&ex.args) {
                out.push(a.into());
            }
            return Some((out, Some(ex.target)));
        }

        // Generic: the header's own statements and conditional are part of
        // the body; exits are breaks (dispatched by flag if there are many).
        let body = self.seq(header, &inner, depth + 1, true)?;
        let mut out = pre;
        out.push(
            While::new(RValue::Literal(Literal::Boolean(true)), Block(body)).into(),
        );
        match fexits.len() {
            0 => Some((out, None)),
            1 => Some((out, Some(fexits[0]))),
            _ => {
                let merge = self
                    .merge_of(outer.region, header)
                    .or_else(|| self.join_all(&fexits, outer.region, usize::MAX));
                let dctx = Ctx {
                    region: outer.region,
                    stop: merge,
                    flag: outer.flag.clone(),
                };
                let fl = flag?;
                let n = fexits.len();
                let mut acc = self.dispatch_arm(fexits[n - 1], &dctx, depth)?;
                for k in (0..n - 1).rev() {
                    let arm = self.dispatch_arm(fexits[k], &dctx, depth)?;
                    let cond = Binary::new(
                        RValue::Local(fl.clone()),
                        number(k + 1),
                        BinaryOperation::Equal,
                    );
                    acc = vec![If::new(cond.into(), Block(arm), Block(acc)).into()];
                }
                out.extend(acc);
                Some((out, merge))
            }
        }
    }

    fn emit(&mut self) -> Option<Vec<Statement>> {
        let ctx = Ctx {
            region: None,
            stop: None,
            flag: None,
        };
        let body = self.seq(self.entry, &ctx, 0, false)?;
        // Every reachable node must have been emitted somewhere.
        for &v in &self.reach {
            if !self.emitted[v] {
                note(|| format!("reachable node {v} was never emitted"));
                return None;
            }
        }
        Some(body)
    }
}

/// True when the tree has no `goto`, no labels, and no leftover for-loop IR.
fn is_clean(block: &Block) -> bool {
    for s in &block.0 {
        let ok = match s {
            Statement::Goto(_)
            | Statement::Label(_)
            | Statement::NumForInit(_)
            | Statement::NumForNext(_)
            | Statement::GenericForInit(_)
            | Statement::GenericForNext(_) => false,
            Statement::If(ifs) => {
                is_clean(&ifs.then_block.lock()) && is_clean(&ifs.else_block.lock())
            }
            Statement::While(w) => is_clean(&w.block.lock()),
            Statement::Repeat(r) => is_clean(&r.block.lock()),
            Statement::NumericFor(f) => is_clean(&f.block.lock()),
            Statement::GenericFor(f) => is_clean(&f.block.lock()),
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Structure `function` without `goto`. `None` means "could not prove a
/// clean structured result"; the caller should fall back to the legacy
/// structurer, which always produces some output.
pub(crate) fn try_lift(function: &Function) -> Option<Block> {
    let Some(mut s) = Structurer::build(function) else {
        note(|| "build gave up (see the reason above)".to_string());
        return None;
    };
    note(|| format!("built: {} loops, {} reachable nodes", s.loops.len(), s.reach.len()));
    let Some(mut body) = s.emit() else {
        note(|| "emit gave up (no more specific reason logged)".to_string());
        return None;
    };
    if let Some(Statement::Return(r)) = body.last() {
        if r.values.is_empty() {
            body.pop();
        }
    }
    let block = Block(body);
    if !is_clean(&block) {
        note(|| "output still contains goto/label/for IR".to_string());
        return None;
    }
    note(|| format!("ok ({} top-level statements)", block.0.len()));
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ast::Local;
    use cfg::block::BlockEdge;
    use petgraph::stable_graph::NodeIndex;

    fn local(name: &str) -> RcLocal {
        RcLocal::new(Local::new(Some(name.to_string())))
    }

    fn comment(text: &str) -> Statement {
        ast::Comment::new(text.to_string()).into()
    }

    fn ret() -> Statement {
        ast::Return { values: vec![] }.into()
    }

    /// Block whose last statement is a conditional `If` (edges are Then/Else).
    fn cond_block(cond: &RcLocal) -> Block {
        Block(vec![If::new(
            RValue::Local(cond.clone()),
            Block::default(),
            Block::default(),
        )
        .into()])
    }

    fn edge(f: &mut Function, a: NodeIndex, b: NodeIndex, t: BranchType) {
        f.graph_mut().add_edge(a, b, BlockEdge::new(t));
    }

    fn text(block: &Block) -> String {
        format!("{block}")
    }

    #[test]
    fn straight_line_and_return() {
        let mut f = Function::new(0);
        let a = f.graph_mut().add_node(Block(vec![comment("a")]));
        let b = f.graph_mut().add_node(Block(vec![comment("b"), ret()]));
        edge(&mut f, a, b, BranchType::Unconditional);
        f.set_entry(a);
        let out = try_lift(&f).expect("structured");
        let s = text(&out);
        assert!(s.contains("-- a") && s.contains("-- b"), "{s}");
        assert!(!s.contains("goto"), "{s}");
    }

    #[test]
    fn diamond_becomes_if_else_with_merge() {
        let c = local("c");
        let mut f = Function::new(0);
        let entry = f.graph_mut().add_node(cond_block(&c));
        let t = f.graph_mut().add_node(Block(vec![comment("then")]));
        let e = f.graph_mut().add_node(Block(vec![comment("else")]));
        let m = f.graph_mut().add_node(Block(vec![comment("merge"), ret()]));
        edge(&mut f, entry, t, BranchType::Then);
        edge(&mut f, entry, e, BranchType::Else);
        edge(&mut f, t, m, BranchType::Unconditional);
        edge(&mut f, e, m, BranchType::Unconditional);
        f.set_entry(entry);
        let out = try_lift(&f).expect("structured");
        let s = text(&out);
        assert!(s.contains("if c then"), "{s}");
        assert!(s.contains("else"), "{s}");
        assert_eq!(s.matches("-- merge").count(), 1, "{s}");
        assert!(!s.contains("goto"), "{s}");
    }

    #[test]
    fn early_return_does_not_duplicate_the_rest() {
        let c = local("c");
        let mut f = Function::new(0);
        let entry = f.graph_mut().add_node(cond_block(&c));
        let r = f.graph_mut().add_node(Block(vec![comment("early"), ret()]));
        let rest = f.graph_mut().add_node(Block(vec![comment("rest"), ret()]));
        edge(&mut f, entry, r, BranchType::Then);
        edge(&mut f, entry, rest, BranchType::Else);
        f.set_entry(entry);
        let out = try_lift(&f).expect("structured");
        let s = text(&out);
        assert_eq!(s.matches("-- rest").count(), 1, "{s}");
        assert_eq!(s.matches("-- early").count(), 1, "{s}");
    }

    #[test]
    fn while_loop_with_break_and_continue() {
        // header: if c then body else exit ; body: if d then exit2 (break)
        // else latch ; latch -> header (continue)
        let c = local("c");
        let d = local("d");
        let mut f = Function::new(0);
        let entry = f.graph_mut().add_node(Block(vec![comment("entry")]));
        let header = f.graph_mut().add_node(cond_block(&c));
        let body = f.graph_mut().add_node(cond_block(&d));
        let latch = f.graph_mut().add_node(Block(vec![comment("latch")]));
        let exit = f.graph_mut().add_node(Block(vec![comment("exit")]));
        let done = f.graph_mut().add_node(Block(vec![comment("done"), ret()]));
        edge(&mut f, entry, header, BranchType::Unconditional);
        edge(&mut f, header, body, BranchType::Then);
        edge(&mut f, header, exit, BranchType::Else);
        edge(&mut f, body, exit, BranchType::Then);
        edge(&mut f, body, latch, BranchType::Else);
        edge(&mut f, latch, header, BranchType::Unconditional);
        edge(&mut f, exit, done, BranchType::Unconditional);
        f.set_entry(entry);
        let out = try_lift(&f).expect("structured");
        let s = text(&out);
        assert!(s.contains("while"), "{s}");
        assert!(s.contains("break"), "{s}");
        assert!(s.contains("-- exit"), "{s}");
        assert!(!s.contains("goto"), "{s}");
    }

    #[test]
    fn irreducible_cycle_is_rejected() {
        // entry branches into both halves of a two-node cycle.
        let c = local("c");
        let mut f = Function::new(0);
        let entry = f.graph_mut().add_node(cond_block(&c));
        let a = f.graph_mut().add_node(Block(vec![comment("a")]));
        let b = f.graph_mut().add_node(Block(vec![comment("b")]));
        edge(&mut f, entry, a, BranchType::Then);
        edge(&mut f, entry, b, BranchType::Else);
        edge(&mut f, a, b, BranchType::Unconditional);
        edge(&mut f, b, a, BranchType::Unconditional);
        f.set_entry(entry);
        assert!(try_lift(&f).is_none());
    }
}
