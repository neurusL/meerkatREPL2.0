use core::panic;
use std::collections::{HashMap, HashSet};
use std::error::Error;

use kameo::{prelude::*, spawn, Actor};

use crate::{
    ast::{Expr, Prog, Service},
    runtime::{def_actor::DefActor, evaluator::eval_srv, message::Msg, var_actor::VarActor},
    static_analysis::var_analysis::calc_dep_srv,
};

use super::Manager;

impl Manager {
    /// to spawn a manager:
    /// let mgr = Manager::new(); spawn(mgr);
    pub fn new(name: String) -> Self {
        Manager {
            name,
            address: None,

            varname_to_actors: HashMap::new(),
            defname_to_actors: HashMap::new(),
            dep_graph: HashMap::new(),

            txn_mgrs: HashMap::new(),
        }
    }

    pub async fn alloc_service(&mut self, srv: &Service) {
        // intial evaluation of srv
        let name_to_vals = eval_srv(srv).reactive_name_to_vals;

        let srv_info = calc_dep_srv(srv);
        self.dep_graph = srv_info.dep_graph;

        for name in srv_info.topo_order.iter() {
            let val = name_to_vals.get(name).expect(&format!(
                "Service alloc: var/def is not initialized: {}",
                name
            ));

            if srv_info.vars.contains(name) {
                self.alloc_var_actor(name, val.clone());
            } else if srv_info.defs.contains(name) {
                self.alloc_def_actor(name, val.clone()).await.unwrap();
            }
        }
    }

    pub fn alloc_var_actor(&mut self, name: &String, val: Expr) {
        let actor_ref = spawn(VarActor::new(name.clone(), val));
        self.varname_to_actors.insert(name.clone(), actor_ref);
    }

    pub async fn alloc_def_actor(
        &mut self,
        name: &String,
        val: Expr,
    ) -> Result<(), Box<dyn Error>> {
        let actor_ref = spawn(DefActor::new(name.clone(), val));
        self.defname_to_actors
            .insert(name.clone(), actor_ref.clone());

        // subscribe to its dependencies
        let depends = self.dep_graph.get(name).expect(&format!(
            "Service alloc: no such def with name: {} in dependency graph",
            name
        ));

        for dep in depends.iter() {
            // synchronously wait for response
            let back_msg = self
                .ask_to_name(
                    dep,
                    Msg::Subscribe {
                        from_name: name.clone(),
                        from_addr: actor_ref.clone(),
                    },
                )
                .await?
                .expect("msg should not be None");

            if !matches!(back_msg, Msg::SubscribeGranted) {
                panic!("Service alloc: receive wrong message type during subscription");
            }
        }

        Ok(())
    }
}

// later when we want to fully implement services and code update
// here are some todos
// - how to process remote var/def values
// - incrementally update dep_graph and related data structures in srv manager
