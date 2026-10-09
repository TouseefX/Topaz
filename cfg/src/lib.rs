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
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

thread_local! {
    static DECOMPILE_DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// `-1` = unset (read `TOPAZ_TIME_BUDGET_SECS` or default 180).
/// `>= 0` = wall-clock seconds, including `0` for already expired.
static BUDGET_SECS: AtomicI64 = AtomicI64::new(-1);

/// Override the wall-clock decompile budget. `0` means the deadline is
/// already due (used by tests). CLI `--time-budget` calls this.
pub fn set_decompile_budget_secs(secs: u64) {
    BUDGET_SECS.store(secs as i64, Ordering::Relaxed);
}

/// Effective budget: CLI/API override, else `TOPAZ_TIME_BUDGET_SECS`, else 180s.
pub fn decompile_budget() -> Duration {
    let o = BUDGET_SECS.load(Ordering::Relaxed);
    if o >= 0 {
        return Duration::from_secs(o as u64);
    }
    std::env::var("TOPAZ_TIME_BUDGET_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(180))
}

pub fn decompile_budget_secs() -> u64 {
    decompile_budget().as_secs()
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
