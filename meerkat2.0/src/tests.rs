use super::*;

#[test]
fn test_upgrade_lock_kicks_out_others() {
    let mut lock_state = LockState::new();

    let txn_a = TxnId(3); // Read
    let txn_b = TxnId(2); // Upgrade (older)
    let txn_c = TxnId(4); // Write (newer)

    let dummy_actor = ActorRef::<Manager>::dead();

    // Adding and granting the read lock
    let read_lock = Lock::new_read(txn_a.clone());
    assert!(lock_state.add_wait(read_lock.clone(), dummy_actor.clone()));
    let _ = lock_state.grant_oldest_wait();

    // Adding the upgrade lock
    assert!(lock_state.add_wait(Lock::new_upgrade(txn_b.clone()), dummy_actor.clone()));

    // Trying to add a write lock that should be blocked by the upgrade lock
    let _ = lock_state.add_wait(Lock::new_write(txn_c.clone()), dummy_actor.clone());

    // 
    let granted = lock_state.grant_oldest_wait();
    assert!(granted.is_some());
    let (lock, _) = granted.unwrap();

    assert!(lock.is_upgrade());
    assert_eq!(lock.txn_id, txn_b);
    assert!(!lock_state.granted_locks.contains_key(&txn_a));
    assert_eq!(lock_state.granted_locks.len(), 1);
}
