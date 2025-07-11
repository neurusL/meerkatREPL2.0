use std::collections::HashSet;
use std::rc::Weak;
use std::time::Duration;

use std::collections::HashMap;
use crate::runtime::table_actor::state::TableValueState;
use crate::runtime::transaction::Txn;
use crate::runtime::Expr;
use kameo::mailbox::Signal;
use kameo::{ error::Infallible, prelude::* };
use log::info;

use super::{ TableActor };
use crate::runtime::evaluator;
use crate::ast::Record;
use crate::runtime::{ message::Msg, evaluator::Evaluator };

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

impl kameo::prelude::Message<Msg> for TableActor {
    type Reply = Msg;

    async fn handle(
        &mut self,
        msg: Msg,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>
    ) -> Self::Reply {
        info!("Table Actor {} Receive: ", self.name);

        match msg {
            Msg::Subscribe { from_name: _, from_addr } => {
                info!("Subsribe from {:?}", from_addr);
                self.pubsub.subscribe(from_addr);
                Msg::SubscribeGranted {
                    name: self.name.clone(),
                    value: self.value.clone().into(),
                    preds: self.latest_write_txn.clone().map_or_else(
                        || HashSet::new(),
                        |txn| HashSet::from([txn])
                    ),
                    schema: self.schema.clone()
                }
            }

            Msg::UserReadTableRequest { txn, table_name, .. } => {
                Msg::UserReadTableResult {
                    txn,
                    name: table_name,
                    result: self.value.clone().into(),
                    pred: self.latest_write_txn.clone(),
                }
            }

            Msg::UserWriteTableRequest { from_mgr_addr, txn, insert } => {
                info!("Table Actor {} inserting row", self.name);

                self.value.update(&insert); 

                let curr_txn = Txn {
                  id: txn.clone(),
                  assns: vec![],
                  inserts: vec![insert.clone()],
                };
                //self.latest_write_txn = Some(txn.clone());
                //self.preds.insert(curr_txn);
                self.pubsub.publish(Msg::PropChange {
                    from_name: self.name.clone(),
                    val: self.value.clone().into(),
                    preds: HashSet::from([curr_txn]),
                    schema: self.schema.clone(),
                }).await;
                info!("Prop change message sent to subscribers");
                
                
    
                from_mgr_addr
                    .tell(Msg::UserWriteTableFinish { txn: txn.clone(), name: self.name.clone() }).await
                    .unwrap();
                info!("Sent UserWriteTableFinish to manager");
                Msg::Unit
            }
            _ => Msg::Unit,
        }
    }
}
