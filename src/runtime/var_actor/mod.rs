use std::collections::HashSet;

use kameo::Actor;
use kameo::prelude::*;

use crate::ast::Expr;
use super::lock::LockState;
use super::pubsub::PubSub;
use super::transaction::TxnId;

pub mod value_state;
pub mod handler;


#[derive(Actor)]
pub struct VarActor {
    pub name: String,                // this actor's var name 
    pub address: ActorRef<VarActor>, // this actor's address

    pub pubsub: PubSub,

    pub value: value_state::VarValueState,

    pub latest_write_txn: Option<TxnId>,

    pub lock_state: LockState,
}
