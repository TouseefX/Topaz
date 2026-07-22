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
    /// Edge params per block — these are live-in but must NOT propagate
    /// to predecessors' live_out (they're defined by the edge argument).
    params: FxHashMap<NodeIndex, FxHashSet<RcLocal>>,
    result: FxHashMap<NodeIndex, LiveSets>,
}

impl<'a> Liveness<'a> {
    /// Compute liveness using a worklist algorithm.
    /// Preserves the original semantics: edge parameters are live-in at their
    /// block but do NOT propagate backwards to predecessors' live_out.
    pub fn calculate(function: &'a Function) -> FxHashMap<NodeIndex, LiveSets> {
        let node_count = function.graph().node_count();
        let mut liveness = Liveness {
            function,
            uses: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
            defs: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
            params: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
            result: FxHashMap::with_capacity_and_hasher(node_count, Default::default()),
        };

        // Collect uses, defs, and params per block
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

            // Collect params
            let mut params = FxHashSet::default();
            for (_, edge) in function.edges_to_block(node) {
                for (param, _) in &edge.arguments {
                    params.insert(param.clone());
                }
            }
            liveness.params.insert(node, params);
        }

        // Initialize: compute initial live_in = params (seeded at entry)
        // and live_out = uses (initial guess)
        for node in function.graph().node_indices() {
            let params = liveness.params.get(&node)
                .map(|p| p.iter().cloned().collect::<FxHashSet<_>>())
                .unwrap_or_default();
            let uses = liveness.uses.get(&node)
                .map(|u| u.iter().cloned().collect::<FxHashSet<_>>())
                .unwrap_or_default();

            // live_out starts as uses (optimistic — everything used is live out)
            // This is the standard initialization for backward dataflow
            let live_sets = LiveSets {
                live_in: params,
                live_out: uses,
            };
            liveness.result.insert(node, live_sets);
        }

        // Worklist: start with all blocks
        let mut worklist: Vec<NodeIndex> = function.graph().node_indices().collect();
        let mut in_worklist: FxHashSet<NodeIndex> =
            worklist.iter().cloned().collect();

        while let Some(node) = worklist.pop() {
            in_worklist.remove(&node);

            // live_out[n] = union over successors s of:
            //   (live_in[s] - params[s])  ← only non-param live-ins propagate back
            let mut new_live_out: FxHashSet<RcLocal> = FxHashSet::default();
            for succ in function.successor_blocks(node) {
                if let Some(succ_live) = liveness.result.get(&succ) {
                    let succ_params = liveness.params.get(&succ);
                    for v in &succ_live.live_in {
                        // Skip params — they don't propagate to predecessors
                        if !succ_params.map_or(false, |p| p.contains(v)) {
                            new_live_out.insert(v.clone());
                        }
                    }
                }
            }

            // Plus uses of this block itself (they're live-out by definition)
            if let Some(u) = liveness.uses.get(&node) {
                new_live_out.extend(u.iter().cloned());
            }

            // live_in = params ∪ uses ∪ (live_out - defs)
            let defs = liveness.defs.get(&node);
            let uses = liveness.uses.get(&node);
            let params = liveness.params.get(&node);

            let mut new_live_in: FxHashSet<RcLocal> = FxHashSet::default();

            // Add params (always live-in)
            if let Some(p) = params {
                new_live_in.extend(p.iter().cloned());
            }
            // Add uses (always live-in)
            if let Some(u) = uses {
                new_live_in.extend(u.iter().cloned());
            }
            // Add live_out - defs
            for v in &new_live_out {
                if !defs.map_or(false, |d| d.contains(v)) {
                    new_live_in.insert(v.clone());
                }
            }

            let old = liveness.result.get_mut(&node).unwrap();
            let changed = old.live_out != new_live_out || old.live_in != new_live_in;

            if changed {
                old.live_out = new_live_out;
                old.live_in = new_live_in;
                // Add predecessors to worklist
                for pred in liveness.function.predecessor_blocks(node) {
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
