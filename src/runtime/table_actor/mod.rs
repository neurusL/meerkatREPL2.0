use std::collections::HashSet;

use kameo::prelude::*;
use kameo::Actor;

use super::lock::LockState;
use super::pubsub::PubSub;
use super::transaction::Txn;
use crate::ast::Expr;

pub mod handler;
pub mod state;

#[derive(Actor)]
pub struct TableActor {
    pub name: String, 
    pub value: state::TableValueState,

    pub pubsub: PubSub,

    pub latest_write_txn: Option<Txn>,
}

impl TableActor {
    pub fn new(name: String, val: Expr) -> TableActor {
        TableActor {
            name,
            value: state::TableValueState::new(val),
            pubsub: PubSub::new(),
            latest_write_txn: None,
        }
    }
}

