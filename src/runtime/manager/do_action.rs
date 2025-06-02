use std::error::Error;

use crate::{
    ast::Assn, 
    runtime::{evaluator::eval_assns, lock::{Lock, LockKind::{Read, Write}}, message::Msg, transaction::TxnId}, 
    static_analysis::var_analysis::read_write::{calc_read_set, calc_write_set}
};

use super::{Manager, TxnManager};


impl Manager {
    pub async fn do_action(&mut self, txn_id: TxnId, assns: &Vec<Assn>) -> Result<(), Box<dyn Error>> {
        // static info of txn, the read and write set
        let mut read_set = calc_read_set(assns);
        let write_set = calc_write_set(assns);
        read_set.retain(|x| !&write_set.contains(x)); // exclude write locks from read locks 

        // set up txn manager 
        let mgr_addr = self.address.clone()
        .expect("manager addr should not be None");

        self.new_txn_mgr(txn_id.clone());


        // send lock requests
        for name in read_set.iter() { 
            self.tell_to_name(&name, 
                Msg::LockRequest { 
                    from_mgr_addr: mgr_addr.clone(), 
                    lock: Lock::new_read(txn_id.clone())
                }, 
            ).await?;
            self.add_request_lock(&txn_id, name.clone(), Read);
        }
        for name in write_set.iter() {
            self.tell_to_name(&name, 
                Msg::LockRequest { 
                    from_mgr_addr: mgr_addr.clone(),
                    lock: Lock::new_write(txn_id.clone())
                }, 
            ).await?;
            self.add_request_lock(&txn_id,name.clone(), Write);
        }

        // wait for all locks to be granted or any lock to be aborted
        while !(self.all_lock_granted(&txn_id) && self.any_lock_aborted(&txn_id)) {
            continue;
        }
        if self.any_lock_aborted(&txn_id) {
            return Ok(())
        }
        assert!(self.all_lock_granted(&txn_id));

        // wait for all reads to finish 
        while !self.all_read_finished(&txn_id) {
            continue;
        }
        assert!(self.all_read_finished(&txn_id));

        // re-eval all src of assn 
        let mut assns = assns.clone();
        let env = self.get_read_results(&txn_id);
        eval_assns(&mut assns, env);

        // send write request  
        for Assn { dest, src } in assns {
            // for right behavior, hand write request synchronously
            // avoiding LockRelease received before UsrWriteVarRequest
            self.ask_to_name(&dest, 
                Msg::UsrWriteVarRequest { 
                    txn: txn_id.clone(), 
                    write_val: src,
                }).await?;
        }

        // finish up then release
        let preds = self.get_preds(&txn_id);
        for name in read_set.iter() {
            self.tell_to_name(&name, 
                Msg::LockRelease { txn: txn_id.clone(), preds: preds.clone() }, 
            ).await?;
        }
        for name in write_set.iter() {
            self.tell_to_name(&name, 
                Msg::LockRelease { txn: txn_id.clone(), preds: preds.clone() }, 
            ).await?;
        }
        
        Ok(())
    }
}