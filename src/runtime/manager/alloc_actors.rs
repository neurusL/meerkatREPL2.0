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

            reactive_name_to_vals: HashMap::new(),
            dep_graph: HashMap::new(),
            dep_transtive: HashMap::new(),

            txn_mgrs: HashMap::new(),
        }
    }

    pub async fn alloc_service(&mut self, srv: &Service) {
        // intial evaluation of srv
        self.reactive_name_to_vals = eval_srv(srv).reactive_name_to_vals;
        let def_to_exprs = eval_srv(srv).def_name_to_exprs;

        let srv_info = calc_dep_srv(srv);
        self.dep_graph = srv_info.dep_graph;
        self.dep_transtive = srv_info.dep_transtive;

        for name in srv_info.topo_order.iter() {
            let val = self.reactive_name_to_vals.get(name).expect(&format!(
                "Service alloc: var/def is not initialized: {}",
                name
            ));

            if srv_info.vars.contains(name) {
                self.alloc_var_actor(name, val.clone());
            } else if srv_info.defs.contains(name) {
                let def_expr = def_to_exprs.get(name)
                .expect(&format!("Service alloc: def expr is not 
                    initialized: {}", name));

                self.alloc_def_actor(name, val.clone(), def_expr.clone()).await.unwrap();
            }
        }
    }

    pub fn alloc_var_actor(
        &mut self, 
        name: &String, 
        val: Expr
    ) {
        let actor_ref = spawn(VarActor::new(name.clone(), val));
        self.varname_to_actors.insert(name.clone(), actor_ref);
    }

    pub async fn alloc_def_actor(
        &mut self,
        name: &String,
        val: Expr,
        expr: Expr,
    ) -> Result<(), Box<dyn Error>> {
        // calculate all information for def actor
        let def_args = self.dep_graph.get(name)
            .expect(&format!("Service alloc: no such def with name: 
                {} in dependency graph", name));

        let def_arg_to_vals = self.reactive_name_to_vals.iter()
        .filter(|(k, _)| def_args.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<HashMap<String, Expr>>();

        let def_arg_to_vars = self.dep_transtive.iter()
        .filter(|(k, _)| def_args.contains(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<HashMap<String, HashSet<String>>>();


        let actor_ref = spawn(DefActor::new(
            name.clone(), expr, val,
            def_arg_to_vals, def_arg_to_vars
        ));
        self.defname_to_actors
            .insert(name.clone(), actor_ref.clone());

        // subscribe to its dependencies
        for name in def_args.iter() {
            // synchronously wait for response
            let back_msg = self
                .ask_to_name(
                    name,
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
