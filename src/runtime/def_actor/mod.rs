use kameo::Actor;
use kameo::prelude::*;

use crate::ast::Expr;

use super::lock::LockState;
use super::message::Msg;
use super::pubsub::PubSub;


#[derive(Actor)]
pub struct DefActor {
    pub name: String,
    pub value: Expr,

    pub pubsub: PubSub,
    pub lock_state: LockState,
}

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

    async fn handle(
        &mut self,
        msg: Msg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        todo!("implement me!")
    }
}