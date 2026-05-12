use petgraph::stable_graph::NodeIndex;
use rangemap::RangeInclusiveMap;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::function::Function;

#[derive(Debug)]
pub(crate) struct UpvaluesOpen {
    pub open: FxHashMap<
        NodeIndex,
        FxHashMap<ast::RcLocal, RangeInclusiveMap<usize, Vec<(NodeIndex, usize)>>>,
    >,
    old_locals: FxHashMap<ast::RcLocal, ast::RcLocal>,
}

impl UpvaluesOpen {
    pub fn new(function: &Function, old_locals: FxHashMap<ast::RcLocal, ast::RcLocal>) -> Self {
        let mut this = Self {
            open: Default::default(),
            old_locals,
        };
        let entry = function.entry().unwrap();
        let mut stack = vec![entry];
        let mut visited = FxHashSet::default();
        while let Some(node) = stack.pop() {
            visited.insert(node);
            let block = function.block(node).unwrap();
            let block_opened = this.open.entry(node).or_default();
            for (stat_index, statement) in block.iter().enumerate() {
                
                
                if let ast::Statement::Assign(assign) = statement {
                    for opened in assign
                        .right
                        .iter()
                        .filter_map(|r| r.as_closure())
                        .flat_map(|c| c.upvalues.iter())
                        .filter_map(|u| match u {
                            ast::Upvalue::Copy(_) => None,
                            ast::Upvalue::Ref(l) => Some(l),
                        })
                        .map(|l| this.old_locals[l].clone())
                    {
                        let open_ranges = block_opened.entry(opened).or_default();
                        let mut open_locations = Vec::new();
                        if let Some((_prev_range, prev_locations)) =
                            open_ranges.get_key_value(&stat_index)
                        {
                            
                            
                            open_locations.extend(prev_locations);
                        }
                        open_locations.push((node, stat_index));
                        open_ranges.insert(stat_index..=block.len() - 1, open_locations);
                    }
                } else if let ast::Statement::Close(close) = statement {
                    for closed in &close.locals {
                        if let Some(open_ranges) = block_opened.get_mut(closed) {
                            open_ranges.remove(stat_index..=block.len() - 1);
                        }
                    }
                }
            }
            for successor in function.successor_blocks(node) {
                
                
                if !visited.contains(&successor) {
                    let successor_block = function.block(successor).unwrap();
                    let open_at_end = this.open[&node]
                        .iter()
                        .filter_map(|(l, m)| {
                            Some((l.clone(), m.get(&(block.len().saturating_sub(1)))?.clone()))
                        })
                        .collect::<Vec<_>>();
                    let successor_open = this.open.entry(successor).or_default();
                    for (open, mut locations) in open_at_end {
                        let open_ranges = successor_open.entry(open).or_default();
                        
                        let range = 0..=successor_block.len().saturating_sub(1);
                        if let Some((prev_range, prev_locations)) = open_ranges.get_key_value(&0) {
                            assert_eq!(prev_range, &range);
                            locations.extend(prev_locations);
                        }
                        open_ranges.insert(range, locations);
                    }

                    stack.push(successor);
                }
            }
        }
        this
    }
}
