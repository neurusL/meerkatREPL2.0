//! Logic for Var Actor
//! 

use std::collections::HashSet;

use tokio::runtime::Handle;

use crate::runtime:: message::Msg;
use super::VarActor;

impl kameo::prelude::Message<Msg> for VarActor {
    type Reply = Option<Msg>;

    async fn handle(
        &mut self,
        msg: Msg,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            Msg::Subscribe { from_name: _, from_addr } => {
                self.pubsub.subscribe(from_addr);
                None
            }

            Msg::LockRequest { lock, from_mgr_addr: from_name } => {
                if !self.lock_state.add_wait(lock.clone(), from_name) {
                    return Some(Msg::LockAbort { 
                        from_name: self.name.clone(),
                        txn: lock.txn_id
                    });
                } 

                None
            },

            Msg::LockAbort { from_name: _, txn } => {
                self.lock_state.remove_granted_or_wait(&txn);

                // roll back to previous stable state of value 
                self.value.roll_back();

                None
            }

            Msg::LockRelease { txn, preds } => {
                assert!(self.lock_state.has_granted(&txn));
                let lock = self.lock_state
                .remove_granted_or_wait(&txn)
                .expect("lock should be granted before release");

                // confirm the updated value
                if let Some(new_value) = self.value.confirm_update() {
                    self.pubsub.publish(Msg::Change {
                        from_name: self.name.clone(),
                        val: new_value,
                        preds: preds.clone(),
                    })
                }

                // clear pred set, and current txn should be the pred for next txn
                if lock.is_write() { self.latest_write_txn = Some(txn.clone()); }

                None
            },

            Msg::UsrReadVarRequest { txn } => {
                assert!(self.lock_state.has_granted(&txn));

                // remove read lock immediately
                self.lock_state.remove_granted_if_read(&txn);

                Some(Msg::UsrReadVarResult {
                    txn,
                    var_name: self.name.clone(),
                    result: self.value.clone().into(),
                    pred: self.latest_write_txn.clone(),
                })
            },

            Msg::UsrWriteVarRequest { txn, write_val } => {
                assert!(self.lock_state.has_granted_write(&txn));

                self.value.update(write_val);

                None
            },

            #[allow(unreachable_patterns)]
            _ => panic!("VarActor should not receive message {:?}", msg),
        }
        
    }
}

impl VarActor {
    async fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // if can grant new waiting lock 
        if let Some(mgr) = self.lock_state.grant_oldest_wait() {
            let msg = Msg::LockGranted {
                from_name: self.name.clone(),
                txn: self.lock_state.oldest_granted_lock_txnid.clone().unwrap(),
            };

            mgr.tell(msg).await?;
        }
        Ok(())
    }
}