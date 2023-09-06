// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Breakdown of fee charge and refund for a transaction.
/// The structure is:
///
/// - Net charge or refund (not in the statement)
///   - total charge: total_charge_gas_units, matches `gas_used` in the on-chain `TransactionInfo`.
///     - gas charge for execution (CPU time): execution_gas_units
///     - gas charge for IO (storage random access): io_gas_units
///     - storage fee charge (storage space): storage_fee_octas, when included in total_charge, this number is converted to gas units according to the user specified gas unit price.
///   - storage deletion refund: storage_fee_refund_octas, this is not included in `gas_used` or `total_charge_gas_units`, the net charge / refund is calculated by total_charge_gas_units * gas_unit_price - storage_fee_refund_octas.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeeStatement {
    /// Total gas charge, not including
    total_charge_gas_units: u64,
    /// Execution gas charge.
    execution_gas_units: u64,
    /// IO gas charge.
    io_gas_units: u64,
    /// Storage fee charge.
    storage_fee_octas: u64,
    /// Storage fee refund.
    storage_fee_refund_octas: u64,
}

impl FeeStatement {
    pub fn zero() -> Self {
        Self {
            total_charge_gas_units: 0,
            execution_gas_units: 0,
            io_gas_units: 0,
            storage_fee_octas: 0,
            storage_fee_refund_octas: 0,
        }
    }

    pub fn new(
        total_charge_gas_units: u64,
        execution_gas_units: u64,
        io_gas_units: u64,
        storage_fee_octas: u64,
        storage_fee_refund_octas: u64,
    ) -> Self {
        Self {
            total_charge_gas_units,
            execution_gas_units,
            io_gas_units,
            storage_fee_octas,
            storage_fee_refund_octas,
        }
    }

    pub fn clear_refunds(&mut self) {
        self.storage_fee_refund_octas = 0;
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

    pub fn storage_fee_used(&self) -> u64 {
        self.storage_fee_octas
    }

    pub fn storage_fee_refund(&self) -> u64 {
        self.storage_fee_refund_octas
    }

    pub fn add_fee_statement(&mut self, other: &FeeStatement) {
        self.total_charge_gas_units += other.total_charge_gas_units;
        self.execution_gas_units += other.execution_gas_units;
        self.io_gas_units += other.io_gas_units;
        self.storage_fee_octas += other.storage_fee_octas;
        self.storage_fee_refund_octas += other.storage_fee_refund_octas;
    }
}
