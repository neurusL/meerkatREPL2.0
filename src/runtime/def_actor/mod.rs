use std::{collections::HashSet, hash::Hash};

use kameo::Actor;

use crate::ast::Expr;

use super::lock::LockState;
use super::pubsub::PubSub;
use super::transaction::Txn;

pub mod handler;
pub mod manager;
pub mod pending;
pub mod history;

#[derive(Actor)]
pub struct DefActor {
    pub name: String,
    pub value: Expr,

    pub pubsub: PubSub,
    pub lock_state: LockState,
}


/// internal representation of a prop change received by a def actor
type ChangeId = i64;
#[derive(Eq, Clone, Debug)]
pub struct PropChange {
    pub id: ChangeId,
    pub from_name: String,
    pub new_val: Expr,
    pub preds: HashSet<Txn>,
}

impl PartialEq for PropChange {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for PropChange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for PropChange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for PropChange {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
