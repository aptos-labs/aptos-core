// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Recording what a transaction reads, so it can be replayed without the chain.

use anyhow::{bail, Result};
use aptos_infallible::Mutex;
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_slot::{StateSlot, StateSlotKind},
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateView, StateViewResult, TStateView,
    },
    transaction::Version,
};
use std::collections::{BTreeSet, HashMap};

/// A [`StateView`] that records every read so the set can be persisted as a replay's read set.
/// Mirrors `aptos-replay-benchmark`'s capturing view, including a preload (the framework) so the
/// prologue never misses framework modules.
///
/// Failed reads are recorded too: the VM swallows some read errors into an executed output (the
/// block-epilogue fallback, for one), which would silently produce an incomplete read set, so
/// [`Self::into_captured`] fails if any read failed.
pub struct ReadSetCapturingStateView<'s, S> {
    captured: Mutex<HashMap<StateKey, StateValue>>,
    /// Keys read and found absent, so a replay can trust their absence.
    absent: Mutex<BTreeSet<StateKey>>,
    failures: Mutex<Vec<String>>,
    state_view: &'s S,
}

impl<'s, S: StateView> ReadSetCapturingStateView<'s, S> {
    /// A view over `state_view` that starts out with `preloaded` values, `absent` keys known to
    /// have none, and the `failures` of building them.
    pub fn new(
        state_view: &'s S,
        preloaded: HashMap<StateKey, StateValue>,
        absent: BTreeSet<StateKey>,
        failures: Vec<String>,
    ) -> Self {
        Self {
            captured: Mutex::new(preloaded),
            absent: Mutex::new(absent),
            failures: Mutex::new(failures),
            state_view,
        }
    }

    /// The values read, and the keys read and found absent.
    pub fn into_captured(self) -> Result<(HashMap<StateKey, StateValue>, BTreeSet<StateKey>)> {
        let failures = self.failures.into_inner();
        if !failures.is_empty() {
            bail!(
                "{} read(s) failed, the dump would be incomplete; first failure: {}",
                failures.len(),
                failures[0]
            );
        }
        Ok((self.captured.into_inner(), self.absent.into_inner()))
    }
}

impl<S: StateView> TStateView for ReadSetCapturingStateView<'_, S> {
    type Key = StateKey;

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        if let Some(value) = self.captured.lock().get(state_key) {
            return Ok(StateSlot::new(
                state_key.clone(),
                StateSlotKind::ColdOccupied {
                    value_version: 0,
                    value: value.clone(),
                },
            ));
        }
        let slot = match self.state_view.get_state_slot(state_key) {
            Ok(slot) => slot,
            Err(err) => {
                self.failures
                    .lock()
                    .push(format!("read of {state_key:?} failed: {err}"));
                return Err(err);
            },
        };
        match slot.as_state_value_opt() {
            Some(value) => {
                self.captured
                    .lock()
                    .entry(state_key.clone())
                    .or_insert_with(|| value.clone());
            },
            None => {
                self.absent.lock().insert(state_key.clone());
            },
        }
        Ok(slot)
    }

    fn next_version(&self) -> Version {
        0
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}
