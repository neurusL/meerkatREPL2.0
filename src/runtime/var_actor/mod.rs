use std::collections::HashSet;

use kameo::Actor;
use kameo::prelude::*;

// use kameo_actors::pubsub::{PubSub, Publish, Subscribe};

use crate::ast::Expr;

use super::lock::LockState;
use super::message::Msg;

pub enum VarValueState {
    Uninit,
    Val(Expr),
    Trans(Expr, Expr),
}

#[derive(Actor)]
pub struct VarActor {
    pub name: String,

    pub value: VarValueState,
    pub latest_write_txn: Option<Expr>,
    pub pred_txns: HashSet<Expr>, 

    pub lock_state: LockState,
}

impl kameo::prelude::Message<Msg> for VarActor {
    type Reply = Option<Msg>;

    async fn handle(
        &mut self,
        msg: Msg,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            Msg::LockRequest { lock } => {
                if !self.lock_state.add_wait(lock.clone()) {
                    return Some(Msg::LockAbort { lock });
                } else {
                    return Some(Msg::LockGranted { 
                        from_name: self.name.clone(),
                        lock
                     });
                }
            },

            Msg::LockRelease { lock } => {
                self.lock_state.remove_granted_or_wait(&lock);
                todo!("more logic to be added");
                None
            },

            Msg::LockGranted {..} => { 
                None
             },
            Msg::LockAbort {..} => {
                panic!("VarActor should not receive LockAbort message");
            },


            Msg::UsrReadVarRequest { txn } => todo!(),
            Msg::UsrReadVarResult { txn, var_name, result, result_provides } => todo!(),
            Msg::UsrWriteVarRequest { txn, write_val, requires } => todo!(),
            Msg::UsrReadDefRequest { txn, requires } => todo!(),
            Msg::UsrReadDefResult { txn, name, result, result_provides } => todo!(),

            _ => todo!("unit shouldn't be handled, dev update messages to be implementedf"),
        }
        
    }
}