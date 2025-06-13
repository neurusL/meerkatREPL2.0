use core::panic;
use std::time::Duration;
use log::info;
use std::{collections::HashSet, error::Error};

use crate::runtime::message::{Msg, CmdMsg};
use kameo::{error::Infallible, prelude::*};

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

use super::Manager;
use super::txn_utils::*;

/// message between manager and REPL
impl kameo::prelude::Message<CmdMsg> for Manager {
    type Reply = Option<CmdMsg>;

    async fn handle(&mut self, msg: CmdMsg, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        use CmdMsg::*;
        info!("MANAGER {} RECEIVE form Command Line: ", self.name);
        match msg {
            TryAssert { name, test: bool_expr } =>  { info!("Try Test");
                let actor_ref = self.alloc_def_actor(
                        &format!("{}_assert_{}", name, bool_expr),
                        bool_expr.clone()
                ).await.unwrap();
                
                Some(ListenToAssert { actor: actor_ref })
            }

            DoTransaction { txn } => { info!("Do Action");
                let txn_id = txn.id.clone();
                let txn_mgr = self.new_txn(txn);
                self.txn_mgrs.insert(txn_id.clone(), txn_mgr);

                // request locks
                self.request_locks(&txn_id).await;

                None
            }

            TransactionAborted { txn_id } => { info!("Transaction Aborted");
                Some(TransactionAborted { txn_id })
            }

            CodeUpdate { srv } => { info!("Code Update");
                self.alloc_service(&srv).await;
                Some(CodeUpdateGranted)
            }
            _ => {
                panic!("Manager should not receive message from REPL");
            }
        }
    }
}

/// message between manager and var/def actors
impl kameo::prelude::Message<Msg> for Manager {
    type Reply = Msg;

    async fn handle(&mut self, msg: Msg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        info!("MANAGER {} RECEIVE: ", self.name);
        match msg {
            Msg::LockGranted { from_name, lock } => { info!("Lock Granted");
                self.add_grant_lock(&lock.txn_id, from_name, lock.lock_kind);
                if self.all_lock_granted(&lock.txn_id) {
                    self.request_reads(&lock.txn_id).await;
                }
                
                Msg::Unit
            }

            Msg::UsrReadVarResult {
                txn: txn_id,
                name,
                result,
                pred,
            } => { info!("UsrReadVarResult");
                if self.is_aborted(&txn_id) {
                    return Msg::Unit
                }

                let pred = pred.map_or_else(
                    || HashSet::new(), 
                    |p| HashSet::from([p])
                );

                self.add_finished_read(&txn_id, name, result, pred);

                if self.all_read_finished(&txn_id) {
                    self.reeval_and_request_writes(&txn_id).await;
                    // todo!() current impl isn't optimized for best concurrency
                    // if re-eval block for too long 
                    // feel free to spawn a new thread 
                    // or put it in tick()

                }
                Msg::Unit
            }
            Msg::UsrReadDefResult {
                txn: txn_id,
                name,
                result,
                preds,
            } => { info!("UsrReadDefResult");
                if self.is_aborted(&txn_id) {
                    return Msg::Unit
                }

                self.add_finished_read(&txn_id, name, result, preds);

                if self.all_read_finished(&txn_id) {
                    self.reeval_and_request_writes(&txn_id).await;
                    // todo!() same above
                }
                Msg::Unit
            }

            Msg::LockAbort { from_name: _, lock } => { info!("Lock Abort");
                self.request_abort_locks(&lock.txn_id).await;
                self.abort_lock(&lock.txn_id); // turn all txn's lock state to aborted 

                // notify another mailbox of manager myself
                _ctx.actor_ref().tell(
                    CmdMsg::TransactionAborted { txn_id: lock.txn_id }
                ).await;

                Msg::Unit
            }
            _ => panic!("Manager should not receive message from var/def actors"),
        }
    }
}

impl Actor for Manager {
    type Error = Infallible;

    /// customized on_start impl of Actor trait    
    /// to allow manager self reference to its addr
    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        info!("MANAGER on_start got ActorRef with id {}", actor_ref.id());
        self.address = Some(actor_ref.clone());
        Ok(())
    }
}

impl Manager {
    pub async fn tell_to_name(&self, name: &String, msg: Msg) -> Result<(), Box<dyn Error>> {
        if let Some(actor) = self.varname_to_actors.get(name) {
            actor.tell(msg).await?;
        } else if let Some(actor) = self.defname_to_actors.get(name) {
            actor.tell(msg).await?;
        } else {
            panic!("Service alloc: no such var or def with name: {}", name);
        }

        Ok(())
    }

    pub async fn ask_to_name(
        &self,
        name: &String,
        msg: Msg,
    ) -> Result<Msg, Box<dyn Error>> {
        let back_msg = if let Some(actor) = self.varname_to_actors.get(name) {
            actor.ask(msg).await?
        } else if let Some(actor) = self.defname_to_actors.get(name) {
            actor.ask(msg).await?
        } else {
            panic!("Service alloc: no such var or def with name: {}", name);
        };

        Ok(back_msg)
    }

    pub async fn tell_to_name2(
        &self,
        name: &String,
        msg_var: Msg,
        msg_def: Msg,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(actor) = self.varname_to_actors.get(name) {
            actor.tell(msg_var).await?;
        } else if let Some(actor) = self.defname_to_actors.get(name) {
            actor.tell(msg_def).await?;
        } else {
            panic!("Service alloc: no such var or def with name: {}", name);
        }

        Ok(())
    }
}
