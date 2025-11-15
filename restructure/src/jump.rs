use ast::SideEffects;
use cfg::block::{ BlockEdge, BranchType };
use itertools::Itertools;
use petgraph::{
    algo::dominators::Dominators,
    stable_graph::NodeIndex,
    visit::{ EdgeRef, IntoEdgeReferences },
    Direction,
};

impl super::GraphStructurer {
    pub(crate) fn simplify_redundant_condition(&mut self, node: NodeIndex) -> bool {
        let block = match self.function.block(node) {
            Some(b) if !b.is_empty() => b,
            _ => {
                return false;
            }
        };

        let _if_stmt = match block.last().and_then(|stmt| stmt.as_if()) {
            Some(stmt) => stmt,
            None => {
                return false;
            }
        };

        let (then_edge, else_edge) = match self.function.conditional_edges(node) {
            Some(edges) if edges.0.target() == edges.1.target() => edges,
            _ => {
                return false;
            }
        };

        let target = then_edge.target();
        let cond = match self.function.block_mut(node) {
            Some(b) =>
                b.pop().and_then(|stmt|
                    stmt
                        .into_if()
                        .ok()
                        .map(|if_stmt| if_stmt.condition)
                ),
            None => {
                return false;
            }
        };

        let new_stat = match cond {
            Some(ast::RValue::Call(call)) => Some(ast::Statement::from(call)),
            Some(ast::RValue::MethodCall(method_call)) => Some(ast::Statement::from(method_call)),
            Some(c) if c.has_side_effects() =>
                Some(
                    (ast::Assign {
                        left: vec![ast::RcLocal::default().into()],
                        right: vec![c],
                        prefix: true,
                        parallel: false,
                    }).into()
                ),
            _ => None,
        };

        if let Some(stat) = new_stat {
            self.function.block_mut(node).unwrap().extend(vec![stat]);
        }

        self.function.set_edges(node, vec![(target, BlockEdge::new(BranchType::Unconditional))]);

        true
    }

    pub(crate) fn try_remove_unnecessary_condition(&mut self, node: NodeIndex) -> bool {
        self.simplify_redundant_condition(node)
    }

    pub(crate) fn match_jump(&mut self, node: NodeIndex, target: Option<NodeIndex>) -> bool {
        if let Some(target) = target {
            if node == target || self.is_for_next(node) {
                return false;
            }

            if
                Self::block_is_no_op(self.function.block(node).unwrap()) &&
                self.function.entry() != &Some(node) &&
                !self.is_loop_header(node)
            {
                return self.redirect_incoming_edges(node, target);
            }

            if self.can_merge_into_target(node, target) {
                return self.merge_into_target(node, target);
            }

            false
        } else {
            self.try_remove_terminating_no_op(node)
        }
    }

    fn redirect_incoming_edges(&mut self, node: NodeIndex, target: NodeIndex) -> bool {
        for (source, edge_id) in self.function
            .graph()
            .edges_directed(node, Direction::Incoming)
            .map(|e| (e.source(), e.id()))
            .collect::<Vec<_>>() {
            let edge = self.function.graph_mut().remove_edge(edge_id).unwrap();
            self.function.graph_mut().add_edge(source, target, edge);
            self.try_remove_unnecessary_condition(source);
        }
        self.function.remove_block(node);
        true
    }

    fn can_merge_into_target(&self, node: NodeIndex, target: NodeIndex) -> bool {
        self.function.predecessor_blocks(target).count() == 1 &&
            !self.function.edges_to_block(node).any(|(t, _)| t == target) &&
            !self.function.edges_to_block(target).any(|(t, _)| t == target) &&
            self.function.entry() != &Some(target) &&
            !self.is_loop_header(target) &&
            !self.is_for_next(target)
    }

    fn merge_into_target(&mut self, node: NodeIndex, target: NodeIndex) -> bool {
        let edges = self.function.remove_edges(target);
        let block = self.function.remove_block(target).unwrap();
        self.function.block_mut(node).unwrap().extend(block.0);
        self.function.set_edges(node, edges);
        true
    }

    fn try_remove_terminating_no_op(&mut self, node: NodeIndex) -> bool {
        if
            !Self::block_is_no_op(self.function.block(node).unwrap()) ||
            self.function.entry() == &Some(node) ||
            self.is_loop_header(node) ||
            self.is_for_next(node)
        {
            return false;
        }

        for pred in self.function.predecessor_blocks(node).collect_vec() {
            if self.function.successor_blocks(pred).collect_vec().len() != 1 {
                return false;
            }
        }

        for edge_id in self.function
            .graph()
            .edges_directed(node, Direction::Incoming)
            .map(|e| e.id())
            .collect::<Vec<_>>() {
            assert_eq!(
                self.function.graph_mut().remove_edge(edge_id).unwrap().branch_type,
                BranchType::Unconditional
            );
        }

        self.function.remove_block(node);
        true
    }
}
