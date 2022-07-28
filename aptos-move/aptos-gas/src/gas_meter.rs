// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{instr::InstructionGasParameters, transaction::TransactionGasParameters};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format_common::Opcodes,
};
use move_core_types::{
    gas_schedule::{AbstractMemorySize, GasAlgebra, GasCarrier},
    vm_status::StatusCode,
};
use move_vm_types::gas::GasMeter;
use std::collections::BTreeMap;

pub trait FromOnChainGasSchedule: Sized {
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self>;
}

pub trait ToOnChainGasSchedule {
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)>;
}

pub trait InitialGasSchedule: Sized {
    fn initial() -> Self;
}

#[derive(Debug, Clone)]
pub struct NativeGasParameters {
    pub move_stdlib: move_stdlib::natives::GasParameters,
    pub aptos_framework: framework::natives::GasParameters,
}

impl FromOnChainGasSchedule for NativeGasParameters {
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
        Some(Self {
            move_stdlib: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
            aptos_framework: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
        })
    }
}

impl ToOnChainGasSchedule for NativeGasParameters {
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
        let mut entries = self.move_stdlib.to_on_chain_gas_schedule();
        entries.extend(self.aptos_framework.to_on_chain_gas_schedule());
        entries
    }
}

impl NativeGasParameters {
    pub fn zeros() -> Self {
        Self {
            move_stdlib: move_stdlib::natives::GasParameters::zeros(),
            aptos_framework: framework::natives::GasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for NativeGasParameters {
    fn initial() -> Self {
        Self {
            move_stdlib: InitialGasSchedule::initial(),
            aptos_framework: InitialGasSchedule::initial(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AptosGasParameters {
    pub instr: InstructionGasParameters,
    pub txn: TransactionGasParameters,
    pub natives: NativeGasParameters,
}

impl FromOnChainGasSchedule for AptosGasParameters {
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
        Some(Self {
            instr: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
            txn: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
            natives: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
        })
    }
}

impl ToOnChainGasSchedule for AptosGasParameters {
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
        let mut entries = self.instr.to_on_chain_gas_schedule();
        entries.extend(self.txn.to_on_chain_gas_schedule());
        entries.extend(self.natives.to_on_chain_gas_schedule());
        entries
    }
}

impl AptosGasParameters {
    pub fn zeros() -> Self {
        Self {
            instr: InstructionGasParameters::zeros(),
            txn: TransactionGasParameters::zeros(),
            natives: NativeGasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for AptosGasParameters {
    fn initial() -> Self {
        Self {
            instr: InitialGasSchedule::initial(),
            txn: InitialGasSchedule::initial(),
            natives: InitialGasSchedule::initial(),
        }
    }
}

pub struct AptosGasMeter {
    gas_params: AptosGasParameters,
    balance: u64,
}

impl AptosGasMeter {
    pub fn new(gas_params: AptosGasParameters, balance: u64) -> Self {
        Self {
            gas_params,
            balance,
        }
    }

    pub fn balance(&self) -> u64 {
        self.balance
    }
}

impl GasMeter for AptosGasMeter {
    fn charge_instr(&mut self, opcode: Opcodes) -> PartialVMResult<()> {
        let cost = self.gas_params.instr.instr_cost(opcode)?;
        self.charge_in_native_unit(cost)
    }

    fn charge_instr_with_size(
        &mut self,
        opcode: Opcodes,
        size: AbstractMemorySize<GasCarrier>,
    ) -> PartialVMResult<()> {
        let cost = self
            .gas_params
            .instr
            .instr_cost_with_size(opcode, size.get())?;
        self.charge_in_native_unit(cost)
    }

    fn charge_in_native_unit(&mut self, amount: u64) -> PartialVMResult<()> {
        if amount > self.balance {
            self.balance = 0;
            Err(PartialVMError::new(StatusCode::OUT_OF_GAS))
        } else {
            self.balance -= amount;
            Ok(())
        }
    }
}

impl AptosGasMeter {
    pub fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: u64) -> VMResult<()> {
        let cost = self.gas_params.txn.calculate_intrinsic_gas(txn_size);
        self.charge_in_native_unit(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }
}
