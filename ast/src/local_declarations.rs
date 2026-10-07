use std::collections::BTreeMap;

use array_tool::vec::Intersect;
use by_address::ByAddress;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use parking_lot::Mutex;
use petgraph::{
    algo::dominators::simple_fast,
    prelude::{DiGraph, NodeIndex},
    Direction,
};
use rustc_hash::{FxHashMap, FxHashSet};
use triomphe::Arc;

use crate::{Assign, Block, LocalRw, RcLocal, Statement};

#[derive(Default)]
pub struct LocalDeclarer {
    block_to_node: FxHashMap<ByAddress<Arc<Mutex<Block>>>, NodeIndex>,
    graph: DiGraph<(Option<Arc<Mutex<Block>>>, usize), ()>,
    local_usages: IndexMap<RcLocal, FxHashMap<NodeIndex, usize>>,
    declarations: FxHashMap<ByAddress<Arc<Mutex<Block>>>, BTreeMap<usize, IndexSet<RcLocal>>>,
}

impl LocalDeclarer {
    fn visit(&mut self, block: Arc<Mutex<Block>>, stat_index: usize) -> NodeIndex {
        let node = self.graph.add_node((Some(block.clone()), stat_index));
        self.block_to_node.insert(block.clone().into(), node);
        for (stat_index, stat) in block.lock().iter().enumerate() {
            
            if !matches!(stat, Statement::GenericFor(_) | Statement::NumericFor(_)) {
                
                
                for local in stat.values_written() {
                    self.local_usages
                        .entry(local.clone())
                        .or_default()
                        .entry(node)
                        .or_insert(stat_index);
                }
            }
            match stat {
                Statement::If(r#if) => {
                    let if_node = self.graph.add_node((None, stat_index));
                    self.graph.add_edge(node, if_node, ());
                    let then_node = self.visit(r#if.then_block.clone(), stat_index);
                    self.graph.add_edge(if_node, then_node, ());
                    let else_node = self.visit(r#if.else_block.clone(), stat_index);
                    self.graph.add_edge(if_node, else_node, ());
                }
                Statement::While(r#while) => {
                    let child = self.visit(r#while.block.clone(), stat_index);
                    self.graph.add_edge(node, child, ());
                }
                Statement::Repeat(repeat) => {
                    let child = self.visit(r#repeat.block.clone(), stat_index);
                    self.graph.add_edge(node, child, ());
                }
                Statement::NumericFor(numeric_for) => {
                    let child = self.visit(r#numeric_for.block.clone(), stat_index);
                    self.graph.add_edge(node, child, ());
                }
                Statement::GenericFor(generic_for) => {
                    let child = self.visit(r#generic_for.block.clone(), stat_index);
                    self.graph.add_edge(node, child, ());
                }
                _ => {}
            }
        }
        node
    }

    pub fn declare_locals(
        mut self,
        root_block: Arc<Mutex<Block>>,
        locals_to_ignore: &FxHashSet<RcLocal>,
    ) {
        let root_node = self.visit(root_block, 0);
        let dominators = simple_fast(&self.graph, root_node);
        for (local, usages) in self.local_usages {
            if locals_to_ignore.contains(&local) {
                continue;
            }
            let (mut node, mut first_stat_index) = if usages.len() == 1 {
                usages.into_iter().next().unwrap()
            } else {
                let node_dominators = usages
                    .keys()
                    .filter_map(|&n| dominators.dominators(n).map(|d| d.collect_vec()))
                    .collect_vec();
                let mut dom_iter = node_dominators.iter().cloned();
                let Some(mut common_dominators) = dom_iter.next() else {
                    continue;
                };
                for node_dominators in dom_iter {
                    common_dominators = common_dominators.intersect(node_dominators);
                }
                let Some(&common_dominator) = common_dominators.first() else {
                    continue;
                };
                let mut min_stat_index = usages.get(&common_dominator).copied();
                for child in self
                    .graph
                    .neighbors_directed(common_dominator, Direction::Outgoing)
                {
                    for node_dominators in &node_dominators {
                        if node_dominators.contains(&child) {
                            if let Some((_, child_idx)) = self.graph.node_weight(child) {
                                min_stat_index = Some(min_stat_index.map_or(*child_idx, |curr| curr.min(*child_idx)));
                            }
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
            let mut block = block.lock();
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
