//! Node splitting. A flow graph is reducible when every retreating edge
//! goes to a node that dominates its source. Otherwise a loop has several
//! entries and no structured form exists without flags. Copying the target
//! of such an edge (same statements, same out-edges) and redirecting that
//! one edge to the copy removes the extra entry. The copy is equivalent code,
//! so the semantics do not change.

use cfg::{block::BlockEdge, function::Function};
use petgraph::{
    algo::dominators::simple_fast,
    stable_graph::{EdgeIndex, NodeIndex},
    visit::EdgeRef,
};
use rustc_hash::FxHashMap;

use crate::structured::note;

/// Floor and ceiling for the number of copies made for one function.
const MIN_COPIES: usize = 64;
const MAX_COPIES: usize = 1024;

/// Make `function` reducible by copying blocks. Returns the number of copies.
pub fn make_reducible(function: &mut Function) -> usize {
    let Some(entry) = *function.entry() else {
        return 0;
    };
    let Some(first) = irreducible_edge(function, entry) else {
        return 0;
    };
    let budget = (function.graph().node_count() * 2).clamp(MIN_COPIES, MAX_COPIES);
    note(|| format!("split: irreducible edge {} -> {}; copying (budget {budget})", first.0.index(), first.1.index()));
    let mut copies = 0;
    let mut next = Some(first);
    while let Some((u, v)) = next {
        if copies >= budget {
            note(|| format!("split: budget of {budget} copies reached; irreducible edges remain"));
            break;
        }
        copy_target(function, u, v);
        copies += 1;
        next = irreducible_edge(function, entry);
    }
    note(|| format!("split: {copies} copies made"));
    copies
}

/// A retreating edge `u -> v` (v is on the DFS stack when u reaches it) whose
/// target does not dominate its source.
fn irreducible_edge(function: &Function, entry: NodeIndex) -> Option<(NodeIndex, NodeIndex)> {
    let g = function.graph();
    let dom = simple_fast(g, entry);
    // Node -> still on the DFS stack (true) or finished (false).
    let mut state: FxHashMap<NodeIndex, bool> = FxHashMap::default();
    let mut stack: Vec<(NodeIndex, Vec<NodeIndex>, usize)> = Vec::new();
    state.insert(entry, true);
    stack.push((entry, g.neighbors(entry).collect(), 0));
    while let Some(top) = stack.last_mut() {
        if top.2 >= top.1.len() {
            let done = top.0;
            stack.pop();
            state.insert(done, false);
            continue;
        }
        let w = top.1[top.2];
        top.2 += 1;
        let u = top.0;
        match state.get(&w) {
            Some(true) => {
                let dominated = dom
                    .dominators(u)
                    .is_some_and(|mut ds| ds.any(|d| d == w));
                if !dominated {
                    return Some((u, w));
                }
            }
            Some(false) => {}
            None => {
                state.insert(w, true);
                stack.push((w, g.neighbors(w).collect(), 0));
            }
        }
    }
    None
}

/// Copy `v` and redirect every edge `u -> v` to the copy. The copy keeps the
/// out-edges of `v` (with their SSA copies).
fn copy_target(function: &mut Function, u: NodeIndex, v: NodeIndex) {
    let g = function.graph_mut();
    let block = g[v].deep_clone();
    let out: Vec<(NodeIndex, BlockEdge)> = g
        .edges(v)
        .map(|e| (e.target(), e.weight().clone()))
        .collect();
    let copy = g.add_node(block);
    for (w, e) in out {
        g.add_edge(copy, w, e);
    }
    let into: Vec<(EdgeIndex, BlockEdge)> = g
        .edges(u)
        .filter(|e| e.target() == v)
        .map(|e| (e.id(), e.weight().clone()))
        .collect();
    for (id, e) in into {
        g.remove_edge(id);
        g.add_edge(u, copy, e);
    }
}
