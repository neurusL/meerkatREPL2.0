use std::collections::HashSet;

use crate::{
    frontend::meerast::Expr,
    runtime::{lock::LockKind, transaction::Txn},
};

use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Int(i32),
    Bool(bool),
    Action(Expr), // Expr have to be Action
    Lambda(Expr), // Expr have to be Lambda
}

#[derive(Debug, Clone)]
pub enum Message {
    ReadVarRequest {
        txn: Txn,
    },
    ReadVarResult {
        var_name: String,
        result: Option<Val>,
        result_provides: HashSet<Txn>,
        txn: Txn,
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
    WriteVarRequest {
        txn: Txn,
        write_val: Val,
        requires: HashSet<Txn>,
    },
    Propagate {
        from_name: String,
        new_val: i32,
        provide: HashSet<Txn>,
        require: HashSet<Txn>,
    },
    Subscribe {
        subscriber_name: String,
        sender_to_subscriber: Sender<Message>,
    },
    SubscriptionGranted {
        from_name: String,
        value: Option<Val>,
        provides: HashSet<Txn>,
    },
}
