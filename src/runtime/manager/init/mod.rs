use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::rc::Rc;
use std::thread::current;

use kameo::{prelude::*, spawn, Actor};
use log::info;

use crate::runtime::manager::Manager;
use crate::{
    ast::{Expr, Prog, Service},
    runtime::{def_actor::DefActor, evaluator::eval_srv, message::Msg, var_actor::VarActor},
    static_analysis::var_analysis::calc_dep_srv,
};

pub mod alloc_actors;

impl Manager {
    pub async fn alloc_service(&mut self, srv: &Service) {
        // intial evaluation of srv
        self.evaluator = eval_srv(srv);

        let srv_info = calc_dep_srv(srv);
        self.dep_graph = srv_info.dep_graph;
        self.dep_tran_vars = srv_info.dep_vars;

        for name in srv_info.topo_order.iter() {
            // Get the latest versioned name
            let latest_name = self.evaluator.version_map.get_latest(name);

            let val = self
                .evaluator
                .reactive_name_to_vals
                .get(&latest_name)
                .unwrap_or_else(|| {
                    panic!(
                        "Service alloc: var/def is not initialized: original name = {}, latest = {}",
                        name, latest_name
                    )
                });

            if srv_info.vars.contains(name) {
                self.alloc_var_actor(&latest_name, val.clone()).await;
            } else if srv_info.defs.contains(name) {
                let def_expr = self.evaluator.def_name_to_exprs.get(&latest_name).expect(&format!(
                    "Service alloc: def expr is not initialized: original name = {}, latest = {}",
                    name, latest_name
                ));

                self.alloc_def_actor(&latest_name, def_expr.clone(), None)
                    .await
                    .unwrap();
            }
        }

        info!("Service allocated: {}", self);
    }
}
