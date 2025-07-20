use kameo::actor::ActorRef;
use std::collections::HashMap;
use std::collections::HashSet;

use super::pubsub::PubSub;
use crate::ast::Expr;
use crate::runtime::manager::Manager;
use crate::runtime::transaction::{TxnId, TxnPred};
use crate::runtime::TestId;
use state::ChangeState;

pub mod handler;
pub mod state;

// pub type TickFunc = Box<
//     dyn for<'a> FnMut(
//             &'a mut DefActor,
//         ) -> Pin<Box
//                 <dyn Future<Output = Result<(), Box<dyn Error + Send>>> + Send + 'a>
//             >
//             + Send + 'static,
// >;

pub struct DefActor {
    pub name: String,
    pub value: Expr, // expr of def
    pub is_assert_actor_of: Option<(TestId, ActorRef<Manager>, Vec<TxnPred>)>,

    pub pubsub: PubSub,
    // pub lock_state: LockState,
    pub read_requests: HashMap<TxnId, (ActorRef<Manager>, Vec<TxnId>)>,

    pub state: ChangeState,
}

impl DefActor {
    pub fn new(
        name: String,
        expr: Expr,                                    // def's expr
        value: Expr,                                   // def's initialized value
        arg_to_values: HashMap<String, Expr>,          // def's args to their initialized values
        arg_to_vars: HashMap<String, HashSet<String>>, // args to their transitively dependent vars
        // if arg itself is var, then arg_to_vars[arg] = {arg}
        testid_and_manager_and_txns: Option<(TestId, ActorRef<Manager>, Vec<TxnPred>)>,
    ) -> DefActor {
        DefActor {
            name,
            value,
            is_assert_actor_of: testid_and_manager_and_txns,
            pubsub: PubSub::new(),
            // lock_state: LockState::new(),
            read_requests: HashMap::new(),
            state: ChangeState::new(expr, arg_to_values, arg_to_vars),
        }
    }
}
