use std::collections::{HashMap, HashSet};

use crate::{ast::Expr, runtime::{lock::LockKind, manager::Manager, transaction::{Txn, TxnId}}};

use super::txn_utils::*;


#[derive(Clone, Debug)]
pub struct TxnManager {
    pub txn: Txn,
    pub reads: HashMap<String, ReadState>,
    pub writes: HashMap<String, WriteState>,
    pub preds: HashSet<Txn>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReadState {
    Requested, // default
    Granted,
    Aborted,
    Read(Expr),
    // Failed,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WriteState {
    Requested, // default
    Granted,
    Aborted,
}

impl TxnManager {
    pub fn new(
        txn: Txn, 
        reads: HashSet<String>, 
        writes: HashSet<String>
    ) -> Self {
        TxnManager {
            txn,
            reads: HashMap::from_iter(reads.iter()
                .map(|name| (name.clone(), ReadState::Requested))),
            writes: HashMap::from_iter(writes.iter()
                .map(|name| (name.clone(), WriteState::Requested))),
            preds: HashSet::new(),
        }
    }

    pub fn add_grant_lock(&mut self, name: String, kind: LockKind) {
        if kind == LockKind::Read {
            assert!(self.reads.get(&name) == Some(ReadState::Requested).as_ref());
            self.reads.insert(name, ReadState::Granted);
        } else {
            // notice in the case the transaction requires both read and write
            // lock on the name, we only send and receive the write lock request
            // and grant, but need additionally update the read lock also granted
            if self.reads.contains_key(&name) {
                self.reads.insert(name.clone(), ReadState::Granted);
            }
            self.writes.insert(name, WriteState::Granted);
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

    pub fn all_read_finished(&self) -> bool {
        self.reads
            .iter()
            .all(|(_, v)| matches!(v, ReadState::Read(_)))
    }

    pub fn get_read_results(&self) -> HashMap<String, Expr> {
        self.reads
            .iter()
            .filter_map(|(name, state)| match state {
                ReadState::Read(result) => Some((name.clone(), result.clone())),
                _ => panic!("read not finished"),
            })
            .collect()
    }

    pub fn abort_lock(&mut self) {
        for (_, state) in self.reads.iter_mut() {
            *state = ReadState::Aborted;
        }
        for (_, state) in self.writes.iter_mut() {
            *state = WriteState::Aborted;
        }
    }

    pub fn is_aborted(&self) -> bool {
        self.reads.iter().any(|(_, v)| *v == ReadState::Aborted)
            || self.writes.iter().any(|(_, v)| *v == WriteState::Aborted)
    }
}

/// derived methods on service manager
/// 
macro_rules! delegate_to_txn {
    // Mutable delegates take (&mut self, &TxnId, ...) ->  call &mut TxnManager
    (mut $fn_name:ident ( $($arg:ident : $arg_ty:ty),* ) ) => {
        pub fn $fn_name(&mut self, txn_id: &TxnId, $($arg : $arg_ty),* ) {
            let mgr = self.txn_mgrs
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
    pub fn new_txn_mgr(
        &mut self, 
        txn: &Txn,
        read_set: HashSet<String>,
        write_set: HashSet<String>,

    ) {
        let new_mgr = TxnManager::new(
            txn.clone(),
            read_set,
            write_set
        );
        self.txn_mgrs.insert(txn.id.clone(), new_mgr);
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
    delegate_to_txn!(mut add_grant_lock(name: String, kind: LockKind));
    delegate_to_txn!(mut add_finished_read(name: String, result: Expr, pred: HashSet<Txn>));
    delegate_to_txn!(mut abort_lock());
    delegate_to_txn!(imm all_lock_granted() -> bool);
    delegate_to_txn!(imm all_read_finished() -> bool);
    delegate_to_txn!(imm is_aborted() -> bool);
}
