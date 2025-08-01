// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{
    DynamicExpression, Fee, FeePerGasUnit, Gas, GasExpression, InternalGas, InternalGasUnit,
    NumBytes, Octa,
};
use aptos_gas_meter::GasAlgebra;
use aptos_gas_schedule::VMGasParameters;
use aptos_vm_types::storage::{
    io_pricing::IoPricing, space_pricing::DiskSpacePricing, StorageGasParameters,
};
use move_binary_format::errors::PartialVMResult;
use std::sync::{Arc, Mutex};

/// Algebra to record abstract gas usage
pub struct CalibrationAlgebra<A> {
    // GasAlgebra that is used to delegate work
    pub base: A,
    // Mapping of simplified like-terms
    // pub coeff_buffer: BTreeMap<String, u64>,
    pub shared_buffer: Arc<Mutex<Vec<DynamicExpression>>>,
}

/// Algebra implementation
impl<A: GasAlgebra> GasAlgebra for CalibrationAlgebra<A> {
    fn feature_version(&self) -> u64 {
        self.base.feature_version()
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        self.base.vm_gas_params()
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        self.base.storage_gas_params()
    }

    fn io_pricing(&self) -> &IoPricing {
        self.base.io_pricing()
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        self.base.disk_space_pricing()
    }

    fn balance_internal(&self) -> InternalGas {
        self.base.balance_internal()
    }

    fn check_consistency(&self) -> PartialVMResult<()> {
        self.base.check_consistency()
    }

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let node = abstract_amount.to_dynamic();
        self.shared_buffer.lock().unwrap().push(node);

        let amount =
            abstract_amount.evaluate(self.base.feature_version(), self.base.vm_gas_params());
        self.base.charge_execution(amount)?;
        Ok(())
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let node = abstract_amount.to_dynamic();
        self.shared_buffer.lock().unwrap().push(node);

        let amount =
            abstract_amount.evaluate(self.base.feature_version(), self.base.vm_gas_params());
        self.base.charge_execution(amount)?;
        Ok(())
    }

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        self.base
            .charge_storage_fee(abstract_amount, gas_unit_price)
    }

    fn count_dependency(&mut self, size: NumBytes) -> PartialVMResult<()> {
        self.base.count_dependency(size)
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.base.execution_gas_used()
    }

    fn io_gas_used(&self) -> InternalGas {
        self.base.io_gas_used()
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.base.storage_fee_used_in_gas_units()
    }

    fn storage_fee_used(&self) -> Fee {
        self.base.storage_fee_used()
    }

    fn inject_balance(&mut self, new_initial_gas: impl Into<Gas>) -> PartialVMResult<()> {
        self.base.inject_balance(new_initial_gas)
    }
}
