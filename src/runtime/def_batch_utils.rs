use std::collections::{HashMap, HashSet};

use crate::runtime::{
        message::{Val, TxnAndName, _PropaChange},
        transaction::Txn,
        def_eval::compute_val,
    };

use inline_colorization::*;

pub fn dfs(
    curr_node: &TxnAndName,
    visited: &mut HashSet<TxnAndName>,
    batch_acc: &mut HashSet<_PropaChange>,
    applied_txns: &HashSet<Txn>,
    graph: &HashMap<TxnAndName, _PropaChange>,
) -> bool {
    println!("dfs current node {:?}", curr_node);
    if visited.get(curr_node) != None
    // there already exists an change (sent by name) in B_acc, s.t. txn in change
        || applied_txns.get(&curr_node.txn) != None
    {
        // TODO!: make sure the understanding is correct
        // txn already exists in applied batch set
        // we can infer for all {txn, name} -> change, change in applied batch set
        println!("dfs find node {:?}", curr_node);
        return true;
    } else {
        match graph.get(curr_node) {
            None => return false,
            Some(_propa_change) => {
                visited.insert(curr_node.clone());
                batch_acc.insert(_propa_change.clone());
                for succ in _propa_change.deps.iter() {
                    if !dfs(succ, visited, batch_acc, applied_txns, graph) {
                        println!("dfs cannot find {:?}", succ);
                        return false;
                    }
                }
                return true;
            }
        }
    }
}

pub fn search_batch(
    propa_changes_to_apply: &HashMap<TxnAndName, _PropaChange>,
    applied_txns: &Vec<Txn>,
) -> HashSet<_PropaChange> {
    // DFS on propa_changes_to_apply,
    // println!("propa_changes_to_apply: {:?}", propa_changes_to_apply);
    let applied_txns_set: HashSet<Txn> = applied_txns.iter().cloned().collect();
    let mut visited: HashSet<TxnAndName> = HashSet::new();
    let mut batch_acc: HashSet<_PropaChange> = HashSet::new();

    for (node, _) in propa_changes_to_apply.iter() {
        if dfs(
            node,
            &mut visited,
            &mut batch_acc,
            &applied_txns_set,
            &propa_changes_to_apply,
        ) {
            println!("find a batch: {:#?}", batch_acc);
            return batch_acc;
        } 
        else {
            visited = HashSet::new();
            batch_acc = HashSet::new();
        }
    }

    println!("cannot find a batch: {:#?}", batch_acc);
    batch_acc
    
}

pub fn apply_batch(
    batch: HashSet<_PropaChange>,
    // worker: &Worker,
    value: &mut Option<Val>,
    applied_txns: &mut Vec<Txn>,
    prev_batch_provides: &mut HashSet<Txn>,
    propa_changes_to_apply: &mut HashMap<TxnAndName, _PropaChange>,
    replica: &mut HashMap<String, Option<Val>>,
) -> (HashSet<Txn>, HashSet<Txn>, Option<Val>) {
    let mut all_provides: HashSet<Txn> = HashSet::new();
    let mut all_requires: HashSet<Txn> = prev_batch_provides.clone();

    // latest change to prevent applying older updates after younger ones
    // from the same dependency (only apply one dependency's latest update
    // in the batch, and ignore others)
    let mut latest_change: HashMap<String, i32> = HashMap::new();

    for change in batch.iter() {
        // change := (value, P, R)
        let change_txns_toapply = &change.propa_change.provides;
        all_provides = all_provides.union(change_txns_toapply).cloned().collect();
        all_requires = all_requires
            .union(&change.propa_change.requires)
            .cloned()
            .collect();

        for txn in change_txns_toapply.iter() {
            propa_changes_to_apply.remove(&TxnAndName {
                txn: txn.clone(),
                name: change.propa_change.from_name.clone(),
            });
        }

        if let Some(id) = latest_change.get(&change.propa_change.from_name) {
            if change.propa_id < *id {
                continue;
            }
        }

        replica.insert(
            change.propa_change.from_name.clone(),
            Some(change.propa_change.new_val.clone()),
        );
        latest_change.insert(change.propa_change.from_name.clone(), change.propa_id);
    }

    // apply all txns in all_provides, the result should be calculated from
    // replicas now
    println!("{color_green}batch: {:?}{color_reset}", batch);
    println!(
        "{color_yellow}replica before compute_val: {:?}{color_reset}",
        replica
    );
    *value = compute_val(&replica);
    println!(
        "{color_yellow}value after compute_val: {:?}{color_reset}",
        value
    );
    for txn in all_provides.iter() {
        applied_txns.push(txn.clone());
    }

    // update prev batch's applied txns, i.e. to be all_provides
    *prev_batch_provides = all_provides.clone();

    return (all_provides, all_requires, value.clone());

    // // broadcast the update to subscribers
    // let msg_propa = Message::PropaMessage { propa_change:
    //     PropaChange {
    //         name: worker.name.clone(),
    //         new_val: value.clone().unwrap(),
    //         provides: all_provides.clone(),
    //         requires: all_requires.clone(),
    //     }
    // };
    // for succ in worker.senders_to_succs.iter() {
    //     let _ = succ.send(msg_propa.clone()).await;
    // }
}