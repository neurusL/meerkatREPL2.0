use std::collections::{HashSet, VecDeque};

use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    backend::{
        message::{Lock, LockKind, Message, Val},
        transaction::{Txn, TxnId, WriteToName},
    },
    frontend::{meerast::Expr, typecheck::Type},
};

pub struct VarWorker {
    pub name: String,
    pub receiver_from_manager: Receiver<Message>,
    pub sender_to_manager: Sender<Message>,
    pub senders_to_subscribers: Vec<Sender<Message>>,

    pub value: Option<Val>,
    pub ty: Option<Type>,
    // I think, and as on the doc, we need only the latest but not all applied txns
    pub latest_value_txn: Option<Txn>,
    // R, i.e. txns required to be applied with or before the update
    pub requires: HashSet<Txn>,
    // locks currently granted out by this def worker, one W or multiple R
    pub locks: HashSet<Lock>,
    // incoming locks allowed to wait by wait-die
    pub lock_queue: VecDeque<Lock>,
}

impl VarWorker {
    pub fn new(
        name: &str,
        receiver_from_manager: Receiver<Message>,
        sender_to_manager: Sender<Message>,
    ) -> Self {
        Self {
            name: name.to_string(),
            receiver_from_manager: receiver_from_manager,
            sender_to_manager: sender_to_manager,
            senders_to_subscribers: vec![],
            value: None,
            ty: None,
            latest_value_txn: None,
            requires: HashSet::new(),
            locks: HashSet::new(),
            lock_queue: VecDeque::new(),
        }
    }

    pub async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::ReadVarRequest { txn } => {
                let mut txn_deps: HashSet<String> = HashSet::new();
                for wr in txn.writes.iter() {
                    let wr_deps = wr.expr.names_contained();
                    txn_deps.extend(wr_deps.into_iter());
                }
                // also test whether the incoming txn is younger?
                let mut holds_wlock = false;
                for lk in self.locks.iter() {
                    match lk.lock_kind {
                        LockKind::WLock => {
                            holds_wlock = true;
                            break;
                        }
                        LockKind::RLock => {}
                    }
                }
                if txn_deps.contains(&self.name) && holds_wlock {
                    let _ = self
                        .sender_to_manager
                        .send(Message::LockAbortMessage {
                            from_worker: self.name.clone(),
                            txn: txn,
                        })
                        .await
                        .unwrap();
                } else {
                    let rlock = Lock {
                        txn: txn.clone(),
                        lock_kind: LockKind::RLock,
                    };
                    self.locks.insert(rlock.clone());
                    let _ = self
                        .sender_to_manager
                        .send(Message::ReadVarResult {
                            txn: txn.clone(),
                            name: self.name.clone(),
                            result: self.value.clone(),
                            result_provides: HashSet::new(),
                        })
                        .await;
                    self.latest_value_txn = Some(txn);
                    self.locks.remove(&rlock);
                }
            }
            Message::WriteVarRequest {
                txn,
                write_val,
                requires,
            } => {
                let mut write_set: HashSet<String> = HashSet::new();
                for wr in txn.writes.iter() {
                    write_set.insert(wr.name.clone());
                }
                todo!()
            }
            Message::LockMessage { kind } => {
                todo!()
            }
            Message::LockAbortMessage { from_worker, txn } => {
                todo!()
            }
            #[allow(unreachable_patterns)]
            _ => panic!(),
        }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.receiver_from_manager.recv().await {
            let _ = self.handle_message(msg).await;
        }
    }
}

#[cfg(test)]
mod test {
    use std::thread::spawn;

    use super::*;

    #[tokio::test]
    async fn single_read() {
        let (sndr, rcvr) = mpsc::channel(1024);
        let var_worker = VarWorker::new("x", rcvr, sndr.clone());
        tokio::spawn(var_worker.run());
        let _ = sndr
            .send(Message::ReadDefRequest {
                txn: Txn {
                    id: TxnId::new(),
                    writes: vec![WriteToName {
                        name: "a".to_string(),
                        expr: Expr::IntConst { val: 2 },
                    }],
                },
                requires: HashSet::new(),
            })
            .await;
    }

    #[tokio::test]
    async fn single_write() {
        todo!()
    }

    #[test]
    fn batch_write_mutually_indep() {
        todo!()
    }

    #[test]
    fn try_read_var_with_wlock() {
        todo!()
    }

    #[test]
    fn try_write_var_with_rlock() {
        todo!()
    }

    #[test]
    fn try_write_var_with_another_wlock() {
        todo!()
    }
}
