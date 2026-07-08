//! Usage-Based Type Inference Naming
//!
//! This implements the "enhanced heuristic naming" item from the Topaz vs
//! Oracle analysis: infer a plausible name for a local from *how it is
//! used* later on, not just from the expression that created it.
//! For example, a local that is later indexed with `.ClassName` or
//! `.Parent`, or passed to `tostring()`/`typeof()`, is very likely a
//! Roblox `Instance`, so naming it `obj` or `node` is more informative
//! than a synthetic `v42`.
//!
//! Design constraints (deliberately conservative, given how many past
//! naming bugs in this codebase stemmed from over-eager or context-blind
//! renaming -- see `name_locals.rs` and `context_naming.rs`):
//!
//! - This is a **pure suggestion source**, not a naming pass on its own.
//!   It only ever produces a candidate `&'static str` hint keyed by
//!   `RcLocal` identity; the actual assignment, uniqueness-dedup, and
//!   collision-avoidance still goes through `Namer::unique_name`/
//!   `name_uses` in `name_locals.rs`, so it can never reintroduce the
//!   `Id = Id` class of bug (two distinct locals silently printing with
//!   the identical name).
//! - It is used only as a **fallback**: callers should prefer any name
//!   already derived from the local's creating expression (via
//!   `Namer::derive_name`), and only consult this module's hints when
//!   that comes up empty. This keeps existing, working naming behavior
//!   completely unchanged for the common case.
//! - Matching is intentionally narrow and high-confidence: only property/
//!   method names that are essentially unambiguous signals of a Roblox
//!   `Instance` (or a small number of other unambiguous shapes) are
//!   recognized, and a hint is only produced when usages agree by a
//!   clear margin (see `Votes::best`). When in doubt, this contributes no
//!   hint and the caller falls back to the existing short synthetic name
//!   (`v42`, `t7`, etc.), matching the analysis's explicit guidance to
//!   "fall back to Oracle-style short names when inference is uncertain".
//! - This never crosses a closure boundary to *rename* anything -- it
//!   only *observes* usages (including inside nested closures, since
//!   that's still useful signal about what a captured variable holds).
//!   Renaming decisions themselves are entirely unchanged, so this cannot
//!   reintroduce the captured-upvalue naming collision bug either.

use rustc_hash::FxHashMap;

use crate::{Block, Literal, RValue, RcLocal, Select, Statement, Traverse};

/// Property names that are effectively unique to Roblox `Instance`
/// objects. Seeing a local indexed with one of these is a strong signal
/// that the local holds an instance/object reference, so `obj`/`node`-
/// style names are more informative than a generic synthetic name.
const INSTANCE_LIKE_FIELDS: &[&str] = &["ClassName", "Parent", "Archivable", "RobloxLocked"];

/// Method names that are effectively unique to Roblox `Instance` objects.
const INSTANCE_LIKE_METHODS: &[&str] = &[
    "FindFirstChild",
    "FindFirstChildOfClass",
    "FindFirstChildWhichIsA",
    "FindFirstAncestor",
    "FindFirstAncestorOfClass",
    "FindFirstAncestorWhichIsA",
    "WaitForChild",
    "GetChildren",
    "GetDescendants",
    "IsA",
    "IsDescendantOf",
    "IsAncestorOf",
    "GetFullName",
];

#[derive(Default, Clone, Copy)]
struct Votes {
    instance_like: u32,
    string_like: u32,
    number_like: u32,
}

impl Votes {
    fn add(&mut self, other: Votes) {
        self.instance_like += other.instance_like;
        self.string_like += other.string_like;
        self.number_like += other.number_like;
    }

    /// Require at least 2 corroborating usages before committing to a
    /// guess, and only report a result when one category is unambiguously
    /// ahead of the combined weight of the others -- a single incidental
    /// use (e.g. a one-off `tostring(x)` for logging) shouldn't be enough
    /// to claim a type, since guessing wrong is worse than falling back
    /// to a generic synthetic name.
    fn best(&self) -> Option<&'static str> {
        let candidates = [
            (self.instance_like, "obj"),
            (self.string_like, "str"),
            (self.number_like, "num"),
        ];
        let (count, name) = candidates.into_iter().max_by_key(|&(c, _)| c)?;
        if count < 2 {
            return None;
        }
        let others_total: u32 = candidates.iter().map(|&(c, _)| c).sum::<u32>() - count;
        if others_total >= count {
            return None;
        }
        Some(name)
    }
}

/// Walks the entire block (including nested control-flow blocks and
/// closures) once, collecting per-local usage votes, and returns a map
/// from local identity to an inferred type-hint string for every local
/// that reached a confident conclusion. Locals not present in the
/// returned map simply have no confident inference; callers should treat
/// that the same as `None`.
pub fn collect_type_hints(block: &Block) -> FxHashMap<RcLocal, &'static str> {
    let mut votes: FxHashMap<RcLocal, Votes> = FxHashMap::default();
    scan_block(block, &mut votes);
    votes
        .into_iter()
        .filter_map(|(local, v)| v.best().map(|hint| (local, hint)))
        .collect()
}

fn scan_block(block: &Block, votes: &mut FxHashMap<RcLocal, Votes>) {
    for statement in &block.0 {
        for rvalue in statement.rvalues() {
            scan_rvalue(rvalue, votes);
        }
        statement.rvalues().into_iter().for_each(|rv| {
            if let RValue::Closure(closure) = rv {
                scan_block(&closure.function.lock().body, votes);
            }
        });
        match statement {
            Statement::If(r#if) => {
                scan_block(&r#if.then_block.lock(), votes);
                scan_block(&r#if.else_block.lock(), votes);
            }
            Statement::While(r#while) => scan_block(&r#while.block.lock(), votes),
            Statement::Repeat(repeat) => scan_block(&repeat.block.lock(), votes),
            Statement::NumericFor(nf) => scan_block(&nf.block.lock(), votes),
            Statement::GenericFor(gf) => scan_block(&gf.block.lock(), votes),
            _ => {}
        }
    }
}

fn local_of(rv: &RValue) -> Option<&RcLocal> {
    if let RValue::Local(l) = rv {
        Some(l)
    } else {
        None
    }
}

fn record(votes: &mut FxHashMap<RcLocal, Votes>, local: &RcLocal, f: impl FnOnce(&mut Votes)) {
    f(votes.entry(local.clone()).or_default());
}

fn scan_rvalue(rv: &RValue, votes: &mut FxHashMap<RcLocal, Votes>) {
    match rv {
        RValue::Index(idx) => {
            if let Some(local) = local_of(&idx.left) {
                if let RValue::Literal(Literal::String(key)) = &*idx.right {
                    if let Ok(key) = std::str::from_utf8(key) {
                        if INSTANCE_LIKE_FIELDS.contains(&key) {
                            record(votes, local, |v| v.instance_like += 1);
                        }
                    }
                }
            }
            scan_rvalue(&idx.left, votes);
            scan_rvalue(&idx.right, votes);
        }
        RValue::MethodCall(mc) => {
            if let Some(local) = local_of(&mc.value) {
                if INSTANCE_LIKE_METHODS.contains(&mc.method.as_str()) {
                    record(votes, local, |v| v.instance_like += 1);
                }
            }
            scan_rvalue(&mc.value, votes);
            for arg in &mc.arguments {
                scan_rvalue(arg, votes);
            }
        }
        RValue::Call(call) => {
            if let RValue::Global(g) = &*call.value {
                if let Ok(name) = std::str::from_utf8(&g.0) {
                    if let Ok(only_arg) = call.arguments.exactly_one_local() {
                        match name {
                            "tostring" => record(votes, only_arg, |v| v.string_like += 1),
                            "typeof" | "type" => record(votes, only_arg, |v| v.instance_like += 1),
                            "tonumber" => record(votes, only_arg, |v| v.number_like += 1),
                            _ => {}
                        }
                    }
                }
            }
            scan_rvalue(&call.value, votes);
            for arg in &call.arguments {
                scan_rvalue(arg, votes);
            }
        }
        RValue::Binary(bin) => {
            scan_rvalue(&bin.left, votes);
            scan_rvalue(&bin.right, votes);
        }
        RValue::Unary(un) => scan_rvalue(&un.value, votes),
        RValue::Select(Select::Call(call)) => scan_rvalue(&RValue::Call(call.clone()), votes),
        RValue::Select(Select::MethodCall(mc)) => {
            scan_rvalue(&RValue::MethodCall(mc.clone()), votes)
        }
        RValue::Table(table) => {
            for (key, value) in &table.0 {
                if let Some(key) = key {
                    scan_rvalue(key, votes);
                }
                scan_rvalue(value, votes);
            }
        }
        _ => {}
    }
}

/// Small helper trait to check "this argument list has exactly one
/// element, and it's a bare local reference" without an intermediate
/// `Vec` allocation.
trait ExactlyOneLocal {
    fn exactly_one_local(&self) -> Result<&RcLocal, ()>;
}

impl ExactlyOneLocal for [RValue] {
    fn exactly_one_local(&self) -> Result<&RcLocal, ()> {
        match self {
            [single] => local_of(single).ok_or(()),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assign, Call, Global, Index, LValue, Local, MethodCall};

    fn local(name: Option<&str>) -> RcLocal {
        RcLocal::new(Local::new(name.map(str::to_string)))
    }

    fn assign_call_stmt(target: &RcLocal, method: &str, args: Vec<RValue>) -> Statement {
        Assign::new(
            vec![LValue::Local(target.clone())],
            vec![RValue::MethodCall(MethodCall::new(
                RValue::Local(target.clone()),
                method.to_string(),
                args,
            ))],
        )
        .into()
    }

    /// Two corroborating Instance-like usages (an unambiguous field access
    /// plus an unambiguous method call) should produce the `"obj"` hint.
    #[test]
    fn infers_instance_from_field_and_method_usage() {
        let x = local(None);
        let mut block: Block = vec![
            Statement::If(crate::If::new(
                RValue::Binary(crate::Binary::new(
                    RValue::Index(Index::new(
                        RValue::Local(x.clone()),
                        crate::Literal::String(b"ClassName".to_vec()).into(),
                    )),
                    crate::Literal::String(b"Part".to_vec()).into(),
                    crate::BinaryOperation::Equal,
                )),
                Block::default(),
                Block::default(),
            )),
            assign_call_stmt(&x, "GetChildren", vec![]),
        ]
        .into();

        let hints = collect_type_hints(&mut block);
        assert_eq!(hints.get(&x), Some(&"obj"));
    }

    /// A single incidental usage should not be enough to commit to a
    /// guess -- being wrong is worse than falling back to a generic name.
    #[test]
    fn does_not_infer_from_a_single_usage() {
        let x = local(None);
        let mut block: Block = vec![assign_call_stmt(&x, "GetChildren", vec![])].into();
        let hints = collect_type_hints(&mut block);
        assert_eq!(hints.get(&x), None);
    }

    /// Conflicting signals (roughly equal votes for two different
    /// categories) should not produce a confident guess.
    #[test]
    fn does_not_infer_when_signals_conflict() {
        let x = local(None);
        let mut block: Block = vec![
            assign_call_stmt(&x, "GetChildren", vec![]),
            assign_call_stmt(&x, "IsA", vec![]),
            Assign::new(
                vec![LValue::Local(RcLocal::default())],
                vec![RValue::Call(Call::new(
                    RValue::Global(Global(b"tostring".to_vec())),
                    vec![RValue::Local(x.clone())],
                ))],
            )
            .into(),
            Assign::new(
                vec![LValue::Local(RcLocal::default())],
                vec![RValue::Call(Call::new(
                    RValue::Global(Global(b"tostring".to_vec())),
                    vec![RValue::Local(x.clone())],
                ))],
            )
            .into(),
        ]
        .into();
        let hints = collect_type_hints(&mut block);
        // instance_like=2 (from the two method calls), string_like=2 (from
        // the two tostring calls) -- tied, so no confident inference.
        assert_eq!(hints.get(&x), None);
    }

    fn collect_type_hints(block: &mut Block) -> FxHashMap<RcLocal, &'static str> {
        super::collect_type_hints(block)
    }
}
