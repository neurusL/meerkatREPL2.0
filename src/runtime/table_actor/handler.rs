use std::collections::HashSet;
use std::rc::Weak;
use std::time::Duration;

use std::collections::HashMap;
use crate::runtime::table_actor::state::TableValueState;
use crate::runtime::Expr;
use kameo::mailbox::Signal;
use kameo::{error::Infallible, prelude::*};
use log::info;

use super::{TableActor};
use crate::runtime::evaluator;
use crate::runtime::{message::Msg, evaluator::Evaluator};

pub const TICK_INTERVAL: Duration = Duration::from_millis(100);

impl kameo::prelude::Message<Msg> for TableActor {
  type Reply = Msg;

  async fn handle(
    &mut self,
    msg: Msg,
    _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
  ) -> Self::Reply {
    info!("Table Actor {} Receive: ", self.name);
    
    match msg {
      Msg::Subscribe {
        from_name: _,
        from_addr,
      } => {
        self.pubsub.subscribe(from_addr);
        Msg::SubscribeGranted {
          name: self.name.clone(),
          value: self.value.clone().into(),
          preds: self
          .latest_write_txn
          .clone()
          .map_or_else(|| HashSet::new(), |txn| HashSet::from([txn])),
        }
      }

      Msg::UserReadTableRequest {
        txn,
        table_name,
        name,
        where_clause,
      } => {
        Msg::UserReadTableResult { 
          txn,
          name: table_name,
          result: self.value.clone().into(),
          pred: self.latest_write_txn.clone(),
        }
      }

      Msg::UserWriteTableRequest { 
        from_mgr_addr,
        txn,
        insert } => {
          info!("Table Actor {} inserting row", self.name);
          
          //self.value.update(new_val, txn_id: txn);   // using update would be more efficient
          match &mut self.value {
            TableValueState::Val(Expr::Table { name, rows }) => {
              rows.push(insert.row.clone());
            }
            _ => panic!("Table is not in a valid state for insertion"),
          }

          //self.latest_write_txn = Some(txn);
          from_mgr_addr
          .tell(Msg::UserWriteTableFinish { txn: txn, name: self.name.clone() })
          .await.unwrap();
          info!("Sent UserTableFinish to manager");
          Msg::Unit
        } 
      _ => Msg::Unit,
    }
  }
}

impl Actor for TableActor {
    type Error = Infallible;

    async fn next(
      &mut self,
      _actor_ref: WeakActorRef<Self>,
      mailbox_rx: &mut MailboxReceiver<Self>,
    ) -> Option<Signal<Self>> {
      let mut interval = tokio::time::interval(TICK_INTERVAL);

      loop {
        tokio::select! {
          maybe_signal = mailbox_rx.recv() => {
            info!("TableActor {} received signal: ", self.name);
            return maybe_signal;
          }
          _ = interval.tick() => {
            info!("{} has value {:?}", self.name, self.value);
            let _ = self.tick().await;

          }        
        }
      }
    }
}

impl TableActor {
  async fn tick(&mut self,) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
  }
}