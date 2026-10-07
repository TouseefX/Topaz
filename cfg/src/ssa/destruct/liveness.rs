use ast::{LocalRw, RcLocal};
use rustc_hash::{FxHashMap, FxHashSet};

use petgraph::stable_graph::NodeIndex;

use crate::function::Function;

/// Packed bitset. Union / difference are word-wise; membership is O(1).
///
/// `HashSet<RcLocal>` live sets were O(|live|) per block per worklist
/// iteration. ClientRenderer’s handler has thousands of SSA names and
/// thousands of blocks — that never finished. Bit tests are the O(1)
/// query that path needed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    fn with_bits(nbits: usize) -> Self {
        Self {
            words: vec![0; nbits.div_ceil(64)],
        }
    }

    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        self.words
            .get(i / 64)
            .is_some_and(|w| w & (1u64 << (i % 64)) != 0)
    }

    #[inline]
    fn insert(&mut self, i: usize) {
        let w = i / 64;
        if w >= self.words.len() {
            self.words.resize(w + 1, 0);
        }
        self.words[w] |= 1u64 << (i % 64);
    }

    /// `self |= other`. Returns whether any bit changed.
    fn union_with(&mut self, other: &Self) -> bool {
        if other.words.len() > self.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        let mut changed = false;
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            let n = *a | *b;
            changed |= n != *a;
            *a = n;
        }
        changed
    }

    /// `self |= add & !sub`
    fn or_and_not(&mut self, add: &Self, sub: &Self) {
        let n = add.words.len().max(sub.words.len()).max(self.words.len());
        self.words.resize(n, 0);
        for i in 0..n {
            let a = add.words.get(i).copied().unwrap_or(0);
            let s = sub.words.get(i).copied().unwrap_or(0);
            self.words[i] |= a & !s;
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct LiveSets {
    pub live_in: BitSet,
    pub live_out: BitSet,
}

#[derive(Default)]
pub struct LivenessResult {
    pub sets: FxHashMap<NodeIndex, LiveSets>,
    pub id_of: FxHashMap<RcLocal, usize>,
}

impl LivenessResult {
    #[inline]
    pub fn live_out_contains(&self, block: NodeIndex, local: &RcLocal) -> bool {
        let Some(&id) = self.id_of.get(local) else {
            return false;
        };
        self.sets
            .get(&block)
            .is_some_and(|s| s.live_out.contains(id))
    }

    #[inline]
    pub fn live_in_contains(&self, block: NodeIndex, local: &RcLocal) -> bool {
        let Some(&id) = self.id_of.get(local) else {
            return false;
        };
        self.sets
            .get(&block)
            .is_some_and(|s| s.live_in.contains(id))
    }
}

/// Namespace matching the previous `Liveness::calculate` call site.
pub struct Liveness;

impl Liveness {
    /// Worklist liveness. Params are live-in but do **not** propagate to
    /// predecessors’ live_out (they’re defined by the CFG edge).
    pub fn calculate(function: &Function) -> LivenessResult {
        calculate(function)
    }
}

fn intern(id_of: &mut FxHashMap<RcLocal, usize>, local: &RcLocal) -> usize {
    let next = id_of.len();
    *id_of.entry(local.clone()).or_insert(next)
}

fn calculate(function: &Function) -> LivenessResult {
    let mut id_of: FxHashMap<RcLocal, usize> = FxHashMap::default();

    for param in &function.parameters {
        intern(&mut id_of, param);
    }
    for (_, block) in function.blocks() {
        for inst in block.iter() {
            for v in inst.values_read() {
                intern(&mut id_of, v);
            }
            for v in inst.values_written() {
                intern(&mut id_of, v);
            }
        }
    }
    for node in function.graph().node_indices() {
        for (_, edge) in function.edges_to_block(node) {
            for (param, arg) in &edge.arguments {
                intern(&mut id_of, param);
                for v in arg.values_read() {
                    intern(&mut id_of, v);
                }
            }
        }
    }

    let nbits = id_of.len();
    let node_count = function.graph().node_count();
    let id = |id_of: &FxHashMap<RcLocal, usize>, l: &RcLocal| -> usize {
        *id_of.get(l).expect("local not interned")
    };

    let mut uses: FxHashMap<NodeIndex, BitSet> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut defs: FxHashMap<NodeIndex, BitSet> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut params: FxHashMap<NodeIndex, BitSet> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut result: FxHashMap<NodeIndex, LiveSets> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());

    for (node, block) in function.blocks() {
        let mut block_uses = BitSet::with_bits(nbits);
        let mut block_defs = BitSet::with_bits(nbits);
        for instruction in block.iter() {
            for v in instruction.values_read() {
                let vid = id(&id_of, v);
                if !block_defs.contains(vid) {
                    block_uses.insert(vid);
                }
            }
            for v in instruction.values_written() {
                block_defs.insert(id(&id_of, v));
            }
        }
        for (pred, edge) in function.edges_to_block(node) {
            let pred_uses = uses
                .entry(pred)
                .or_insert_with(|| BitSet::with_bits(nbits));
            for rv in edge.arguments.iter().flat_map(|(_, v)| v.values_read()) {
                pred_uses.insert(id(&id_of, rv));
            }
        }
        uses.entry(node)
            .or_insert_with(|| BitSet::with_bits(nbits))
            .union_with(&block_uses);
        defs.insert(node, block_defs);

        let mut block_params = BitSet::with_bits(nbits);
        for (_, edge) in function.edges_to_block(node) {
            for (param, _) in &edge.arguments {
                block_params.insert(id(&id_of, param));
            }
        }
        params.insert(node, block_params);
    }

    for node in function.graph().node_indices() {
        uses.entry(node)
            .or_insert_with(|| BitSet::with_bits(nbits));
        defs.entry(node)
            .or_insert_with(|| BitSet::with_bits(nbits));
        params
            .entry(node)
            .or_insert_with(|| BitSet::with_bits(nbits));
        let live_in = params[&node].clone();
        let live_out = uses[&node].clone();
        result.insert(
            node,
            LiveSets {
                live_in,
                live_out,
            },
        );
    }

    let mut worklist: Vec<NodeIndex> = function.graph().node_indices().collect();
    let mut in_worklist: FxHashSet<NodeIndex> = worklist.iter().copied().collect();

    while let Some(node) = worklist.pop() {
        in_worklist.remove(&node);

        let mut new_live_out = BitSet::with_bits(nbits);
        for succ in function.successor_blocks(node) {
            if let Some(succ_live) = result.get(&succ) {
                // live_in[s] - params[s]
                new_live_out.or_and_not(&succ_live.live_in, &params[&succ]);
            }
        }
        new_live_out.union_with(&uses[&node]);

        let mut new_live_in = params[&node].clone();
        new_live_in.union_with(&uses[&node]);
        new_live_in.or_and_not(&new_live_out, &defs[&node]);

        let old = result.get_mut(&node).unwrap();
        let changed = old.live_out != new_live_out || old.live_in != new_live_in;
        if changed {
            old.live_out = new_live_out;
            old.live_in = new_live_in;
            for pred in function.predecessor_blocks(node) {
                if in_worklist.insert(pred) {
                    worklist.push(pred);
                }
            }
        }
    }

    LivenessResult { sets: result, id_of }
}
