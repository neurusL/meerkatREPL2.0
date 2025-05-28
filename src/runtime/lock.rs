use std::{cmp::Reverse, collections::{BinaryHeap, HashSet, VecDeque}};

use super::transaction::{Txn, TxnId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LockKind {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lock {
    pub lock_kind: LockKind,
    pub txn_id: TxnId,
}

/// TODO: think more carefully about order between Read and Write??
impl PartialOrd for Lock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.txn_id.cmp(&other.txn_id))
    }
}

impl Ord for Lock {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.txn_id.cmp(&other.txn_id)
    }
}

/// lock state for an actor
/// where support:
/// 1. peek min granted lock 
/// 2. peek and pop min, delete arbitrary waiting lock
pub struct LockState {
    pub granted_locks: HashSet<Lock>,             // current lock granted
    pub oldest_granted_lock: Option<Lock>,
    pub waiting_locks: BinaryHeap<Reverse<Lock>>, // locks waiting to be granted
}

impl LockState {
    pub fn new() -> Self {
        LockState {
            granted_locks: HashSet::new(),
            oldest_granted_lock: None,
            waiting_locks: BinaryHeap::new(),
        }
    }

    /// only allow lock older than all granted locks to wait
    /// return true if lock added to waiting list
    pub fn add_wait(&mut self, lock: Lock) -> bool {
        if let Some(oldest_lock) = &self.oldest_granted_lock {
            // if receive lock younger than oldest granted lock, ignore 
            if lock.txn_id > oldest_lock.txn_id { return false; }
        } 
        self.waiting_locks.push(Reverse(lock)); // Reverse for min-heap
        return true;
    }

    fn pop_oldest_wait(&mut self) -> Option<Lock> {
        self.waiting_locks.pop().map(|Reverse(l)| l)
    }

    pub fn grant_oldest_wait(&mut self) {
        if let Some(lock) = self.pop_oldest_wait() {
            self.granted_locks.insert(lock.clone());
            
            if let Some(prev_oldest) = &self.oldest_granted_lock {
                if lock.txn_id < prev_oldest.txn_id {
                    self.oldest_granted_lock = Some(lock.clone());
                }
            }
        }
        assert!(self.check_granted_isvalid());
    }

    pub fn remove_granted(&mut self, lock: &Lock) {
        self.granted_locks.remove(&lock);
        if self.oldest_granted_lock.as_ref() == Some(lock) {
            self.oldest_granted_lock = None;
        }
    }

    pub fn remove_granted_or_wait(&mut self, lock: &Lock) {
        self.remove_granted(lock);

        todo!("now I need a data structure efficiently support removing arbitrary waiting lock")

    }

    pub fn clear_granted(&mut self) {
        self.granted_locks.clear();
        self.oldest_granted_lock = None;
    }

    pub fn has_granted(&self, lock: Lock) -> bool {
        self.granted_locks.contains(&lock)
    }

    pub fn check_granted_isvalid(&self) -> bool {
        //either all Read or unique Write lock 
        self.granted_locks.iter().all(|lock| 
            lock.lock_kind == LockKind::Read)
        || self.granted_locks.len() == 1
    }

}
