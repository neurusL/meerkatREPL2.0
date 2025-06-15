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
                let def_expr = self.evaluator.def_name_to_exprs.get(name).expect(&format!(
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
}
