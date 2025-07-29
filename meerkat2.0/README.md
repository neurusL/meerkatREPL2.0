# meerkat2.0 – Upgrade Lock Implementation

This module demonstrates the implementation of an `Upgrade` lock in the meerkat runtime system's locking mechanism. It extends the existing `Read` and `Write` lock logic with an additional `Upgrade` lock that takes priority over both and enforces abort behavior for conflicting transactions.

##  What Was Implemented?

- Introduced a third `LockKind` variant: `Upgrade`
- Defined `Lock::new_upgrade()` and `Lock::is_upgrade()` helpers
- Enhanced the lock-granting logic in `grant_oldest_wait()` to:
  - Give Upgrade locks immediate priority
  - Abort other waiting locks and clear granted read locks
- Maintained deadlock prevention via wait-die by honoring transaction age
