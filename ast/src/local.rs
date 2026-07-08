use crate::{type_system::Infer, SideEffects, Traverse, Type, TypeSystem};
use by_address::ByAddress;
use derive_more::From;
use enum_dispatch::enum_dispatch;
use nohash_hasher::NoHashHasher;
use parking_lot::Mutex;
use std::{
    cell::Cell,
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use triomphe::Arc;

#[derive(Debug, Default, From, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Local(pub Option<String>);

impl Local {
    pub fn new(name: Option<String>) -> Self {
        Self(name)
    }
}

impl fmt::Display for Local {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            Some(name) => write!(f, "{}", name),
            None => write!(f, "UNNAMED_LOCAL"),
        }
    }
}

thread_local! {
    static NEXT_LOCAL_ID: Cell<u64> = const { Cell::new(0) };
}

/// Resets the thread-local monotonic id counter used to make `RcLocal`'s
/// `Hash`/`Ord` deterministic (see `RcLocal`'s doc comment). Must be called
/// at the start of every top-level decompile entry point, so that two
/// independent decompiles in the same long-lived thread/process (e.g. a
/// `topaz serve` worker handling many requests) don't have their relative
/// iteration order depend on how many prior jobs that thread happened to
/// process.
pub fn reset_local_id_counter() {
    NEXT_LOCAL_ID.with(|c| c.set(0));
}

fn next_local_id() -> u64 {
    NEXT_LOCAL_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    })
}

/// A reference-counted handle to a [`Local`].
///
/// The second field is a deterministic, monotonically-increasing
/// creation-order id used for `Hash`/`Ord`/`PartialOrd` instead of the
/// underlying allocation's raw pointer address. `RcLocal` is used as the
/// key type of many `HashMap`/`HashSet`s throughout SSA construction and
/// destruction; hashing/ordering by pointer address makes iteration order
/// (and therefore synthetic-name numbering and statement sequentialization
/// in the final output) depend on ASLR / allocator state / prior allocation
/// history rather than on anything about the program being decompiled.
/// Ordering/hashing by creation-order id instead makes output deterministic
/// across runs.
///
/// `PartialEq`/`Eq` still delegate to `ByAddress` (identity semantics are
/// unchanged: two `RcLocal`s are equal iff they point at the same
/// underlying `Local`).
///
/// Kept as a tuple struct (no `Deref` impl) so that existing call sites
/// written as `local.0 .0.lock()` (`.0` for this tuple field, `.0` again
/// for `ByAddress`'s inner field) continue to compile unchanged.
#[derive(Debug, Clone)]
pub struct RcLocal(pub ByAddress<Arc<Mutex<Local>>>, u64);

impl Default for RcLocal {
    fn default() -> Self {
        RcLocal(ByAddress(Arc::default()), next_local_id())
    }
}

impl PartialEq for RcLocal {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for RcLocal {}

impl Hash for RcLocal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Creation-order id, NOT pointer address -- see the struct's doc
        // comment for why.
        self.1.hash(state);
    }
}

impl PartialOrd for RcLocal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RcLocal {
    fn cmp(&self, other: &Self) -> Ordering {
        // Creation-order id, NOT pointer address -- see the struct's doc
        // comment for why.
        self.1.cmp(&other.1)
    }
}

impl Infer for RcLocal {
    fn infer<'a: 'b, 'b>(&'a mut self, system: &mut TypeSystem<'b>) -> Type {
        system.type_of(self).clone()
    }
}

impl Display for RcLocal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 .0.lock().0 {
            Some(name) => write!(f, "{}", name),
            None => {
                let mut hasher = NoHashHasher::<u8>::default();
                self.hash(&mut hasher);
                write!(f, "UNNAMED_{}", hasher.finish())
            }
        }
    }
}

impl SideEffects for RcLocal {}

impl Traverse for RcLocal {}

impl RcLocal {
    pub fn new(local: Local) -> Self {
        Self(ByAddress(Arc::new(Mutex::new(local))), next_local_id())
    }
}

impl LocalRw for RcLocal {
    fn values_read(&self) -> Vec<&RcLocal> {
        vec![self]
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        vec![self]
    }
}

#[enum_dispatch]
pub trait LocalRw {
    fn values_read(&self) -> Vec<&RcLocal> {
        Vec::new()
    }

    fn values_read_mut(&mut self) -> Vec<&mut RcLocal> {
        Vec::new()
    }

    fn values_written(&self) -> Vec<&RcLocal> {
        Vec::new()
    }

    fn values_written_mut(&mut self) -> Vec<&mut RcLocal> {
        Vec::new()
    }

    fn values(&self) -> Vec<&RcLocal> {
        self.values_read()
            .into_iter()
            .chain(self.values_written())
            .collect()
    }

    fn replace_values_read(&mut self, old: &RcLocal, new: &RcLocal) {
        for value in self.values_read_mut() {
            if value == old {
                *value = new.clone();
            }
        }
    }

    fn replace_values_written(&mut self, old: &RcLocal, new: &RcLocal) {
        for value in self.values_written_mut() {
            if value == old {
                *value = new.clone();
            }
        }
    }

    fn replace_values(&mut self, old: &RcLocal, new: &RcLocal) {
        self.replace_values_read(old, new);
        self.replace_values_written(old, new);
    }
}
