use ast::{LocalRw, RcLocal};
use rustc_hash::{FxHashMap, FxHashSet};

use petgraph::{stable_graph::NodeIndex, visit::DfsPostOrder};

use crate::function::Function;

/// Dense word-vector when the local universe is small; sorted id list when
/// SSA invented tens of thousands of names (phi at every join). A dense
/// million-bit set × 15k blocks is gigabytes and never finishes. Sparse
/// union is O(|live|), which stays in the hundreds even on ClientRenderer.
const DENSE_LIMIT: usize = 16_384;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum LiveBits {
    #[default]
    Empty,
    Dense(Vec<u64>),
    Sparse(Vec<u32>),
}

impl LiveBits {
    fn new(nbits: usize) -> Self {
        if nbits <= DENSE_LIMIT {
            Self::Dense(vec![0; nbits.div_ceil(64)])
        } else {
            Self::Sparse(Vec::new())
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Empty => {}
            Self::Dense(w) => w.fill(0),
            Self::Sparse(v) => v.clear(),
        }
    }

    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        match self {
            Self::Empty => false,
            Self::Dense(w) => w
                .get(i / 64)
                .is_some_and(|word| word & (1u64 << (i % 64)) != 0),
            Self::Sparse(v) => v.binary_search(&(i as u32)).is_ok(),
        }
    }

    #[inline]
    fn insert(&mut self, i: usize) {
        match self {
            Self::Empty => {
                *self = Self::Sparse(vec![i as u32]);
            }
            Self::Dense(w) => {
                let word = i / 64;
                if word >= w.len() {
                    w.resize(word + 1, 0);
                }
                w[word] |= 1u64 << (i % 64);
            }
            Self::Sparse(v) => {
                let id = i as u32;
                if let Err(pos) = v.binary_search(&id) {
                    v.insert(pos, id);
                }
            }
        }
    }

    fn union_with(&mut self, other: &Self) -> bool {
        match (&mut *self, other) {
            (Self::Dense(a), Self::Dense(b)) => {
                if b.len() > a.len() {
                    a.resize(b.len(), 0);
                }
                let mut changed = false;
                for (x, y) in a.iter_mut().zip(b.iter()) {
                    let n = *x | *y;
                    changed |= n != *x;
                    *x = n;
                }
                changed
            }
            (Self::Sparse(a), Self::Sparse(b)) => {
                if b.is_empty() {
                    return false;
                }
                if a.is_empty() {
                    *a = b.clone();
                    return !b.is_empty();
                }
                let mut out = Vec::with_capacity(a.len() + b.len());
                let mut i = 0;
                let mut j = 0;
                let mut changed = false;
                while i < a.len() && j < b.len() {
                    match a[i].cmp(&b[j]) {
                        std::cmp::Ordering::Less => {
                            out.push(a[i]);
                            i += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            out.push(b[j]);
                            j += 1;
                            changed = true;
                        }
                        std::cmp::Ordering::Equal => {
                            out.push(a[i]);
                            i += 1;
                            j += 1;
                        }
                    }
                }
                if j < b.len() {
                    changed = true;
                    out.extend_from_slice(&b[j..]);
                }
                if i < a.len() {
                    out.extend_from_slice(&a[i..]);
                }
                let changed = changed || out.len() != a.len();
                *a = out;
                changed
            }
            (me, other) => {
                // Mixed / Empty: fall back to inserting.
                let mut changed = false;
                match other {
                    Self::Empty => {}
                    Self::Dense(w) => {
                        for (wi, word) in w.iter().enumerate() {
                            let mut bits = *word;
                            let base = wi * 64;
                            while bits != 0 {
                                let b = bits.trailing_zeros() as usize;
                                bits &= bits - 1;
                                if !me.contains(base + b) {
                                    me.insert(base + b);
                                    changed = true;
                                }
                            }
                        }
                    }
                    Self::Sparse(v) => {
                        for &id in v {
                            if !me.contains(id as usize) {
                                me.insert(id as usize);
                                changed = true;
                            }
                        }
                    }
                }
                changed
            }
        }
    }

    /// `self |= add & !sub`
    fn or_and_not(&mut self, add: &Self, sub: &Self) {
        match (add, sub) {
            (Self::Dense(a), Self::Dense(s)) => {
                if let Self::Dense(dst) = self {
                    let n = a.len().max(s.len()).max(dst.len());
                    dst.resize(n, 0);
                    for i in 0..n {
                        let av = a.get(i).copied().unwrap_or(0);
                        let sv = s.get(i).copied().unwrap_or(0);
                        dst[i] |= av & !sv;
                    }
                    return;
                }
            }
            (Self::Sparse(a), Self::Sparse(s)) => {
                for &id in a {
                    if s.binary_search(&id).is_err() {
                        self.insert(id as usize);
                    }
                }
                return;
            }
            _ => {}
        }
        // Mixed: walk add's members.
        match add {
            Self::Empty => {}
            Self::Dense(w) => {
                for (wi, word) in w.iter().enumerate() {
                    let mut bits = *word;
                    let base = wi * 64;
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        let i = base + b;
                        if !sub.contains(i) {
                            self.insert(i);
                        }
                    }
                }
            }
            Self::Sparse(v) => {
                for &id in v {
                    if !sub.contains(id as usize) {
                        self.insert(id as usize);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct LiveSets {
    pub live_in: LiveBits,
    pub live_out: LiveBits,
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

pub struct Liveness;

impl Liveness {
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

    let mut uses: FxHashMap<NodeIndex, LiveBits> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut defs: FxHashMap<NodeIndex, LiveBits> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut params: FxHashMap<NodeIndex, LiveBits> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());
    let mut result: FxHashMap<NodeIndex, LiveSets> =
        FxHashMap::with_capacity_and_hasher(node_count, Default::default());

    for (node, block) in function.blocks() {
        let mut block_uses = LiveBits::new(nbits);
        let mut block_defs = LiveBits::new(nbits);
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
            let pred_uses = uses.entry(pred).or_insert_with(|| LiveBits::new(nbits));
            for rv in edge.arguments.iter().flat_map(|(_, v)| v.values_read()) {
                pred_uses.insert(id(&id_of, rv));
            }
        }
        uses.entry(node)
            .or_insert_with(|| LiveBits::new(nbits))
            .union_with(&block_uses);
        defs.insert(node, block_defs);

        let mut block_params = LiveBits::new(nbits);
        for (_, edge) in function.edges_to_block(node) {
            for (param, _) in &edge.arguments {
                block_params.insert(id(&id_of, param));
            }
        }
        params.insert(node, block_params);
    }

    for node in function.graph().node_indices() {
        uses.entry(node)
            .or_insert_with(|| LiveBits::new(nbits));
        defs.entry(node)
            .or_insert_with(|| LiveBits::new(nbits));
        params
            .entry(node)
            .or_insert_with(|| LiveBits::new(nbits));
        result.insert(
            node,
            LiveSets {
                live_in: params[&node].clone(),
                live_out: uses[&node].clone(),
            },
        );
    }

    // Backward dataflow in CFG postorder: successors are processed first,
    // so a reducible graph converges in 1–3 passes instead of O(n) worklist
    // waves (each wave walking every block).
    let mut order = Vec::with_capacity(node_count);
    let mut seen: FxHashSet<NodeIndex> =
        FxHashSet::with_capacity_and_hasher(node_count, Default::default());
    if let Some(entry) = *function.entry() {
        let mut dfs = DfsPostOrder::new(function.graph(), entry);
        while let Some(n) = dfs.next(function.graph()) {
            order.push(n);
            seen.insert(n);
        }
    }
    for n in function.graph().node_indices() {
        if seen.insert(n) {
            order.push(n);
        }
    }

    // Reuse two scratch sets. Allocating a fresh Dense/Sparse vector per
    // node per iteration was gigabytes of churn on 20k-block CFGs.
    let mut scratch_out = LiveBits::new(nbits);
    let mut scratch_in = LiveBits::new(nbits);
    let mut changed = true;
    let mut iters = 0u32;
    let max_iters = if node_count > 2000 { 8 } else { 32 };
    while changed {
        changed = false;
        iters += 1;
        if iters > max_iters || crate::past_decompile_deadline() {
            break;
        }
        for &node in &order {
            scratch_out.clear();
            for succ in function.successor_blocks(node) {
                if let Some(succ_live) = result.get(&succ) {
                    scratch_out.or_and_not(&succ_live.live_in, &params[&succ]);
                }
            }
            scratch_out.union_with(&uses[&node]);

            scratch_in.clear();
            scratch_in.union_with(&params[&node]);
            scratch_in.union_with(&uses[&node]);
            scratch_in.or_and_not(&scratch_out, &defs[&node]);

            let old = result.get_mut(&node).unwrap();
            if old.live_out != scratch_out || old.live_in != scratch_in {
                std::mem::swap(&mut old.live_out, &mut scratch_out);
                std::mem::swap(&mut old.live_in, &mut scratch_in);
                changed = true;
            }
        }
    }

    LivenessResult {
        sets: result,
        id_of,
    }
}
