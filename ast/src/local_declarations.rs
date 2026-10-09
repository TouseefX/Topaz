use std::collections::BTreeMap;

use by_address::ByAddress;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use petgraph::{
    algo::dominators::{simple_fast, Dominators},
    prelude::{DiGraph, NodeIndex},
    Direction,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Assign, LocalRw, RcLocal, SharedBlock, Statement};

/// Euler-tour dominates: O(n) preprocess, O(1) query. Replaces
/// `dominators(n).collect_vec()` ∩ Intersect which was O(depth²) per local.
struct DomIdx {
    inn: FxHashMap<NodeIndex, u32>,
    out: FxHashMap<NodeIndex, u32>,
}

impl DomIdx {
    fn build(nodes: impl IntoIterator<Item = NodeIndex>, doms: &Dominators<NodeIndex>) -> Self {
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

    #[inline]
    fn dominates(&self, a: NodeIndex, b: NodeIndex) -> bool {
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

#[derive(Default)]
pub struct LocalDeclarer {
    block_to_node: FxHashMap<ByAddress<SharedBlock>, NodeIndex>,
    graph: DiGraph<(Option<SharedBlock>, usize), ()>,
    local_usages: IndexMap<RcLocal, FxHashMap<NodeIndex, usize>>,
    declarations: FxHashMap<ByAddress<SharedBlock>, BTreeMap<usize, IndexSet<RcLocal>>>,
    /// `for i, v in ...` / `for i = ...` binders. Recording their body
    /// reads (v7) inserted `local i, v` as the first statement of the
    /// loop, shadowing the iterator with nil (CameraShaker `Update`,
    /// sample `ipairs`).
    binder_depth: FxHashMap<RcLocal, u32>,
}

impl LocalDeclarer {
    fn note_usage(&mut self, local: RcLocal, node: NodeIndex, stat_index: usize) {
        if self.binder_depth.contains_key(&local) {
            return;
        }
        self.local_usages
            .entry(local)
            .or_default()
            .entry(node)
            .and_modify(|i| *i = (*i).min(stat_index))
            .or_insert(stat_index);
    }

    fn push_binders(&mut self, binders: &[RcLocal]) {
        for b in binders {
            *self.binder_depth.entry(b.clone()).or_insert(0) += 1;
        }
    }

    fn pop_binders(&mut self, binders: &[RcLocal]) {
        for b in binders.iter().rev() {
            if let Some(d) = self.binder_depth.get_mut(b) {
                *d = d.saturating_sub(1);
                if *d == 0 {
                    self.binder_depth.remove(b);
                }
            }
        }
    }

    fn visit(&mut self, block: SharedBlock, stat_index: usize) -> NodeIndex {
        // Shared / cyclic `SharedBlock` (half-destructed CFG) used
        // to re-enter `block.lock()` and sleep forever on parking_lot.
        if let Some(&existing) = self.block_to_node.get(&ByAddress(block.clone())) {
            return existing;
        }
        let node = self.graph.add_node((Some(block.clone()), stat_index));
        self.block_to_node.insert(block.clone().into(), node);

        enum Nested {
            If {
                stat_index: usize,
                then_b: SharedBlock,
                else_b: SharedBlock,
            },
            One {
                stat_index: usize,
                child: SharedBlock,
                binders: Vec<RcLocal>,
            },
        }
        let mut nested = Vec::new();
        {
            let Some(guard) = block.try_lock() else {
                return node;
            };
            for (stat_index, stat) in guard.iter().enumerate() {
                if !matches!(stat, Statement::GenericFor(_) | Statement::NumericFor(_)) {
                    for local in stat.values_written() {
                        self.note_usage(local.clone(), node, stat_index);
                    }
                }
                // Reads count too: `DTWait` copied `elapsed` to a loop-body
                // temp then `return`ed it *after* the while. Declaring at
                // the write made `return v74` a global (`nil`).
                for local in stat.values_read() {
                    self.note_usage(local.clone(), node, stat_index);
                }
                match stat {
                    Statement::If(r#if) => nested.push(Nested::If {
                        stat_index,
                        then_b: r#if.then_block.clone(),
                        else_b: r#if.else_block.clone(),
                    }),
                    Statement::While(r#while) => nested.push(Nested::One {
                        stat_index,
                        child: r#while.block.clone(),
                        binders: Vec::new(),
                    }),
                    Statement::Repeat(repeat) => nested.push(Nested::One {
                        stat_index,
                        child: repeat.block.clone(),
                        binders: Vec::new(),
                    }),
                    Statement::NumericFor(numeric_for) => nested.push(Nested::One {
                        stat_index,
                        child: numeric_for.block.clone(),
                        binders: vec![numeric_for.counter.clone()],
                    }),
                    Statement::GenericFor(generic_for) => nested.push(Nested::One {
                        stat_index,
                        child: generic_for.block.clone(),
                        binders: generic_for.res_locals.clone(),
                    }),
                    _ => {}
                }
            }
        }
        for n in nested {
            match n {
                Nested::If {
                    stat_index,
                    then_b,
                    else_b,
                } => {
                    let if_node = self.graph.add_node((None, stat_index));
                    self.graph.add_edge(node, if_node, ());
                    let then_node = self.visit(then_b, stat_index);
                    self.graph.add_edge(if_node, then_node, ());
                    let else_node = self.visit(else_b, stat_index);
                    self.graph.add_edge(if_node, else_node, ());
                }
                Nested::One {
                    stat_index,
                    child,
                    binders,
                } => {
                    self.push_binders(&binders);
                    let child = self.visit(child, stat_index);
                    self.pop_binders(&binders);
                    self.graph.add_edge(node, child, ());
                }
            }
        }
        node
    }

    pub fn declare_locals(
        mut self,
        root_block: SharedBlock,
        locals_to_ignore: &FxHashSet<RcLocal>,
    ) {
        let root_node = self.visit(root_block, 0);
        let dominators = simple_fast(&self.graph, root_node);
        let dom_idx = DomIdx::build(self.graph.node_indices(), &dominators);
        for (local, usages) in self.local_usages {
            if locals_to_ignore.contains(&local) {
                continue;
            }
            let (mut node, mut first_stat_index) = if usages.len() == 1 {
                usages.into_iter().next().unwrap()
            } else {
                // LCA of usage nodes on the dominator tree. O(|uses| · depth)
                // with O(1) dominates — not O(depth²) Intersect of ancestor lists.
                let usage_nodes: Vec<NodeIndex> = usages.keys().copied().collect();
                let Some((&first, rest)) = usage_nodes.split_first() else {
                    continue;
                };
                let mut cand = first;
                let mut failed = false;
                for &n in rest {
                    while !dom_idx.dominates(cand, n) {
                        match dominators.immediate_dominator(cand) {
                            Some(p) => cand = p,
                            None => {
                                failed = true;
                                break;
                            }
                        }
                    }
                    if failed {
                        break;
                    }
                }
                if failed {
                    continue;
                }
                let common_dominator = cand;
                let mut min_stat_index = usages.get(&common_dominator).copied();
                for child in self
                    .graph
                    .neighbors_directed(common_dominator, Direction::Outgoing)
                {
                    if usage_nodes.iter().any(|&u| dom_idx.dominates(child, u)) {
                        if let Some((_, child_idx)) = self.graph.node_weight(child) {
                            min_stat_index =
                                Some(min_stat_index.map_or(*child_idx, |curr| curr.min(*child_idx)));
                        }
                    }
                }
                let Some(min_stat_index) = min_stat_index else {
                    continue;
                };
                (common_dominator, min_stat_index)
            };
            let mut parent_hops = 0u32;
            loop {
                let Some((block, _)) = self.graph.node_weight(node) else {
                    break;
                };
                if block.is_some() {
                    break;
                }
                parent_hops += 1;
                if parent_hops > 4096 {
                    break;
                }
                let Some((_, parent_stat_index)) = self.graph.node_weight(node) else {
                    break;
                };
                let parent_stat_index = *parent_stat_index;
                let Ok(parent) = self
                    .graph
                    .neighbors_directed(node, Direction::Incoming)
                    .exactly_one()
                else {
                    break;
                };
                (node, first_stat_index) = (parent, parent_stat_index);
            }
            let Some(block) = self
                .graph
                .node_weight(node)
                .and_then(|(b, _)| b.clone())
            else {
                continue;
            };
            self.declarations
                .entry(block.into())
                .or_default()
                .entry(first_stat_index)
                .or_default()
                .insert(local);
        }

        for (ByAddress(block), declarations) in self.declarations {
            let Some(mut block) = block.try_lock() else {
                continue;
            };
            for (stat_index, mut locals) in declarations.into_iter().rev() {
                match &mut block[stat_index] {
                    Statement::Assign(assign)
                        if assign
                            .left
                            .iter()
                            .all(|l| l.as_local().is_some_and(|l| locals.contains(l))) =>
                    {
                        locals.retain(|l| {
                            !assign
                                .left
                                .iter()
                                .map(|l| l.as_local().unwrap())
                                .contains(l)
                        });
                        assign.prefix = true;
                    }
                    _ => {}
                }
                if !locals.is_empty() {
                    let mut declaration =
                        Assign::new(locals.into_iter().map(|l| l.into()).collect_vec(), vec![]);
                    declaration.prefix = true;
                    block.insert(stat_index, declaration.into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Assign, Block, Call, GenericFor, Index, Literal, Local, NumericFor, RcLocal, Return,
        Statement, While, share_block,
    };
    use triomphe::Arc;

    #[test]
    fn hoist_loop_temp_used_after_while() {
        let v74 = RcLocal::new(Local::new(Some("v74".into())));
        let v75 = RcLocal::new(Local::new(Some("v75".into())));
        let inner = Block(vec![Assign::new(
            vec![v74.clone().into()],
            vec![v75.clone().into()],
        )
        .into()]);
        let body = Block(vec![
            Assign::new(
                vec![v75.clone().into()],
                vec![Literal::Number(0.0).into()],
            )
            .into(),
            Statement::While(While::new(Literal::Boolean(true).into(), inner)),
            Return::new(vec![v74.clone().into()]).into(),
        ]);
        let shared = share_block(body);
        LocalDeclarer::default().declare_locals(Arc::clone(&shared), &FxHashSet::default());
        let s = shared.lock().to_string();
        let while_idx = s.find("while").unwrap_or_else(|| panic!("{s}"));
        let local_v74 = s.find("local v74").unwrap_or_else(|| panic!("{s}"));
        assert!(
            local_v74 < while_idx,
            "v74 must be declared outside the while:\n{s}"
        );
        assert!(s.contains("return v74"), "{s}");
        let after_while = &s[while_idx..];
        assert!(
            !after_while.contains("local v74"),
            "must not redeclare v74 after/inside while:\n{s}"
        );
    }

    #[test]
    fn does_not_shadow_numeric_for_binder() {
        let i2 = RcLocal::new(Local::new(Some("i2".into())));
        let arr = RcLocal::new(Local::new(Some("arr".into())));
        let v20 = RcLocal::new(Local::new(Some("v20".into())));
        let inner = Block(vec![Assign::new(
            vec![v20.clone().into()],
            vec![Index::new(arr.into(), i2.clone().into()).into()],
        )
        .into()]);
        let body = Block(vec![Statement::NumericFor(NumericFor::new(
            Literal::Number(1.0).into(),
            Literal::Number(10.0).into(),
            Literal::Number(1.0).into(),
            i2,
            inner,
        ))]);
        let shared = share_block(body);
        LocalDeclarer::default().declare_locals(Arc::clone(&shared), &FxHashSet::default());
        let s = shared.lock().to_string();
        assert!(s.contains("for i2"), "{s}");
        assert!(
            !s.contains("local i2"),
            "numeric-for binder shadowed:\n{s}"
        );
        assert!(s.contains("local v20"), "body temp should still be local:\n{s}");
    }

    #[test]
    fn does_not_shadow_generic_for_binders() {
        let index2 = RcLocal::new(Local::new(Some("index2".into())));
        let player2 = RcLocal::new(Local::new(Some("player2".into())));
        let inner = Block(vec![Return::new(vec![player2.clone().into()]).into()]);
        let iter = Call::new(
            crate::Global::new(b"ipairs".to_vec()).into(),
            vec![crate::Table::default().into()],
        );
        let body = Block(vec![Statement::GenericFor(GenericFor::new(
            vec![index2, player2],
            vec![iter.into()],
            inner,
        ))]);
        let shared = share_block(body);
        LocalDeclarer::default().declare_locals(Arc::clone(&shared), &FxHashSet::default());
        let s = shared.lock().to_string();
        assert!(
            !s.contains("local index2") && !s.contains("local player2"),
            "generic-for binders shadowed:\n{s}"
        );
        assert!(s.contains("for index2, player2"), "{s}");
    }
}
