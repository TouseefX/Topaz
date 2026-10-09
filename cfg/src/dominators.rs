//! Dense Cooper–Harvey–Kennedy immediate dominators.
//!
//! `petgraph::algo::dominators::simple_fast` stores the idom tree in a
//! `HashMap` and walks it on every intersect. On a 15–20k-node diamond CFG
//! that is O(n²) hash lookups **per solve**, and collapse/SSA used to run
//! that ~36 times — that's the leftover "forever" vs LunaUX ~20s. Same
//! algorithm, reverse-postorder arrays, O(1) intersect steps.

use std::hash::Hash;

use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{IntoNeighborsDirected, IntoNodeIdentifiers};
use petgraph::Direction;
use rustc_hash::{FxHashMap, FxHashSet};

/// Immediate-dominator tree. `immediate_dominator(root)` is `None`, matching
/// petgraph so existing `DomIndex` / loop-header code stays honest.
#[derive(Clone, Debug)]
pub struct IDom<N = NodeIndex> {
    root: N,
    /// `node → idom`. Root maps to itself.
    map: FxHashMap<N, N>,
}

impl<N: Copy + Eq + Hash> IDom<N> {
    pub fn dummy(root: N) -> Self {
        let mut map = FxHashMap::default();
        map.insert(root, root);
        Self { root, map }
    }

    pub fn root(&self) -> N {
        self.root
    }

    #[inline]
    pub fn immediate_dominator(&self, n: N) -> Option<N> {
        if n == self.root {
            None
        } else {
            self.map.get(&n).copied()
        }
    }
}

/// Dominators of every node reachable from `root`.
pub fn compute_idoms<G>(graph: G, root: G::NodeId) -> IDom<G::NodeId>
where
    G: IntoNeighborsDirected + IntoNodeIdentifiers + Copy,
    G::NodeId: Copy + Eq + Hash,
{
    let mut succs: FxHashMap<G::NodeId, Vec<G::NodeId>> = FxHashMap::default();
    let mut preds: FxHashMap<G::NodeId, Vec<G::NodeId>> = FxHashMap::default();
    for n in graph.node_identifiers() {
        succs.insert(
            n,
            graph.neighbors_directed(n, Direction::Outgoing).collect(),
        );
        preds.insert(
            n,
            graph.neighbors_directed(n, Direction::Incoming).collect(),
        );
    }
    compute_from_adj(root, &succs, &preds)
}

/// Post-dominators via a virtual super-sink (no graph mutation).
pub fn compute_post_idoms<N, E>(
    graph: &petgraph::stable_graph::StableDiGraph<N, E>,
) -> IDom<NodeIndex> {
    let mut orig_succ: FxHashMap<NodeIndex, Vec<NodeIndex>> = FxHashMap::default();
    let mut orig_pred: FxHashMap<NodeIndex, Vec<NodeIndex>> = FxHashMap::default();
    let mut exits = Vec::new();
    for n in graph.node_indices() {
        let s: Vec<_> = graph.neighbors_directed(n, Direction::Outgoing).collect();
        let p: Vec<_> = graph.neighbors_directed(n, Direction::Incoming).collect();
        if s.is_empty() {
            exits.push(n);
        }
        orig_succ.insert(n, s);
        orig_pred.insert(n, p);
    }
    compute_post_idoms_from_orig(orig_succ, orig_pred, exits)
}

fn compute_post_idoms_from_orig(
    orig_succ: FxHashMap<NodeIndex, Vec<NodeIndex>>,
    orig_pred: FxHashMap<NodeIndex, Vec<NodeIndex>>,
    exits: Vec<NodeIndex>,
) -> IDom<NodeIndex> {
    let sink = NodeIndex::end();
    let mut succs: FxHashMap<NodeIndex, Vec<NodeIndex>> = FxHashMap::default();
    let mut preds: FxHashMap<NodeIndex, Vec<NodeIndex>> = FxHashMap::default();
    for (n, p) in &orig_pred {
        succs.insert(*n, p.clone());
    }
    for (n, s) in &orig_succ {
        preds.insert(*n, s.clone());
    }
    succs.insert(sink, exits.clone());
    preds.insert(sink, Vec::new());
    for e in &exits {
        preds.entry(*e).or_default().push(sink);
    }
    compute_from_adj(sink, &succs, &preds)
}

pub fn compute_from_adj<N: Copy + Eq + Hash>(
    root: N,
    succs: &FxHashMap<N, Vec<N>>,
    preds: &FxHashMap<N, Vec<N>>,
) -> IDom<N> {
    let empty: Vec<N> = Vec::new();
    let mut post = Vec::new();
    let mut seen: FxHashSet<N> = FxHashSet::default();
    let mut stack: Vec<(N, usize)> = vec![(root, 0)];
    seen.insert(root);
    while let Some(&(n, i)) = stack.last() {
        let ch = succs.get(&n).unwrap_or(&empty);
        if i < ch.len() {
            stack.last_mut().unwrap().1 = i + 1;
            let c = ch[i];
            if seen.insert(c) {
                stack.push((c, 0));
            }
        } else {
            stack.pop();
            post.push(n);
        }
    }
    post.reverse();
    let n = post.len();
    if n == 0 {
        return IDom {
            root,
            map: FxHashMap::default(),
        };
    }

    let mut dense: FxHashMap<N, u32> =
        FxHashMap::with_capacity_and_hasher(n, Default::default());
    for (i, &node) in post.iter().enumerate() {
        dense.insert(node, i as u32);
    }

    let mut pred_idx: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, &node) in post.iter().enumerate() {
        for &pred in preds.get(&node).unwrap_or(&empty) {
            if let Some(&p) = dense.get(&pred) {
                pred_idx[i].push(p);
            }
        }
    }

    let mut idom = vec![u32::MAX; n];
    idom[0] = 0;

    let mut changed = true;
    let mut iters = 0u32;
    while changed {
        changed = false;
        iters += 1;
        // Reducible CFGs (diamond chains) converge in 2–3 passes. 8 is
        // plenty; 64 × O(n) on 20k nodes was still seconds per solve.
        if iters > 8 || crate::past_decompile_deadline() {
            break;
        }
        for i in 1..n {
            let ps = &pred_idx[i];
            let mut new_idom = None;
            for &p in ps {
                if idom[p as usize] != u32::MAX {
                    new_idom = Some(p);
                    break;
                }
            }
            let Some(mut nid) = new_idom else {
                continue;
            };
            for &p in ps {
                if p == nid {
                    continue;
                }
                if idom[p as usize] != u32::MAX {
                    nid = intersect(&idom, nid, p);
                }
            }
            if idom[i] != nid {
                idom[i] = nid;
                changed = true;
            }
        }
    }

    let mut map = FxHashMap::with_capacity_and_hasher(n, Default::default());
    for (i, &node) in post.iter().enumerate() {
        let d = idom[i];
        if d != u32::MAX {
            map.insert(node, post[d as usize]);
        }
    }
    IDom { root, map }
}

/// Walk both fingers toward the root (smaller RPO index). Dominators are
/// always earlier in reverse postorder, so this terminates.
#[inline]
fn intersect(idom: &[u32], mut f1: u32, mut f2: u32) -> u32 {
    while f1 != f2 {
        while f1 > f2 {
            f1 = idom[f1 as usize];
        }
        while f2 > f1 {
            f2 = idom[f2 as usize];
        }
    }
    f1
}

#[cfg(test)]
mod tests {
    use petgraph::stable_graph::StableDiGraph;

    use super::compute_idoms;

    #[test]
    fn diamond() {
        let mut g: StableDiGraph<(), ()> = StableDiGraph::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        let d = g.add_node(());
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        g.add_edge(b, d, ());
        g.add_edge(c, d, ());
        let idom = compute_idoms(&g, a);
        assert_eq!(idom.immediate_dominator(a), None);
        assert_eq!(idom.immediate_dominator(b), Some(a));
        assert_eq!(idom.immediate_dominator(c), Some(a));
        assert_eq!(idom.immediate_dominator(d), Some(a));
    }

    #[test]
    fn chain() {
        let mut g: StableDiGraph<(), ()> = StableDiGraph::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        let idom = compute_idoms(&g, a);
        assert_eq!(idom.immediate_dominator(b), Some(a));
        assert_eq!(idom.immediate_dominator(c), Some(b));
    }
}
