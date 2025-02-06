// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosGasMeter, GasAlgebra};
use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, GasScalingFactor, Octa};
use aptos_gas_schedule::VMGasParameters;
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use aptos_vm_types::{
    change_set::ChangeSetInterface,
    module_and_script_storage::module_storage::AptosModuleStorage,
    resolver::ExecutorView,
    storage::{io_pricing::IoPricing, space_pricing::DiskSpacePricing},
};
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CodeOffset,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, InternalGasUnit, NumArgs, NumBytes, NumTypeNodes},
    identifier::IdentStr,
    language_storage::ModuleId,
};
use move_vm_types::{
    gas::{GasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};
use once_cell::sync::OnceCell;
use std::fmt::Debug;

/// Gas algebra used by unmetered gas meter.
pub struct UnmeteredGasAlgebra;

impl GasAlgebra for UnmeteredGasAlgebra {
    fn feature_version(&self) -> u64 {
        unreachable!("Not used when not metering gas")
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        unreachable!("Not used when not metering gas")
    }

    fn io_pricing(&self) -> &IoPricing {
        unreachable!("Not used when not metering gas")
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        unreachable!("Not used when not metering gas")
    }

    fn balance_internal(&self) -> InternalGas {
        InternalGas::zero()
    }

    fn check_consistency(&self) -> PartialVMResult<()> {
        Ok(())
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

    fn count_dependency(&mut self, _size: NumBytes) -> PartialVMResult<()> {
        Ok(())
    }

    fn execution_gas_used(&self) -> InternalGas {
        InternalGas::zero()
    }

    fn io_gas_used(&self) -> InternalGas {
        InternalGas::zero()
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        InternalGas::zero()
    }

    fn storage_fee_used(&self) -> Fee {
        Fee::zero()
    }

    fn inject_balance(&mut self, _extra_balance: impl Into<Gas>) -> PartialVMResult<()> {
        Ok(())
    }
}

/// Gas meter which does not charge gas. Unfortunately, using it is not fully representable of
/// not charging gas because there is still some work done outside.
pub struct AptosUnmeteredGasMeter {
    algebra: UnmeteredGasAlgebra,
    // For sponsored account creation hack, we check the disk space pricing to calculate the
    // expected cost of account creation. For now, just use
    disk_pricing: DiskSpacePricing,
}

impl AptosUnmeteredGasMeter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            algebra: UnmeteredGasAlgebra,
            disk_pricing: DiskSpacePricing::V2,
        }
    }
}

impl GasMeter for AptosUnmeteredGasMeter {
    fn balance_internal(&self) -> InternalGas {
        InternalGas::zero()
    }

    fn charge_simple_instr(&mut self, _instr: SimpleInstruction) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_br_true(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_br_false(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_branch(&mut self, _target_offset: CodeOffset) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_pop(&mut self, _popped_val: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        _num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_call_generic(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        _num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_ld_const(&mut self, _size: NumBytes) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_ld_const_after_deserialization(
        &mut self,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_copy_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_move_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_store_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_pack(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_pack_variant(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_unpack(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_unpack_variant(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_read_ref(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_write_ref(
        &mut self,
        _new_val: impl ValueView,
        _old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_eq(&mut self, _lhs: impl ValueView, _rhs: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_neq(&mut self, _lhs: impl ValueView, _rhs: impl ValueView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_borrow_global(
        &mut self,
        _is_mut: bool,
        _is_generic: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_exists(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        _exists: bool,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_move_from(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_move_to(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        _val: impl ValueView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_borrow(
        &mut self,
        _is_mut: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        _expect_num_elements: NumArgs,
        _elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_load_resource(
        &mut self,
        _addr: AccountAddress,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
        _bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_native_function(
        &mut self,
        _amount: InternalGas,
        _ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView> + Clone>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        _args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_create_ty(&mut self, _num_nodes: NumTypeNodes) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_dependency(
        &mut self,
        _is_new: bool,
        _addr: &AccountAddress,
        _name: &IdentStr,
        _size: NumBytes,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

impl AptosGasMeter for AptosUnmeteredGasMeter {
    type Algebra = UnmeteredGasAlgebra;

    fn algebra(&self) -> &Self::Algebra {
        &self.algebra
    }

    fn algebra_mut(&mut self) -> &mut Self::Algebra {
        &mut self.algebra
    }

    fn charge_storage_fee(
        &mut self,
        _amount: Fee,
        _gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, _txn_size: NumBytes) -> VMResult<()> {
        Ok(())
    }

    fn charge_keyless(&mut self) -> VMResult<()> {
        Ok(())
    }

    fn charge_io_gas_for_transaction(&mut self, _txn_size: NumBytes) -> VMResult<()> {
        Ok(())
    }

    fn charge_io_gas_for_event(&mut self, _event: &ContractEvent) -> VMResult<()> {
        Ok(())
    }

    fn charge_io_gas_for_write(&mut self, _key: &StateKey, _op: &WriteOpSize) -> VMResult<()> {
        Ok(())
    }

    fn process_storage_fee_for_all(
        &mut self,
        _change_set: &mut impl ChangeSetInterface,
        _txn_size: NumBytes,
        _gas_unit_price: FeePerGasUnit,
        _executor_view: &dyn ExecutorView,
        _module_storage: &impl AptosModuleStorage,
    ) -> VMResult<Fee> {
        Ok(Fee::zero())
    }

    fn feature_version(&self) -> u64 {
        unreachable!("Not used when not metering gas")
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        unreachable!("Not used when not metering gas")
    }

    fn io_pricing(&self) -> &IoPricing {
        unreachable!("Not used when not metering gas")
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        &self.disk_pricing
    }

    fn balance(&self) -> Gas {
        Gas::zero()
    }

    fn gas_unit_scaling_factor(&self) -> GasScalingFactor {
        GasScalingFactor::one()
    }

    fn execution_gas_used(&self) -> Gas {
        Gas::zero()
    }

    fn io_gas_used(&self) -> Gas {
        Gas::zero()
    }

    fn storage_fee_used(&self) -> Fee {
        Fee::zero()
    }

    fn inject_balance(&mut self, _extra_balance: impl Into<Gas>) -> VMResult<()> {
        Ok(())
    }
}

static METER_GAS: OnceCell<bool> = OnceCell::new();

/// If benchmarking, conditionally enable unmetered or metered executions.
pub fn enable_gas_metering(enable: bool) {
    if cfg!(feature = "benchmark") {
        METER_GAS.set(enable).ok();
    }
}

/// Returns whether the gas needs to be metered. When benchmarking, it is possible to configure it
/// to be turned off.
pub fn is_gas_metering_enabled() -> bool {
    if cfg!(feature = "benchmark") {
        METER_GAS.get().cloned().unwrap_or(true)
    } else {
        true
    }
}
