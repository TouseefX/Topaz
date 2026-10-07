use petgraph::{algo::dominators::Dominators, stable_graph::NodeIndex};
use rustc_hash::{FxHashMap, FxHashSet};

/// Euler-tour times on a dominator tree.
/// `a` dominates `b` iff `inn[a] <= inn[b] && out[b] <= out[a]` — O(1).
#[derive(Default, Clone)]
pub struct DomIndex {
    inn: FxHashMap<NodeIndex, u32>,
    out: FxHashMap<NodeIndex, u32>,
}

impl DomIndex {
    pub fn build(
        nodes: impl IntoIterator<Item = NodeIndex>,
        doms: &Dominators<NodeIndex>,
    ) -> Self {
        let present: FxHashSet<NodeIndex> = nodes.into_iter().collect();
        let mut children: FxHashMap<NodeIndex, Vec<NodeIndex>> =
            FxHashMap::with_capacity_and_hasher(present.len(), Default::default());
        let mut roots = Vec::new();
        for &n in &present {
            match doms.immediate_dominator(n) {
                Some(p) if present.contains(&p) => children.entry(p).or_default().push(n),
                _ => roots.push(n),
            }
        }
        let mut inn = FxHashMap::with_capacity_and_hasher(present.len(), Default::default());
        let mut out = FxHashMap::with_capacity_and_hasher(present.len(), Default::default());
        let mut time = 0u32;
        for root in roots {
            let mut stack: Vec<(NodeIndex, usize)> = vec![(root, 0)];
            inn.insert(root, time);
            time += 1;
            loop {
                let Some(&(node, i)) = stack.last() else {
                    break;
                };
                if let Some(&child) = children.get(&node).and_then(|v| v.get(i)) {
                    stack.last_mut().unwrap().1 = i + 1;
                    inn.insert(child, time);
                    time += 1;
                    stack.push((child, 0));
                } else {
                    stack.pop();
                    out.insert(node, time);
                    time += 1;
                }
            }
        }
        Self { inn, out }
    }

    /// Does `a` dominate `b`? (reflexive)
    #[inline]
    pub fn dominates(&self, a: NodeIndex, b: NodeIndex) -> bool {
        match (
            self.inn.get(&a),
            self.inn.get(&b),
            self.out.get(&a),
            self.out.get(&b),
        ) {
            (Some(&ain), Some(&bin), Some(&aout), Some(&bout)) => ain <= bin && bout <= aout,
            _ => a == b,
        }
    }
}
