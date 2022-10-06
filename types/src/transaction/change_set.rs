// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::write_set::WriteOp;
use crate::{contract_event::ContractEvent, write_set::WriteSet};
use move_core_types::vm_status::{StatusCode, VMStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChangeSet {
    write_set: WriteSet,
    events: Vec<ContractEvent>,
}

impl ChangeSet {
    pub fn new(
        write_set: WriteSet,
        events: Vec<ContractEvent>,
        gas_feature_version: u64,
    ) -> Result<Self, VMStatus> {
        static MAX_ITEM_SIZE_ALLOWED: usize = 1 << 20;
        static MAX_EVENT_SIZE_ALLOWED: usize = 1 << 20;
        static MAX_TOTAL_EVENT_SIZE_ALLOWED: usize = 10 << 20;

        if gas_feature_version >= 3 {
            for (key, op) in &write_set {
                match op {
                    WriteOp::Creation(data) | WriteOp::Modification(data) => {
                        if data.len() + key.size() > MAX_ITEM_SIZE_ALLOWED {
                            return Err(VMStatus::Error(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
                        }
                    }
                    WriteOp::Deletion => (),
                }
            }

            let mut total_event_size = 0;
            for event in &events {
                let size = event.event_data().len();
                if size > MAX_EVENT_SIZE_ALLOWED {
                    return Err(VMStatus::Error(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
                }
                total_event_size += size;
                if total_event_size > MAX_TOTAL_EVENT_SIZE_ALLOWED {
                    return Err(VMStatus::Error(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
                }
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
