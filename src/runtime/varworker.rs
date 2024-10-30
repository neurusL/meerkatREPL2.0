use std::collections::{HashMap, HashSet, VecDeque};

use crate::runtime::{
    lock::{Lock, LockKind},
    message::{Message, Val},
    transaction::Txn,
};
use tokio::sync::mpsc::{Receiver, Sender};

pub struct VarWorker {
    pub name: String,
    pub receiver_from_manager: Receiver<Message>,
    pub sender_to_manager: Sender<Message>,
    pub senders_to_subscribers: HashMap<String, Sender<Message>>,

    pub value: Option<Val>,
    pub latest_write_txn: Option<Txn>,
    pub locks: HashSet<Lock>,
    pub lock_queue: VecDeque<Lock>,
    pub pending_w_locks: HashSet<Lock>,
    pub pred_txns: HashSet<Txn>,
}

impl VarWorker {
    pub fn new(
        name: &str,
        receiver_from_manager: Receiver<Message>,
        sender_to_manager: Sender<Message>,
    ) -> Self {
        VarWorker {
            name: name.to_string(),
            receiver_from_manager,
            sender_to_manager,
            senders_to_subscribers: HashMap::new(),
            value: None,
            latest_write_txn: None,
            locks: HashSet::new(),
            lock_queue: VecDeque::new(),
            pending_w_locks: HashSet::new(),
            pred_txns: HashSet::new(),
        }
    }

    pub fn has_write_lock(&self) -> bool {
        for lk in self.locks.iter() {
            if lk.lock_kind == LockKind::Write {
                return true;
            }
        }
        false
    }

    pub async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::VarLockRequest { lock_kind, txn } => {
                // let mut has_lingering_w_lock = false;
                let mut oldest_txn_id = txn.id.clone();
                for lk in self.locks.iter() {
                    if lk.txn.id <= oldest_txn_id {
                        oldest_txn_id = lk.txn.id.clone();
                    }
                }
                if txn.id == oldest_txn_id
                    || (!self.has_write_lock() && self.pending_w_locks.is_empty())
                {
                    self.lock_queue.push_back(Lock {
                        lock_kind: lock_kind,
                        txn: txn,
                    });
                } else {
                    let _ = self
                        .sender_to_manager
                        .send(Message::VarLockAbort { txn: txn })
                        .await
                        .unwrap();
                }
            }
            Message::VarLockRelease { txn } => {
                let to_be_removed: HashSet<Lock> = self
                    .locks
                    .iter()
                    .cloned()
                    .filter(|t| t.txn.id == txn.id)
                    .collect();
                for tbr in to_be_removed.iter() {
                    self.locks.remove(tbr);
                }
            }
            Message::ReadVarRequest { txn } => {
                let mut self_r_locks_held_by_txn: HashSet<Lock> = HashSet::new();
                // let mut self_w_locks_held_by_txn: HashSet<Lock> = HashSet::new();
                for rl in self.locks.iter() {
                    if rl.lock_kind == LockKind::Read {
                        if txn.id == rl.txn.id {
                            self_r_locks_held_by_txn.insert(rl.clone());
                        }
                    }
                }
                self.pred_txns.insert(txn.clone());
                let _ = self
                    .sender_to_manager
                    .send(Message::ReadVarResult {
                        var_name: self.name.clone(),
                        result: self.value.clone(),
                        result_provides: if self.latest_write_txn == None {
                            HashSet::new()
                        } else {
                            let mut rslt = HashSet::new();
                            rslt.insert(self.latest_write_txn.clone().unwrap());
                            rslt
                        },
                        txn: txn,
                    })
                    .await;
                for rl in self_r_locks_held_by_txn.into_iter() {
                    self.locks.remove(&rl);
                }
            }
            Message::Subscribe {
                subscriber_name,
                sender_to_subscriber,
            } => {
                self.senders_to_subscribers
                    .insert(subscriber_name, sender_to_subscriber.clone());
                let respond_msg = Message::SubscriptionGranted {
                    from_name: self.name.clone(),
                    value: self.value.clone(),
                    provides: self.pred_txns.clone(),
                };
                let _ = sender_to_subscriber.send(respond_msg).await.unwrap();
            }
            Message::WriteVarRequest {
                txn,
                write_val,
                requires,
            } => {
                let mut w_lock_granted = false;
                let w_lock_for_txn: Lock;
                for lk in self.locks.iter() {
                    if lk.lock_kind == LockKind::Write && lk.txn.id == txn.id {
                        w_lock_granted = true;
                        w_lock_for_txn = lk.clone();
                        break;
                    }
                }
                if w_lock_granted {
                    self.pending_w_locks.insert(Lock {
                        lock_kind: LockKind::Write,
                        txn: txn.clone(),
                    });
                    todo!()
                }
            }
            #[allow(unreachable_patterns)]
            _ => todo!(),
        }
    }

    pub async fn perform_transaction(&mut self, txn: Txn) {
        todo!()
    }
}
