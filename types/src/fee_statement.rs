// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FeeStatement {
    V0 {
        /// maps to `V0::gas_used`
        total_charge_gas_units: u64,
    },
    V1 {
        /// Execution gas charge.
        execution_gas_units: u64,
        /// IO gas charge.
        io_gas_units: u64,
        /// Storage gas charge.
        storage_gas_units: u64,
    },
}

impl FeeStatement {
    pub fn empty_v0() -> Self {
        FeeStatement::V0 {
            total_charge_gas_units: 0,
        }
    }

    pub fn empty_v1() -> Self {
        FeeStatement::V1 {
            execution_gas_units: 0,
            io_gas_units: 0,
            storage_gas_units: 0,
        }
    }

    pub fn new_v0(total_charge_gas_units: u64) -> Self {
        FeeStatement::V0 {
            total_charge_gas_units,
        }
    }

    pub fn new_v1(execution_gas_units: u64, io_gas_units: u64, storage_gas_units: u64) -> Self {
        FeeStatement::V1 {
            execution_gas_units,
            io_gas_units,
            storage_gas_units,
        }
    }

    pub fn gas_used(&self) -> u64 {
        match self {
            FeeStatement::V0 {
                total_charge_gas_units,
            } => *total_charge_gas_units,
            FeeStatement::V1 {
                execution_gas_units,
                io_gas_units,
                storage_gas_units,
            } => execution_gas_units + io_gas_units + storage_gas_units,
        }
    }

    pub fn execution_gas_used(&self) -> u64 {
        match self {
            FeeStatement::V0 {
                total_charge_gas_units,
            } => *total_charge_gas_units,
            FeeStatement::V1 {
                execution_gas_units,
                ..
            } => *execution_gas_units,
        }
    }

    pub fn io_gas_used(&self) -> u64 {
        match self {
            FeeStatement::V0 {
                total_charge_gas_units,
            } => *total_charge_gas_units,
            FeeStatement::V1 { io_gas_units, .. } => *io_gas_units,
        }
    }
}
