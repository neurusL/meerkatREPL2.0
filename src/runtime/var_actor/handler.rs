//! Logic for Var Actor
//!


use std::time::Duration;

use kameo::{error::Infallible, prelude::*};
use kameo::mailbox::Signal;

use super::VarActor;
use crate::runtime::{lock::Lock, message::Msg};

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

impl kameo::prelude::Message<Msg> for VarActor {
    type Reply = Option<Msg>;

    async fn handle(
        &mut self,
        msg: Msg,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            Msg::Subscribe {
                from_name: _,
                from_addr,
            } => {
                self.pubsub.subscribe(from_addr);
                Some(Msg::SubscribeGranted)
            }

            Msg::LockRequest {
                lock,
                from_mgr_addr: from_name,
            } => {
                if !self.lock_state.add_wait(lock.clone(), from_name) {
                    return Some(Msg::LockAbort {
                        from_name: self.name.clone(),
                        lock,
                    });
                }

                Some(Msg::LockGranted { from_name: self.name.clone(), lock })
            }

            Msg::LockAbort { lock, ..} => {
                self.lock_state.remove_granted_or_wait(&lock.txn_id);

                // roll back to previous stable state of value
                // unconfirmed write has same txn as aborted
                self.value.roll_back_if_relevant(&lock.txn_id);

                None
            }

            Msg::LockRelease { txn, mut preds } => {
                assert!(self.lock_state.has_granted(&txn.id));
                let lock = self.lock_state
                    .remove_granted_or_wait(&txn.id)
                    .expect("lock should be granted before release");

                // if lock is read then nothing else to do
                // else if lock is write:
                if lock.is_write() {
                    let (new_value, unconfirmed_txn) = self
                        .value
                        .confirm_update()
                        .expect("should have unconfirmed value update");
                    assert!(unconfirmed_txn == txn.id);

                    self.latest_write_txn = Some(txn.clone());

                    // except for preds calculated by manager 
                    // the txn itself should also have been applied when 
                    // value is updated
                    preds.insert(txn.clone());

                    self.pubsub.publish(Msg::PropChange {
                        from_name: self.name.clone(),
                        val: new_value,
                        preds: preds.clone(),
                    }).await;
                }

                None
            }

            Msg::UsrReadVarRequest { txn } => {
                assert!(self.lock_state.has_granted(&txn));

                // remove read lock immediately
                self.lock_state.remove_granted_if_read(&txn);

                Some(Msg::UsrReadVarResult {
                    txn,
                    name: self.name.clone(),
                    result: self.value.clone().into(),
                    pred: self.latest_write_txn.clone(),
                })
            }

            Msg::UsrWriteVarRequest { txn, write_val } => {
                assert!(self.lock_state.has_granted_write(&txn));

                self.value.update(write_val, txn);

                None
            }

            #[allow(unreachable_patterns)]
            _ => panic!("VarActor should not receive message {:?}", msg),
        }
    }
}

impl Actor for VarActor {
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
                    let _ = self.tick().await;
                }
            }
            // println!("[{}] ticked, now value is {:?}", self.name, self.value);
        }
    }
}

impl VarActor {
    async fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // if can grant new waiting lock
        if let Some((lock, mgr)) = self.lock_state.grant_oldest_wait() {
            println!("grant lock: {:?} to {:?}", lock, mgr);
            let msg = Msg::LockGranted {
                from_name: self.name.clone(),
                lock,
            };

            mgr.tell(msg).await?;
            println!("message is sent");
        }
        Ok(())
    }
}
