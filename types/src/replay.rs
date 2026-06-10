// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared data types for the replay / benchmark tooling: a captured read-set
//! and a block of transactions. They live here (rather than in
//! `aptos-replay-benchmark`) so lightweight consumers can decode the files the
//! benchmark produces without pulling in the benchmark's node-stack
//! dependencies (the debugger, consensus, storage, ...).

use crate::{
    state_store::{
        state_key::StateKey,
        state_slot::{StateSlot, StateSlotKind},
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateViewResult, TStateView,
    },
    transaction::{PersistedAuxiliaryInfo, Transaction, Version},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The read-set captured when executing a block of transactions.
#[derive(Serialize, Deserialize)]
pub struct ReadSet {
    data: HashMap<StateKey, StateValue>,
}

impl ReadSet {
    /// Builds a read-set from captured state values.
    pub fn new(data: HashMap<StateKey, StateValue>) -> Self {
        Self { data }
    }

    /// Consumes the read-set, returning the captured state values.
    pub fn into_data(self) -> HashMap<StateKey, StateValue> {
        self.data
    }
}

impl TStateView for ReadSet {
    type Key = StateKey;

    fn next_version(&self) -> Version {
        0
    }

    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        let slot = match self.data.get(state_key) {
            Some(state_value) => StateSlot::new(state_key.clone(), StateSlotKind::ColdOccupied {
                value_version: 0,
                value: state_value.clone(),
            }),
            None => StateSlot::new(state_key.clone(), StateSlotKind::ColdVacant),
        };
        Ok(slot)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        unreachable!("Should not be called when benchmarking")
    }
}

/// On-disk representation of a block of transactions and their persisted
/// auxiliary info, saved to the local filesystem by the replay tooling.
#[derive(Serialize, Deserialize)]
pub struct TransactionBlock {
    /// The version of the first transaction in the block.
    pub begin_version: Version,
    /// Non-empty list of transactions in a block.
    pub transactions: Vec<Transaction>,
    /// Persisted auxiliary info for each transaction, aligned with `transactions`.
    #[serde(default = "Vec::new")]
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}
