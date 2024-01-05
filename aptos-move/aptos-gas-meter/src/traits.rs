// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, GasScalingFactor, Octa};
use aptos_gas_schedule::VMGasParameters;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOpSize};
use aptos_vm_types::{
    change_set::VMChangeSet,
    storage::{
        io_pricing::IoPricing,
        space_pricing::{ChargeAndRefund, DiskSpacePricing},
    },
};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::gas_algebra::{InternalGas, InternalGasUnit, NumBytes};
use move_vm_types::gas::GasMeter as MoveGasMeter;
use std::fmt::Debug;

/// An abstraction of the part of the gas meter that manages the core algebra,
/// defining how the gas charges should be handled.
pub trait GasAlgebra {
    /// Returns the gas feature version.
    fn feature_version(&self) -> u64;

    /// Returns the struct containing all (regular) gas parameters.
    fn vm_gas_params(&self) -> &VMGasParameters;

    /// Returns the struct containing the storage-specific gas parameters.
    fn io_pricing(&self) -> &IoPricing;

    /// Returns the disk space pricing strategy.
    fn disk_space_pricing(&self) -> &DiskSpacePricing;

    /// Returns the current balance, measured in internal gas units.
    fn balance_internal(&self) -> InternalGas;

    /// Checks if the internal states (counters) are consistent.
    fn check_consistency(&self) -> PartialVMResult<()>;

    /// Charges gas under the execution category.
    ///
    /// The amount charged can be a quantity or an abstract expression containing
    /// gas parameters.
    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + Debug,
    ) -> PartialVMResult<()>;

    /// Charges gas charge under the IO category.
    ///
    /// The amount charged can be a quantity or an abstract expression containing
    /// gas parameters.
    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()>;

    /// Charges storage fee.
    ///
    /// The amount charged can be a quantity or an abstract expression containing
    /// gas parameters.
    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    /// Returns the amount of gas used under the execution category.
    fn execution_gas_used(&self) -> InternalGas;

    /// Returns the amount of gas used under the IO category.
    fn io_gas_used(&self) -> InternalGas;

    /// Returns the amount of storage fee used, measured in internal gas units.
    fn storage_fee_used_in_gas_units(&self) -> InternalGas;

    /// Returns the amount of storage fee used.
    fn storage_fee_used(&self) -> Fee;
}

/// Trait representing a gas meter used inside the Aptos VM.
///
/// It extends Move VM's `GasMeter` trait with a few Aptos-specific callbacks, along with
/// some primitives that would allow gas meters to be composable.
pub trait AptosGasMeter: MoveGasMeter {
    type Algebra: GasAlgebra;

    /// Returns a reference to the underlying algebra object.
    fn algebra(&self) -> &Self::Algebra;

    /// Returns a mutable reference to the underlying algebra object.
    fn algebra_mut(&mut self) -> &mut Self::Algebra;

    /// Charges fee for utilizing short-term or long-term storage.
    ///
    /// Since the fee is measured in APT/Octa, it needs to be converted into gas units
    /// according to the given unit price.
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    /// Charges an intrinsic cost for executing the transaction.
    ///
    /// The cost stays constant for transactions below a certain size, but will grow proportionally
    /// for bigger ones.
    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()>;

    /// Charges IO gas for an item in the write set.
    ///
    /// This is to be differentiated from the storage fee, which is meant to cover the long-term
    /// storage costs.
    fn charge_io_gas_for_write(&mut self, key: &StateKey, op: &WriteOpSize) -> VMResult<()>;

    /// Charges the storage fees for writes, events & txn storage in a lump sum, minimizing the
    /// loss of precision. Refundable portion of the charge is recorded on the WriteOp itself,
    /// due to which mutable references are required on the parameter list wherever proper.
    ///
    /// The contract requires that this function behaves in a way that is consistent to
    /// the ones defining the costs.
    /// Due to this reason, you should normally not override the default implementation,
    /// unless you are doing something special, such as injecting additional logging logic.
    fn process_storage_fee_for_all(
        &mut self,
        change_set: &mut VMChangeSet,
        txn_size: NumBytes,
        gas_unit_price: FeePerGasUnit,
    ) -> VMResult<Fee> {
        // The new storage fee are only active since version 7.
        if self.feature_version() < 7 {
            return Ok(0.into());
        }

        // TODO(Gas): right now, some of our tests use a unit price of 0 and this is a hack
        // to avoid causing them issues. We should revisit the problem and figure out a
        // better way to handle this.
        if gas_unit_price.is_zero() {
            return Ok(0.into());
        }

        let pricing = self.disk_space_pricing();
        let params = &self.vm_gas_params().txn;

        // Calculate the storage fees.
        let mut write_fee = Fee::new(0);
        let mut total_refund = Fee::new(0);
        for (key, op_size, metadata_opt) in change_set.write_set_iter_mut() {
            let ChargeAndRefund { charge, refund } =
                pricing.charge_refund_write_op(params, key, &op_size, metadata_opt);
            total_refund += refund;

            write_fee += charge;
        }

        let event_fee = change_set
            .events()
            .iter()
            .fold(Fee::new(0), |acc, (event, _)| {
                acc + pricing.storage_fee_per_event(params, event)
            });
        let event_discount = pricing.storage_discount_for_events(params, event_fee);
        let event_net_fee = event_fee
            .checked_sub(event_discount)
            .expect("discount should always be less than or equal to total amount");
        let txn_fee = pricing.storage_fee_for_transaction_storage(params, txn_size);
        let fee = write_fee + event_net_fee + txn_fee;

        self.charge_storage_fee(fee, gas_unit_price)
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(total_refund)
    }

    // Below are getters reexported from the gas algebra.
    // Gas meter instances should not reimplement these themselves.

    /// Returns the gas feature version.
    fn feature_version(&self) -> u64 {
        self.algebra().feature_version()
    }

    /// Returns the struct that contains all (non-storage) gas parameters.
    fn vm_gas_params(&self) -> &VMGasParameters {
        self.algebra().vm_gas_params()
    }

    // Returns a reference to the struct containing all storage gas parameters.
    fn io_pricing(&self) -> &IoPricing {
        self.algebra().io_pricing()
    }

    /// Returns the disk space pricing strategy.
    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        self.algebra().disk_space_pricing()
    }

    /// Returns the remaining balance, measured in (external) gas units.
    ///
    /// The number should be rounded down when converting from internal to external gas units.
    fn balance(&self) -> Gas {
        self.algebra()
            .balance_internal()
            .to_unit_round_down_with_params(&self.vm_gas_params().txn)
    }

    /// Returns the scaling factor between internal and external gas units.
    fn gas_unit_scaling_factor(&self) -> GasScalingFactor {
        self.algebra().vm_gas_params().txn.scaling_factor()
    }

    /// Return the total gas used for execution.
    fn execution_gas_used(&self) -> Gas {
        self.algebra()
            .execution_gas_used()
            .to_unit_round_up_with_params(&self.vm_gas_params().txn)
    }

    /// Return the total gas used for io.
    fn io_gas_used(&self) -> Gas {
        self.algebra()
            .io_gas_used()
            .to_unit_round_up_with_params(&self.vm_gas_params().txn)
    }

    /// Return the total fee used for storage.
    fn storage_fee_used(&self) -> Fee {
        self.algebra().storage_fee_used()
    }
}
