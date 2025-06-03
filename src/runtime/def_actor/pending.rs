//! how we key the pending changes of a def actor F
//! 
//! we maintain a dependency graph of pending changes, where applying a change
//! requires for all transactions t in change, if t writes t ancestor of F,
//! then we want to see all relevant args(F) has t in hand to be applied
//!

use std::collections::{HashMap, HashSet};

use crate::{ast::Assn, runtime::transaction::{Txn, TxnId}};

use super::{PropChange, ChangeId};

pub struct PendingChanges {
    /// relevant var maps to args of def F's expression
    /// when we see a transaction t writes to a relevant var f,
    /// then all var_to_inputs[f] should see transaction t
    pub var_to_inputs: HashMap<String, HashSet<String>>,

    /// dependency graph: 
    /// 
    /// in a way that a change depends on (arg_name, txn_id)'s
    /// # req_to_changes 
    /// key: (arg_name, txn_id)
    /// value: a set of changes whose from_name is arg_name, and preds contains txn_id
    pub req_to_changes: HashMap<(String, TxnId), HashSet<ChangeId>>,
    /// # changes_to_req: 
    /// key: change 
    /// value: (arg_name, txn_id) that the change depends on
    pub changes_to_req: HashMap<ChangeId, HashSet<(String, TxnId)>>,

}

impl PendingChanges {
    pub fn add_change(&mut self, change: &PropChange) {
        // extract all (arg_name, txn_id) that change depends on 
        for Txn {id: txn_id, assns} in change.preds.iter() {
            for Assn {dest, ..} in assns.iter() {
                for arg in self.var_to_inputs.get(dest)
                    .expect(&format!("var {} not found in var_to_inputs", dest))
                    .iter() 
                {
                    self.changes_to_req.entry(change.id)
                    .or_insert(HashSet::new())
                    .insert((arg.clone(), txn_id.clone()));
                }
            }
        }
    }

    pub fn search_batch() {}
}