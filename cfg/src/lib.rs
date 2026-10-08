pub mod block;
pub mod dom_index;
pub mod dominators;
pub mod dot;
pub mod function;
pub mod pattern;
pub mod snapshot;
pub mod ssa;

pub use dom_index::DomIndex;
pub use dominators::{compute_idoms, compute_post_idoms, IDom};

pub use snapshot::{CfgEdge, CfgNode, CfgSnapshot, EdgeKind};

use std::cell::Cell;
use std::time::Instant;

thread_local! {
    static DECOMPILE_DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Set by the decompile thread so SSA / collapse can bail instead of hanging.
pub fn set_decompile_deadline(deadline: Option<Instant>) {
    DECOMPILE_DEADLINE.with(|c| c.set(deadline));
}

#[inline]
pub fn decompile_deadline() -> Option<Instant> {
    DECOMPILE_DEADLINE.with(|c| c.get())
}

#[inline]
pub fn past_decompile_deadline() -> bool {
    DECOMPILE_DEADLINE.with(|c| c.get().is_some_and(|t| Instant::now() >= t))
}
