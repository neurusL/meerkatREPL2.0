use std::collections::{HashMap, HashSet};

use kameo::actor::ActorRef;

use crate::{
    ast::Expr,
    new_runtime::{
        evaluator::{EvalError, Evaluator},
        message::{BasisStamp, ReactiveConfiguration, StampedValue, Version},
        service::ServiceActor,
    },
};

use super::ReactiveRef;

pub struct Reactive {
    service: ActorRef<ServiceActor>,
    definition: Option<Definition>,
    value: Option<StampedValue>,
    read_by: BasisStamp,

    changed: bool,
    version: Version,
}

impl Reactive {
    pub fn new(config: ReactiveConfiguration, service: ActorRef<ServiceActor>) -> Reactive {
        let mut reactive = Reactive {
            service: service.clone(),
            definition: None,
            value: None,
            read_by: BasisStamp::empty(),
            changed: false,
            version: Version::ZERO,
        };

        match config {
            ReactiveConfiguration::Variable { value } => {
                reactive.value = Some(StampedValue {
                    value,
                    basis: BasisStamp::empty(),
                });
                reactive.changed = true;
            }
            ReactiveConfiguration::Definition { expr } => {
                reactive.definition = Some(Definition::new(expr, service));
            }
        }

        reactive
    }

    pub fn reconfigure(&mut self, config: ReactiveConfiguration, stamp: &BasisStamp) {
        match config {
            ReactiveConfiguration::Variable { value } => {
                self.definition = None;
                self.value = Some(StampedValue {
                    value,
                    basis: stamp.clone(),
                });
            }
            ReactiveConfiguration::Definition { expr } => {
                let definition = if let Some(definition) = &mut self.definition {
                    definition.reconfigure(expr);
                    definition
                } else {
                    self.definition
                        .insert(Definition::new(expr, self.service.clone()))
                };

                self.value = definition.compute();
            }
        }

        self.changed = true;
        self.version = self.version.increment();
    }

    pub fn inputs(&self) -> impl Iterator<Item = &ReactiveRef> {
        self.definition.iter().flat_map(|d| d.inputs.keys())
    }

    pub fn add_update(&mut self, sender: ReactiveRef, value: StampedValue) {
        if let Some(definition) = &mut self.definition {
            definition.add_update(sender, value)
        } else {
            panic!("attempted to add update to variable")
        }
    }

    pub fn next_value<'a>(
        &mut self,
        roots: impl Fn(&ReactiveRef) -> Option<&'a HashSet<ReactiveRef>>,
    ) -> Option<&StampedValue> {
        if self.changed {
            self.changed = false;

            if self.value.is_some() {
                return self.value.as_ref();
            }
        }

        if let Some(definition) = &mut self.definition {
            if let Some(new_value) = definition.find_and_apply_batch(roots) {
                self.value = Some(new_value);

                return self.value.as_ref();
            }
        }

        None
    }

    pub fn value(&self) -> Option<&StampedValue> {
        self.value.as_ref()
    }

    pub fn finished_read(&mut self, basis: &BasisStamp) {
        self.read_by.merge_from(basis);
    }

    pub fn write(&mut self, mut value: StampedValue) {
        assert!(self.definition.is_none());
        value.basis.merge_from(&self.read_by);
        self.value = Some(value);
        self.read_by.clear();
        self.changed = true;
    }

    pub fn version(&self) -> Version {
        self.version
    }
}

struct Definition {
    service: ActorRef<ServiceActor>,
    evaluator: Evaluator,
    inputs: HashMap<ReactiveRef, Input>,
    expr: Expr,
}

#[derive(Debug)]
struct Input {
    value: Option<StampedValue>,
    updates: Vec<StampedValue>,
}

struct EvalContext<'a>(&'a HashMap<ReactiveRef, Input>);

#[derive(Debug)]
struct BatchInput<'a> {
    roots: HashSet<ReactiveRef>,
    basis: BasisStamp,
    remaining_updates: &'a [StampedValue],
    update_count: usize,
}

impl Definition {
    pub fn new(expr: Expr, service: ActorRef<ServiceActor>) -> Definition {
        let mut inputs = HashMap::new();

        let mut evaluator = Evaluator::new();

        evaluator.visit_reads(&expr, &service, &mut |address: ReactiveRef| {
            inputs.insert(address.clone(), Input::new());
        });

        Definition {
            service,
            evaluator,
            inputs,
            expr,
        }
    }

    pub fn reconfigure(&mut self, expr: Expr) {
        let mut referenced_inputs = HashSet::new();
        self.evaluator
            .visit_reads(&expr, &self.service, &mut |address: ReactiveRef| {
                referenced_inputs.insert(address.clone());
                self.inputs
                    .entry(address.clone())
                    .or_insert_with(Input::new);
            });
        self.inputs
            .retain(|address, _| referenced_inputs.contains(address));
        self.expr = expr;
    }

    fn compute(&mut self) -> Option<StampedValue> {
        let mut expr = self.expr.clone();
        let result = self
            .evaluator
            .eval_expr(&mut expr, &self.service, &mut |address| {
                self.inputs
                    .get(&address)
                    .unwrap()
                    .value
                    .as_ref()
                    .map(|v| v.value.clone())
            });
        match result {
            Ok(()) => Some(StampedValue {
                value: expr,
                basis: self
                    .inputs
                    .values()
                    .map(|input| &input.value.as_ref().unwrap().basis)
                    .fold(BasisStamp::empty(), |mut a, b| {
                        a.merge_from(&b);
                        a
                    }),
            }),
            Err(EvalError::UnknownVariable(_)) => None,
            Err(EvalError::Other(msg)) => panic!("{msg}"),
        }
    }

    fn add_update(&mut self, sender: ReactiveRef, value: StampedValue) {
        self.inputs
            .get_mut(&sender)
            .expect("received update from unknown input")
            .updates
            .push(value);
    }

    // a: [4, 5, 6, 7]
    // b: [3, 4]
    //
    // building batch: { a1, b1, b2 }
    // { b: 2, a: 3 }

    fn find_and_apply_batch<'a>(
        &mut self,
        roots: impl Fn(&ReactiveRef) -> Option<&'a HashSet<ReactiveRef>>,
    ) -> Option<StampedValue> {
        let mut found = None;

        let mut explored = HashSet::new();
        'seeds: for seed in self.inputs.keys() {
            let mut inputs = self
                .inputs
                .iter()
                .map(|(address, input)| {
                    (
                        address,
                        BatchInput {
                            roots: roots(address)
                                .expect("input is locally inaccessible")
                                .clone(),
                            basis: input
                                .value
                                .as_ref()
                                .map(|v| v.basis.clone())
                                .unwrap_or(BasisStamp::empty()),
                            // Don't include any updates if this is an input we've already con-
                            // sidered as a seed. Since it was considered already, we know there
                            // are definitely no valid batches available now that involve this
                            // input.
                            remaining_updates: if !explored.contains(address) {
                                &*input.updates
                            } else {
                                &[]
                            },
                            update_count: if !explored.contains(address) {
                                input.updates.len()
                            } else {
                                0
                            },
                        },
                    )
                })
                .collect::<HashMap<_, _>>();

            let seed_input = inputs.get_mut(seed).unwrap();
            let Some((seed_update, rest)) = seed_input.remaining_updates.split_first() else {
                explored.insert(seed.clone());
                continue 'seeds;
            };
            seed_input.remaining_updates = rest;
            seed_input.basis = seed_update.basis.clone();

            let mut basis = seed_update.basis.clone();

            while {
                let mut changed = false;
                for (_, input) in inputs.iter_mut() {
                    while !basis.prec_eq_wrt_roots(&input.basis, &input.roots) {
                        let Some((update, rest)) = input.remaining_updates.split_first() else {
                            // We need an update from this input, but the input does not have an
                            // update to give us. That means there is no batch possible for the
                            // current seed.
                            explored.insert(seed.clone());
                            continue 'seeds;
                        };

                        input.remaining_updates = rest;
                        input.basis = update.basis.clone();
                        basis.merge_from(&update.basis);

                        changed = true;
                    }
                }
                changed
            } {}

            // Explanation: The number of updates we popped off the queue of each input.
            let update_counts = inputs
                .into_iter()
                .map(|(address, input)| {
                    (
                        address.clone(),
                        input.update_count - input.remaining_updates.len(),
                    )
                })
                .collect::<Vec<_>>();

            found = Some((update_counts, basis));
        }

        let Some((update_counts, mut basis)) = found else {
            return None;
        };

        let mut complete = true;
        for (address, update_count) in update_counts {
            let input = self.inputs.get_mut(&address).unwrap();

            debug_assert!(input.updates.len() <= update_count);

            if let Some(value) = input.updates.drain(0..update_count).last() {
                input.value = Some(value);
            } else if let Some(value) = &input.value {
                // The basis we computed earlier only includes basis stamps from updated inputs.
                // But we need to include the basis stamp from every input. Since this one was not
                // updated, it has not been included yet, and so we need to add it.
                basis.merge_from(&value.basis);
            } else {
                complete = false;
            }
        }

        if !complete {
            return None;
        }

        let mut expr = self.expr.clone();
        let result = self
            .evaluator
            .eval_expr(&mut expr, &self.service, &mut |address| {
                self.inputs
                    .get(&address)
                    .unwrap()
                    .value
                    .as_ref()
                    .map(|v| v.value.clone())
            });
        match result {
            Ok(()) => {}
            Err(err) => panic!("expr did not fully evaluate: {}", err),
        }

        Some(StampedValue { value: expr, basis })
    }
}

impl Input {
    pub fn new() -> Input {
        Input {
            value: None,
            updates: Vec::new(),
        }
    }
}

/*
impl<'a> ExprEvalContext<ReactiveAddress> for EvalContext<'a> {
    fn read(&mut self, address: &ReactiveAddress) -> Option<&Value> {
        match self.0.get(address) {
            Some(input) => match &input.value {
                Some(value) => Some(&value.value),
                None => None,
            },
            None => None,
        }
    }
}*/
