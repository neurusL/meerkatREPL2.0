use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use crate::{
    frontend::meerast::Expr,
    runtime::{lock::LockKind, transaction::Txn},
};

use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Val {
    Int(i32),
    Bool(bool),
    Action(Expr), // Expr have to be Action
    Lambda(Expr), // Expr have to be Lambda
}

#[derive(Debug, Clone)]
pub enum Message {
    UsrReadVarRequest {
        txn: Txn,
    },
    UsrReadVarResult {
        var_name: String,
        result: Option<Val>,
        result_provides: HashSet<Txn>,
        txn: Txn,
    },
    UsrWriteVarRequest {
        txn: Txn,
        write_val: Val,
        requires: HashSet<Txn>,
    },
    UsrReadDefRequest {
        txn: Txn,
        requires: HashSet<Txn>, // ?? Why we need this
    },
    UsrReadDefResult {
        txn: Txn,
        name: String,
        result: Option<Val>,
        result_provide: HashSet<Txn>,
    },

    DevReadRequest {
        txn: Txn, 
    },
    DevReadResult { // grant access to Delta(name)
        name: String,
        txn: Txn,
    },

    DevWriteDefRequest {
        txn: Txn,
        write_expr: Expr,
    },

    VarLockRequest {
        lock_kind: LockKind,
        txn: Txn,
    },
    VarLockRelease {
        txn: Txn,
    },
    VarLockGranted {
        txn: Txn,
    },
    VarLockAbort {
        txn: Txn,
    },

    DefLockRequest {
        lock_kind: LockKind,
        txn: Txn,
    },
    DefLockRelease {
        txn: Txn,
    },
    DefLockGranted {
        txn: Txn,
    },
    DefLockAbort {
        txn: Txn,
    },

    Propagate {
        propa_change: PropaChange, // a small change, make batch valid easier
    },
    Subscribe {
        subscriber: String,
        sender_to_subscriber: Sender<Message>,
    },
    SubscriptionGranted {
        subscribe: String,
        value: Option<Val>,
        provides: HashSet<Txn>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PropaChange {
    pub from_name: String,
    pub new_val: Val,
    pub provides: HashSet<Txn>,
    pub requires: HashSet<Txn>,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TxnAndName {
    pub txn: Txn,
    pub name: String,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct _PropaChange {
    pub propa_id: i32,
    pub propa_change: PropaChange,
    pub deps: HashSet<TxnAndName>,
}

impl Hash for _PropaChange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.propa_id.hash(state);
    }
}