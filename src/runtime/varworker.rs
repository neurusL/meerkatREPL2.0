use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    frontend::meerast::Expr,
    runtime::{
        lock::{Lock, LockKind},
        message::{Message, Val},
        transaction::{Txn, TxnId, WriteToName},
    },
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use inline_colorization::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PendingWrite {
    pub w_lock: Lock,
    pub value: Val,
}

pub struct VarWorker {
    pub name: String,
    pub receiver_from_manager: Receiver<Message>,
    pub sender_to_manager: Sender<Message>,
    pub senders_to_subscribers: HashMap<String, Sender<Message>>,

    pub value: Option<Val>,
    pub latest_write_txn: Option<Txn>,
    pub locks: HashSet<Lock>,
    pub lock_queue: HashSet<Lock>,
    pub pending_writes: HashSet<PendingWrite>,
    // supplement page 2 top:
    // R is required to be applied before P:
    // For state var f:
    //      - R contains the last update’s P set, that is <oldT, oldW> i.e.
    //      the previous transaction that wrote to this variable
    //      - R also contains { v.lastWrite | v in reads(t) } where reads(t)
    //      be the set of variables (state vars?) read by t
    // R = latest_write_txn + what is computed by manager
    // so is `pred_txns` redundant?
    pub pred_txns: HashSet<Txn>, // nextChangeRequires in the go hig impl
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
            lock_queue: HashSet::new(),
            pending_writes: HashSet::new(),
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
                    || (!self.has_write_lock() && self.pending_writes.is_empty())
                {
                    self.lock_queue.insert(Lock {
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
                // I do not think this really resolves the action sequence problem.
                // self.pred_txns.insert(txn.clone());
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
                    // next change requires??
                    // todo!()
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
                    provides: {
                        let mut pvd = HashSet::new();
                        match self.latest_write_txn {
                            Some(ref t) => {
                                pvd.insert(t.clone());
                            }
                            None => {}
                        }
                        pvd
                    },
                };
                let _ = sender_to_subscriber.send(respond_msg).await.unwrap();
            }
            Message::WriteVarRequest {
                txn,
                write_val,
                requires,
            } => {
                let mut w_lock_granted = false;
                // Just dummy lock to convince rust that w_lock_for_txn is always
                // initialized. In fact if not w_locked, program panics.
                let mut w_lock_for_txn: Lock = Lock {
                    lock_kind: LockKind::Read,
                    txn: txn.clone(),
                };
                for lk in self.locks.iter() {
                    if lk.lock_kind == LockKind::Write && lk.txn.id == txn.id {
                        w_lock_granted = true;
                        w_lock_for_txn = lk.clone();
                        break;
                    }
                }
                // println!(
                //     "{color_blue}receive WriteVarRequest, self.locks: {:?}{color_reset}",
                //     self.locks
                // );
                if !w_lock_granted {
                    panic!("attempt to write when no write lock");
                }
                if w_lock_granted {
                    self.locks.remove(&w_lock_for_txn);
                    self.pending_writes.insert(PendingWrite {
                        w_lock: Lock {
                            lock_kind: LockKind::Write,
                            txn: txn.clone(),
                        },
                        value: write_val,
                    });
                    for req_txn in requires.iter() {
                        self.pred_txns.insert(req_txn.clone());
                    }
                }
            }
            #[allow(unreachable_patterns)]
            _ => panic!(),
        }
    }

    pub async fn tick(&mut self) {
        let mut has_granted_write_lock = false;
        for lk in self.locks.iter() {
            if lk.lock_kind == LockKind::Write {
                has_granted_write_lock = true;
            }
        }
        if !has_granted_write_lock && !self.pending_writes.is_empty() && self.locks.is_empty() {
            let pending_w = self.pending_writes.iter().next().unwrap().clone();
            let mut propa_pvds = HashSet::new();
            propa_pvds.insert(Txn {
                id: pending_w.w_lock.txn.id.clone(),
                writes: pending_w.w_lock.txn.writes.clone(),
            });
            let propa_reqs = self.pred_txns.clone();
            self.value = Some(pending_w.value.clone());
            for (_, sndr) in self.senders_to_subscribers.iter() {
                let _ = sndr
                    .send(Message::Propagate {
                        from_name: self.name.clone(),
                        new_val: pending_w.value.clone(),
                        provide: propa_pvds.clone(),
                        require: propa_reqs.clone(),
                    })
                    .await
                    .unwrap();
            }
            self.latest_write_txn = Some(pending_w.w_lock.txn.clone());
            self.pred_txns.clear();
            self.pred_txns.insert(pending_w.w_lock.txn.clone());
            self.locks.clear();
            has_granted_write_lock = false;
        }
        while !has_granted_write_lock {
            let mut max_queued_lock = Lock {
                lock_kind: LockKind::Read,
                txn: Txn {
                    id: TxnId::new(),
                    writes: vec![],
                },
            }; // dummy initial value to convince rust.
            let mut max_assigned = false;
            for queued_lock in self.lock_queue.iter() {
                if !max_assigned || max_queued_lock.txn.id < queued_lock.txn.id {
                    max_queued_lock = queued_lock.clone();
                    max_assigned = true;
                }
            }
            // there exists some lock
            if max_assigned {
                self.lock_queue.remove(&max_queued_lock);
                self.locks.insert(max_queued_lock.clone());
                let _ = self
                    .sender_to_manager
                    .send(Message::VarLockGranted {
                        txn: max_queued_lock.txn.clone(),
                    })
                    .await
                    .unwrap();
            } else {
                break;
            }
        }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.receiver_from_manager.recv().await {
            let _ = self.handle_message(msg).await;
            let _ = self.tick().await;
        }
    }
}

#[tokio::test]
async fn reate_write_read() {
    let (sndr_to_worker, mut rcvr_from_manager) = mpsc::channel(1024);
    let (sndr_to_manager, mut rcvr_from_worker) = mpsc::channel(1024);
    let worker = VarWorker::new("a", rcvr_from_manager, sndr_to_manager.clone());
    tokio::spawn(worker.run());
    let write_txn = Txn {
        id: TxnId::new(),
        writes: vec![WriteToName {
            name: "a".to_string(),
            expr: Expr::IntConst { val: 5 },
        }],
    };
    let w_lock_msg = Message::VarLockRequest {
        lock_kind: LockKind::Write,
        txn: write_txn.clone(),
    };
    let _ = sndr_to_worker.send(w_lock_msg).await.unwrap();
    if let Some(msg) = rcvr_from_worker.recv().await {
        match msg {
            Message::VarLockGranted { txn } => {
                println!(
                    "{color_green}var write lock granted for txn {:?}{color_reset}",
                    txn.id
                );
            }
            _ => panic!(),
        }
    }
    let write_msg = Message::WriteVarRequest {
        txn: write_txn.clone(),
        write_val: Val::Int(5),
        requires: HashSet::new(),
    };
    let _ = sndr_to_worker.send(write_msg).await.unwrap();

    let read_txn = Txn {
        id: TxnId::new(),
        writes: vec![],
    };
    let r_lock_msg = Message::VarLockRequest {
        lock_kind: LockKind::Read,
        txn: read_txn.clone(),
    };
    let _ = sndr_to_worker.send(r_lock_msg.clone()).await.unwrap();
    if let Some(msg) = rcvr_from_worker.recv().await {
        match msg {
            Message::VarLockGranted { txn } => {
                println!("var read lock granted {:?}", txn);
            }
            _ => panic!(),
        }
    }
    let read_msg = Message::ReadVarRequest {
        txn: read_txn.clone(),
    };
    let _ = sndr_to_worker.send(read_msg).await.unwrap();
    if let Some(msg) = rcvr_from_worker.recv().await {
        match msg {
            Message::ReadVarResult {
                var_name,
                result,
                result_provides,
                txn,
            } => {
                println!(
                    "{color_green}manager receive ReadVarResult: \
var_name={:?}, result={:?}, result_provides={:?}{color_reset}",
                    var_name,
                    result,
                    result_provides
                        .iter()
                        .cloned()
                        .map(|x| x.id)
                        .collect::<Vec<_>>()
                );
                assert_eq!(Some(Val::Int(5)), result);
            }
            _ => panic!(),
        }
    }
}
