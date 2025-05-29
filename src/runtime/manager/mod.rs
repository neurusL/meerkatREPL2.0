use kameo::Actor;
use kameo::prelude::*;

use super::message::Msg;


#[derive(Actor)]
pub struct Manager {
}

impl kameo::prelude::Message<Msg> for Manager {
    type Reply = Option<Msg>;

    async fn handle(
        &mut self,
        msg: Msg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        todo!("implement me!")
    }
}