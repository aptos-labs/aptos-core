// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, write_set::WriteSet};
use move_core_types::vm_status::VMStatus;
use serde::{Deserialize, Serialize};

pub trait CheckChangeSet {
    fn check_change_set(&self, change_set: &ChangeSet) -> Result<(), VMStatus>;
}

#[cfg(any(test, feature = "fuzzing"))]
pub struct NoOpChangeSetChecker;

#[cfg(any(test, feature = "fuzzing"))]
impl CheckChangeSet for NoOpChangeSetChecker {
    fn check_change_set(&self, _change_set: &ChangeSet) -> Result<(), VMStatus> {
        Ok(())
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    write_set: WriteSet,
    events: Vec<ContractEvent>,
}

impl ChangeSet {
    pub fn new(
        write_set: WriteSet,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> Result<Self, VMStatus> {
        let myself = Self { write_set, events };
        checker.check_change_set(&myself)?;
        Ok(myself)
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
