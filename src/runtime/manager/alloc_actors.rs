use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::rc::Rc;
use std::thread::current;

use kameo::{prelude::*, spawn, Actor};
use log::info;

use crate::runtime::def_actor::TickFunc;
use crate::{
    ast::{Expr, Prog, Service},
    runtime::{def_actor::DefActor, evaluator::eval_srv, message::Msg, var_actor::VarActor},
    static_analysis::var_analysis::calc_dep_srv,
};

use super::Manager;

impl Manager {
    pub async fn alloc_service(&mut self, srv: &Service) {
        // intial evaluation of srv
        self.evaluator = eval_srv(srv);

        let def_to_exprs = eval_srv(srv).def_name_to_exprs;

        let srv_info = calc_dep_srv(srv);

        self.dep_graph = srv_info.dep_graph;
        self.dep_tran_vars = srv_info.dep_vars;

        for name in srv_info.topo_order.iter() {
            let val = self
                .evaluator
                .reactive_name_to_vals
                .get(name)
                .expect(&format!(
                    "Service alloc: var/def is not initialized: {}",
                    name
                ));

            if srv_info.vars.contains(name) {
                self.alloc_var_actor(name, val.clone()).await;
            } else if srv_info.defs.contains(name) {
                let def_expr = def_to_exprs.get(name).expect(&format!(
                    "Service alloc: def expr is not initialized: {}",
                    name
                ));

                self.alloc_def_actor(name, def_expr.clone(), None)
                    .await
                    .unwrap();
            }
        }

        info!("Service allocated: {}", self);
    }

    pub async fn alloc_var_actor(&mut self, name: &String, val: Expr) {
        let actor_ref = spawn(VarActor::new(name.clone(), val));
        self.varname_to_actors.insert(name.clone(), actor_ref);
    }

    /// current impl of alloc_def_actor rely on a centralized manager
    /// preprocess all information
    ///
    /// should change to distributed and incrementally allocate def actor
    /// to accommodate for code update
    pub async fn alloc_def_actor(
        &mut self,
        name: &String,
        expr: Expr,
        customized_tick: Option<TickFunc>,
    ) -> Result<ActorRef<DefActor>, Box<dyn Error>> {
        // calculate all information for def actor
        let def_args = self.dep_graph.get(name).map_or_else(
            || expr.free_var(&HashSet::new()), // incrementally calculated
            |def_args| def_args.clone(),       // precalculated by centralized manager
        );

        let def_arg_to_vals = def_args.iter()
        .map(|name| (
            name.clone(), 
            self.evaluator.reactive_name_to_vals.get(name)
            .expect(&format!("DefActor alloc: var/def is not initialized: {}", name))
            .clone()))
        .collect::<HashMap<String, Expr>>();


        let def_arg_to_vars = def_args.iter()
        .map(|name|(
            name.clone(), 
            self.dep_tran_vars.get(name)
            .expect(&format!("DefActor alloc: var/def is not initialized: {}", name))
            .clone()
        )).collect::<HashMap<String, HashSet<String>>>();

        let mut val = expr.clone();
        let _ = self.evaluator.eval_expr(&mut val);

        let actor_ref = spawn(DefActor::new(
            name.clone(),
            expr,
            val,
            def_arg_to_vals,
            def_arg_to_vars,
            customized_tick,
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
                .await?;

            if !matches!(back_msg, Msg::SubscribeGranted) {
                panic!("Service alloc: receive wrong message type during subscription");
            }
        }

        Ok(actor_ref)
    }
}

// later when we want to fully implement services and code update
// here are some todos
// - how to process remote var/def values
// - incrementally update dep_graph and related data structures in srv manager
