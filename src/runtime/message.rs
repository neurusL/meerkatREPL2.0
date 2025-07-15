use std::collections::HashSet;

use kameo::{actor::ActorRef, Actor, Reply};
use tokio::sync::mpsc::Sender;

use crate::{
    ast::{Assn, Expr, Prog, Service, Test, Insert, Field},
    runtime::{lock::Lock, transaction::TxnId, TestId},
};

use super::{def_actor::DefActor, manager::Manager, transaction::Txn, var_actor::VarActor};

#[derive(Debug, Clone, Reply)]
pub enum Msg {
    Unit,

    UsrReadVarRequest {
        from_mgr_addr: ActorRef<Manager>,
        txn: TxnId,
    },
    UsrReadVarResult {
        txn: TxnId,
        name: String,
        result: Expr,
        pred: Option<Txn>,
    },

    UsrReadDefRequest {
        from_mgr_addr: ActorRef<Manager>,
        txn: TxnId,
        // requires: HashSet<Txn>, // ?? Why we need this
    },
    UsrReadDefResult {
        txn: TxnId,
        name: String,
        result: Expr,
        preds: HashSet<Txn>,
    },

    UsrWriteVarRequest {
        from_mgr_addr: ActorRef<Manager>,
        txn: TxnId,
        write_val: Expr,
        // requires: HashSet<Txn>,
    },
    UsrWriteVarFinish {
        txn: TxnId,
        name: String,
    },
    // table operations
    UserReadTableRequest {
        txn: TxnId,
        name: String,
        table_name: String,
        where_clause: Expr 
    },
    UserReadTableResult {
        txn: TxnId,
        name: String,
        result: Expr,    // Expr::Table in this case
        pred: Option<Txn>
    },
    UserWriteTableRequest {
        from_mgr_addr: ActorRef<Manager>,
        txn: Txn,
        insert: Insert,
    },
    UserWriteTableFinish {
        txn: TxnId,
        name: String,
    },

    // UnsafeRead, // for test only
    // UnsafeReadResult {
    //     result: Expr,
    // },
    LockRequest {
        // for notifying var/def that a lock is requested
        from_mgr_addr: ActorRef<Manager>,
        lock: Lock,
    },
    LockRelease {
        // for notifying var/def that a lock should be released
        txn: Txn,
        preds: HashSet<Txn>,
    },
    LockGranted {
        // for notifying manager that a lock request is granted
        from_name: String,
        lock: Lock,
    },
    LockAbort {
        // for notifying manager that a lock request is aborted
        // then manager forward to peers that lock request is aborted
        from_name: String,
        lock: Lock,
    },

    Subscribe {
        from_name: String,
        from_addr: ActorRef<DefActor>,
    },

    SubscribeGranted {
        name: String,
        value: Expr,
        preds: HashSet<Txn>,
    },

    // propagate change of name's value, with a set of txns (pred) as prereq
    PropChange {
        from_name: String,
        val: Expr,
        preds: HashSet<Txn>,
    },
}

#[derive(Debug, Clone, Reply)]
pub enum CmdMsg {
    // Meerkat 2.0 only support non-distributed CodeUpdate
    CodeUpdate {
        srv: Service,
    },
    CodeUpdateGranted {
        srv_name: String,
    },

    DoAction {
        from_client_addr: Sender<CmdMsg>,
        txn_id: TxnId,
        action: Expr,
    },

    TransactionAborted {
        txn_id: TxnId,
    },
    TransactionCommitted {
        txn_id: TxnId,
    },

    TryAssert {
        name: String,
        test: Expr,
        test_id: TestId,
    },
    AssertSucceeded {
        test_id: TestId,
    },
}
