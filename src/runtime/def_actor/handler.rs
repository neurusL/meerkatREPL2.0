use std::collections::{HashMap, HashSet};
use std::time::Duration;

use kameo::{error::Infallible, prelude::*};
use kameo::mailbox::Signal;

use crate::runtime::{lock, message::Msg};

use super::DefActor;

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);


impl kameo::prelude::Message<Msg> for DefActor {
    type Reply = Option<Msg>;

    async fn handle(&mut self, msg: Msg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        match msg {
            Msg::Subscribe { from_addr, .. } => {
                self.pubsub.subscribe(from_addr);
                Some(Msg::SubscribeGranted)
            },
            Msg::SubscribeGranted => { None },

            Msg::LockRequest { 
                lock,
                from_mgr_addr 
            } => {
                if !self.lock_state.add_wait(lock.clone(), from_mgr_addr) {
                    return Some(Msg::LockAbort {
                        from_name: self.name.clone(),
                        lock,
                    });
                }
                None
            },

            Msg::LockRelease { txn, .. } => {
                assert!(self.lock_state.has_granted(&txn.id));

                let lock = self.lock_state
                    .remove_granted_or_wait(&txn.id)
                    .expect("lock should be granted before release");

                assert!(lock.is_read());
                None
            },

            Msg::LockAbort { lock, .. } => {
                self.lock_state.remove_granted_or_wait(&lock.txn_id);
                None
            },

            Msg::UsrReadDefRequest { txn } => {
                assert!(self.lock_state.has_granted(&txn));

                // remove read lock immediately
                self.lock_state.remove_granted_if_read(&txn);
                
                Some(Msg::UsrReadDefResult {
                    txn,
                    name: self.name.clone(),
                    result: self.value.clone().into(),
                    preds: self.state.get_all_applied_txns(), // todo!("switch to undropped txns later")
                })
            },

            Msg::PropChange { from_name, val, preds } => {
                self.state.receive_change(from_name, val, preds);
                None
            },

            _ => panic!("DefActor: unexpected message: {:?}", msg),
        }
    }
}

impl Actor for DefActor {
    type Error = Infallible;

    async fn next(
            &mut self,
            _actor_ref: WeakActorRef<Self>,
            mailbox_rx: &mut MailboxReceiver<Self>,
        ) -> Option<Signal<Self>> {
        let mut interval = tokio::time::interval(TICK_INTERVAL);

        loop {
            tokio::select! {
                // if a real message waiting, return immediately:
                maybe_signal = mailbox_rx.recv() => {
                    return maybe_signal;
                }

                // else, every 100 ms ticks
                _ = interval.tick() => {
                    if let Err(e) = self.tick().await {
                        eprintln!("[{}] tick() failed: {:?}", self.name, e);
                    }
                }
            }
        }
    }
}

impl DefActor {
    async fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // if can grant new waiting lock
        if let Some((lock, mgr)) = self.lock_state.grant_oldest_wait() {
            let msg = Msg::LockGranted {
                from_name: self.name.clone(),
                lock,
            };

            mgr.tell(msg).await?;
        }

        // if we search for new batch of changes 
        let changes = self.state.search_batch();
        if changes.len() > 0 {
            self.value = self.state.apply_batch(&changes);
            let preds = self.state.get_preds_of_changes(&changes);
            
            let msg = Msg::PropChange {
                from_name: self.name.clone(),
                val: self.value.clone().into(),
                preds,
            };
            self.pubsub.publish(msg).await;
        }

        Ok(())
    }
}