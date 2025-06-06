use std::{collections::HashSet, error::Error};

use crate::{
    ast::Assn,
    runtime::{
        evaluator::eval_assns,
        lock::{
            Lock,
            LockKind::{Read, Write},
        },
        message::Msg,
        transaction::{Txn, TxnId},
    },
    static_analysis::var_analysis::read_write::{calc_read_set, calc_write_set},
};

use super::Manager;

impl Manager {
    pub async fn do_action(
        &mut self,
        txn: &Txn,
    ) -> Result<(), Box<dyn Error>> {
        // static info of txn, the read and write set
        let mut read_set = calc_read_set(&txn.assns);
        let write_set = calc_write_set(&txn.assns);
        read_set.retain(|x| !&write_set.contains(x)); // exclude write locks from read locks

        // set up txn manager

        self.new_txn_mgr(&txn.id);

        // send lock requests
        self.request_locks(&txn.id, &read_set, &write_set).await?;

        // wait for all locks to be granted or any lock to be aborted
        while !(self.all_lock_granted(&txn.id) && self.any_lock_aborted(&txn.id)) {
            continue;
        }
        if self.any_lock_aborted(&txn.id) {
            // abort all locks
            return self.abort_locks(&txn.id, &read_set, &write_set).await;
        }
        assert!(self.all_lock_granted(&txn.id));

        // wait for all reads to finish
        while !self.all_read_finished(&txn.id) {
            continue;
        }
        assert!(self.all_read_finished(&txn.id));

        // re-eval all src of assn
        let env = self.get_read_results(&txn.id);
        let evaled_assns = eval_assns(&txn.assns, env);

        // send write request
        for Assn { dest, src } in evaled_assns {
            // for right behavior, hand write request synchronously
            // avoiding LockRelease received before UsrWriteVarRequest
            self.ask_to_name(
                &dest,
                Msg::UsrWriteVarRequest {
                    txn: txn.id.clone(),
                    write_val: src,
                },
            )
            .await?;
        }

        // finish up then release
        let preds = self.get_preds(&txn.id);
        for name in read_set.iter().chain(write_set.iter()) {
            self.tell_to_name(
                &name,
                Msg::LockRelease {
                    txn: txn.clone(),
                    preds: preds.clone(),
                },
            )
            .await?;
        }

        Ok(())
    }

    async fn request_locks(
        &mut self,
        txn_id: &TxnId,
        read_set: &HashSet<String>,
        write_set: &HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        let mgr_addr = self
            .address
            .clone()
            .expect("manager addr should not be None");

        for name in read_set.iter() {
            self.tell_to_name(
                &name,
                Msg::LockRequest {
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_read(txn_id.clone()),
                },
            )
            .await?;
            self.add_request_lock(&txn_id, name.clone(), Read);
        }
        for name in write_set.iter() {
            self.tell_to_name(
                &name,
                Msg::LockRequest {
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_write(txn_id.clone()),
                },
            )
            .await?;
            self.add_request_lock(&txn_id, name.clone(), Write);
        }

        Ok(())
    }

    async fn abort_locks(
        &mut self,
        txn_id: &TxnId,
        read_set: &HashSet<String>,
        write_set: &HashSet<String>,
    ) -> Result<(), Box<dyn Error>> {
        for name in read_set.iter() {
            self.tell_to_name(
                &name,
                Msg::LockAbort {
                    from_name: self.name.clone(),
                    lock: Lock {
                        lock_kind: Read,
                        txn_id: txn_id.clone(),
                    },
                },
            )
            .await?;
        }
        for name in write_set.iter() {
            self.tell_to_name(
                &name,
                Msg::LockAbort {
                    from_name: self.name.clone(),
                    lock: Lock {
                        lock_kind: Write,
                        txn_id: txn_id.clone(),
                    },
                },
            )
            .await?;
        }

        return Ok(());
    }
}
