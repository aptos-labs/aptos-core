// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, delta_set::DeltaSet, write_set::WriteSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    write_set: WriteSet,
    events: Vec<ContractEvent>,
}

impl ChangeSet {
    pub fn new(write_set: WriteSet, events: Vec<ContractEvent>) -> Self {
        Self { write_set, events }
    }

    pub fn into_inner(self) -> (WriteSet, Vec<ContractEvent>) {
        (self.write_set, self.events)
    }

    pub fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }
}

/// Extension of `ChangeSet` that also holds deltas.
pub struct ChangeSetExt {
    delta_set: DeltaSet,
    change_set: ChangeSet,
}

impl ChangeSetExt {
    pub fn new(delta_set: DeltaSet, change_set: ChangeSet) -> Self {
        ChangeSetExt {
            delta_set,
            change_set,
        }
    }

    pub fn into_inner(self) -> (DeltaSet, ChangeSet) {
        (self.delta_set, self.change_set)
    }
}
