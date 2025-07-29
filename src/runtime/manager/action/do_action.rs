//! utils for managing a transaction (do action)
//!
//! a transactions is processed in the following step:
//! 1. new txn manager of the transaction
//! 2. request read/write/upgrade locks, if they are required for a reactive name
//!    we send only write lock request
//! 3. if all locks granted, send all read requests, wait for all reads to finish
//! 4. if any lock aborted, abort all locks
//! 5. if all read finished, evaluate the transaction and send write requests
//! 6. upon finished, release all locks
//!
//! notes:
//! * abort all locks is different from releasing all locks and distinguished
//!   by two message types, while AbortLock enforce reactive names abort the
//!   granted/waiting locks and roll back to previous state, ReleaseLock commit
//!   the change brought by the transaction
//! * alternatively, we can spawn new thread for each transaction, combined with
//!   channel to communicate between threads OR with Arc<Mutex or DashMap>
//!   to lock the shared state of each transaction.
use core::panic;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use tokio::sync::mpsc::Sender;

use crate::{
    ast::{Assn, Expr},
    runtime::{
        def_actor::state,
        evaluator::eval_assns,
        lock::{Lock, LockKind},
        manager::{
            action::{DirectReadState, TransReadState, TxnManager, WriteState},
            Manager,
        },
        message::{CmdMsg, Msg},
        transaction::{Txn, TxnId},
    },
    static_analysis::var_analysis::read_write::{calc_read_sets as calc_read_set, calc_write_set},
};

impl Manager {
    /// 1. initialize a new transaction manager
    pub fn add_new_txn(
        &mut self,
        txn_id: TxnId,
        assns: Vec<Assn>,
        from_client: Sender<CmdMsg>,
    ) {
        // static info of txn, the read and write set, which may overlap
        let direct_read_set = calc_read_set(&assns, &self.evaluator.reactive_names);
        let write_set = calc_write_set(&assns);

        let txn = Txn::new(txn_id.clone(), assns);

        // set up txn manager
        let txn_mgr = TxnManager::new(
            txn,
            from_client,
            direct_read_set,
            &self.dep_tran_vars,
            write_set,
        );

        self.txn_mgrs.insert(txn_id, txn_mgr);
    }

    /// 2. request trans read and write lock
    pub async fn request_locks(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let mgr_addr = self
            .address
            .clone()
            .expect("manager addr should not be None");

        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();


        // Handle upgrade locks for vars in both read and write sets
        for (name, state) in txn_mgr.reads.iter() {
            if txn_mgr.writes.contains_key(name) {
                self.tell_to_name(
                    &name,
                    Msg::LockRequest {
                        from_mgr_addr: mgr_addr.clone(),
                        lock: Lock::new_upgrade(txn_id.clone()),
                    },
                )
                .await?;
                continue;
        // send lock requests
        // notice it's possible for a reactive name to be both read and write
        // in this case, we only send write lock request
        for (name, state) in txn_mgr.trans_reads.iter() {
            if txn_mgr.writes.contains_key(name) {
                assert!(*state == TransReadState::Requested);
                continue; // already request for write lock
            }

            // Normal read lock request
            self.tell_to_name(
                &name,
                Msg::LockRequest {
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_read(txn_id.clone()),
                },
            )
            .await?;
            assert!(*state == TransReadState::Requested);
        }

        // Handle write locks for vars in write set
        for (name, state) in txn_mgr.writes.iter() {
            // Skip if upgrade lock is already requested in read pass
            if txn_mgr.reads.contains_key(name) {
                continue; // already requested for upgrade lock
            }

            self.tell_to_name(
                &name,
                Msg::LockRequest {
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_write(txn_id.clone()),
                },
            )
            .await?;
            assert!(*state == WriteState::Requested);
        }

        Ok(())
    }

    /// 3. send all read requests
    /// (if all locks granted, which is handled by Manager::handler when
    /// receive new LockGranted message)
    pub async fn request_reads(&mut self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();
        assert!(txn_mgr.all_lock_granted());

        for (name, state) in txn_mgr.direct_reads.iter() {
            // calculate transactions needed to be applied before read
            // since we record all such preds in trans_reads' state
            // we sum up all name's transitive dependent names' preds
            let mut pred = Vec::new();

            if let DirectReadState::RequestedAndDepend(name_trans_read) = state {
                for name in name_trans_read.iter() {
                    if let TransReadState::Granted(pred_id) = txn_mgr
                        .trans_reads
                        .get(name)
                        .expect(&format!("trans read state not found"))
                    {
                        pred_id.as_ref().map(|id| pred.push(id.clone()));
                    }
                }
            } else {
                panic!("direct read state should be RequestedAndDepend");
            }

            self.tell_to_name2(
                &name,
                Msg::UsrReadVarRequest {
                    txn: txn_mgr.txn.id.clone(),
                    from_mgr_addr: self.address.clone().unwrap(),
                },
                Msg::UsrReadDefRequest {
                    txn_id: txn_mgr.txn.id.clone(),
                    from_mgr_addr: self.address.clone().unwrap(),
                    pred,
                },
            )
            .await?;
        }

        Ok(())
    }

    /// 5. evaluate the transaction and send write requests
    /// (if all reads finished, which is handled by Manager::handler when
    /// receive new ReadVarResult message)
    pub async fn reeval_and_request_writes(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();
        assert!(txn_mgr.all_read_finished());

        let env = txn_mgr.get_read_results();
        let assns = eval_assns(&txn_mgr.txn.assns, env);

        for Assn { dest, src } in assns {
            self.tell_to_name(
                &dest,
                Msg::UsrWriteVarRequest {
                    from_mgr_addr: self.address.clone().unwrap(),
                    txn: txn_mgr.txn.id.clone(),
                    write_val: src,
                },
            )
            .await?;
        }
        Ok(())
    }

    /// 4. abort all locks
    /// (if any lock aborted, which is handled by Manager::handler when
    /// receive new LockAbort message)
    pub async fn request_abort_locks(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();

        for name in txn_mgr.direct_reads.keys() {
            if txn_mgr.writes.contains_key(name) {
                continue; // ALready handled in write phase
            }
            self.tell_to_name(
                &name,
                Msg::LockAbort {
                    from_name: self.name.clone(),
                    lock: Lock {
                        lock_kind: LockKind::Read,
                        txn_id: txn_id.clone(),
                    },
                },
            )
            .await?;
        }
        for name in txn_mgr.writes.keys() {
            self.tell_to_name(
                &name,
                Msg::LockAbort {
                    from_name: self.name.clone(),
                    lock: Lock {
                        lock_kind: LockKind::Write,
                        txn_id: txn_id.clone(),
                    },
                },
            )
            .await?;
        }

        return Ok(());
    }

    /// 6. release all locks at the end of transaction
    pub async fn release_locks(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();

        // release all locks while avoiding duplicated release
        let mut names = txn_mgr.reads.keys().collect::<HashSet<&String>>();
        // release all locks while avoid duplicated release
        let mut names = txn_mgr.trans_reads.keys().collect::<HashSet<&String>>();
        names.extend(txn_mgr.writes.keys());

        for name in names {
            self.tell_to_name(
                &name,
                Msg::LockRelease {
                    txn: txn_mgr.txn.clone(),
                    preds: txn_mgr.preds.clone(),
                },
            )
            .await?;
        }
        Ok(())
    }
}

impl Manager {
    pub fn eval_assert(&mut self, expr: &Expr) -> Result<bool, String> {
        let mut expr = expr.clone();
        self.evaluator.eval_assert(&mut expr)?;

        Ok(expr == Expr::Bool { val: true })
    }

    pub fn eval_action(&mut self, mut expr: Expr) -> Result<Vec<Assn>, String> {
        self.evaluator.eval_expr(&mut expr)?;

        if let Expr::Action { assns } = expr {
            Ok(assns.clone())
        } else {
            Err(format!("do requires action expression"))
        }
    }
}
