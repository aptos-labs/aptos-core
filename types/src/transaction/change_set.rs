// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, write_set::WriteSet};
use anyhow::Result;
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

    pub fn squash(self, other: Self) -> Result<Self> {
        let write_set = self
            .write_set
            .into_mut()
            .squash(other.write_set.into_mut())?
            .freeze()?;

        let mut events = self.events;
        events.extend(other.events);

        Ok(Self { write_set, events })
    }
}
