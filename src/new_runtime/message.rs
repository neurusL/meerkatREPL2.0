use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
    time::SystemTime,
};

use kameo::actor::{ActorRef, ReplyRecipient};

use crate::{
    ast::{Expr, Service},
    new_runtime::{
        service::{Export, Import, ReactiveInputs, ReactiveName, ReactiveRef, ServiceActor},
        Configurator,
    },
};

#[derive(Debug, Clone)]
pub enum Msg {
    // messages sent by the system itself
    Unreachable {
        message: Box<Msg>,
    },

    // propagation
    Propagate {
        sender: ReactiveRef,
        value: StampedValue,
    },

    // transaction - initial lock request
    Lock {
        txid: TxId,
        kind: LockKind,
    },
    LockGranted {
        txid: TxId,
        service: ActorRef<ServiceActor>,
        reactives: HashMap<ReactiveName, Version>,
    },

    // transaction - messages available to shared and exclusive locks
    ReadValue {
        txid: TxId,
        reactive: ReactiveName,
        basis: BasisStamp,
    },
    ReturnedValue {
        txid: TxId,
        service: ActorRef<ServiceActor>,
        reactive: ReactiveName,
        value: Expr,
    },

    // transaction - messages available to exclusive locks
    Write {
        txid: TxId,
        reactive: ReactiveName,
        value: Expr,
    },
    ReadConfiguration {
        txid: TxId,
    },
    ReturnedConfiguration {
        txid: TxId,
        service: ActorRef<ServiceActor>,
        imports: HashMap<ReactiveRef, Import>,
        inputs: HashMap<ReactiveName, ReactiveInputs>,
        exports: HashMap<ReactiveName, Export>,
    },
    Configure {
        txid: TxId,
        imports: HashMap<ReactiveRef, Option<ImportConfiguration>>,
        reactives: HashMap<ReactiveName, Option<ReactiveConfiguration>>,
        exports: HashMap<ReactiveName, HashSet<ActorRef<ServiceActor>>>,
    },
    Retire {
        txid: TxId,
    },

    // transaction - messages related to ending the lock
    Preempt {
        txid: TxId,
    },
    Abort {
        txid: TxId,
    },
    PrepareCommit {
        txid: TxId,
    },
    CommitPrepared {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        basis: BasisStamp,
    },
    Commit {
        txid: TxId,
        basis: BasisStamp,
    },

    // messages sent/received by managers
    Do {
        action: Expr,
        create: Service,
        expected_versions: HashMap<ReactiveRef, Version>,
    },
    /*Directory {
        state: DirectoryState,
    },*/
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u64);

impl Version {
    pub const ZERO: Version = Version(0);

    #[must_use]
    pub fn increment(self) -> Version {
        Version(self.0 + 1)
    }
}

#[derive(Debug, Clone)]
pub struct ImportConfiguration {
    pub roots: HashSet<ReactiveRef>,
}

#[derive(Debug, Clone)]
pub struct StampedValue {
    pub value: Expr,
    pub basis: BasisStamp,
}

#[derive(Debug, Clone)]
pub struct BasisStamp {
    pub roots: HashMap<ReactiveRef, Iteration>,
}

impl BasisStamp {
    pub fn empty() -> BasisStamp {
        BasisStamp {
            roots: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn latest(&self, address: &ReactiveRef) -> Iteration {
        self.roots.get(address).copied().unwrap_or(Iteration(0))
    }

    pub fn add(&mut self, address: ReactiveRef, iteration: Iteration) {
        match self.roots.entry(address) {
            Entry::Vacant(entry) => {
                entry.insert(iteration);
            }
            Entry::Occupied(mut entry) => {
                *entry.get_mut() = (*entry.get()).max(iteration);
            }
        }
    }

    pub fn merge_from(&mut self, other: &BasisStamp) {
        for (address, iteration) in &other.roots {
            self.add(address.clone(), *iteration);
        }
    }

    pub fn clear(&mut self) {
        self.roots.clear();
    }

    pub fn prec_eq_wrt_roots(&self, other: &BasisStamp, roots: &HashSet<ReactiveRef>) -> bool {
        for root in roots {
            if self.latest(root) > other.latest(root) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Iteration(u64);

impl Iteration {
    pub const ZERO: Iteration = Iteration(0);

    #[must_use]
    pub fn increment(self) -> Iteration {
        Iteration(self.0 + 1)
    }
}

#[derive(Debug, Clone)]
pub enum ReactiveConfiguration {
    Variable { value: Expr },
    Definition { expr: Expr },
}

/*#[derive(Debug, Clone)]
pub struct DirectoryState {
    pub managers: HashMap<Address, bool>,

    // Multi-value register semantics:
    // If multiple nodes are assigned the same name concurrently, the directory will store all of them.
    pub nodes: HashMap<Name, HashMap<Address, Option<Version>>>,
}*/

#[derive(Debug, Clone)]
pub struct TxId {
    pub priority: TxPriority,
    pub timestamp: Timestamp,
    pub manager: ReplyRecipient<Msg, ()>,
}

impl PartialEq for TxId {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.timestamp == other.timestamp
            && self.manager.id() == other.manager.id()
    }
}

impl Eq for TxId {}

impl Hash for TxId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.priority.hash(state);
        self.timestamp.hash(state);
        self.manager.id().hash(state);
    }
}

impl PartialOrd for TxId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
            .then_with(|| {
                self.manager
                    .id()
                    .to_string()
                    .cmp(&other.manager.id().to_string())
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TxPriority {
    High = 0,
    Low = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    epoch_micros: u64,
}

pub struct MonotonicTimestampGenerator {
    latest: Timestamp,
}

impl MonotonicTimestampGenerator {
    pub fn new() -> MonotonicTimestampGenerator {
        MonotonicTimestampGenerator {
            latest: Timestamp { epoch_micros: 0 },
        }
    }

    pub fn generate_timestamp(&mut self) -> Timestamp {
        #[cfg(not(target_arch = "wasm32"))]
        let epoch_micros = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        #[cfg(target_arch = "wasm32")]
        compile_error!("Wasm support has not yet been implemented.");

        if epoch_micros > self.latest.epoch_micros {
            self.latest = Timestamp { epoch_micros };
        } else {
            self.latest.epoch_micros += 1;
        }

        self.latest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockKind {
    Shared,
    Exclusive,
}
