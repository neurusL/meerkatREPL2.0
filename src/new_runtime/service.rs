use std::{
    collections::{btree_map::Entry, hash_map, BTreeMap, HashMap, HashSet, VecDeque},
    hash::Hash,
};

use held_locks::{ExclusiveLockState, HeldLocks, Read, SharedLockState};
use kameo::{actor::ActorRef, prelude::Message, Actor};
use reactive::Reactive;

use crate::new_runtime::message::{BasisStamp, Iteration, LockKind, Msg, StampedValue, TxId};

mod held_locks;
mod reactive;

type Context = kameo::message::Context<ServiceActor, ()>;

pub struct ServiceActor {
    queued: BTreeMap<TxId, LockKind>,
    held: HeldLocks,
    preempted: HashSet<TxId>,

    imports: HashMap<ReactiveRef, Import>,
    reactives: HashMap<ReactiveName, Reactive>,
    iterations: HashMap<ReactiveName, Iteration>,
    exports: HashMap<ReactiveName, Export>,

    subscriptions: HashMap<ReactiveName, HashSet<ReactiveName>>,
    roots: HashMap<ReactiveName, HashSet<ReactiveRef>>,
    topo: VecDeque<ReactiveName>,
}

#[derive(Debug, Clone)]
pub struct ReactiveRef {
    pub service: ActorRef<ServiceActor>,
    pub name: ReactiveName,
}

impl PartialEq for ReactiveRef {
    fn eq(&self, other: &Self) -> bool {
        self.service.id() == other.service.id() && self.name == other.name
    }
}

impl Eq for ReactiveRef {}

impl Hash for ReactiveRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.service.id().hash(state);
        self.name.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReactiveName(pub String);

#[derive(Debug, Clone)]
pub struct Import {
    pub roots: HashSet<ReactiveRef>,
    pub importers: HashSet<ReactiveName>,
}

#[derive(Debug, Clone)]
pub struct ReactiveInputs {
    pub inputs: HashSet<ReactiveRef>,
}

#[derive(Debug, Clone)]
pub struct Export {
    /// Exports' roots only contain cross-network roots, since they are themselves sources standing
    /// in for each of the local reactive state variables (if any).
    pub roots: HashSet<ReactiveRef>,
    pub importers: HashSet<ActorRef<ServiceActor>>,
}

#[derive(Debug)]
struct Cyclical;

impl Actor for ServiceActor {
    type Args = ();
    type Error = ();

    async fn on_start(_: (), _: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(ServiceActor {
            queued: BTreeMap::new(),
            held: HeldLocks::None,
            preempted: HashSet::new(),
            imports: HashMap::new(),
            reactives: HashMap::new(),
            iterations: HashMap::new(),
            exports: HashMap::new(),
            subscriptions: HashMap::new(),
            roots: HashMap::new(),
            topo: VecDeque::new(),
        })
    }
}

impl ServiceActor {
    async fn grant_locks(&mut self, ctx: &Context) {
        let mut granted = Vec::new();

        for (txid, kind) in self.queued.iter() {
            match &mut self.held {
                // if no locks are held, we can grant this queued lock unconditionally
                held @ HeldLocks::None => match kind {
                    LockKind::Shared => {
                        *held = HeldLocks::Shared(BTreeMap::from([(
                            txid.clone(),
                            SharedLockState::default(),
                        )]));
                    }
                    LockKind::Exclusive => {
                        *held = HeldLocks::Exclusive(
                            txid.clone(),
                            SharedLockState::default(),
                            ExclusiveLockState::default(),
                        );
                    }
                },

                // if shared locks are held, we can grant only shared locks
                HeldLocks::Shared(held) => match kind {
                    LockKind::Shared => {
                        held.insert(txid.clone(), SharedLockState::default());
                    }
                    LockKind::Exclusive => {
                        // request preemption of all held shared locks younger than the queued
                        // exclusive lock
                        for (shared_txid, _) in held.iter_mut().rev() {
                            if shared_txid < txid {
                                break;
                            }

                            Self::preempt(&mut self.preempted, shared_txid).await;
                        }

                        break;
                    }
                },

                // if an exclusive lock is held, we can grant no locks
                HeldLocks::Exclusive(held_txid, _, _) => {
                    // request preemption of the exclusive lock if it is younger than the queued lock
                    if txid < held_txid {
                        Self::preempt(&mut self.preempted, txid).await;
                    }

                    break;
                }
            }

            // if control flow reaches here, the lock has now been granted
            granted.push(txid.clone());
        }

        for txid in granted {
            self.queued.remove(&txid);
            txid.manager
                .tell(Msg::LockGranted {
                    txid: txid.clone(),
                    service: ctx.actor_ref().clone(),
                    reactives: self
                        .reactives
                        .iter()
                        .map(|(name, r)| (name.clone(), r.version()))
                        .collect(),
                })
                .await
                .unwrap();
        }
    }

    async fn commit<'a>(
        &mut self,
        mut basis: BasisStamp,
        shared_state: SharedLockState,
        exclusive_state: ExclusiveLockState,
        ctx: &mut Context,
    ) -> bool {
        for (id, read) in shared_state.reads {
            if !read.complete.is_empty() {
                self.reactives.get_mut(&id).unwrap().finished_read(&basis);
            }
        }

        let mut modified = exclusive_state
            .writes
            .keys()
            .cloned()
            .collect::<HashSet<_>>();

        for (name, value) in exclusive_state.writes {
            // The direct writes won't necessarily be included in the basis since this reactive
            // might not be exported. Local-only basis roots like these are filtered out when
            // propagating basis stamps to other network nodes in propagate().
            basis.roots.insert(
                ReactiveRef {
                    service: ctx.actor_ref().clone(),
                    name: name.clone(),
                },
                exclusive_state.prepared_iterations[&name],
            );

            self.reactives.get_mut(&name).unwrap().write(StampedValue {
                value,
                basis: basis.clone(),
            });
        }

        for (address, config) in exclusive_state.imports {
            if let Some(config) = config {
                match self.imports.entry(address) {
                    hash_map::Entry::Vacant(e) => {
                        e.insert(Import {
                            roots: config.roots,
                            importers: HashSet::new(),
                        });
                    }
                    hash_map::Entry::Occupied(e) => {
                        e.into_mut().roots = config.roots;
                    }
                }
            } else if let Some(removed) = self.imports.remove(&address) {
                assert!(
                    removed
                        .importers
                        .into_iter()
                        .all(|id| exclusive_state.reactives.contains_key(&id)),
                    "not all importers of a removed import {:?} are being updated",
                    address,
                );
            }
        }

        let reactives_changed = !exclusive_state.reactives.is_empty();

        for (name, config) in exclusive_state.reactives {
            if let Some(config) = config {
                self.subscriptions
                    .entry(name.clone())
                    .or_insert_with(HashSet::new);
                self.iterations
                    .entry(name.clone())
                    .or_insert(Iteration::ZERO);

                let (reactive, mut prior_inputs) = match self.reactives.entry(name.clone()) {
                    hash_map::Entry::Vacant(e) => (
                        e.insert(Reactive::new(config, ctx.actor_ref().clone())),
                        HashSet::new(),
                    ),
                    hash_map::Entry::Occupied(e) => {
                        let reactive = e.into_mut();
                        let prior_inputs = reactive.inputs().cloned().collect::<HashSet<_>>();
                        reactive.reconfigure(config, &basis);
                        (reactive, prior_inputs)
                    }
                };

                for input in reactive.inputs() {
                    if prior_inputs.contains(input) {
                        prior_inputs.remove(input);
                        continue;
                    }

                    if input.service.id() == ctx.actor_ref().id() {
                        self.subscriptions
                            .entry(input.name.clone())
                            .or_insert_with(HashSet::new)
                            .insert(name.clone());
                    } else {
                        self.imports
                            .get_mut(input)
                            .expect("attempted to reference nonexistent import")
                            .importers
                            .insert(name.clone());
                    }
                }

                for removed in prior_inputs {
                    if removed.service.id() == ctx.actor_ref().id() {
                        self.subscriptions
                            .get_mut(&removed.name)
                            .unwrap()
                            .remove(&name);
                    } else {
                        let import = self.imports.get_mut(&removed).unwrap();
                        import.importers.remove(&name);
                        if import.importers.is_empty() {
                            self.imports.remove(&removed);
                        }
                    }
                }

                modified.insert(name);
            } else if let Some(removed) = self.reactives.remove(&name) {
                self.iterations.remove(&name);

                for input in removed.inputs() {
                    if input.service.id() == ctx.actor_ref().id() {
                        self.subscriptions
                            .get_mut(&input.name)
                            .map(|i| i.remove(&name));
                    } else {
                        self.imports
                            .get_mut(input)
                            .map(|i| i.importers.remove(&name));
                    }
                }
            }
        }

        if reactives_changed {
            self.recompute_topo();
            self.recompute_roots(&ctx);
        }

        for (name, addrs) in exclusive_state.exports {
            if addrs.is_empty() {
                self.exports.remove(&name);
            } else {
                self.exports.insert(
                    name.clone(),
                    Export {
                        roots: self.roots[&name]
                            .iter()
                            .filter(|r| r.service.id() != ctx.actor_ref().id())
                            .cloned()
                            .collect(),
                        importers: addrs,
                    },
                );
            }
        }

        self.iterations.extend(exclusive_state.prepared_iterations);

        self.propagate(modified, &ctx).await;

        false
    }

    fn recompute_topo(&mut self) {
        let mut visited = HashMap::new();
        self.topo.clear();
        for name in self.reactives.keys() {
            Self::topo_dfs(
                &self.subscriptions,
                &mut self.topo,
                &mut visited,
                name.clone(),
            )
            .expect("dependency graph is locally cyclical");
        }
    }

    fn topo_dfs(
        subscriptions: &HashMap<ReactiveName, HashSet<ReactiveName>>,
        topo: &mut VecDeque<ReactiveName>,
        visited: &mut HashMap<ReactiveName, bool>,
        name: ReactiveName,
    ) -> Result<(), Cyclical> {
        match visited.get(&name) {
            Some(true) => return Ok(()),
            Some(false) => return Err(Cyclical),
            None => (),
        }

        visited.insert(name.clone(), false);

        for sub in &subscriptions[&name] {
            Self::topo_dfs(subscriptions, topo, visited, sub.clone())?;
        }

        topo.push_front(name.clone());
        visited.insert(name, true);

        Ok(())
    }

    fn recompute_roots(&mut self, ctx: &Context) {
        self.roots.clear();
        for name in &self.topo {
            let mut roots = HashSet::new();

            let mut has_inputs = false;
            for input in self.reactives[name].inputs() {
                has_inputs = true;

                if input.service.id() == ctx.actor_ref().id() {
                    roots.extend(self.roots[&input.name].iter().cloned());
                } else {
                    roots.extend(self.imports[input].roots.iter().cloned());
                }
            }

            if !has_inputs {
                roots.insert(ReactiveRef {
                    service: ctx.actor_ref().clone(),
                    name: name.clone(),
                });
            }

            self.roots.insert(name.clone(), roots);
        }

        for (id, export) in &mut self.exports {
            export.roots = self.roots[id]
                .iter()
                .filter(|r| r.service.id() != ctx.actor_ref().id())
                .cloned()
                .collect();
        }
    }

    async fn preempt(preempted: &mut HashSet<TxId>, txid: &TxId) {
        if preempted.insert(txid.clone()) {
            txid.manager
                .tell(Msg::Preempt { txid: txid.clone() })
                .await
                .unwrap();
        }
    }

    async fn propagate(&mut self, modified: HashSet<ReactiveName>, ctx: &Context) {
        let mut found = false;
        for name in &self.topo {
            if !found {
                if modified.contains(name) {
                    found = true;
                } else {
                    continue;
                }
            }

            let roots = |address: &ReactiveRef| {
                if address.service.id() == ctx.actor_ref().id() {
                    self.roots.get(&address.name)
                } else {
                    self.imports.get(address).map(|i| &i.roots)
                }
            };

            while let Some(value) = self
                .reactives
                .get_mut(name)
                .unwrap()
                .next_value(roots)
                .cloned()
            {
                println!(
                    "new value for {name:?} on {:?}:\n    {value:?}\n",
                    ctx.actor_ref()
                );
                for sub in self.subscriptions.get(name).unwrap() {
                    self.reactives.get_mut(sub).unwrap().add_update(
                        ReactiveRef {
                            service: ctx.actor_ref().clone(),
                            name: name.clone(),
                        },
                        value.clone(),
                    );
                }

                let value_without_local_only_bases = StampedValue {
                    value: value.value,
                    basis: BasisStamp {
                        roots: value
                            .basis
                            .roots
                            .into_iter()
                            .filter(|(a, _)| {
                                a.service.id() != ctx.actor_ref().id()
                                    || self.exports.contains_key(&a.name)
                            })
                            .collect(),
                    },
                };

                for addr in self
                    .exports
                    .get(name)
                    .iter()
                    .copied()
                    .flat_map(|e| e.importers.iter())
                {
                    addr.tell(Msg::Propagate {
                        sender: ReactiveRef {
                            service: ctx.actor_ref().clone(),
                            name: name.clone(),
                        },
                        value: value_without_local_only_bases.clone(),
                    })
                    .await
                    .unwrap();
                }
            }
        }

        self.grant_reads(&ctx).await;
    }

    async fn grant_reads(&mut self, ctx: &Context) {
        let mut messages = Vec::new();

        self.held.visit_shared(|txid, state| {
            for (name, read) in &mut state.reads {
                // If a read is completed, the `complete` stamp has to have SOMETHING in it -- no
                // value can have an empty basis stamp. Hence if `complete` is empty, the read is
                // pending and needs to be sent out.
                //
                // Alternatively, we may have already completed a read, but another is pending.
                if read.complete.is_empty() || !read.pending.is_empty() {
                    if let Some(value) = self.reactives.get(&name).unwrap().value() {
                        let roots = self.roots.get(name).unwrap();

                        if read.pending.prec_eq_wrt_roots(&value.basis, roots) {
                            messages.push((txid.clone(), name.clone(), value.value.clone()));

                            read.complete.merge_from(&value.basis);
                        }
                    }
                }
            }
        });

        for (txid, reactive, value) in messages {
            txid.manager
                .clone()
                .tell(Msg::ReturnedValue {
                    txid,
                    service: ctx.actor_ref().clone(),
                    reactive,
                    value,
                })
                .await
                .unwrap();
        }
    }
}

impl Message<Msg> for ServiceActor {
    type Reply = ();

    async fn handle(&mut self, message: Msg, ctx: &mut Context) {
        match message {
            Msg::Lock { txid, kind } => {
                let Entry::Vacant(e) = self.queued.entry(txid) else {
                    panic!("lock was double-requested");
                };

                e.insert(kind);

                self.grant_locks(&ctx).await;
            }
            Msg::Abort { txid } => {
                match &mut self.held {
                    HeldLocks::None => panic!("abort of unheld lock requested"),
                    HeldLocks::Shared(held) => {
                        if held.remove(&txid).is_none() {
                            panic!("abort of unheld lock requested")
                        }

                        if held.is_empty() {
                            self.held = HeldLocks::None;
                        }
                    }
                    HeldLocks::Exclusive(held_txid, _, _) => {
                        if held_txid == &txid {
                            self.held = HeldLocks::None;
                        } else {
                            panic!("abort of unheld lock requested")
                        }
                    }
                }

                self.grant_locks(&ctx).await;
            }
            Msg::PrepareCommit { txid } => {
                let state = self
                    .held
                    .shared(&txid)
                    .expect("attempted to prepare commit for unheld lock");

                let mut basis =
                    state
                        .reads
                        .values()
                        .fold(BasisStamp::empty(), |mut basis, read| {
                            basis.merge_from(&read.complete);
                            basis
                        });

                if let Some(exclusive) = self.held.exclusive_mut(&txid) {
                    // For any direct writes to local reactives, we want to increment the iterations
                    // of all transitively dependent local reactives, including the written nodes
                    // themselves.
                    for name in &self.topo {
                        if exclusive.writes.contains_key(name) {
                            exclusive
                                .prepared_iterations
                                .insert(name.clone(), self.iterations[name].increment());
                        } else if self.reactives[name].inputs().any(|input| {
                            input.service.id() == ctx.actor_ref().id()
                                && exclusive.prepared_iterations.contains_key(&input.name)
                        }) {
                            exclusive
                                .prepared_iterations
                                .insert(name.clone(), self.iterations[name].increment());
                        }
                    }

                    // Only include exported reactives as roots in the basis. Note that we have to
                    // take care to respect the set of exports that will be set following commit of
                    // the transaction, rather than the current self.exports.
                    basis.roots.extend(
                        exclusive
                            .prepared_iterations
                            .iter()
                            .filter(|(id, _)| {
                                exclusive
                                    .exports
                                    .get(id)
                                    .map_or(self.exports.contains_key(id), |export| {
                                        !export.is_empty()
                                    })
                            })
                            .map(|(name, iter)| {
                                (
                                    ReactiveRef {
                                        service: ctx.actor_ref().clone(),
                                        name: name.clone(),
                                    },
                                    *iter,
                                )
                            }),
                    );
                }

                // TODO: **comprehensively** validate the update (ideally equivalent to fully
                // executing it), perhaps by doing it and adding an 'undo log' entry, so that no
                // can occur after CommitPrepared is sent

                txid.manager
                    .tell(Msg::CommitPrepared {
                        service: ctx.actor_ref().clone(),
                        txid: txid.clone(),
                        basis,
                    })
                    .await
                    .unwrap();
            }
            Msg::Commit { txid, basis } => {
                match std::mem::replace(&mut self.held, HeldLocks::None) {
                    HeldLocks::None => panic!("release of unheld lock requested"),
                    HeldLocks::Shared(mut held) => {
                        let data = held.remove(&txid);

                        if !held.is_empty() {
                            // restore the remaining held shared locks
                            self.held = HeldLocks::Shared(held);
                        }

                        if let Some(data) = data {
                            if self
                                .commit(basis, data, ExclusiveLockState::default(), ctx)
                                .await
                            {
                                ctx.actor_ref().stop_gracefully().await.unwrap();
                            } else {
                                return;
                            }
                        } else {
                            panic!("release of unheld lock requested")
                        }
                    }
                    HeldLocks::Exclusive(held_txid, shared_data, exclusive_data) => {
                        if held_txid == txid {
                            if self.commit(basis, shared_data, exclusive_data, ctx).await {
                                ctx.actor_ref().stop_gracefully().await.unwrap();
                            } else {
                                return;
                            }
                        } else {
                            // restore the unmatched exclusive lock
                            self.held =
                                HeldLocks::Exclusive(held_txid, shared_data, exclusive_data);

                            panic!("release of unheld lock requested")
                        }
                    }
                }

                self.grant_locks(&ctx).await;
            }
            Msg::ReadValue {
                txid,
                reactive,
                basis,
            } => {
                let Some(lock) = self.held.shared_mut(&txid) else {
                    panic!("attempted to read without a lock")
                };

                let Some(r) = self.reactives.get(&reactive) else {
                    panic!("attempted to read reactive {reactive:?} that could not be found")
                };

                let e = lock.reads.entry(reactive.clone());

                if let hash_map::Entry::Occupied(e) = &e {
                    let r = e.get();
                    if r.complete.is_empty() || !r.pending.is_empty() {
                        panic!("attempted to read while another read is still pending")
                    }
                }

                let read = e.or_insert(Read {
                    pending: BasisStamp::empty(),
                    complete: BasisStamp::empty(),
                });

                if let Some(value) = r.value() {
                    if basis.prec_eq_wrt_roots(&value.basis, self.roots.get(&reactive).unwrap()) {
                        txid.manager
                            .tell(Msg::ReturnedValue {
                                txid: txid.clone(),
                                service: ctx.actor_ref().clone(),
                                reactive: reactive,
                                value: value.value.clone(),
                            })
                            .await
                            .unwrap();

                        read.complete.merge_from(&value.basis);
                    } else {
                        read.pending = basis;
                    }
                } else {
                    read.pending = basis;
                }
            }
            Msg::Write {
                txid,
                reactive,
                value,
            } => {
                let state = self
                    .held
                    .exclusive_mut(&txid)
                    .expect("attempted to write without an exclusive lock");
                assert!(self.reactives.contains_key(&reactive));
                state.writes.insert(reactive, value);
            }
            Msg::ReadConfiguration { txid } => {
                self.held
                    .exclusive(&txid)
                    .expect("attempted to read configuration without an exclusive lock");

                txid.manager
                    .tell(Msg::ReturnedConfiguration {
                        txid: txid.clone(),
                        service: ctx.actor_ref().clone(),
                        imports: self.imports.clone(),
                        inputs: self
                            .reactives
                            .iter()
                            .map(|(name, r)| {
                                (
                                    name.clone(),
                                    ReactiveInputs {
                                        inputs: r.inputs().cloned().collect(),
                                    },
                                )
                            })
                            .collect(),
                        exports: self.exports.clone(),
                    })
                    .await
                    .unwrap();
            }
            Msg::Configure {
                txid,
                imports,
                reactives,
                exports,
            } => {
                let state = self
                    .held
                    .exclusive_mut(&txid)
                    .expect("attempted to configure without an exclusive lock");
                state.imports.extend(imports);
                state.reactives.extend(reactives);
                state.exports.extend(exports);
            }
            Msg::Propagate { sender, value } => {
                let Some(import) = self.imports.get(&sender) else {
                    return;
                };

                for id in &import.importers {
                    self.reactives
                        .get_mut(&id)
                        .unwrap()
                        .add_update(sender.clone(), value.clone());
                }

                self.propagate(import.importers.clone(), &ctx).await;
            }
            _ => todo!(),
        }
    }
}
