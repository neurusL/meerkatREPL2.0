use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;

use kameo::prelude::*;

use crate::runtime::manager::txn_manager::TxnManager;

use super::def_actor::DefActor;
use super::evaluator::Evaluator;
use super::transaction::TxnId;
use super::var_actor::VarActor;

pub mod alloc_actors;
pub mod txn_utils;
pub mod txn_manager;
// pub mod try_test;
pub mod handler;

#[derive(Debug)]
pub struct Manager {
    /// basic info of the manager
    pub name: String,
    pub address: Option<ActorRef<Manager>>,

    pub varname_to_actors: HashMap<String, ActorRef<VarActor>>,
    pub defname_to_actors: HashMap<String, ActorRef<DefActor>>,

    /// analysis and evaluation of program stored at manager
    pub evaluator: Evaluator,
    
    pub dep_graph: HashMap<String, HashSet<String>>,
    pub dep_transtive: HashMap<String, HashSet<String>>,

    /// manager transactions and tests submitted to manager from client/developer
    pub txn_mgrs: HashMap<TxnId, TxnManager>,
}

impl Manager {
    /// to spawn a manager:
    /// let mgr = Manager::new(); spawn(mgr);
    pub fn new(name: String) -> Self {
        Manager {
            name,
            address: None,

            varname_to_actors: HashMap::new(),
            defname_to_actors: HashMap::new(),

            evaluator: Evaluator::new(HashMap::new()),
            dep_graph: HashMap::new(),
            dep_transtive: HashMap::new(),

            txn_mgrs: HashMap::new(),
        }
    }
}

impl Display for Manager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} and actor ref: {:?}\n", self.name, self.address)?;
        write!(f, "varname_to_actors: {:?}\n defname_to_actors: {:?}\n", 
            self.varname_to_actors, 
            self.defname_to_actors)?;
        // write!(f, "txn_mgrs: {:?}\n", self.txn_mgrs)?;
        Ok(())
    }
}
