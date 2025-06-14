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
use std::{collections::HashMap, thread::sleep};

use crate::{
    ast::{Expr, Prog, ReplCmd, Service, Test},
    runtime::{message::CmdMsg, transaction::TxnId},
};
use kameo::{
    actor::ActorRef,
    prelude::{Context, Message},
    spawn, Actor,
};
use log::info;
use manager::Manager;
use tokio::sync::mpsc::{self, Receiver, Sender};

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

const MPSC_CHANNEL_SIZE: usize = 100;

pub async fn run(prog: &Prog) -> Result<(), Box<dyn std::error::Error>> {
    let (dev_tx, dev_rx) = mpsc::channel::<CmdMsg>(MPSC_CHANNEL_SIZE);
    let (cli_tx, cli_rx) = mpsc::channel::<CmdMsg>(MPSC_CHANNEL_SIZE);

    assert!(
        prog.services.len() == 1 && prog.tests.len() == 1,
        "Only support one service and one test for now"
    );

    let srv = &prog.services[0];
    let test = &prog.tests[0];

    let srv_actor_ref = run_srv(srv, dev_tx.clone()).await?;
    run_test(test, srv_actor_ref, cli_tx.clone(), cli_rx, dev_rx).await?;

    Ok(())
}

pub async fn run_srv(
    srv: &Service,
    dev_tx: Sender<CmdMsg>,
) -> Result<ActorRef<Manager>, Box<dyn std::error::Error>> {
    // initialize the service's manager
    let srv_manager = Manager::new(srv.name.clone(), dev_tx);
    let srv_actor_ref = spawn(srv_manager);

    // synchronously wait for manager to be initialized
    if let Some(CmdMsg::CodeUpdateGranted { .. }) = srv_actor_ref
        .ask(CmdMsg::CodeUpdate { srv: srv.clone() })
        .await?
    {
        println!("Service {} initialized", srv.name);
    } else {
        panic!("Service {} initialization failed", srv.name);
    }

    Ok(srv_actor_ref)
}

pub async fn run_test(
    test: &Test,
    srv_actor_ref: ActorRef<Manager>,
    cli_tx: Sender<CmdMsg>,
    mut cli_rx: Receiver<CmdMsg>,
    mut dev_rx: Receiver<CmdMsg>,
) -> Result<(), Box<dyn std::error::Error>> {
    // start testing on the service
    println!("testing {}", test.name);
    // one handler loop keep listening to incoming messages from manager
    // the main thread keep processing actions and asserts
    // if handler loop hear TransactionAbort, roll back to that transaction
    // if handler loop hear all assert true, roll forward to next action
    let mut txn_to_cmd_idx = HashMap::new();
    let mut process_cmd_idx = 0;

    while process_cmd_idx < test.commands.len() {
        let cmd = &test.commands[process_cmd_idx];
        match cmd {
            ReplCmd::Do(action) => {
                let txn_id = TxnId::new();
                txn_to_cmd_idx.insert(txn_id.clone(), process_cmd_idx);
                srv_actor_ref
                    .tell(CmdMsg::DoAction {
                        txn_id,
                        action: action.clone(),
                        from_client_addr: cli_tx.clone(),
                    })
                    .await?;
            }
            ReplCmd::Assert(expr) => {
                srv_actor_ref
                    .tell(CmdMsg::TryAssert {
                        name: test.name.clone(),
                        test: expr.clone(),
                    })
                    .await?;
            }
        }

        'listen: loop {
            // sleep 5 s for debugging
            // sleep(std::time::Duration::from_secs(5));
            println!("process_cmd_idx: {}", process_cmd_idx);
            tokio::select! {
                maybe_msg = dev_rx.recv() => {
                    if let Some(CmdMsg::AssertSucceeded) = maybe_msg {
                        println!("assert succeeded");
                        process_cmd_idx += 1;
                        break 'listen;
                    }
                }
                maybe_msg = cli_rx.recv() => {
                    if let Some(msg) = maybe_msg {
                        match msg {
                            CmdMsg::TransactionAborted { txn_id } => {
                                process_cmd_idx = *txn_to_cmd_idx.get(&txn_id)
                                .expect("txn_id not found");
                                break 'listen;
                            }
                            CmdMsg::TransactionCommitted { txn_id } => {
                                info!("Transaction {:?} committed", txn_id);
                                process_cmd_idx += 1;
                                break 'listen;
                            }
                            _ => panic!("unexpected message")
                        }
                    }
                }
            }

            println!(".");
        }
    }
    println!("pass");
    Ok(())
}
