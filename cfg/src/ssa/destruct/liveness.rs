use ast::{LocalRw, RcLocal};
use rustc_hash::{FxHashMap, FxHashSet};

use petgraph::stable_graph::NodeIndex;

use crate::function::Function;

#[derive(Debug, Default)]
pub struct LiveSets {
    pub live_in: FxHashSet<RcLocal>,
    pub live_out: FxHashSet<RcLocal>,
}

#[derive(Debug)]
pub struct Liveness<'a> {
    function: &'a Function,
    uses: FxHashMap<NodeIndex, FxHashSet<RcLocal>>,
    defs: FxHashMap<NodeIndex, FxHashSet<RcLocal>>,
    result: FxHashMap<NodeIndex, LiveSets>,
}

impl<'a> Liveness<'a> {
    /// Compute liveness using a worklist algorithm.
    /// This is O(n * v) in practice rather than the previous per-variable
    /// DFS which could be exponential on pathological CFGs.
    pub fn calculate(function: &'a Function) -> FxHashMap<NodeIndex, LiveSets> {
        let node_count = function.graph().node_count();
        let mut liveness = Liveness {
            function,
            uses: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
            defs: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
            result: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
        };

        // Collect uses and defs per block
        for (node, block) in function.blocks() {
            let mut uses = FxHashSet::default();
            let mut defs = FxHashSet::default();
            for instruction in block.iter() {
                for v in instruction.values_read() {
                    if !defs.contains(v) {
                        uses.insert(v.clone());
                    }
                }
                for v in instruction.values_written() {
                    defs.insert(v.clone());
                }
            }
            // Edge arguments count as uses in the predecessor
            for (pred, edge) in function.edges_to_block(node) {
                let pred_uses = liveness.uses.entry(pred).or_default();
                for rv in edge.arguments.iter().flat_map(|(_, v)| v.values_read()) {
                    pred_uses.insert(rv.clone());
                }
            }
            liveness.uses.insert(node, uses);
            liveness.defs.insert(node, defs);
        }

        // Initialize live_out for all blocks
        // and seed params (params are live-in at their block unless def'd before use)
        for node in function.graph().node_indices() {
            let mut live_sets = LiveSets::default();
            // Edge params are live at entry
            for (_, edge) in function.edges_to_block(node) {
                for (param, _) in &edge.arguments {
                    live_sets.live_in.insert(param.clone());
                }
            }
            liveness.result.insert(node, live_sets);
        }

        // Worklist: start with all blocks
        let mut worklist: Vec<NodeIndex> = function.graph().node_indices().collect();
        let mut in_worklist: FxHashSet<NodeIndex> =
            worklist.iter().cloned().collect();

        while let Some(node) = worklist.pop() {
            in_worklist.remove(&node);

            // live_out[n] = union of live_in of all successors
            let mut new_live_out: FxHashSet<RcLocal> = FxHashSet::default();
            for succ in function.successor_blocks(node) {
                if let Some(succ_live) = liveness.result.get(&succ) {
                    new_live_out.extend(succ_live.live_in.iter().cloned());
                }
            }

            // Also account for edge arguments as live-out uses
            if let Some(u) = liveness.uses.get(&node) {
                new_live_out.extend(u.iter().cloned());
            }

            // Standard dataflow: live_in = uses ∪ (live_out - defs)
            let defs = liveness.defs.get(&node);
            let uses = liveness.uses.get(&node);

            let mut new_live_in: FxHashSet<RcLocal> =
                FxHashSet::with_capacity_and_hasher(
                    uses.map_or(0, |u| u.len()) + new_live_out.len(),
                    Default::default(),
                );
            if let Some(uses) = uses {
                new_live_in.extend(uses.iter().cloned());
            }
            for v in &new_live_out {
                if !defs.map_or(false, |d| d.contains(v)) {
                    new_live_in.insert(v.clone());
                }
            }

            // Keep edge params
            for (_, edge) in function.edges_to_block(node) {
                for (param, _) in &edge.arguments {
                    new_live_in.insert(param.clone());
                }
            }

            let old = liveness.result.get_mut(&node).unwrap();
            let changed = old.live_out != new_live_out || old.live_in != new_live_in;

            if changed {
                old.live_out = new_live_out;
                old.live_in = new_live_in;
                // Add predecessors to worklist
                for pred in function.predecessor_blocks(node) {
                    if !in_worklist.contains(&pred) {
                        in_worklist.insert(pred);
                        worklist.push(pred);
                    }
                }
            }
        }

        liveness.result
    }
}
