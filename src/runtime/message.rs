use std::collections::HashSet;

use kameo::{actor::ActorRef, Reply};

use crate::{
    ast::Expr,
    runtime::{lock::Lock, transaction::TxnId},
};

use super::{manager::Manager, var_actor::VarActor};


#[derive(Debug, Clone, Reply)]
pub enum Msg {
    UsrReadVarRequest {
        txn: TxnId,
    },
    UsrReadVarResult {
        txn: TxnId,
        var_name: String,
        result: Option<Expr>,
        result_preds: HashSet<TxnId>,
    },
    UsrWriteVarRequest {
        txn: TxnId,
        write_val: Expr,
        // requires: HashSet<TxnId>,
    },

    UsrReadDefRequest {
        txn: TxnId,
        requires: HashSet<TxnId>, // ?? Why we need this
    },
    UsrReadDefResult {
        txn: TxnId,
        name: String,
        result: Option<Expr>,
        result_preds: HashSet<TxnId>,
    },

    // Propagate {
    //     propa_change: PropaChange, // a small change, make batch valid easier
    // },

    LockRequest { // for notifying var/def that a lock is requested
        from_mgr: ActorRef<Manager>, 
        lock: Lock,
    },
    LockRelease { // for notifying var/def that a lock should be released
        txn: TxnId,
        preds: HashSet<TxnId>,

    },
    LockGranted { // for notifying manager that a lock request is granted
        from_name: ActorRef<VarActor>,
        txn: TxnId,
    },
    LockAbort { // for notifying manager that a lock request is aborted
                // then manager forward to peers that lock request is aborted
        from_name: ActorRef<VarActor>,
        txn: TxnId,
    },
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