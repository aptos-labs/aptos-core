// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The gas algebra V1 runs with. The schedule itself is zeroed by
//! [`mono_move_replay_common::gas::make_gas_free`].

use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, Octa};
use aptos_gas_meter::GasAlgebra;
use aptos_gas_schedule::VMGasParameters;
use aptos_vm_types::storage::{
    io_pricing::IoPricing, space_pricing::DiskSpacePricing, StorageGasParameters,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasUnit, NumBytes};
use std::fmt::Debug;

/// A gas algebra that charges nothing and otherwise defers to `A`: dependency limits still apply,
/// and the meter around it still tracks memory. Zeroing the schedule cannot reach charges
/// hard-coded outside it (value traversal on resource loads and in natives), so V1 runs with this
/// to match MonoMove's unmetered execution.
pub(crate) struct ZeroChargeAlgebra<A>(pub A);

impl<A: GasAlgebra> GasAlgebra for ZeroChargeAlgebra<A> {
    fn feature_version(&self) -> u64 {
        self.0.feature_version()
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        self.0.vm_gas_params()
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        self.0.storage_gas_params()
    }

    fn io_pricing(&self) -> &IoPricing {
        self.0.io_pricing()
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        self.0.disk_space_pricing()
    }

    fn balance_internal(&self) -> InternalGas {
        self.0.balance_internal()
    }

    fn check_consistency(&self) -> PartialVMResult<()> {
        self.0.check_consistency()
    }

    fn charge_execution(
        &mut self,
        _abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + Debug,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_io(
        &mut self,
        _abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_storage_fee(
        &mut self,
        _abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        _gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn count_dependency(&mut self, size: NumBytes) -> PartialVMResult<()> {
        self.0.count_dependency(size)
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.0.execution_gas_used()
    }

    fn io_gas_used(&self) -> InternalGas {
        self.0.io_gas_used()
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.0.storage_fee_used_in_gas_units()
    }

    fn storage_fee_used(&self) -> Fee {
        self.0.storage_fee_used()
    }

    fn inject_balance(&mut self, extra_balance: impl Into<Gas>) -> PartialVMResult<()> {
        self.0.inject_balance(extra_balance)
    }
}
