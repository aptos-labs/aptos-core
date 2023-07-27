// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::GasAlgebra;
use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, Octa};
use aptos_gas_schedule::VMGasParameters;
use aptos_logger::error;
use aptos_vm_types::storage::StorageGasParameters;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    gas_algebra::{InternalGas, InternalGasUnit},
    vm_status::StatusCode,
};
use std::fmt::Debug;

/// Base gas algebra implementation that tracks the gas usage using its internal counters.
///
/// Abstract gas amounts are always evaluated to concrete values at the spot.
pub struct StandardGasAlgebra {
    feature_version: u64,
    vm_gas_params: VMGasParameters,
    storage_gas_params: StorageGasParameters,

    balance: InternalGas,

    execution_gas_used: InternalGas,
    io_gas_used: InternalGas,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,
}

impl StandardGasAlgebra {
    pub fn new(
        gas_feature_version: u64,
        vm_gas_params: VMGasParameters,
        storage_gas_params: StorageGasParameters,
        balance: impl Into<Gas>,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&vm_gas_params.txn);

        Self {
            feature_version: gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            balance,
            execution_gas_used: 0.into(),
            io_gas_used: 0.into(),
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
        }
    }
}

impl StandardGasAlgebra {
    fn charge(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        match self.balance.checked_sub(amount) {
            Some(new_balance) => {
                self.balance = new_balance;
                Ok(())
            },
            None => {
                self.balance = 0.into();
                Err(PartialVMError::new(StatusCode::OUT_OF_GAS))
            },
        }
    }
}

impl GasAlgebra for StandardGasAlgebra {
    fn feature_version(&self) -> u64 {
        self.feature_version
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        &self.vm_gas_params
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        &self.storage_gas_params
    }

    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + Debug,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        self.charge(amount)?;

        self.execution_gas_used += amount;
        if self.feature_version >= 7
            && self.execution_gas_used > self.vm_gas_params.txn.max_execution_gas
        {
            Err(PartialVMError::new(StatusCode::EXECUTION_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        self.charge(amount)?;

        self.io_gas_used += amount;
        if self.feature_version >= 7 && self.io_gas_used > self.vm_gas_params.txn.max_io_gas {
            Err(PartialVMError::new(StatusCode::IO_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        let txn_params = &self.vm_gas_params.txn;

        // Because the storage fees are defined in terms of fixed APT costs, we need
        // to convert them into gas units.
        //
        // u128 is used to protect against overflow and preserve as much precision as
        // possible in the extreme cases.
        fn div_ceil(n: u128, d: u128) -> u128 {
            if n % d == 0 {
                n / d
            } else {
                n / d + 1
            }
        }
        let gas_consumed_internal = div_ceil(
            (u64::from(amount) as u128) * (u64::from(txn_params.gas_unit_scaling_factor) as u128),
            u64::from(gas_unit_price) as u128,
        );
        let gas_consumed_internal = InternalGas::new(
            if gas_consumed_internal > u64::MAX as u128 {
                error!(
                    "Something's wrong in the gas schedule: gas_consumed_internal ({}) > u64::MAX",
                    gas_consumed_internal
                );
                u64::MAX
            } else {
                gas_consumed_internal as u64
            },
        );

        self.charge(gas_consumed_internal)?;

        self.storage_fee_in_internal_units += gas_consumed_internal;
        self.storage_fee_used += amount;
        if self.feature_version >= 7
            && self.storage_fee_used > self.vm_gas_params.txn.max_storage_fee
        {
            return Err(PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED));
        }

        Ok(())
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.execution_gas_used
    }

    fn io_gas_used(&self) -> InternalGas {
        self.io_gas_used
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.storage_fee_in_internal_units
    }

    fn storage_fee_used(&self) -> Fee {
        self.storage_fee_used
    }
}
