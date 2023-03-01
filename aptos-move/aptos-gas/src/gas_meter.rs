// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains the official gas meter implementation, along with some top-level gas
//! parameters and traits to help manipulate them.

use crate::{
    algebra::{AbstractValueSize, Fee, Gas},
    instr::InstructionGasParameters,
    misc::MiscGasParameters,
    transaction::{ChangeSetConfigs, TransactionGasParameters},
    FeePerGasUnit, StorageGasParameters,
};
use aptos_logger::error;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS, contract_event::ContractEvent,
    state_store::state_key::StateKey, write_set::WriteOp,
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    gas_algebra::{InternalGas, NumArgs, NumBytes},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{GasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};
use std::collections::BTreeMap;

// Change log:
// - V7
//   - Native support for exists<T>
//   - New formulae for storage fees based on fixed APT costs
//   - Lower gas price (other than the newly introduced storage fees) by upping the scaling factor
// - V6
//   - Added a new native function - blake2b_256.
// - V5
//   - u16, u32, u256
//   - free_write_bytes_quota
//   - configurable ChangeSetConfigs
// - V4
//   - Consider memory leaked for event natives
// - V3
//   - Add memory quota
//   - Storage charges:
//     - Distinguish between new and existing resources
//     - One item write comes with 1K free bytes
//     - abort with STORATGE_WRITE_LIMIT_REACHED if WriteOps or Events are too large
// - V2
//   - Table
//     - Fix the gas formula for loading resources so that they are consistent with other
//       global operations.
// - V1
//   - TBA
pub const LATEST_GAS_FEATURE_VERSION: u64 = 7;

pub(crate) const EXECUTION_GAS_MULTIPLIER: u64 = 20;

/// A trait for converting from a map representation of the on-chain gas schedule.
pub trait FromOnChainGasSchedule: Sized {
    /// Constructs a value of this type from a map representation of the on-chain gas schedule.
    /// `None` should be returned when the gas schedule is missing some required entries.
    /// Unused entries should be safely ignored.
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Option<Self>;
}

/// A trait for converting to a list of entries of the on-chain gas schedule.
pub trait ToOnChainGasSchedule {
    /// Converts `self` into a list of entries of the on-chain gas schedule.
    /// Each entry is a key-value pair where the key is a string representing the name of the
    /// parameter, where the value is the gas parameter itself.
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)>;
}

/// A trait for defining an initial value to be used in the genesis.
pub trait InitialGasSchedule: Sized {
    /// Returns the initial value of this type, which is used in the genesis.
    fn initial() -> Self;
}

/// Gas parameters for all native functions.
#[derive(Debug, Clone)]
pub struct NativeGasParameters {
    pub move_stdlib: move_stdlib::natives::GasParameters,
    pub aptos_framework: aptos_framework::natives::GasParameters,
    pub table: move_table_extension::GasParameters,
}

impl FromOnChainGasSchedule for NativeGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Option<Self> {
        Some(Self {
            move_stdlib: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            aptos_framework: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            table: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
        })
    }
}

impl ToOnChainGasSchedule for NativeGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        let mut entries = self.move_stdlib.to_on_chain_gas_schedule(feature_version);
        entries.extend(
            self.aptos_framework
                .to_on_chain_gas_schedule(feature_version),
        );
        entries.extend(self.table.to_on_chain_gas_schedule(feature_version));
        entries
    }
}

impl NativeGasParameters {
    pub fn zeros() -> Self {
        Self {
            move_stdlib: move_stdlib::natives::GasParameters::zeros(),
            aptos_framework: aptos_framework::natives::GasParameters::zeros(),
            table: move_table_extension::GasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for NativeGasParameters {
    fn initial() -> Self {
        Self {
            move_stdlib: InitialGasSchedule::initial(),
            aptos_framework: InitialGasSchedule::initial(),
            table: InitialGasSchedule::initial(),
        }
    }
}

/// Gas parameters for everything that is needed to run the Aptos blockchain, including
/// instructions, transactions and native functions from various packages.
#[derive(Debug, Clone)]
pub struct AptosGasParameters {
    pub misc: MiscGasParameters,
    pub instr: InstructionGasParameters,
    pub txn: TransactionGasParameters,
    pub natives: NativeGasParameters,
}

impl FromOnChainGasSchedule for AptosGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Option<Self> {
        Some(Self {
            misc: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            instr: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            txn: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule, feature_version)?,
            natives: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
        })
    }
}

impl ToOnChainGasSchedule for AptosGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        let mut entries = self.instr.to_on_chain_gas_schedule(feature_version);
        entries.extend(self.txn.to_on_chain_gas_schedule(feature_version));
        entries.extend(self.natives.to_on_chain_gas_schedule(feature_version));
        entries.extend(self.misc.to_on_chain_gas_schedule(feature_version));
        entries
    }
}

impl AptosGasParameters {
    pub fn zeros() -> Self {
        Self {
            misc: MiscGasParameters::zeros(),
            instr: InstructionGasParameters::zeros(),
            txn: TransactionGasParameters::zeros(),
            natives: NativeGasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for AptosGasParameters {
    fn initial() -> Self {
        Self {
            misc: InitialGasSchedule::initial(),
            instr: InitialGasSchedule::initial(),
            txn: InitialGasSchedule::initial(),
            natives: InitialGasSchedule::initial(),
        }
    }
}

/// The official gas meter used inside the Aptos VM.
/// It maintains an internal gas counter, measured in internal gas units, and carries an environment
/// consisting all the gas parameters, which it can lookup when performing gas calculations.
pub struct AptosGasMeter {
    feature_version: u64,
    gas_params: AptosGasParameters,
    storage_gas_params: StorageGasParameters,
    balance: InternalGas,
    memory_quota: AbstractValueSize,

    execution_gas_used: InternalGas,
    io_gas_used: InternalGas,
    storage_fee_used: Fee,

    should_leak_memory_for_native: bool,
}

impl AptosGasMeter {
    pub fn new(
        gas_feature_version: u64,
        gas_params: AptosGasParameters,
        storage_gas_params: StorageGasParameters,
        balance: impl Into<Gas>,
    ) -> Self {
        let memory_quota = gas_params.txn.memory_quota;
        let balance = balance.into().to_unit_with_params(&gas_params.txn);

        Self {
            feature_version: gas_feature_version,
            gas_params,
            storage_gas_params,
            balance,
            execution_gas_used: 0.into(),
            io_gas_used: 0.into(),
            storage_fee_used: 0.into(),
            memory_quota,
            should_leak_memory_for_native: false,
        }
    }

    pub fn balance(&self) -> Gas {
        self.balance
            .to_unit_round_down_with_params(&self.gas_params.txn)
    }

    #[inline]
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

    #[inline]
    fn charge_execution(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        self.charge(amount)?;

        self.execution_gas_used += amount;
        if self.feature_version >= 7
            && self.execution_gas_used > self.gas_params.txn.max_execution_gas
        {
            Err(PartialVMError::new(StatusCode::EXECUTION_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn charge_io(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        self.charge(amount)?;

        self.io_gas_used += amount;
        if self.feature_version >= 7 && self.io_gas_used > self.gas_params.txn.max_io_gas {
            Err(PartialVMError::new(StatusCode::IO_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()> {
        if self.feature_version >= 3 {
            match self.memory_quota.checked_sub(amount) {
                Some(remaining_quota) => {
                    self.memory_quota = remaining_quota;
                    Ok(())
                },
                None => {
                    self.memory_quota = 0.into();
                    Err(PartialVMError::new(StatusCode::MEMORY_LIMIT_EXCEEDED))
                },
            }
        } else {
            Ok(())
        }
    }

    #[inline]
    fn release_heap_memory(&mut self, amount: AbstractValueSize) {
        if self.feature_version >= 3 {
            self.memory_quota += amount;
        }
    }

    pub fn feature_version(&self) -> u64 {
        self.feature_version
    }

    pub fn change_set_configs(&self) -> &ChangeSetConfigs {
        &self.storage_gas_params.change_set_configs
    }
}

impl GasMeter for AptosGasMeter {
    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    #[inline]
    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        let cost = self.gas_params.instr.simple_instr_cost(instr)?;
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // TODO(Gas): https://github.com/aptos-labs/aptos-core/issues/5485
        if self.should_leak_memory_for_native {
            return Ok(());
        }

        self.release_heap_memory(args.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self
                .gas_params
                .misc
                .abs_val
                .abstract_heap_size(val, self.feature_version)
        }));

        Ok(())
    }

    #[inline]
    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView>>,
    ) -> PartialVMResult<()> {
        if let Some(ret_vals) = ret_vals {
            self.use_heap_memory(ret_vals.fold(AbstractValueSize::zero(), |acc, val| {
                acc + self
                    .gas_params
                    .misc
                    .abs_val
                    .abstract_heap_size(val, self.feature_version)
            }))?;
        }

        self.charge_execution(amount)
    }

    #[inline]
    fn charge_load_resource(
        &mut self,
        loaded: Option<(NumBytes, impl ValueView)>,
    ) -> PartialVMResult<()> {
        if self.feature_version != 0 {
            // TODO(Gas): Rewrite this in a better way.
            if let Some((_, val)) = &loaded {
                self.use_heap_memory(
                    self.gas_params
                        .misc
                        .abs_val
                        .abstract_heap_size(val, self.feature_version),
                )?;
            }
        }
        let cost = self
            .storage_gas_params
            .pricing
            .calculate_read_gas(loaded.map(|(num_bytes, _)| num_bytes));
        self.charge_io(cost)
    }

    #[inline]
    fn charge_pop(&mut self, popped_val: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(popped_val, self.feature_version),
        );

        self.charge_execution(self.gas_params.instr.pop)
    }

    #[inline]
    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;

        let mut cost = params.call_base + params.call_per_arg * NumArgs::new(args.len() as u64);
        if self.feature_version >= 3 {
            cost += params.call_per_local * num_locals;
        }

        self.charge_execution(cost)
    }

    #[inline]
    fn charge_call_generic(
        &mut self,
        module_id: &ModuleId,
        _func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        // Save the info for charge_native_function_before_execution.
        self.should_leak_memory_for_native = (*module_id.address() == CORE_CODE_ADDRESS
            && module_id.name().as_str() == "table")
            || (self.feature_version >= 4
                && *module_id.address() == CORE_CODE_ADDRESS
                && module_id.name().as_str() == "event");

        let params = &self.gas_params.instr;

        let mut cost = params.call_generic_base
            + params.call_generic_per_ty_arg * NumArgs::new(ty_args.len() as u64)
            + params.call_generic_per_arg * NumArgs::new(args.len() as u64);
        if self.feature_version >= 3 {
            cost += params.call_generic_per_local * num_locals;
        }

        self.charge_execution(cost)
    }

    #[inline]
    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()> {
        let instr = &self.gas_params.instr;
        self.charge_execution(instr.ld_const_base + instr.ld_const_per_byte * size)
    }

    #[inline]
    fn charge_ld_const_after_deserialization(
        &mut self,
        val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(val, self.feature_version),
        )?;
        Ok(())
    }

    #[inline]
    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .gas_params
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version);

        self.use_heap_memory(heap_size)?;

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        let instr_params = &self.gas_params.instr;
        let cost = instr_params.copy_loc_base
            + instr_params.copy_loc_per_abs_val_unit * (stack_size + heap_size);

        self.charge_execution(cost)
    }

    #[inline]
    fn charge_move_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.move_loc_base)
    }

    #[inline]
    fn charge_store_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.st_loc_base)
    }

    #[inline]
    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        self.use_heap_memory(args.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self
                .gas_params
                .misc
                .abs_val
                .abstract_stack_size(val, self.feature_version)
        }))?;

        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.pack_base + params.pack_per_field * num_args,
            true => params.pack_generic_base + params.pack_generic_per_field * num_args,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        self.release_heap_memory(args.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self
                .gas_params
                .misc
                .abs_val
                .abstract_stack_size(val, self.feature_version)
        }));

        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.unpack_base + params.unpack_per_field * num_args,
            true => params.unpack_generic_base + params.unpack_generic_per_field * num_args,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .gas_params
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version);

        self.use_heap_memory(heap_size)?;

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        let instr_params = &self.gas_params.instr;
        let cost = instr_params.read_ref_base
            + instr_params.read_ref_per_abs_val_unit * (stack_size + heap_size);
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_write_ref(
        &mut self,
        _new_val: impl ValueView,
        old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(old_val, self.feature_version),
        );

        self.charge_execution(self.gas_params.instr.write_ref_base)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(&lhs, self.feature_version),
        );
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(&rhs, self.feature_version),
        );

        let instr_params = &self.gas_params.instr;
        let abs_val_params = &self.gas_params.misc.abs_val;
        let per_unit = instr_params.eq_per_abs_val_unit;

        let cost = instr_params.eq_base
            + per_unit
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version)
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version));

        self.charge_execution(cost)
    }

    #[inline]
    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(&lhs, self.feature_version),
        );
        self.release_heap_memory(
            self.gas_params
                .misc
                .abs_val
                .abstract_heap_size(&rhs, self.feature_version),
        );

        let instr_params = &self.gas_params.instr;
        let abs_val_params = &self.gas_params.misc.abs_val;
        let per_unit = instr_params.neq_per_abs_val_unit;

        let cost = instr_params.neq_base
            + per_unit
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version)
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version));

        self.charge_execution(cost)
    }

    #[inline]
    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match (is_mut, is_generic) {
            (false, false) => params.imm_borrow_global_base,
            (false, true) => params.imm_borrow_global_generic_base,
            (true, false) => params.mut_borrow_global_base,
            (true, true) => params.mut_borrow_global_generic_base,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_exists(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _exists: bool,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.exists_base,
            true => params.exists_generic_base,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_move_from(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.move_from_base,
            true => params.move_from_generic_base,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_move_to(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: impl ValueView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.move_to_base,
            true => params.move_to_generic_base,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        self.use_heap_memory(args.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self.gas_params.misc.abs_val.abstract_packed_size(val)
        }))?;

        let params = &self.gas_params.instr;
        let cost = params.vec_pack_base + params.vec_pack_per_elem * num_args;
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
        elems: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(elems.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self.gas_params.misc.abs_val.abstract_packed_size(val)
        }));

        let params = &self.gas_params.instr;
        let cost =
            params.vec_unpack_base + params.vec_unpack_per_expected_elem * expect_num_elements;
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.vec_len_base)
    }

    #[inline]
    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_mut {
            false => params.vec_imm_borrow_base,
            true => params.vec_mut_borrow_base,
        };
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(self.gas_params.misc.abs_val.abstract_packed_size(val))?;

        self.charge_execution(self.gas_params.instr.vec_push_back_base)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        if let Some(val) = val {
            self.release_heap_memory(self.gas_params.misc.abs_val.abstract_packed_size(val));
        }

        self.charge_execution(self.gas_params.instr.vec_pop_back_base)
    }

    #[inline]
    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.vec_swap_base)
    }

    #[inline]
    fn charge_drop_frame(
        &mut self,
        locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(locals.fold(AbstractValueSize::zero(), |acc, val| {
            acc + self
                .gas_params
                .misc
                .abs_val
                .abstract_heap_size(val, self.feature_version)
        }));

        Ok(())
    }
}

impl AptosGasMeter {
    pub fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let cost = self.gas_params.txn.calculate_intrinsic_gas(txn_size);
        self.charge_execution(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub fn charge_write_set_gas_for_io<'a>(
        &mut self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> VMResult<()> {
        let cost = self.storage_gas_params.pricing.calculate_write_set_gas(ops);
        self.charge_io(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub fn charge_storage_fee<'a>(
        &mut self,
        write_ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
        events: impl IntoIterator<Item = &'a ContractEvent>,
        txn_size: NumBytes,
        gas_unit_price: FeePerGasUnit,
    ) -> VMResult<()> {
        // The new storage fee are only active since version 7.
        if self.feature_version < 7 {
            return Ok(());
        }

        // TODO(Gas): right now, some of our tests use a unit price of 0 and this is a hack
        // to avoid causing them issues. We should revisit the problem and figure out a
        // better way to handle this.
        if gas_unit_price.is_zero() {
            return Ok(());
        }

        // Calculate the storage fees.
        let txn_params = &self.gas_params.txn;
        let write_fee = txn_params.calculate_write_set_storage_fee(write_ops);
        let event_fee = txn_params.calculate_event_storage_fee(events);
        let txn_fee = txn_params.calculate_transaction_storage_fee(txn_size);
        let fee = write_fee + event_fee + txn_fee;

        // Because the storage fees are defined in terms of fixed APT costs, we need
        // to convert them into gas units.
        //
        // u128 is used to protect against overflow and preverse as much precision as
        // possible in the extreme cases.
        fn div_ceil(n: u128, d: u128) -> u128 {
            if n % d == 0 {
                n / d
            } else {
                n / d + 1
            }
        }
        let gas_consumed_internal = div_ceil(
            (u64::from(fee) as u128) * (u64::from(txn_params.gas_unit_scaling_factor) as u128),
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

        self.charge(gas_consumed_internal)
            .map_err(|e| e.finish(Location::Undefined))?;

        self.storage_fee_used += fee;
        if self.feature_version >= 7 && self.storage_fee_used > self.gas_params.txn.max_storage_fee
        {
            return Err(
                PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED).finish(Location::Undefined)
            );
        }

        Ok(())
    }
}
