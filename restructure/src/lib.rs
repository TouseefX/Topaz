use cfg::{block::BranchType, compute_idoms, compute_post_idoms, function::Function, DomIndex, IDom};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use petgraph::{
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::*,
};
use tuple::Map;

mod conditional;
mod jump;
mod r#loop;

/// Post-dominators of `graph` (virtual super-sink, no mutation).
pub fn post_dominators<N, E>(graph: &StableDiGraph<N, E>) -> IDom<NodeIndex> {
    compute_post_idoms(graph)
}

struct GraphStructurer {
    pub function: Function,
    loop_headers: FxHashSet<NodeIndex>,
    label_to_node: FxHashMap<ast::Label, NodeIndex>,
    dom_idx: DomIndex,
    post_idx: DomIndex,
}

impl GraphStructurer {
    fn find_loop_headers(&mut self) {
        self.loop_headers.clear();
        let Some(entry) = *self.function.entry() else {
            return;
        };
        depth_first_search(self.function.graph(), Some(entry), |event| {
            if let DfsEvent::BackEdge(_, header) = event {
                self.loop_headers.insert(header);
            }
        });
    }
    fn new(function: Function) -> Self {
        let mut this = Self {
            function,
            loop_headers: FxHashSet::default(),
            label_to_node: FxHashMap::default(),
            dom_idx: DomIndex::default(),
            post_idx: DomIndex::default(),
        };
        this.find_loop_headers();
        this
    }

    fn block_is_no_op(block: &ast::Block) -> bool {
        !block.iter().any(|s| s.as_comment().is_none())
    }

    fn try_match_pattern(
        &mut self,
        node: NodeIndex,
        dominators: &IDom<NodeIndex>,
        post_dom: &IDom<NodeIndex>,
    ) -> bool {
        let successors = self.function.successor_blocks(node).collect_vec();

        
        if self.try_collapse_loop(node, dominators, post_dom) {
            return true;
        }

        if self.try_remove_unnecessary_condition(node) {
            return true;
        }

        let changed = match successors.len() {
            0 => false,
            1 => self.match_jump(node, Some(successors[0])),
            2 => {
                let Some((then_target, else_target)) = self
                    .function
                    .conditional_edges(node)
                    .map(|e| e.map(|x| x.target()))
                else {
                    return false;
                };
                self.match_conditional(node, then_target, else_target)
            }
            _ => false,
        };

        
        changed
    }

    fn match_blocks(
        &mut self,
        dominators: &IDom<NodeIndex>,
        post_dom: &IDom<NodeIndex>,
    ) -> bool {
        let Some(entry) = *self.function.entry() else {
            return false;
        };
        let dfs = Dfs::new(self.function.graph(), entry)
            .iter(self.function.graph())
            .collect::<FxHashSet<_>>();
        let mut dfs_postorder = DfsPostOrder::new(self.function.graph(), entry);
        let mut order = Vec::new();
        while let Some(node) = dfs_postorder.next(self.function.graph()) {
            order.push(node);
        }

        let mut changed = false;
        let mut n_checked = 0u32;

        for node in order {
            n_checked += 1;
            if n_checked & 15 == 0 && cfg::past_decompile_deadline() {
                return changed;
            }
            if !self.function.has_block(node) {
                continue;
            }
            if self.try_match_pattern(node, &dominators, &post_dom) {
                changed = true;
            }
        }

        for node in self
            .function
            .graph()
            .node_indices()
            .filter(|node| !dfs.contains(node))
            .collect_vec()
        {
            if self.function.has_block(node)
                && self.function.predecessor_blocks(node).next().is_none()
            {
                if self
                    .function
                    .block(node)
                    .unwrap()
                    .first()
                    .and_then(|s| s.as_label())
                    .is_none()
                {
                    self.function.remove_block(node);
                } else {
                    let matched = self.try_match_pattern(node, &dominators, &post_dom);
                    changed |= matched;
                }
            }
        }

        changed
    }

    fn insert_goto_for_edge(&mut self, edge: EdgeIndex) {
        let (source, target) = self.function.graph().edge_endpoints(edge).unwrap();
        if self.function.graph().edge_weight(edge).unwrap().branch_type == BranchType::Unconditional
            && self.function.predecessor_blocks(target).count() == 1
            && self.function.successor_blocks(source).count() == 1
        {
            
            let edges = self.function.remove_edges(target);
            let block = self.function.remove_block(target).unwrap();
            self.function.block_mut(source).unwrap().extend(block.0);
            self.function.set_edges(source, edges);
        } else {
            
            let label = ast::Label(format!("l{}", target.index()));
            let target_block = self.function.block_mut(target).unwrap();
            if target_block.first().and_then(|s| s.as_label()).is_none() {
                self.label_to_node.insert(label.clone(), target);
                target_block.insert(0, label.clone().into());
            }
            let goto_block = self.function.new_block();
            self.function
                .block_mut(goto_block)
                .unwrap()
                .push(ast::Goto::new(label).into());

            let edge = self.function.graph_mut().remove_edge(edge).unwrap();
            self.function.graph_mut().add_edge(source, goto_block, edge);
        }
    }

    fn remove_last_return(block: ast::Block) -> ast::Block {
        if let Some(ast::Statement::Return(last_statement)) = block.last() {
            if last_statement.values.is_empty() {
                let take = block.len() - 1;
                return block.0.into_iter().take(take).collect_vec().into();
            }
        }
        block
    }

    fn collapse(&mut self) {
        let n = self.function.graph().node_count();
        if n <= 1 {
            return;
        }
        // LunaUX does a 60k-line dump in ~20s. 24×12 CHK dominance runs
        // on a 15k-node CFG is why we still looked hung: that's hundreds
        // of full dominator solves. Large graphs: one match per outer
        // with fresh dominators, then gotos. Small graphs keep the old
        // inner recompute (cheap when n is tiny).
        // Nested diamonds collapse one layer per outer. Sequential diamonds
        // all collapse in one match_blocks pass. Huge CFGs: fewer CHK
        // solves, more gotos, still full SSA before we get here.
        let large = n > 2000;
        let (outer_cap, inner_cap, insert_cap) = if n > 8000 {
            (4u32, 1u32, 1024u32)
        } else if large {
            (6u32, 1u32, 512u32)
        } else if n > 400 {
            (16u32, 6u32, 96u32)
        } else {
            (24u32, 12u32, 64u32)
        };

        let mut guard = 0u32;
        loop {
            if cfg::past_decompile_deadline() {
                break;
            }
            guard += 1;
            if guard > outer_cap {
                break;
            }
            self.find_loop_headers();
            let Some(entry) = *self.function.entry() else {
                break;
            };
            let mut dominators = compute_idoms(self.function.graph(), entry);
            let mut post_dom = if self.loop_headers.is_empty() {
                IDom::dummy(entry)
            } else {
                post_dominators(self.function.graph())
            };
            self.dom_idx = DomIndex::build(self.function.graph().node_indices(), &dominators);
            self.post_idx = DomIndex::build(self.function.graph().node_indices(), &post_dom);

            let mut inner = 0u32;
            let mut graph_changed = false;
            while inner < inner_cap {
                if inner > 0 && !large {
                    dominators = compute_idoms(self.function.graph(), entry);
                    post_dom = if self.loop_headers.is_empty() {
                        IDom::dummy(entry)
                    } else {
                        post_dominators(self.function.graph())
                    };
                    self.dom_idx =
                        DomIndex::build(self.function.graph().node_indices(), &dominators);
                    self.post_idx =
                        DomIndex::build(self.function.graph().node_indices(), &post_dom);
                }
                if !self.match_blocks(&dominators, &post_dom) {
                    break;
                }
                graph_changed = true;
                inner += 1;
            }
            if self.function.graph().node_count() <= 1 {
                break;
            }

            let Some(entry) = *self.function.entry() else {
                break;
            };
            // Matching rewrote the CFG — recompute. Otherwise the first
            // solve is still valid and a second HashMap CHK used to cost
            // as much as the whole decompile.
            let dominators = if graph_changed {
                compute_idoms(self.function.graph(), entry)
            } else {
                dominators
            };
            let dom_idx = DomIndex::build(self.function.graph().node_indices(), &dominators);
            let edges = self.function.graph().edge_indices().collect::<Vec<_>>();

            // Insert many gotos in one pass. Matching after every edge was
            // O(E × dominators) and never returned on large CFGs.
            let mut inserted = 0u32;
            for &edge in &edges {
                if inserted >= insert_cap {
                    break;
                }
                if self.function.graph().edge_weight(edge).is_none() {
                    continue;
                }
                let (source, target) = self.function.graph().edge_endpoints(edge).unwrap();
                if dom_idx.dominates(source, target) || dom_idx.dominates(target, source) {
                    continue;
                }
                self.insert_goto_for_edge(edge);
                inserted += 1;
            }
            if inserted == 0 {
                if let Some(&edge) = edges
                    .iter()
                    .find(|e| self.function.graph().edge_weight(**e).is_some())
                {
                    self.insert_goto_for_edge(edge);
                } else {
                    break;
                }
            }
            self.find_loop_headers();
        }
    }

    fn structure(mut self) -> ast::Block {
        self.collapse();
        if self.function.graph().node_count() != 1 {
            let mut res_block = ast::Block::default();
            let Some(entry) = *self.function.entry() else {
                return res_block;
            };
            let mut stack = vec![entry];
            let mut visited = FxHashSet::default();
            while let Some(node) = stack.pop() {
                if visited.contains(&node) {
                    continue;
                }
                visited.insert(node);

                fn collect_gotos(block: &ast::Block, gotos: &mut FxHashSet<ast::Label>) {
                    for statement in &block.0 {
                        match statement {
                            ast::Statement::Goto(goto) => {
                                gotos.insert(goto.0.clone());
                            }
                            ast::Statement::If(r#if) => {
                                collect_gotos(&r#if.then_block.lock(), gotos);
                                collect_gotos(&r#if.else_block.lock(), gotos);
                            }
                            ast::Statement::While(r#while) => {
                                collect_gotos(&r#while.block.lock(), gotos);
                            }
                            ast::Statement::Repeat(repeat) => {
                                collect_gotos(&repeat.block.lock(), gotos);
                            }
                            ast::Statement::NumericFor(numeric_for) => {
                                collect_gotos(&numeric_for.block.lock(), gotos);
                            }
                            ast::Statement::GenericFor(generic_for) => {
                                collect_gotos(&generic_for.block.lock(), gotos);
                            }
                            _ => {}
                        }
                    }
                }

                let block = self.function.remove_block(node).unwrap();
                let mut goto_destinations = FxHashSet::default();
                collect_gotos(&block, &mut goto_destinations);
                for label in goto_destinations {
                    if let Some(&target_node) = self.label_to_node.get(&label) {
                        if self.function.has_block(target_node) {
                            stack.push(target_node);
                        }
                    }
                }
                if let Some(ast::Statement::Goto(goto)) = res_block.last()
                
                    && goto.0.0[1..] == node.index().to_string()
                {
                    res_block.pop();
                }
                if !block
                    .first()
                    .is_some_and(|s| matches!(s, ast::Statement::Label(_)))
                {
                    res_block.push(ast::Comment::new(format!("block {}", node.index())).into());
                }
                res_block.extend(block.0)
            }
            
            for node in self.function.graph().node_indices().collect::<Vec<_>>() {
                let block = self.function.remove_block(node).unwrap();
                if !block
                    .first()
                    .is_some_and(|s| matches!(s, ast::Statement::Label(_)))
                {
                    res_block.push(ast::Comment::new(format!("block {}", node.index())).into());
                }
                res_block.extend(block.0)
            }

            res_block
        } else if let Some(entry) = *self.function.entry() {
            Self::remove_last_return(
                self.function.remove_block(entry).unwrap_or_default(),
            )
        } else {
            ast::Block::default()
        }
    }
}

pub fn lift(function: cfg::function::Function) -> ast::Block {
    GraphStructurer::new(function).structure()
}
