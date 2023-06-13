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
        /// maps to `V0::gas_used`
        total_charge_gas_units: u64,
        /// Execution gas charge.
        execution_gas_units: u64,
        /// IO gas charge.
        io_gas_units: u64,
        /// Storage gas charge.
        storage_gas_units: u64,
        /// Storage fee charge.
        storage_fee_units: u64,
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
            total_charge_gas_units: 0,
            execution_gas_units: 0,
            io_gas_units: 0,
            storage_gas_units: 0,
            storage_fee_units: 0,
        }
    }

    pub fn new_v0(total_charge_gas_units: u64) -> Self {
        FeeStatement::V0 {
            total_charge_gas_units,
        }
    }

    pub fn new_v1(
        total_charge_gas_units: u64,
        execution_gas_units: u64,
        io_gas_units: u64,
        storage_gas_units: u64,
        storage_fee_units: u64,
    ) -> Self {
        FeeStatement::V1 {
            total_charge_gas_units,
            execution_gas_units,
            io_gas_units,
            storage_gas_units,
            storage_fee_units,
        }
    }

    pub fn gas_used(&self) -> u64 {
        match self {
            FeeStatement::V0 {
                total_charge_gas_units,
            } => *total_charge_gas_units,
            FeeStatement::V1 {
                total_charge_gas_units,
                ..
            } => *total_charge_gas_units,
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
            FeeStatement::V0 { .. } => 0,
            FeeStatement::V1 { io_gas_units, .. } => *io_gas_units,
        }
    }

    pub fn storage_gas_used(&self) -> u64 {
        match self {
            FeeStatement::V0 { .. } => 0,
            FeeStatement::V1 {
                storage_gas_units, ..
            } => *storage_gas_units,
        }
    }

    pub fn storage_fee_used(&self) -> u64 {
        match self {
            FeeStatement::V0 { .. } => 0,
            FeeStatement::V1 {
                storage_fee_units, ..
            } => *storage_fee_units,
        }
    }

    pub fn fee_statement(&self) -> (u64, u64, u64, u64, u64) {
        match self {
            FeeStatement::V0 {
                total_charge_gas_units,
            } => (*total_charge_gas_units, 0, 0, 0, 0),
            FeeStatement::V1 {
                total_charge_gas_units,
                execution_gas_units,
                io_gas_units,
                storage_gas_units,
                storage_fee_units,
            } => (
                *total_charge_gas_units,
                *execution_gas_units,
                *io_gas_units,
                *storage_gas_units,
                *storage_fee_units,
            ),
        }
    }
}
