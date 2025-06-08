use core::panic;
use std::{collections::HashSet, error::Error, sync::Condvar};

use crate::runtime::message::Msg;
use kameo::prelude::*;

use super::{Manager, TxnManager};

impl kameo::prelude::Message<Msg> for Manager {
    type Reply = Option<Msg>;

    async fn handle(&mut self, msg: Msg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        println!(">>>>>>>>>>> receive >>>>>>>>>> {:?}", msg);
        match msg {
            Msg::TryTest { test } =>  {
                let _ = self.try_test(test).await;
                Some(Msg::TryTestPass)
            }

            Msg::CodeUpdate { srv } => {
                self.alloc_service(&srv).await;
                Some(Msg::CodeUpdateGranted)
            }

            Msg::LockGranted { from_name, lock } => {
                println!("receive lock granted from {}", from_name);

                self.get_mut_txn_mgr(&lock.txn_id)
                    .add_grant_lock(from_name, lock.lock_kind);
                
                None
            }

            Msg::UsrReadVarResult {
                txn,
                name,
                result,
                pred,
            } => {
                let pred = if let Some(p) = pred {
                    HashSet::from([p])
                } else {
                    HashSet::new()
                };

                self.add_finished_read(&txn, name, result, pred);
                None
            }
            Msg::UsrReadDefResult {
                txn,
                name,
                result,
                preds,
            } => {
                self.add_finished_read(&txn, name, result, preds);
                None
            }
            Msg::LockAbort { from_name, lock } => {
                self.add_abort_lock(&lock.txn_id, from_name, lock.lock_kind);

                None
            }
            _ => panic!("Manager: receive wrong message type"),
        }
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
    ) -> Result<Option<Msg>, Box<dyn Error>> {
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
