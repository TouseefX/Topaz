

use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

use crate::block::BranchType;
use crate::function::Function;

#[derive(Debug, Clone)]
pub struct CfgSnapshot {
    pub name: String,
    pub entry: Option<usize>,
    pub nodes: Vec<CfgNode>,
    pub edges: Vec<CfgEdge>,
}

#[derive(Debug, Clone)]
pub struct CfgNode {
    pub id: usize,
    pub label: String,
    pub is_entry: bool,
    pub statement_count: usize,
}

#[derive(Debug, Clone)]
pub struct CfgEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    Unconditional,
    Then,
    Else,
}

impl From<BranchType> for EdgeKind {
    fn from(b: BranchType) -> Self {
        match b {
            BranchType::Unconditional => EdgeKind::Unconditional,
            BranchType::Then => EdgeKind::Then,
            BranchType::Else => EdgeKind::Else,
        }
    }
}

impl CfgSnapshot {
    pub fn from_function(function: &Function, name: impl Into<String>) -> Self {
        let graph = function.graph();
        let entry_idx = function.entry().map(|i| i.index());

        let mut nodes = Vec::with_capacity(graph.node_count());
        for (idx, block) in function.blocks() {
            let label = block.iter().map(|s| s.to_string()).join("\n");
            nodes.push(CfgNode {
                id: idx.index(),
                label,
                is_entry: Some(idx.index()) == entry_idx,
                statement_count: block.len(),
            });
        }

        let mut edges = Vec::with_capacity(graph.edge_count());
        for edge in graph.edge_references() {
            edges.push(CfgEdge {
                from: edge.source().index(),
                to: edge.target().index(),
                kind: edge.weight().branch_type.clone().into(),
            });
        }

        Self {
            name: name.into(),
            entry: entry_idx,
            nodes,
            edges,
        }
    }
}
