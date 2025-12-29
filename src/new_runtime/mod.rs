use std::{
    collections::{HashMap, HashSet},
    mem,
};

use kameo::{prelude::*, Actor};
use tokio::sync::oneshot;

use crate::{
    ast::{Decl, Expr, Prog, ReplCmd},
    new_runtime::{
        evaluator::{Evaluator},
        message::{
            BasisStamp, LockKind, MonotonicTimestampGenerator, Msg, ReactiveConfiguration, TxId,
        },
        service::{ReactiveName, ServiceActor},
    },
};

mod evaluator;
mod message;
mod service;

pub async fn run(prog: &Prog) -> Result<(), Box<dyn std::error::Error>> {
    for service in &prog.services {
        let actor = ServiceActor::spawn(());

        let mut reactives = HashMap::<ReactiveName, Option<ReactiveConfiguration>>::new();
        for decl in &service.decls {
            match decl {
                Decl::Import { srv_name } => todo!(),
                Decl::VarDecl { name, val } => {
                    reactives.insert(
                        ReactiveName(name.clone()),
                        Some(ReactiveConfiguration::Variable { value: val.clone() }),
                    );
                }
                Decl::DefDecl {
                    name,
                    val,
                    is_pub: _,
                } => {
                    reactives.insert(
                        ReactiveName(name.clone()),
                        Some(ReactiveConfiguration::Definition { expr: val.clone() }),
                    );
                }
            }
        }

        Configurator::spawn((actor.clone(), reactives))
            .wait_for_shutdown()
            .await;

        for test in prog.tests.iter().filter(|t| t.name == service.name) {
            let mut basis = BasisStamp::empty();
            for cmd in &test.commands {
                match cmd {
                    ReplCmd::Assert(expr) => {
                        Asserter::spawn((actor.clone(), expr.clone(), basis.clone()))
                            .wait_for_shutdown()
                            .await;
                    }
                    ReplCmd::Do(expr) => {
                        let (tx, rx) = oneshot::channel();
                        Doer::spawn((actor.clone(), expr.clone(), basis.clone(), tx))
                            .wait_for_shutdown()
                            .await;
                        basis = rx.await.unwrap();
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Configurator {
    LockRequested {
        service: ActorRef<ServiceActor>,
        reactives: HashMap<ReactiveName, Option<ReactiveConfiguration>>,
        txid: TxId,
    },
    AwaitingValues {
        service: ActorRef<ServiceActor>,
        reactives: HashMap<ReactiveName, Option<ReactiveConfiguration>>,
        txid: TxId,
        partial_values: HashMap<ReactiveName, Expr>,
        needed: HashSet<ReactiveName>,
    },
    PrepareRequested {
        service: ActorRef<ServiceActor>,
        txid: TxId,
    },
    Fail,
}

impl Actor for Configurator {
    type Args = (
        ActorRef<ServiceActor>,
        HashMap<ReactiveName, Option<ReactiveConfiguration>>,
    );
    type Error = ();

    async fn on_start(
        (service, reactives): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let mut timestamp_generator = MonotonicTimestampGenerator::new();

        let txid = TxId {
            priority: message::TxPriority::High,
            timestamp: timestamp_generator.generate_timestamp(),
            manager: actor_ref.clone().reply_recipient(),
        };

        service
            .tell(Msg::Lock {
                txid: txid.clone(),
                kind: LockKind::Exclusive,
            })
            .await
            .unwrap();

        Ok(Configurator::LockRequested {
            service,
            reactives,
            txid,
        })
    }
}

impl Message<Msg> for Configurator {
    type Reply = ();

    async fn handle(&mut self, msg: Msg, ctx: &mut Context<Self, Self::Reply>) {
        match mem::replace(self, Configurator::Fail) {
            Configurator::LockRequested {
                service,
                reactives,
                txid,
            } => match msg {
                Msg::LockGranted {
                    txid: granted_txid,
                    service: sender_service,
                    ..
                } => {
                    assert_eq!(txid, granted_txid);
                    assert_eq!(service.id(), sender_service.id());

                    let mut reads = HashSet::new();
                    let mut e = Evaluator::new();
                    for reactive in reactives.values() {
                        if let Some(ReactiveConfiguration::Variable { value }) = reactive {
                            e.visit_reads(value, &service, &mut |read| {
                                assert_eq!(read.service.id(), service.id());
                                reads.insert(read.name);
                            });
                        }
                    }

                    if reads.is_empty() {
                        service
                            .tell(Msg::Configure {
                                txid: txid.clone(),
                                imports: HashMap::new(),
                                reactives,
                                exports: HashMap::new(),
                            })
                            .await
                            .unwrap();

                        service
                            .tell(Msg::PrepareCommit { txid: txid.clone() })
                            .await
                            .unwrap();

                        *self = Configurator::PrepareRequested { service, txid };
                    } else {
                        *self = Configurator::AwaitingValues {
                            service,
                            reactives,
                            txid,
                            partial_values: HashMap::new(),
                            needed: reads,
                        };
                    }
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Configurator::AwaitingValues {
                service,
                mut reactives,
                txid,
                mut partial_values,
                mut needed,
            } => match msg {
                Msg::ReturnedValue {
                    txid: returned_txid,
                    service: sender_service,
                    reactive,
                    value,
                } => {
                    assert_eq!(txid, returned_txid);
                    assert_eq!(service, sender_service);
                    assert!(!partial_values.contains_key(&reactive));
                    assert!(needed.remove(&reactive));

                    partial_values.insert(reactive, value);

                    if needed.is_empty() {
                        let mut e = Evaluator::new();
                        for reactive in reactives.values_mut() {
                            if let Some(ReactiveConfiguration::Variable { value }) = reactive {
                                e.eval_expr(value, &service, &mut |r| {
                                    if r.service == service {
                                        partial_values.get(&r.name).cloned()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap();
                            }
                        }

                        service
                            .tell(Msg::Configure {
                                txid: txid.clone(),
                                imports: HashMap::new(),
                                reactives,
                                exports: HashMap::new(),
                            })
                            .await
                            .unwrap();

                        service
                            .tell(Msg::PrepareCommit { txid: txid.clone() })
                            .await
                            .unwrap();

                        *self = Configurator::PrepareRequested { service, txid };
                    } else {
                        *self = Configurator::AwaitingValues {
                            service,
                            reactives,
                            txid,
                            partial_values,
                            needed,
                        };
                    }
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Configurator::PrepareRequested { service, txid } => match msg {
                Msg::CommitPrepared {
                    service: sender_service,
                    txid: prepared_txid,
                    basis,
                } => {
                    assert_eq!(service.id(), sender_service.id());
                    assert_eq!(txid, prepared_txid);

                    service
                        .tell(Msg::Commit {
                            txid: txid.clone(),
                            basis,
                        })
                        .await
                        .unwrap();

                    ctx.stop();
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            _ => panic!("Invalid state: {self:?}"),
        }
    }
}

#[derive(Debug)]
enum Doer {
    LockRequested {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        expr: Expr,
        basis: BasisStamp,
        finished: oneshot::Sender<BasisStamp>,
    },
    AwaitingValues {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        expr: Expr,
        partial_values: HashMap<ReactiveName, Expr>,
        needed: HashSet<ReactiveName>,
        basis: BasisStamp,
        finished: oneshot::Sender<BasisStamp>,
    },
    PrepareRequested {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        finished: oneshot::Sender<BasisStamp>,
    },
    Done,
}

impl Actor for Doer {
    type Args = (
        ActorRef<ServiceActor>,
        Expr,
        BasisStamp,
        oneshot::Sender<BasisStamp>,
    );
    type Error = ();

    async fn on_start(
        (service, expr, basis, finished): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, ()> {
        let mut timestamp_generator = MonotonicTimestampGenerator::new();

        let txid = TxId {
            priority: message::TxPriority::High,
            timestamp: timestamp_generator.generate_timestamp(),
            manager: actor_ref.clone().reply_recipient(),
        };

        service
            .tell(Msg::Lock {
                txid: txid.clone(),
                kind: LockKind::Exclusive,
            })
            .await
            .unwrap();

        Ok(Doer::LockRequested {
            service,
            txid,
            expr,
            basis,
            finished,
        })
    }
}

impl Doer {
    async fn execute_expr(&mut self, service: ActorRef<ServiceActor>, txid: TxId, mut expr: Expr, basis:BasisStamp, partial_values: HashMap<ReactiveName, Expr>, finished: oneshot::Sender<BasisStamp>) {
        let mut needed = HashSet::new();
        let mut e = Evaluator::new();

        let is_done = match expr {
            Expr::Action { .. } => true,
            _ => do_partial_eval(&service, &mut expr, &partial_values, &mut needed, &mut e),
        };

        if !is_done {
            // request variables that are read and not yet available
            request_vars(&service, &txid, &basis, &needed).await;

            *self = Doer::AwaitingValues {
                service,
                txid,
                expr,
                partial_values,
                needed,
                basis,
                finished,
            };
        } else {
            // we have a value; make sure it is an Action
            match expr {
                Expr::Action { mut assns } => {
                    // partially evaluate the code inside the action (RHS of assignments)
                    let mut done = true;
                    for assn in &mut assns {
                        done = done && do_partial_eval(&service, &mut assn.src, &partial_values, &mut needed, &mut e);
                    }
                    
                    if !done {
                        // request variables that are read and not yet available
                        request_vars(&service, &txid, &basis, &needed).await;

                        *self = Doer::AwaitingValues {
                            service,
                            txid,
                            expr: Expr::Action { assns },
                            partial_values,
                            needed,
                            basis,
                            finished,
                        };
                    } else {
                        // All the right hand sides are values!  Commit the transaction.
                        for assn in assns {
                            let name: ReactiveName = ReactiveName(assn.dest.clone());

                            service
                                .tell(Msg::Write {
                                    txid: txid.clone(),
                                    reactive: name,
                                    value: assn.src,
                                })
                                .await
                                .unwrap();
                        }

                        service
                            .tell(Msg::PrepareCommit { txid: txid.clone() })
                            .await
                            .unwrap();

                        *self = Doer::PrepareRequested {
                            service,
                            txid,
                            finished,
                        };
                    }
                }
                _ => panic!("Cannot evaluate a non-action {expr:?}"),
            }
        }

    }
}

fn do_partial_eval(service: &ActorRef<ServiceActor>, expr: &mut Expr, partial_values: &HashMap<ReactiveName, Expr>, needed: &mut HashSet<ReactiveName>, e: &mut Evaluator) -> bool {
    e.partial_eval(expr, service, &mut |read| {
        assert_eq!(read.service.id(), service.id());
        let opt_value = partial_values.get(&read.name);
        if opt_value == None {
            needed.insert(read.name);
        }
        opt_value.cloned()
    }).unwrap()
}

async fn request_vars(service: &ActorRef<ServiceActor>, txid: &TxId, basis: &BasisStamp, needed: &HashSet<ReactiveName>) {
    for reactive in needed {
        service
            .tell(Msg::ReadValue {
                txid: txid.clone(),
                reactive: reactive.clone(),
                basis: basis.clone(),
            })
            .await
            .unwrap();
    }
}

impl Message<Msg> for Doer {
    type Reply = ();

    async fn handle(&mut self, msg: Msg, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        match mem::replace(self, Doer::Done) {
            Doer::LockRequested {
                service,
                txid,
                expr,
                basis,
                finished,
            } => match msg {
                Msg::LockGranted {
                    txid: granted_txid,
                    service: sender_service,
                    ..
                } => {
                    assert_eq!(txid, granted_txid);
                    assert_eq!(service, sender_service);

                    let partial_values = HashMap::new();
                    self.execute_expr(service, txid, expr, basis, partial_values, finished).await;
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Doer::AwaitingValues {
                service,
                txid,
                mut expr,
                mut partial_values,
                mut needed,
                basis,
                finished,
            } => match msg {
                Msg::ReturnedValue {
                    txid: returned_txid,
                    service: sender_service,
                    reactive,
                    value,
                } => {
                    assert_eq!(txid, returned_txid);
                    assert_eq!(service, sender_service);
                    assert!(!partial_values.contains_key(&reactive));
                    assert!(needed.remove(&reactive));

                    partial_values.insert(reactive, value);

                    if needed.is_empty() {
                        self.execute_expr(service, txid, expr, basis, partial_values, finished).await;


                    } else {
                        *self = Doer::AwaitingValues {
                            service,
                            txid,
                            expr,
                            partial_values,
                            needed,
                            basis,
                            finished,
                        };
                    }
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Doer::PrepareRequested {
                service,
                txid,
                finished,
            } => match msg {
                Msg::CommitPrepared {
                    service: sender_service,
                    txid: prepared_txid,
                    basis,
                } => {
                    assert_eq!(service, sender_service);
                    assert_eq!(txid, prepared_txid);

                    service
                        .tell(Msg::Commit {
                            txid: txid.clone(),
                            basis: basis.clone(),
                        })
                        .await
                        .unwrap();

                    finished.send(basis).unwrap();

                    ctx.stop();
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            _ => panic!("Invalid state: {self:?}"),
        }
    }
}

#[derive(Debug)]
enum Asserter {
    LockRequested {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        expr: Expr,
        basis: BasisStamp,
    },
    AwaitingValues {
        service: ActorRef<ServiceActor>,
        txid: TxId,
        expr: Expr,
        partial_values: HashMap<ReactiveName, Expr>,
        needed: HashSet<ReactiveName>,
    },
    PrepareRequested {
        service: ActorRef<ServiceActor>,
        txid: TxId,
    },
    Done,
}

impl Actor for Asserter {
    type Args = (ActorRef<ServiceActor>, Expr, BasisStamp);
    type Error = ();

    async fn on_start(
        (service, expr, basis): Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, ()> {
        let mut timestamp_generator = MonotonicTimestampGenerator::new();

        let txid = TxId {
            priority: message::TxPriority::High,
            timestamp: timestamp_generator.generate_timestamp(),
            manager: actor_ref.clone().reply_recipient(),
        };

        service
            .tell(Msg::Lock {
                txid: txid.clone(),
                kind: LockKind::Exclusive,
            })
            .await
            .unwrap();

        Ok(Asserter::LockRequested {
            service,
            txid,
            expr,
            basis,
        })
    }
}

impl Message<Msg> for Asserter {
    type Reply = ();

    async fn handle(&mut self, msg: Msg, ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        match mem::replace(self, Asserter::Done) {
            Asserter::LockRequested {
                service,
                txid,
                expr,
                basis,
            } => match msg {
                Msg::LockGranted {
                    txid: granted_txid,
                    service: sender_service,
                    ..
                } => {
                    assert_eq!(txid, granted_txid);
                    assert_eq!(service, sender_service);

                    let mut reads = HashSet::new();
                    let mut e = Evaluator::new();
                    e.visit_reads(&expr, &service, &mut |read| {
                        assert_eq!(read.service.id(), service.id());
                        reads.insert(read.name);
                    });

                    for reactive in &reads {
                        service
                            .tell(Msg::ReadValue {
                                txid: txid.clone(),
                                reactive: reactive.clone(),
                                basis: basis.clone(),
                            })
                            .await
                            .unwrap();
                    }

                    *self = Asserter::AwaitingValues {
                        service,
                        txid,
                        expr,
                        partial_values: HashMap::new(),
                        needed: reads,
                    };
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Asserter::AwaitingValues {
                service,
                txid,
                mut expr,
                mut partial_values,
                mut needed,
            } => match msg {
                Msg::ReturnedValue {
                    txid: returned_txid,
                    service: sender_service,
                    reactive,
                    value,
                } => {
                    assert_eq!(txid, returned_txid);
                    assert_eq!(service, sender_service);
                    assert!(!partial_values.contains_key(&reactive));
                    assert!(needed.remove(&reactive));

                    partial_values.insert(reactive, value);

                    if needed.is_empty() {
                        let mut e = Evaluator::new();
                        let expr_string = expr.to_string();
                        e.eval_expr(&mut expr, &service, &mut |r| {
                            if r.service == service {
                                partial_values.get(&r.name).cloned()
                            } else {
                                None
                            }
                        })
                        .unwrap();

                        assert_eq!(expr, Expr::Bool { val: true }, "assertion should succeed");
                        println!("assertion {expr_string} succeeded");

                        service
                            .tell(Msg::PrepareCommit { txid: txid.clone() })
                            .await
                            .unwrap();

                        *self = Asserter::PrepareRequested { service, txid };
                    } else {
                        *self = Asserter::AwaitingValues {
                            service,
                            txid,
                            expr,
                            partial_values,
                            needed,
                        };
                    }
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            Asserter::PrepareRequested { service, txid } => match msg {
                Msg::CommitPrepared {
                    service: sender_service,
                    txid: prepared_txid,
                    basis,
                } => {
                    assert_eq!(service, sender_service);
                    assert_eq!(txid, prepared_txid);

                    service
                        .tell(Msg::Commit {
                            txid: txid.clone(),
                            basis: basis.clone(),
                        })
                        .await
                        .unwrap();

                    ctx.stop();
                }
                _ => {
                    panic!("Unexpected message in current state: {msg:?}\nCurrent state: {self:?}")
                }
            },
            _ => panic!("Invalid state: {self:?}"),
        }
    }
}
