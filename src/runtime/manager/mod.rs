use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::Infallible;

use kameo::prelude::*;
use kameo::Actor;

use crate::ast::Expr;

use super::def_actor::DefActor;
use super::lock::Lock;
use super::lock::LockKind;
use super::message::Msg;
use super::transaction::Txn;
use super::transaction::TxnId;
use super::var_actor::VarActor;

pub mod alloc_actors;
pub mod do_action;
pub mod handler;

#[derive(Clone)]
pub struct Manager {
    pub name: String,
    pub address: Option<ActorRef<Manager>>,

    pub varname_to_actors: HashMap<String, ActorRef<VarActor>>,
    pub defname_to_actors: HashMap<String, ActorRef<DefActor>>,
    pub dep_graph: HashMap<String, HashSet<String>>,

    pub txn_mgrs: HashMap<TxnId, TxnManager>,
}

impl Actor for Manager {
    type Error = Infallible;

    /// customized on_start impl of Actor trait    
    /// to allow manager self reference to its addr
    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        self.address = Some(actor_ref.clone());
        Ok(())
    }
}

macro_rules! delegate_to_txn {
    // Mutable delegates take (&mut self, &TxnId, ...) ->  call &mut TxnManager
    (mut $fn_name:ident ( $($arg:ident : $arg_ty:ty),* ) ) => {
        pub fn $fn_name(&mut self, txn_id: &TxnId, $($arg : $arg_ty),* ) {
            let mgr = self
                .txn_mgrs
                .get_mut(txn_id)
                .expect("txn manager not found");
            mgr.$fn_name($($arg),*);
        }
    };
    // Immutable delegates take (&self, &TxnId) -> call &TxnManager
    (imm $fn_name:ident () -> $ret:ty) => {
        pub fn $fn_name(&self, txn_id: &TxnId) -> $ret {
            let mgr = self
                .txn_mgrs
                .get(txn_id)
                .expect("txn manager not found");
            mgr.$fn_name()
        }
    };
}

impl Manager {
    pub fn new_txn_mgr(&mut self, txn_id: &TxnId) {
        let new_mgr = TxnManager::new(txn_id.clone());
        self.txn_mgrs.insert(txn_id.clone(), new_mgr);
    }

    pub fn get_mut_txn_mgr(&mut self, txn_id: &TxnId) -> &mut TxnManager {
        self.txn_mgrs
            .get_mut(txn_id)
            .expect("txn manager not found")
    }

    pub fn get_read_results(&self, txn_id: &TxnId) -> HashMap<String, Expr> {
        self.txn_mgrs
            .get(txn_id)
            .expect(&format!("txn manager not found"))
            .reads
            .iter()
            .filter_map(|(name, state)| match state {
                ReadState::Read(result) => Some((name.clone(), result.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn get_preds(&self, txn_id: &TxnId) -> HashSet<Txn> {
        self.txn_mgrs
            .get(txn_id)
            .expect(&format!("txn manager not found"))
            .preds
            .clone()
    }

    // invoke the macro to generate one‐line wrappers:
    delegate_to_txn!(mut add_request_lock(name: String, kind: LockKind));
    delegate_to_txn!(mut add_grant_lock(name: String, kind: LockKind));
    delegate_to_txn!(mut add_abort_lock(name: String, kind: LockKind));
    delegate_to_txn!(mut add_finished_read(name: String, result: Expr, pred: HashSet<Txn>));
    delegate_to_txn!(imm all_lock_granted() -> bool);
    delegate_to_txn!(imm any_lock_aborted() -> bool);
    delegate_to_txn!(imm all_read_finished() -> bool);
}

#[derive(Clone)]
pub struct TxnManager {
    pub txn_id: TxnId,
    pub reads: HashMap<String, ReadState>,
    pub writes: HashMap<String, WriteState>,
    pub preds: HashSet<Txn>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ReadState {
    Requested,
    Granted,
    Aborted,
    Read(Expr),
    Failed,
}

#[derive(Clone, PartialEq, Eq)]
pub enum WriteState {
    Requested,
    Granted,
    Aborted,
}

impl TxnManager {
    pub fn new(txn_id: TxnId) -> Self {
        TxnManager {
            txn_id,
            reads: HashMap::new(),
            writes: HashMap::new(),
            preds: HashSet::new(),
        }
    }

    pub fn add_request_lock(&mut self, name: String, kind: LockKind) {
        if kind == LockKind::Read {
            self.reads.insert(name, ReadState::Requested);
        } else {
            self.writes.insert(name, WriteState::Requested);
        }
    }

    pub fn add_grant_lock(&mut self, name: String, kind: LockKind) {
        if kind == LockKind::Read {
            assert!(self.reads.get(&name) == Some(ReadState::Requested).as_ref());
            self.reads.insert(name, ReadState::Granted);
        } else {
            assert!(self.writes.get(&name) == Some(WriteState::Requested).as_ref());
            self.writes.insert(name, WriteState::Granted);
        }
    }

    pub fn add_abort_lock(&mut self, name: String, kind: LockKind) {
        if kind == LockKind::Read {
            self.reads.insert(name, ReadState::Aborted);
        } else {
            self.writes.insert(name, WriteState::Aborted);
        }
    }

    pub fn add_finished_read(&mut self, name: String, result: Expr, pred: HashSet<Txn>) {
        assert!(self.reads.get(&name) == Some(ReadState::Granted).as_ref());
        self.reads.insert(name, ReadState::Read(result));

        self.preds.extend(pred);
    }

    pub fn all_lock_granted(&self) -> bool {
        self.reads.iter().all(|(_, v)| *v == ReadState::Granted)
            && self.writes.iter().all(|(_, v)| *v == WriteState::Granted)
    }

    pub fn any_lock_aborted(&self) -> bool {
        self.reads.iter().any(|(_, v)| *v == ReadState::Aborted)
            || self.writes.iter().any(|(_, v)| *v == WriteState::Aborted)
    }

    pub fn all_read_finished(&self) -> bool {
        self.reads
            .iter()
            .all(|(_, v)| matches!(v, ReadState::Read(_)))
    }
}
