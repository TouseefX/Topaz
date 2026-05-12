use ast::{LocalRw, RcLocal};
use indexmap::IndexSet;
use petgraph::{
    prelude::DiGraph,
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, Walker},
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::function::Function;


#[derive(Debug)]
pub struct ParamDependencyGraph {
    
    pub graph: DiGraph<RcLocal, ()>,
    pub local_to_node: FxHashMap<RcLocal, NodeIndex>,
}

impl ParamDependencyGraph {
    pub fn new(function: &mut Function, node: NodeIndex) -> Self {
        let mut this = Self {
            graph: DiGraph::new(),
            local_to_node: FxHashMap::default(),
        };

        let edges = function.edges_to_block(node);
        for (edge_index, edge) in edges.enumerate() {
            if edge_index == 0 {
                for (param, _) in &edge.1.arguments {
                    this.add_node(param.clone());
                }
            }
            
            for (param, arg) in &edge.1.arguments {
                for read in arg.values_read() {
                    if let Some(&defining_param_node) = this.local_to_node.get(read) {
                        let param_node = this.local_to_node[param];
                        if param_node != defining_param_node {
                            this.graph.add_edge(param_node, defining_param_node, ());
                        }
                    }
                }
            }
        }

        this
    }

    pub fn remove_node(&mut self, node: NodeIndex) -> Option<RcLocal> {
        if let Some(local) = self.graph.remove_node(node) {
            self.local_to_node.remove(&local);
            Some(local)
        } else {
            None
        }
    }

    pub fn add_node(&mut self, local: RcLocal) -> NodeIndex {
        let node = self.graph.add_node(local.clone());
        self.local_to_node.insert(local, node);
        node
    }

    
    pub fn compute_directed_fvs(&self) -> IndexSet<NodeIndex> {
        let mut directed_fvs = IndexSet::new();
        let mut dfs_post_order = DfsPostOrder::empty(&self.graph);
        dfs_post_order.stack.extend(self.graph.node_indices());
        let mut topological_order = dfs_post_order.iter(&self.graph).collect::<Vec<_>>();
        topological_order.reverse();

        let mut smaller_order = FxHashSet::default();
        for node in topological_order {
            if self
                .graph
                .neighbors(node)
                .any(|n| smaller_order.contains(&n))
            {
                directed_fvs.insert(node);
            } else {
                smaller_order.insert(node);
            }
        }

        directed_fvs
    }
}

