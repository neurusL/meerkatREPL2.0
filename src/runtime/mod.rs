use core::panic;
use std::collections::HashMap;

use kameo::{actor::ActorRef, spawn};
use manager::Manager;
use message::Msg;
use crate::ast::{Prog, Test};

// pub mod instr;
pub mod lock;
pub mod message;
pub mod transaction;
// pub mod varworker;
// pub mod defworker;
// pub mod def_batch_utils;
// pub mod def_eval;

pub mod evaluator;

pub mod def_actor;
pub mod manager;
pub mod pubsub;
pub mod var_actor;


pub struct RuntimeManager {
    pub srv_to_mgr: HashMap<String, ActorRef<Manager>>,
}

impl RuntimeManager {
    pub async fn run(prog: &Prog) -> Result<(), Box<dyn std::error::Error>> {
        // initialize all services' managers
        let mut srv_to_mgr = HashMap::new();

        for srv in prog.services.iter() {
            let srv_manager = Manager::new(srv.name.clone());
            let srv_actor_ref = spawn(srv_manager);
            srv_to_mgr.insert(srv.name.clone(), srv_actor_ref.clone());
            
            // synchronously wait for manager to be initialized
            if let Some(Msg::CodeUpdateGranted) = srv_actor_ref
            .ask(Msg::CodeUpdate{ srv: srv.clone() }).await? {
                println!("Service {} initialized", srv.name);
            } else {
                panic!("Service {} initialization failed", srv.name);
            }
        }

        // println!("{:?}", prog.tests);
        for test in prog.tests.iter() {
            let srv_actor_ref = srv_to_mgr.get_mut(&test.name)
            .expect(&format!("Test: manager for service {:?} not found", test.name));
            
            if let Some(Msg::TryTestPass) = srv_actor_ref.ask(Msg::TryTest { test: test.clone() }).await? {
                println!("Test for {} passed", test.name);
            } else {
                panic!("Test for {} failed", test.name);
            }
        }

        Ok(())
    }
}