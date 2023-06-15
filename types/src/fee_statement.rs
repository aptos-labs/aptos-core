// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeeStatement {
    /// Total gas charge.
    total_charge_gas_units: u64,
    /// Execution gas charge.
    execution_gas_units: u64,
    /// IO gas charge.
    io_gas_units: u64,
    /// Storage gas charge.
    storage_gas_units: u64,
    /// Storage fee charge.
    storage_fee_octas: u64,
}

impl FeeStatement {
    pub fn zero() -> Self {
        Self {
            total_charge_gas_units: 0,
            execution_gas_units: 0,
            io_gas_units: 0,
            storage_gas_units: 0,
            storage_fee_octas: 0,
        }
    }

    pub fn new(
        total_charge_gas_units: u64,
        execution_gas_units: u64,
        io_gas_units: u64,
        storage_gas_units: u64,
        storage_fee_octas: u64,
    ) -> Self {
        Self {
            total_charge_gas_units,
            execution_gas_units,
            io_gas_units,
            storage_gas_units,
            storage_fee_octas,
        }
    }

    pub fn new_from_fee_statement(fee_statement: &FeeStatement) -> Self {
        Self {
            total_charge_gas_units: fee_statement.total_charge_gas_units,
            execution_gas_units: fee_statement.execution_gas_units,
            io_gas_units: fee_statement.io_gas_units,
            storage_gas_units: fee_statement.storage_gas_units,
            storage_fee_octas: fee_statement.storage_fee_octas,
        }
    }

    pub fn gas_used(&self) -> u64 {
        self.total_charge_gas_units
    }

    pub fn execution_gas_used(&self) -> u64 {
        self.execution_gas_units
    }

    pub fn io_gas_used(&self) -> u64 {
        self.io_gas_units
    }

    pub fn storage_gas_used(&self) -> u64 {
        self.storage_gas_units
    }

    pub fn storage_fee_used(&self) -> u64 {
        self.storage_fee_octas
    }

    pub fn add_fee_statement(&mut self, other: &FeeStatement) {
        self.total_charge_gas_units += other.total_charge_gas_units;
        self.execution_gas_units += other.execution_gas_units;
        self.io_gas_units += other.io_gas_units;
        self.storage_gas_units += other.storage_gas_units;
        self.storage_fee_octas += other.storage_fee_octas;
    }

    pub fn fee_statement(&self) -> FeeStatement {
        self.clone()
    }
}
