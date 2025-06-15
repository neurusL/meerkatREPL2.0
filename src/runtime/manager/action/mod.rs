//! when transaction is received by manager
//! we allocate a new transaction manager on it to monitor the transaction
use std::collections::{HashMap, HashSet};

use tokio::sync::mpsc::Sender;

use crate::{
    ast::Expr,
    runtime::{message::CmdMsg, transaction::Txn},
};

pub mod do_action;
pub mod txn_manager;

#[derive(Clone, Debug)]
pub struct TxnManager {
    pub txn: Txn,
    /// channel to client who submitted the txn
    pub from_client: Sender<CmdMsg>,

    /// map of each read to the state
    pub reads: HashMap<String, ReadState>,
    /// .. to write state
    pub writes: HashMap<String, WriteState>,
    /// preds to apply this transaction
    pub preds: HashSet<Txn>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReadState {
    Requested,  // default
    Granted,    // lock granted
    Aborted,    // lock aborted
    Read(Expr), // successfully read, get value of name
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WriteState {
    Requested, // default
    Granted,   // lock granted
    Aborted,   // lock aborted
    Writed,    // successfully write to name
}

impl TxnManager {
    pub fn new(
        txn: Txn,
        from_client: Sender<CmdMsg>,
        reads: HashSet<String>,
        writes: HashSet<String>,
    ) -> Self {
        TxnManager {
            txn,
            from_client,
            reads: HashMap::from_iter(
                reads
                    .iter()
                    .map(|name| (name.clone(), ReadState::Requested)),
            ),
            writes: HashMap::from_iter(
                writes
                    .iter()
                    .map(|name| (name.clone(), WriteState::Requested)),
            ),
            preds: HashSet::new(),
        }
    }
}
