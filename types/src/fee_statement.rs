// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
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

    pub fn new_v1_from_fee_statement(fee_statement: &FeeStatement) -> Self {
        FeeStatement::V1 {
            total_charge_gas_units: fee_statement.gas_used(),
            execution_gas_units: fee_statement.execution_gas_used(),
            io_gas_units: fee_statement.io_gas_used(),
            storage_gas_units: fee_statement.storage_gas_used(),
            storage_fee_units: fee_statement.storage_fee_used(),
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

    pub fn add_fee_statement(&mut self, other: &FeeStatement) -> anyhow::Result<()> {
        match (self, other) {
            (
                FeeStatement::V0 {
                    total_charge_gas_units,
                },
                FeeStatement::V0 {
                    total_charge_gas_units: other_total_charge_gas_units,
                },
            ) => {
                *total_charge_gas_units += *other_total_charge_gas_units;
                Ok(())
            },
            (
                FeeStatement::V1 {
                    total_charge_gas_units,
                    execution_gas_units,
                    io_gas_units,
                    storage_gas_units,
                    storage_fee_units,
                },
                FeeStatement::V1 {
                    total_charge_gas_units: other_total_charge_gas_units,
                    execution_gas_units: other_execution_gas_units,
                    io_gas_units: other_io_gas_units,
                    storage_gas_units: other_storage_gas_units,
                    storage_fee_units: other_storage_fee_units,
                },
            ) => {
                *total_charge_gas_units += *other_total_charge_gas_units;
                *execution_gas_units += *other_execution_gas_units;
                *io_gas_units += *other_io_gas_units;
                *storage_gas_units += *other_storage_gas_units;
                *storage_fee_units += *other_storage_fee_units;
                Ok(())
            },
            _ => bail!("Cannot add different versions of FeeStatement"),
        }
    }

    pub fn fee_statement(&self) -> FeeStatement {
        self.clone()
    }
}
