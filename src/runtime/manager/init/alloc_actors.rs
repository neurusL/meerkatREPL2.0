use core::panic;
use std::collections::{HashMap, HashSet};
use std::error::Error;

use kameo::{prelude::*, spawn};

use crate::runtime::manager::Manager;
use crate::runtime::transaction::TxnPred;
use crate::runtime::TestId;
use crate::{
    ast::Expr,
    runtime::{def_actor::DefActor, message::Msg, var_actor::VarActor},
};

impl Manager {
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
    ) -> Result<ActorRef<DefActor>, Box<dyn Error>> {
        // calculate all information for def actor
        let latest_name = self.evaluator.version_map.get_latest(name);

        let def_args = self.dep_graph.get(name).map_or_else(
            || expr.free_var(&HashSet::new()), // incrementally calculated
            |deps| deps.clone(),       // precalculated by centralized manager
        // calculate all information for def actor, default is used for non-source code part, like assertions
        let def_args = self.dep_graph.get(name).map_or_else(
            || expr.free_var(&self.evaluator.reactive_names, &HashSet::new()), // incrementally calculated
            |def_args| def_args.clone(), // precalculated by centralized manager
        );

        // let def_args_versioned: HashSet<String> = def_args_unversioned
        //     .iter()
        //     .map(|arg| self.evaluator.version_map.get_latest(name))
        //     .collect();

        let def_arg_to_vals = def_args
            .iter()
            .map(|dep_name| {
                let latest_dep = self.evaluator.version_map.get_latest(dep_name);
                (
                    latest_dep.clone(),
                    self.evaluator
                        .reactive_name_to_vals
                        .get(&latest_dep)
                        .unwrap_or_else(|| {
                            panic!(
                                "DefActor alloc: var/def '{}' (versioned: '{}') not found in reactive_name_to_vals",
                                dep_name, latest_dep
                            )
                        })
                        .clone(),
                )
            })
            .collect::<HashMap<String, Expr>>();

        let def_arg_to_vars = def_args
            .iter()
            .map(|dep_name| {
                let latest_dep = self.evaluator.version_map.get_latest(dep_name);
                let base_name = self.evaluator.version_map.get_base(dep_name);
                (
                    latest_dep.clone(),
                    self.dep_tran_vars
                        .get(&base_name)
                        .unwrap_or_else(|| {
                            panic!(
                                "DefActor alloc: var/def is not initialized in dep_tran_vars: {}",
                                latest_dep
                            )
                            
                        })
                        .clone(),
                )
            })
            .collect::<HashMap<String, HashSet<String>>>();

        let mut val = expr.clone();
        let _ = self.evaluator.eval_expr(&mut val);

        let actor_ref = spawn(DefActor::new(
            latest_name.clone(),
            expr,
            val,
            def_arg_to_vals,
            def_arg_to_vars,
        ));
        self.defname_to_actors
            .insert(latest_name.clone(), actor_ref.clone());

        // subscribe to its dependencies
        // println!("{} subscribe to {:?}", name, def_args);
        for dep_name in def_args.iter() {
            let latest_dep = self.evaluator.version_map.get_latest(dep_name);
            // println!("{}", name);
            // synchronously wait for response
            let back_msg = self
                .ask_to_name(
                    &latest_dep,
                    Msg::Subscribe {
                        from_name: latest_name.clone(),
                        from_addr: actor_ref.clone(),
                    },
                )
                .await?;

            if !matches!(back_msg, Msg::SubscribeGranted { .. }) {
                panic!("Service alloc: receive wrong message type during subscription");
            }

            actor_ref.tell(back_msg).await?;
        }

        Ok(actor_ref)
    }
}

// later when we want to fully implement services and code update
// here are some todos
// - how to process remote var/def values
// - incrementally update dep_graph and related data structures in srv manager
