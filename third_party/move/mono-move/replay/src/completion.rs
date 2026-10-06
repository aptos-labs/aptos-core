// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Completes a record's state so the comparison can conclude on it.
//!
//! A record holds the state one V1 run read. A replay can read more: MonoMove reads keys V1 does
//! not, the overrides change V1's path, and legacy dumps never recorded the keys V1 found absent.
//! Completion replays both VMs on the patched record state, as `compare` does, and fetches every
//! key either read that the record holds no observation of, as a value or as an absence. It
//! repeats, since a fetched value can lead to new reads, until no key is left.

use crate::{
    isolated::{self, ReplayInput, V2Limits, V2Outcome},
    overrides::OverrideConfig,
    v1,
};
use anyhow::Result;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue, TStateView},
    transaction::{PersistedAuxiliaryInfo, Transaction},
};
use mono_move_replay_common::modules::{close_module_graph, module_id_of};
use std::collections::{BTreeMap, BTreeSet};

/// Rounds of replay and fetch before giving up; each round only adds keys, so this bounds a
/// transaction whose reads keep growing with the state it is given.
const MAX_ROUNDS: usize = 8;

/// What completing a record's state did.
#[derive(Debug, Default)]
pub struct Completion {
    /// Keys added to the state.
    pub values: usize,
    /// Keys added as absent.
    pub absent: usize,
    /// Whether keys were still unobserved after the last round.
    pub incomplete: bool,
    /// Whether MonoMove's process crashed in its latest run (see [`isolated`]). Its reads are
    /// completed unless there are too many; the record is kept, so the comparison reports the crash.
    pub v2_crashed: bool,
}

/// The latest MonoMove run during completion: each key it read with the patched value it saw, and
/// whether it crashed. MonoMove sees the state only through those reads, so while they are
/// unchanged its run is too, whichever keys were fetched and however the overrides then
/// rewrote others.
struct LastV2 {
    seen: BTreeMap<StateKey, Option<StateValue>>,
    crashed: bool,
}

/// Completes `state` and `absent`, the record's own reads, layered over `framework`, by fetching
/// the keys a replay reads with `fetch`, which returns the value at the record's state version.
pub fn complete(
    txn: &Transaction,
    aux_info: PersistedAuxiliaryInfo,
    framework: &BTreeMap<StateKey, StateValue>,
    state: &mut BTreeMap<StateKey, StateValue>,
    absent: &mut BTreeSet<StateKey>,
    config: &OverrideConfig,
    limits: &V2Limits,
    mut fetch: impl FnMut(&StateKey) -> Result<Option<StateValue>>,
) -> Result<Completion> {
    let mut completion = Completion::default();
    // The latest MonoMove run. MonoMove is deterministic, so it is run again only once a key it
    // read has a different patched value: a run that hit the time limit costs the limit once, not
    // every round.
    let mut v2: Option<LastV2> = None;
    // Whether a crashed run's reads were fetched already: once, so that a crash on a key the
    // corpus guessed can be replayed on complete state, without chasing a crash that reads ever
    // further.
    let mut crash_reads_fetched = false;
    for _ in 0..MAX_ROUNDS {
        let unobserved = unobserved_reads(
            txn,
            aux_info,
            framework,
            state,
            absent,
            config,
            limits,
            &mut v2,
            !crash_reads_fetched,
        )?;
        // Only the crash's own keys: those also required are fetched as required.
        let required: BTreeSet<StateKey> = unobserved.required.into_iter().collect();
        let optional: Vec<StateKey> = unobserved
            .crash
            .into_iter()
            .filter(|key| !required.contains(key))
            .collect();
        crash_reads_fetched |= !optional.is_empty();
        if required.is_empty() && optional.is_empty() {
            completion.v2_crashed = v2.is_some_and(|v2| v2.crashed);
            return Ok(completion);
        }
        let mut fetched_module = false;
        for key in required {
            let value = fetch(&key)?;
            fetched_module |= value.is_some() && module_id_of(&key).is_some();
            record(&mut completion, state, absent, key, value);
        }
        // A crash's keys are fetched at best: one that cannot be is skipped, never the record.
        for key in optional {
            if let Ok(value) = fetch(&key) {
                fetched_module |= value.is_some() && module_id_of(&key).is_some();
                record(&mut completion, state, absent, key, value);
            }
        }
        // A module fetched brings its dependencies, which MonoMove loads up front: closed here at
        // once rather than one per round. One that cannot be fetched leaves the closure incomplete,
        // as for the record's own modules.
        if fetched_module {
            let closure = close_module_graph(
                state,
                |id| framework.contains_key(&StateKey::module_id(id)),
                &mut fetch,
            )?;
            anyhow::ensure!(
                closure.missing.is_empty(),
                "incomplete module closure, missing {:?}",
                closure.missing
            );
            completion.values += closure.fetched;
        }
    }
    // Only the required reads decide: a crashed run's are what the comparison reports the crash
    // with, not a reason to drop the record.
    completion.incomplete = !unobserved_reads(
        txn, aux_info, framework, state, absent, config, limits, &mut v2, false,
    )?
    .required
    .is_empty();
    completion.v2_crashed = v2.is_some_and(|v2| v2.crashed);
    Ok(completion)
}

/// The keys read on the patched state that it holds no observation of: those V1 and a MonoMove run
/// that finished read, which completion requires, and those a MonoMove process that crashed read,
/// which it fetches at best (up to [`isolated::CRASH_FETCH_LIMIT`]). A VM that fails or panics
/// still counts with the reads it made before. `v2` holds the latest MonoMove run, reused while
/// the keys it read are unchanged.
fn unobserved_reads(
    txn: &Transaction,
    aux_info: PersistedAuxiliaryInfo,
    framework: &BTreeMap<StateKey, StateValue>,
    state: &BTreeMap<StateKey, StateValue>,
    absent: &BTreeSet<StateKey>,
    config: &OverrideConfig,
    limits: &V2Limits,
    v2: &mut Option<LastV2>,
    with_crash_reads: bool,
) -> Result<Unobserved> {
    let input = ReplayInput {
        values: framework
            .iter()
            .chain(state.iter())
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        absent: absent.clone(),
        config: *config,
        txn: txn.clone(),
        aux_info,
    };
    let patched = input.patched()?;
    if let Some(last) = v2.as_ref() {
        for (key, value) in &last.seen {
            if patched.get_state_value(key)? != *value {
                *v2 = None;
                break;
            }
        }
    }

    let view = patched.view();
    let v1_stopped = matches!(
        v1::run_caught(&view, txn, aux_info),
        Ok(Ok(run)) if run.hit_metering_limit
    );
    let mut reads = view.into_reads();
    if v1_stopped {
        // Only V1 on observed state is V1 on chain state: a stop on guessed state (default
        // features, a key guessed absent) is completed first and run again; MonoMove is not run on
        // a transaction V1 may have bounded only by its meter.
        let required = patched.unobserved(&reads)?;
        if required.is_empty() {
            return Err(v1::MeteringStop.into());
        }
        return Ok(Unobserved {
            required,
            crash: vec![],
        });
    }

    if v2.is_none() {
        let report = isolated::run_v2(&input, limits)?;
        let mut seen = BTreeMap::new();
        for key in report.reads {
            let value = patched.get_state_value(&key)?;
            seen.insert(key, value);
        }
        *v2 = Some(LastV2 {
            seen,
            crashed: matches!(report.outcome, V2Outcome::Crashed(_)),
        });
    }
    let mut crash = vec![];
    if let Some(v2) = v2.as_ref() {
        if !v2.crashed {
            reads.extend(v2.seen.keys().cloned());
        } else if with_crash_reads && v2.seen.len() <= isolated::CRASH_FETCH_LIMIT {
            // Fetched too, so that a crash on a guessed key (or one that only hit the time limit
            // under load) can be replayed on complete state; past the bound, MonoMove ran away.
            let crash_reads = v2.seen.keys().cloned().collect();
            crash = patched.unobserved(&crash_reads)?;
        }
    }
    Ok(Unobserved {
        required: patched.unobserved(&reads)?,
        crash,
    })
}

/// The keys a round of completion found unobserved.
struct Unobserved {
    /// Read by V1, or by a MonoMove run that finished: the record is incomplete without them.
    required: Vec<StateKey>,
    /// Read by a MonoMove process that crashed: fetched at best.
    crash: Vec<StateKey>,
}

/// Adds `key`, fetched as `value`, to the state or the absences.
fn record(
    completion: &mut Completion,
    state: &mut BTreeMap<StateKey, StateValue>,
    absent: &mut BTreeSet<StateKey>,
    key: StateKey,
    value: Option<StateValue>,
) {
    match value {
        Some(value) => {
            state.insert(key, value);
            completion.values += 1;
        },
        None => {
            absent.insert(key);
            completion.absent += 1;
        },
    }
}
