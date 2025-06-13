//！ runtime and test infrastructure 
//!
//!  # How to use
//!  triggered when developer run @test(service_name) {
//!     do(action);
//!     assert(boolean_expr);
//!     assert(boolean_expr); // block next do(action) until evaled(true)
//!     ...
//!     do(action);
//!     do(action);
//!     ...
//! }
//! 
//!  # implementation Idea 
//!  1. each service initialize its own manager when declared 
//!  2. test instantiated by service_name, do actions on service_name's manager 
//!  3. test treat assert(boolean_expr) by internally allocate a def actor 
//!     of boolean_expr 
//!  4. test_manager will wait for bool_expr to be true before processing next 
//!     action, on the other hand, timeout means assertion failed
use core::panic;
use std::collections::{HashMap, HashSet};

use kameo::{actor::ActorRef, spawn};
use manager::Manager;
use message::Msg;
use crate::{ast::{Expr, Prog, Test}, runtime::message::CmdMsg};

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
            if let Some(CmdMsg::CodeUpdateGranted) = srv_actor_ref
            .ask(CmdMsg::CodeUpdate{ srv: srv.clone() }).await? {
                println!("Service {} initialized", srv.name);
            } else {
                panic!("Service {} initialization failed", srv.name);
            }
        }

        for test in prog.tests.iter() {
            println!("testing {} ...... ", test.name);
            let srv_actor_ref = srv_to_mgr.get_mut(&test.name)
            .expect(&format!("Test: manager for service {:?} not found", test.name));
            
            // if let Msg::TryTestResult { assert_actors } = srv_actor_ref.ask(Msg::TryTest { test: test.clone() }).await? {
            //     let mut assert_to_pass: HashSet<kameo::prelude::ActorID> = HashSet::from_iter(
            //         assert_actors.iter().map(|actor_ref| actor_ref.id())
            //     );
            //     loop {
            //         for actor_ref in assert_actors.iter() {
            //             if let Msg::UnsafeReadResult { 
            //                 result: Expr::Bool { val: true },
            //             } = actor_ref.ask(Msg::UnsafeRead).await? {
            //                 assert_to_pass.remove(&actor_ref.id());
            //                 if assert_to_pass.is_empty() {
            //                     println!("Test for {} passed", test.name);
            //                     break;
            //                 }
            //             }
            //         }
            //     }
            // } else {
            //     panic!("Test for {} failed", test.name);
            // }

            todo!()
        }

        Ok(())
    }
}