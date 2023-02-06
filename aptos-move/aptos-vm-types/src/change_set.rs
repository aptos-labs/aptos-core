// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_cache::OutputData;
use aptos_types::{contract_event::ContractEvent, state_store::state_key::StateKey};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct ChangeSet {
    delta_change_set: DeltaChangeSet,
    write_change_set: WriteChangeSet,
    events: Vec<ContractEvent>,
}

#[derive(Debug)]
pub struct DeltaChangeSet {
    inner: BTreeMap<StateKey, DeltaChange>,
}

#[derive(Debug)]
pub struct WriteChangeSet {
    inner: BTreeMap<StateKey, WriteChange>,
}
impl WriteChangeSet {
    pub fn get(&self, key: &StateKey) -> Option<&WriteChange> {
        self.get(key)
    }
}

#[derive(Debug)]
pub enum WriteChange {
    Creation(OutputData),
    Modification(OutputData),
    Deletion,
}

#[derive(Debug)]
pub enum DeltaChange {
    // TODO: Move delta op here?
}
