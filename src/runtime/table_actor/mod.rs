use std::collections::HashMap;
use std::collections::HashSet;

use kameo::prelude::*;
use kameo::Actor;

use super::lock::LockState;
use super::pubsub::PubSub;
use super::transaction::Txn;
use crate::ast::{Expr, Field};

pub mod handler;
pub mod state;

#[derive(Actor)]
pub struct TableActor {
    pub name: String, 
    pub value: state::TableValueState,
    pub schema: Vec<Field>,
    pub pubsub: PubSub,

    pub latest_write_txn: Option<Txn>,
    //pub preds: HashSet<Txn>,
}

impl TableActor {
    pub fn new(name: String, val: Expr, schema: Vec<Field> ) -> TableActor {
        TableActor {
            name,
            value: state::TableValueState::new(val),
            schema,
            pubsub: PubSub::new(),
            latest_write_txn: None,
            //preds: HashSet::new(),
        }
    }
}

