use std::{collections::{BTreeMap, HashMap, HashSet}, hash::Hash};


use super::{history::AppliedChanges, pending::PendingChanges};



pub struct PropChangeManager {
    // relevant var maps to input of def 
    // when we see a transaction t writes to a relevant var f,
    // then all var_to_inputs[f] should see transaction t
    pub var_to_inputs: HashMap<String, HashSet<String>>,

    pub pending_changes: PendingChanges,
    pub applied_changes: AppliedChanges,
}
