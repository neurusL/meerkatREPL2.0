use std::collections::HashSet;

use kameo::{actor::ActorRef, Reply};

use crate::{
    ast::{Expr, Prog},
    runtime::{lock::Lock, transaction::TxnId},
};

use super::{def_actor::DefActor, manager::Manager, var_actor::VarActor};


#[derive(Debug, Clone, Reply)]
pub enum Msg {
    UsrReadVarRequest {
        txn: TxnId,
    },
    UsrReadVarResult {
        txn: TxnId,
        name: String,
        result: Expr,
        pred: Option<TxnId>,
    },
    UsrWriteVarRequest {
        txn: TxnId,
        write_val: Expr,
        // requires: HashSet<TxnId>,
    },

    UsrReadDefRequest {
        txn: TxnId,
        // requires: HashSet<TxnId>, // ?? Why we need this
    },
    UsrReadDefResult {
        txn: TxnId,
        name: String,
        result: Expr,
        preds: HashSet<TxnId>,
    },

    // Propagate {
    //     propa_change: PropaChange, // a small change, make batch valid easier
    // },

    LockRequest { // for notifying var/def that a lock is requested
        from_mgr_addr: ActorRef<Manager>, 
        lock: Lock,
    },
    LockRelease { // for notifying var/def that a lock should be released
        txn: TxnId,
        preds: HashSet<TxnId>,

    },
    LockGranted { // for notifying manager that a lock request is granted
        from_name: String,
        lock: Lock,
    },
    LockAbort { // for notifying manager that a lock request is aborted
                // then manager forward to peers that lock request is aborted
        from_name: String,
        lock: Lock,
    },


    Subscribe {
        from_name: String,
        from_addr: ActorRef<DefActor>,
    },

    SubscribeGranted,

    Change {
        from_name: String,
        val: Expr,
        preds: HashSet<TxnId>,
    },

    // Meerkat 2.0 only support non-distributed CodeUpdate
    CodeUpdate {
        prog: Prog,
    }
}

// #[derive(PartialEq, Eq, Clone, Debug)]
// pub struct PropaChange {
//     pub from_name: String,
//     pub new_val: Val,
//     pub provides: HashSet<Txn>,
//     pub requires: HashSet<Txn>,
// }

// #[derive(PartialEq, Eq, Hash, Clone, Debug)]
// pub struct TxnAndName {
//     pub txn: Txn,
//     pub name: String,
// }

// #[derive(PartialEq, Eq, Clone, Debug)]
// pub struct _PropaChange {
//     pub propa_id: i32,
//     pub propa_change: PropaChange,
//     pub deps: HashSet<TxnAndName>,
// }

// impl Hash for _PropaChange {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.propa_id.hash(state);
//     }
// }