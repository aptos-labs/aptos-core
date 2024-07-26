// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::GasAlgebra;
use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, NumBytes, NumModules, Octa};
use aptos_gas_schedule::{gas_feature_versions, VMGasParameters};
use aptos_logger::error;
use aptos_vm_types::storage::{
    io_pricing::IoPricing, space_pricing::DiskSpacePricing, StorageGasParameters,
};
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

    initial_balance: InternalGas,
    balance: InternalGas,

    max_execution_gas: InternalGas,
    execution_gas_used: InternalGas,

    max_io_gas: InternalGas,
    io_gas_used: InternalGas,

    max_storage_fee: Fee,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,

    num_dependencies: NumModules,
    total_dependency_size: NumBytes,
}

impl StandardGasAlgebra {
    pub fn new(
        gas_feature_version: u64,
        vm_gas_params: VMGasParameters,
        storage_gas_params: StorageGasParameters,
        is_approved_gov_script: bool,
        balance: impl Into<Gas>,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&vm_gas_params.txn);

        let (max_execution_gas, max_io_gas, max_storage_fee) = if is_approved_gov_script
            && gas_feature_version >= gas_feature_versions::RELEASE_V1_13
        {
            (
                vm_gas_params.txn.max_execution_gas_gov,
                vm_gas_params.txn.max_io_gas_gov,
                vm_gas_params.txn.max_storage_fee_gov,
            )
        } else {
            (
                vm_gas_params.txn.max_execution_gas,
                vm_gas_params.txn.max_io_gas,
                vm_gas_params.txn.max_storage_fee,
            )
        };

        Self {
            feature_version: gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            initial_balance: balance,
            balance,
            max_execution_gas,
            execution_gas_used: 0.into(),
            max_io_gas,
            io_gas_used: 0.into(),
            max_storage_fee,
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
            num_dependencies: 0.into(),
            total_dependency_size: 0.into(),
        }
    }
}

impl StandardGasAlgebra {
    fn charge(&mut self, amount: InternalGas) -> (InternalGas, PartialVMResult<()>) {
        match self.balance.checked_sub(amount) {
            Some(new_balance) => {
                self.balance = new_balance;
                (amount, Ok(()))
            },
            None => {
                let old_balance = self.balance;
                self.balance = 0.into();
                (
                    old_balance,
                    Err(PartialVMError::new(StatusCode::OUT_OF_GAS)),
                )
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

    fn io_pricing(&self) -> &IoPricing {
        &self.storage_gas_params.io_pricing
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        &self.storage_gas_params.space_pricing
    }

    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    fn check_consistency(&self) -> PartialVMResult<()> {
        let total = self
            .initial_balance
            .checked_sub(self.balance)
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "Current balance ({}) exceedes the initial balance ({}) -- how is this ever possible?",
                        self.balance,
                        self.initial_balance
                    ),
                )
            })?;

        let total_calculated =
            self.execution_gas_used + self.io_gas_used + self.storage_fee_in_internal_units;
        if total != total_calculated {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "The per-category costs do not add up. {} (total) != {} = {} (exec) + {} (io) + {} (storage)",
                        total,
                        total_calculated,
                        self.execution_gas_used,
                        self.io_gas_used,
                        self.storage_fee_in_internal_units,
                    ),
                ),
            );
        }

        Ok(())
    }

    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + Debug,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        let (actual, res) = self.charge(amount);
        if self.feature_version >= 12 {
            self.execution_gas_used += actual;
        }
        res?;

        if self.feature_version < 12 {
            self.execution_gas_used += amount;
        }
        if self.feature_version >= 7 && self.execution_gas_used > self.max_execution_gas {
            println!(
                "self.execution_gas_used:{}, self.max_execution_gas:{}",
                self.execution_gas_used, self.max_execution_gas
            );
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

        let (actual, res) = self.charge(amount);
        if self.feature_version >= 12 {
            self.io_gas_used += actual;
        }
        res?;

        if self.feature_version < 12 {
            self.io_gas_used += amount;
        }
        if self.feature_version >= 7 && self.io_gas_used > self.max_io_gas {
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

        let (actual, res) = self.charge(gas_consumed_internal);
        if self.feature_version >= 12 {
            self.storage_fee_in_internal_units += actual;
            self.storage_fee_used += amount;
        }
        res?;

        if self.feature_version < 12 {
            self.storage_fee_in_internal_units += gas_consumed_internal;
            self.storage_fee_used += amount;
        }
        if self.feature_version >= 7 && self.storage_fee_used > self.max_storage_fee {
            return Err(PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED));
        }

        Ok(())
    }

    fn count_dependency(&mut self, size: NumBytes) -> PartialVMResult<()> {
        if self.feature_version >= 15 {
            self.num_dependencies += 1.into();
            self.total_dependency_size += size;

            if self.num_dependencies > self.vm_gas_params.txn.max_num_dependencies {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
            if self.total_dependency_size > self.vm_gas_params.txn.max_total_dependency_size {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
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
