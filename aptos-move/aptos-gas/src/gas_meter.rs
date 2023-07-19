// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains the official gas meter implementation, along with some top-level gas
//! parameters and traits to help manipulate them.

use crate::{
    algebra::{Fee, Gas},
    instr::InstructionGasParameters,
    misc::MiscGasParameters,
    transaction::TransactionGasParameters,
    FeePerGasUnit, GasScalingFactor, StorageGasParameters,
};
use aptos_logger::error;
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::CodeOffset,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, NumArgs, NumBytes},
    language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{GasMeter as MoveGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};
use std::collections::BTreeMap;

// Change log:
// - V10
//   - Added generate_unique_address and get_txn_hash native functions
//   - Storage gas charges (excluding "storage fees") stop respecting the storage gas curves
// - V9
//   - Accurate tracking of the cost of loading resource groups
// - V8
//   - Added BLS12-381 operations.
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
//     - abort with STORAGE_WRITE_LIMIT_REACHED if WriteOps or Events are too large
// - V2
//   - Table
//     - Fix the gas formula for loading resources so that they are consistent with other
//       global operations.
// - V1
//   - TBA
pub const LATEST_GAS_FEATURE_VERSION: u64 = 10;

pub(crate) const EXECUTION_GAS_MULTIPLIER: u64 = 20;

/// A trait for converting from a map representation of the on-chain gas schedule.
pub trait FromOnChainGasSchedule: Sized {
    /// Constructs a value of this type from a map representation of the on-chain gas schedule.
    /// `None` should be returned when the gas schedule is missing some required entries.
    /// Unused entries should be safely ignored.
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String>;
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
    pub move_stdlib: aptos_move_stdlib::natives::GasParameters,
    pub aptos_framework: aptos_framework::natives::GasParameters,
    pub table: move_table_extension::GasParameters,
}

impl FromOnChainGasSchedule for NativeGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String> {
        Ok(Self {
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
            move_stdlib: aptos_move_stdlib::natives::GasParameters::zeros(),
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
    ) -> Result<Self, String> {
        Ok(Self {
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

/// Trait representing a gas meter used inside the Aptos VM.
///
/// It extends Move VM's `GasMeter` trait with a few Aptos-specific callbacks, along with
/// some primitives that would allow gas meters to be composable.
pub trait AptosGasMeter: MoveGasMeter {
    /// Returns the gas feature version.
    fn feature_version(&self) -> u64;

    /// Returns the struct that contains all (non-storage) gas parameters.
    fn gas_params(&self) -> &AptosGasParameters;

    /// Returns the remaining balance, measured in (external) gas units.
    ///
    /// The number should be rounded down when converting from internal to external gas units.
    fn balance(&self) -> Gas;

    /// Returns the scaling factor between internal and external gas units.
    fn gas_unit_scaling_factor(&self) -> GasScalingFactor;

    /// Charges gas for performing operations that fall under the execution category.
    fn charge_execution(&mut self, amount: InternalGas) -> PartialVMResult<()>;

    /// Charges gas for performing operations that fall under the IO category.
    fn charge_io(&mut self, amount: InternalGas) -> PartialVMResult<()>;

    /// Charges fee for utlizing short-term or long-term storage.
    ///
    /// Since the fee is measured in APT/Octa, it needs to be converted into gas units
    /// according to the given unit price.
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()>;

    /// Charge an intrinsic cost for executing the transaction.
    ///
    /// The cost stays constant for transactions below a certain size, but will grow proportionally
    /// for bigger ones.
    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()>;

    /// Calculates the IO gas cost required to perform a write operation.
    fn io_gas_per_write(&self, key: &StateKey, op: &WriteOp) -> InternalGas;

    /// Charges IO gas for all items in the specified write set.
    ///
    /// The contract requires that this function behaviors in a way that is consistent to
    /// `fn io_gas_per_write`.
    /// Due to this reason, you should normally not override the default implementation,
    /// unless you are doing something special, such as injecting additional logging logic.
    fn charge_io_gas_for_write_set<'a>(
        &mut self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> VMResult<()> {
        for (key, op) in ops {
            self.charge_io(self.io_gas_per_write(key, op))
                .map_err(|e| e.finish(Location::Undefined))?;
        }
        Ok(())
    }

    /// Calculates the storage fee for a write operation.
    fn storage_fee_per_write(&self, key: &StateKey, op: &WriteOp) -> Fee;

    /// Calculates the storage fee for an event.
    fn storage_fee_per_event(&self, event: &ContractEvent) -> Fee;

    /// Calculates the discount applied to the event storage fees, based on a free quota.
    fn storage_discount_for_events(&self, total_cost: Fee) -> Fee;

    /// Calculates the storage fee for the transaction.
    fn storage_fee_for_transaction_storage(&self, txn_size: NumBytes) -> Fee;

    /// Charges the storage fees for writes, events & txn storage in a lump sum, minimizing the
    /// loss of precision.
    ///
    /// The contract requires that this function behaviors in a way that is consistent to
    /// the ones defining the costs.
    /// Due to this reason, you should normally not override the default implementation,
    /// unless you are doing something special, such as injecting additional logging logic.
    fn charge_storage_fee_for_all<'a>(
        &mut self,
        write_ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
        events: impl IntoIterator<Item = &'a ContractEvent>,
        txn_size: NumBytes,
        gas_unit_price: FeePerGasUnit,
    ) -> VMResult<()> {
        // The new storage fee are only active since version 7.
        if self.feature_version() < 7 {
            return Ok(());
        }

        // TODO(Gas): right now, some of our tests use a unit price of 0 and this is a hack
        // to avoid causing them issues. We should revisit the problem and figure out a
        // better way to handle this.
        if gas_unit_price.is_zero() {
            return Ok(());
        }

        // Calculate the storage fees.
        let write_fee = write_ops.into_iter().fold(Fee::new(0), |acc, (key, op)| {
            acc + self.storage_fee_per_write(key, op)
        });
        let event_fee = events.into_iter().fold(Fee::new(0), |acc, event| {
            acc + self.storage_fee_per_event(event)
        });
        let event_discount = self.storage_discount_for_events(event_fee);
        let event_net_fee = event_fee
            .checked_sub(event_discount)
            .expect("discount should always be less than or equal to total amount");
        let txn_fee = self.storage_fee_for_transaction_storage(txn_size);
        let fee = write_fee + event_net_fee + txn_fee;

        self.charge_storage_fee(fee, gas_unit_price)
            .map_err(|err| err.finish(Location::Undefined))?;

        Ok(())
    }

    /// Return the total gas used for execution.
    fn execution_gas_used(&self) -> Gas;

    /// Return the total gas used for io.
    fn io_gas_used(&self) -> Gas;

    /// Return the total gas used for storage.
    fn storage_fee_used_in_gas_units(&self) -> Gas;

    /// Return the total fee used for storage.
    fn storage_fee_used(&self) -> Fee;
}

/// The official gas meter used inside the Aptos VM.
/// It maintains an internal gas counter, measured in internal gas units, and carries an environment
/// consisting all the gas parameters, which it can lookup when performing gas calculations.
pub struct StandardGasMeter {
    feature_version: u64,
    gas_params: AptosGasParameters,
    storage_gas_params: StorageGasParameters,
    balance: InternalGas,

    execution_gas_used: InternalGas,
    io_gas_used: InternalGas,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,
}

impl StandardGasMeter {
    pub fn new(
        gas_feature_version: u64,
        gas_params: AptosGasParameters,
        storage_gas_params: StorageGasParameters,
        balance: impl Into<Gas>,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&gas_params.txn);

        Self {
            feature_version: gas_feature_version,
            gas_params,
            storage_gas_params,
            balance,
            execution_gas_used: 0.into(),
            io_gas_used: 0.into(),
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
        }
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

    pub fn feature_version(&self) -> u64 {
        self.feature_version
    }
}

impl MoveGasMeter for StandardGasMeter {
    #[inline]
    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    #[inline]
    fn charge_br_false(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.br_false)
    }

    #[inline]
    fn charge_br_true(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.br_true)
    }

    #[inline]
    fn charge_branch(&mut self, _target_offset: CodeOffset) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.branch)
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
        _args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline]
    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        _ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView>>,
    ) -> PartialVMResult<()> {
        self.charge_execution(amount)
    }

    #[inline]
    fn charge_load_resource(
        &mut self,
        _addr: AccountAddress,
        _ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        if self.feature_version <= 8 && val.is_none() && bytes_loaded != 0.into() {
            return Err(PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message("in legacy versions, number of bytes loaded must be zero when the resource does not exist ".to_string()));
        }
        let cost = self
            .storage_gas_params
            .pricing
            .calculate_read_gas(val.is_some(), bytes_loaded);
        self.charge_io(cost)
    }

    #[inline]
    fn charge_pop(&mut self, _popped_val: impl ValueView) -> PartialVMResult<()> {
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
        _module_id: &ModuleId,
        _func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
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
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline]
    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .gas_params
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version);

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
        _old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.write_ref_base)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
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

        let params = &self.gas_params.instr;
        let cost = params.vec_pack_base + params.vec_pack_per_elem * num_args;
        self.charge_execution(cost)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
        _elems: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
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
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.vec_push_back_base)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.vec_pop_back_base)
    }

    #[inline]
    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge_execution(self.gas_params.instr.vec_swap_base)
    }

    #[inline]
    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

impl AptosGasMeter for StandardGasMeter {
    fn feature_version(&self) -> u64 {
        self.feature_version
    }

    fn gas_params(&self) -> &AptosGasParameters {
        &self.gas_params
    }

    fn balance(&self) -> Gas {
        self.balance
            .to_unit_round_down_with_params(&self.gas_params.txn)
    }

    fn gas_unit_scaling_factor(&self) -> GasScalingFactor {
        self.gas_params.txn.gas_unit_scaling_factor
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
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        let txn_params = &self.gas_params.txn;

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
        if self.feature_version >= 7 && self.storage_fee_used > self.gas_params.txn.max_storage_fee
        {
            return Err(PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED));
        }

        Ok(())
    }

    fn io_gas_per_write(&self, key: &StateKey, op: &WriteOp) -> InternalGas {
        self.storage_gas_params.pricing.io_gas_per_write(key, op)
    }

    fn storage_fee_per_write(&self, key: &StateKey, op: &WriteOp) -> Fee {
        self.gas_params.txn.storage_fee_per_write(key, op)
    }

    fn storage_fee_per_event(&self, event: &ContractEvent) -> Fee {
        self.gas_params.txn.storage_fee_per_event(event)
    }

    fn storage_discount_for_events(&self, total_cost: Fee) -> Fee {
        self.gas_params.txn.storage_discount_for_events(total_cost)
    }

    fn storage_fee_for_transaction_storage(&self, txn_size: NumBytes) -> Fee {
        self.gas_params
            .txn
            .storage_fee_for_transaction_storage(txn_size)
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let cost = self.gas_params.txn.calculate_intrinsic_gas(txn_size);
        self.charge_execution(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn execution_gas_used(&self) -> Gas {
        self.execution_gas_used
            .to_unit_round_up_with_params(&self.gas_params.txn)
    }

    fn io_gas_used(&self) -> Gas {
        self.io_gas_used
            .to_unit_round_up_with_params(&self.gas_params.txn)
    }

    fn storage_fee_used_in_gas_units(&self) -> Gas {
        self.storage_fee_in_internal_units
            .to_unit_round_up_with_params(&self.gas_params.txn)
    }

    fn storage_fee_used(&self) -> Fee {
        self.storage_fee_used
    }
}
