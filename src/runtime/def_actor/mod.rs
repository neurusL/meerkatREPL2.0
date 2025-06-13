use kameo::Actor;
use std::collections::HashMap;
use std::{collections::HashSet, hash::Hash};

use super::lock::LockState;
use super::pubsub::PubSub;
use crate::ast::Expr;
use state::ChangeState;

pub mod handler;
pub mod state;

pub struct DefActor {
    pub name: String,
    pub value: Expr, // expr of def

    pub pubsub: PubSub,
    pub lock_state: LockState,

    pub state: ChangeState,
}

impl DefActor {
    pub fn new(
        name: String,
        expr: Expr,                                    // def's expr
        value: Expr,                                   // def's initialized value
        arg_to_values: HashMap<String, Expr>,          // def's args to their initialized values
        arg_to_vars: HashMap<String, HashSet<String>>, // args to their transitively dependent vars
    ) -> DefActor {
        DefActor {
            name,
            value,
            pubsub: PubSub::new(),
            lock_state: LockState::new(),
            state: ChangeState::new(expr, arg_to_values, arg_to_vars),
        }
    }
}
