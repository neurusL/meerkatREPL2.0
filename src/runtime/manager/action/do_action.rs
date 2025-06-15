//! utils for managing a transaction (do action)
//!
//! a transactions is processed in the following step:
//! 1. new txn manager of the transaction
//! 2. request read and write lock, if both required for a reactive name
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
use std::{collections::HashSet, error::Error};

use tokio::sync::mpsc::Sender;

use crate::{ast::{Assn, Expr}, runtime::{evaluator::eval_assns, lock::{Lock, LockKind}, manager::{action::{ReadState, TxnManager, WriteState}, Manager}, message::{CmdMsg, Msg}, transaction::{Txn, TxnId}}, static_analysis::var_analysis::read_write::{calc_read_set, calc_write_set}};

impl Manager {
    /// 1. initialize a new transaction manager
    pub fn new_txn(
        &mut self,
        txn_id: TxnId,
        assns: Vec<Assn>,
        from_client: Sender<CmdMsg>,
    ) -> TxnManager {
        // static info of txn, the read and write set, which may overlap
        let read_set = calc_read_set(&assns);
        let write_set = calc_write_set(&assns);

        let txn = Txn::new(txn_id, assns);

        // set up txn manager
        let txn_mgr = TxnManager::new(txn, from_client, read_set, write_set);

        txn_mgr
    }

    /// 2. request read and write lock
    pub async fn request_locks(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let mgr_addr = self
            .address
            .clone()
            .expect("manager addr should not be None");

        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();

        // send lock requests
        // notice it's possible for a reactive name to be both read and write
        // in this case, we only send write lock request
        for (name, state) in txn_mgr.reads.iter() {
            if txn_mgr.writes.contains_key(name) {
                assert!(*state == ReadState::Requested);
                continue; // already request for write lock
            }
            self.tell_to_name(
                &name,
                Msg::LockRequest {
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_read(txn_id.clone()),
                },
            )
            .await?;
            assert!(*state == ReadState::Requested);
        }

        for (name, state) in txn_mgr.writes.iter() {
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

        for name in txn_mgr.reads.keys() {
            self.tell_to_name(
                &name,
                Msg::UsrReadVarRequest {
                    txn: txn_mgr.txn.id.clone(),
                    from_mgr_addr: self.address.clone().unwrap(),
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

        for name in txn_mgr.reads.keys() {
            if txn_mgr.writes.contains_key(name) {
                continue; // should be aborted for write lock
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

    /// 6. release all locks
    pub async fn release_locks(&self, txn_id: &TxnId) -> Result<(), Box<dyn Error>> {
        let txn_mgr = self.txn_mgrs.get(txn_id).unwrap();

        // release all locks while avoid duplicated release
        let mut names = txn_mgr.reads.keys().collect::<HashSet<&String>>();
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
