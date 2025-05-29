use core::panic;
use std::collections::HashSet;

use kameo::Actor;
use kameo::prelude::*;

// use kameo_actors::pubsub::{PubSub, Publish, Subscribe};

use crate::ast::Expr;

use super::lock::LockState;
use super::message::Msg;
use super::transaction::TxnId;

#[derive(Debug, Clone)]
pub enum VarValueState {
    Uninit,            // uninitialized
    Val(Expr),         // stable state of a var actor value
    Trans(Option<Expr>, Expr), // when receive write request, var actor is in transition
}

impl VarValueState {
    pub fn update(&mut self, new_val: Expr) {
        use self::VarValueState::*;
        match self {
            VarValueState::Uninit => *self = Trans(None, new_val),
            VarValueState::Val(expr) => *self = Trans(Some(expr.clone()), new_val),
            VarValueState::Trans(_, _) => panic!("unrosolved transient state"),
        }
    }
    pub fn confirm_update(&mut self) {
        if let VarValueState::Trans(_, new_val) = self {
            *self = VarValueState::Val(new_val.clone());
        }
    }

    pub fn roll_back(&mut self) {
        if let VarValueState::Trans(old_val, _) = self {
            if let Some(val) = old_val {
                *self = VarValueState::Val(val.clone());
            } else {
                *self = VarValueState::Uninit;
            }
        }
    }
}

impl Into<Option<Expr>> for VarValueState {
    fn into(self) -> Option<Expr> {
        match self {
            VarValueState::Uninit => None,
            VarValueState::Val(val) => Some(val),
            VarValueState::Trans(_, _) => panic!("transient var value"),
        }
    }
}

#[derive(Actor)]
pub struct VarActor {
    pub name: String,

    pub value: VarValueState,
    pub latest_write_txn: Option<Expr>,
    pub preds: HashSet<TxnId>, 

    pub lock_state: LockState,
}

impl kameo::prelude::Message<Msg> for VarActor {
    type Reply = Option<Msg>;

    async fn handle(
        &mut self,
        msg: Msg,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            Msg::LockRequest { lock } => {
                if !self.lock_state.add_wait(lock.clone()) {
                    return Some(Msg::LockAbort { 
                        from_name: self.name.clone(),
                        txn: lock.txn_id
                    });
                } else {
                    return Some(Msg::LockGranted { 
                        from_name: self.name.clone(),
                        txn: lock.txn_id
                     });
                }
            },
            Msg::LockAbort { from_name: _, txn } => {
                self.lock_state.remove_granted_or_wait(&txn);

                // roll back to previous stable state of value 
                self.value.roll_back();

                None
            }
            Msg::LockRelease { txn, preds } => {
                assert!(self.lock_state.has_granted(&txn));
                self.lock_state.remove_granted_or_wait(&txn);

                self.preds.extend(preds);

                // confirm the updated value
                self.value.confirm_update();

                None
            },

            Msg::UsrReadVarRequest { txn } => {
                assert!(self.lock_state.has_granted(&txn));

                self.preds.insert(txn.clone());

                // remove read lock immediately
                self.lock_state.remove_granted_if_read(&txn);

                Some(Msg::UsrReadVarResult {
                    txn,
                    var_name: self.name.clone(),
                    result: self.value.clone().into(),
                    result_preds: self.preds.clone(),
                })
            },

            Msg::UsrWriteVarRequest { txn, write_val, requires } => {
                assert!(self.lock_state.has_granted_write(&txn));

                self.value.update(write_val);
                None
            },

            #[allow(unreachable_patterns)]
            _ => panic!("VarActor should not receive message {:?}", msg),
        }
        
    }
}