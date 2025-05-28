use std::collections::HashSet;

use kameo::Reply;

use crate::{
    ast::Expr,
    runtime::{lock::Lock, transaction::TxnId},
};


#[derive(Debug, Clone, Reply)]
pub enum Msg {
    UsrReadVarRequest {
        txn: TxnId,
    },
    UsrReadVarResult {
        txn: TxnId,
        var_name: String,
        result: Option<Expr>,
        result_provides: HashSet<TxnId>,
    },
    UsrWriteVarRequest {
        txn: TxnId,
        write_val: Expr,
        requires: HashSet<TxnId>,
    },

    UsrReadDefRequest {
        txn: TxnId,
        requires: HashSet<TxnId>, // ?? Why we need this
    },
    UsrReadDefResult {
        txn: TxnId,
        name: String,
        result: Option<Expr>,
        result_provides: HashSet<TxnId>,
    },

    // Propagate {
    //     propa_change: PropaChange, // a small change, make batch valid easier
    // },

    LockRequest {
        lock: Lock,
    },
    LockRelease {
        lock: Lock,
    },
    LockGranted {
        txn: TxnId,
    },
    LockAbort {
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