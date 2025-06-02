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
    pub value: value_state::VarValueState,

    pub pubsub: PubSub,
    pub lock_state: LockState,

    pub latest_write_txn: Option<TxnId>,
}

impl VarActor {
    pub fn new(name: String, val: Expr) -> VarActor {
        VarActor {
            name,
            value: value_state::VarValueState::new(val),
            pubsub: PubSub::new(),
            lock_state: LockState::new(),
            latest_write_txn: None,
        }
    }
}

