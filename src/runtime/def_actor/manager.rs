use std::{collections::{BTreeMap, HashMap, HashSet}, hash::Hash};


use super::{history::AppliedChanges, pending::PendingChanges};



pub struct PropChangeManager {
    pub pending_changes: PendingChanges,
    pub applied_changes: AppliedChanges,
}
