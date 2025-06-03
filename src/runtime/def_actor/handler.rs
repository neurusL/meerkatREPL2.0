use kameo::prelude::*;
use crate::{ast::Expr, runtime::{lock::LockState, message::Msg, pubsub::PubSub}};

use super::DefActor;

impl DefActor {
    pub fn new(name: String, val: Expr) -> DefActor {
        DefActor {
            name,
            value: val,
            pubsub: PubSub::new(),
            lock_state: LockState::new(),
        }
    }
}

impl kameo::prelude::Message<Msg> for DefActor {
    type Reply = Option<Msg>;

    async fn handle(&mut self, msg: Msg, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        todo!("implement me!")
    }
}
