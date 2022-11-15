// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::write_set::WriteOp;
use crate::{contract_event::ContractEvent, write_set::WriteSet};
use move_core_types::vm_status::{StatusCode, VMStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct ChangeSetLimits {
    pub max_bytes_per_write_op: u64,
    pub max_bytes_all_write_ops_per_transaction: u64,
    pub max_bytes_per_event: u64,
    pub max_bytes_all_events_per_transaction: u64,
    pub creation_as_modify: bool,
}

impl ChangeSetLimits {
    pub fn pre_gas_feature_version_3() -> Self {
        Self {
            max_bytes_per_write_op: u64::MAX,
            max_bytes_all_write_ops_per_transaction: u64::MAX,
            max_bytes_per_event: u64::MAX,
            max_bytes_all_events_per_transaction: u64::MAX,
            creation_as_modify: true,
        }
    }

    // For gas feature version 3 and 4.
    // Version 5 makes it configurable on chain and is supposed to be equal to this or stricter
    pub fn loose() -> ChangeSetLimits {
        const MB: u64 = 1 << 20;

        ChangeSetLimits {
            max_bytes_per_write_op: MB,
            max_bytes_all_write_ops_per_transaction: u64::MAX,
            max_bytes_per_event: MB,
            max_bytes_all_events_per_transaction: MB << 10,
            creation_as_modify: false,
        }
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
        limits: &ChangeSetLimits,
    ) -> Result<Self, VMStatus> {
        const ERR: StatusCode = StatusCode::STORAGE_WRITE_LIMIT_REACHED;

        let mut write_set_size = 0;
        for (key, op) in &write_set {
            match op {
                WriteOp::Creation(data) | WriteOp::Modification(data) => {
                    let write_op_size = (data.len() + key.size()) as u64;
                    if write_op_size > limits.max_bytes_per_write_op {
                        return Err(VMStatus::Error(ERR));
                    }
                    write_set_size += write_op_size;
                }
                WriteOp::Deletion => (),
            }
            if write_set_size > limits.max_bytes_all_write_ops_per_transaction {
                return Err(VMStatus::Error(ERR));
            }
        }

        let mut total_event_size = 0;
        for event in &events {
            let size = event.event_data().len() as u64;
            if size > limits.max_bytes_per_event {
                return Err(VMStatus::Error(ERR));
            }
            total_event_size += size;
            if total_event_size > limits.max_bytes_all_events_per_transaction {
                return Err(VMStatus::Error(ERR));
            }
        }

        Ok(Self { write_set, events })
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
