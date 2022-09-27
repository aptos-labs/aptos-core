// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module contains the official gas meter implementation, along with some top-level gas
//! parameters and traits to help manipulate them.

use crate::{
    algebra::Gas, instr::InstructionGasParameters, misc::MiscGasParameters,
    transaction::StorageGasParameters, transaction::TransactionGasParameters,
};
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
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

pub const LATEST_GAS_FEATURE_VERSION: u64 = 1;

/// A trait for converting from a map representation of the on-chain gas schedule.
pub trait FromOnChainGasSchedule: Sized {
    /// Constructs a value of this type from a map representation of the on-chain gas schedule.
    /// `None` should be returned when the gas schedule is missing some required entries.
    /// Unused entries should be safely ignored.
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self>;
}

/// A trait for converting to a list of entries of the on-chain gas schedule.
pub trait ToOnChainGasSchedule {
    /// Converts `self` into a list of entries of the on-chain gas schedule.
    /// Each entry is a key-value pair where the key is a string representing the name of the
    /// parameter, where the value is the gas parameter itself.
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)>;
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
    pub aptos_framework: framework::natives::GasParameters,
    pub table: move_table_extension::GasParameters,
}

impl FromOnChainGasSchedule for NativeGasParameters {
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
        Some(Self {
            move_stdlib: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
            aptos_framework: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
            table: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
        })
    }
}

impl ToOnChainGasSchedule for NativeGasParameters {
    fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
        let mut entries = self.move_stdlib.to_on_chain_gas_schedule();
        entries.extend(self.aptos_framework.to_on_chain_gas_schedule());
        entries.extend(self.table.to_on_chain_gas_schedule());
        entries
    }
}

impl NativeGasParameters {
    pub fn zeros() -> Self {
        Self {
            move_stdlib: move_stdlib::natives::GasParameters::zeros(),
            aptos_framework: framework::natives::GasParameters::zeros(),
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
    fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
        Some(Self {
            misc: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule)?,
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
        entries.extend(self.misc.to_on_chain_gas_schedule());
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
    storage_gas_params: Option<StorageGasParameters>,
    balance: InternalGas,
}

impl AptosGasMeter {
    pub fn new(
        gas_feature_version: u64,
        gas_params: AptosGasParameters,
        storage_gas_params: Option<StorageGasParameters>,
        balance: impl Into<Gas>,
    ) -> Self {
        assert!(
            (gas_feature_version == 0 && storage_gas_params.is_none())
                || (gas_feature_version > 0 && storage_gas_params.is_some()),
            "Invalid gas meter configuration"
        );

        let balance = balance.into().to_unit_with_params(&gas_params.txn);

        Self {
            feature_version: gas_feature_version,
            gas_params,
            storage_gas_params,
            balance,
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
            }
            None => {
                self.balance = 0.into();
                Err(PartialVMError::new(StatusCode::OUT_OF_GAS))
            }
        }
    }
}

impl GasMeter for AptosGasMeter {
    #[inline]
    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        let cost = self.gas_params.instr.simple_instr_cost(instr)?;
        self.charge(cost)
    }

    #[inline]
    fn charge_native_function(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        self.charge(amount)
    }

    #[inline]
    fn charge_load_resource(&mut self, loaded: Option<NumBytes>) -> PartialVMResult<()> {
        let cost = match self.feature_version {
            0 => {
                let txn_params = &self.gas_params.txn;

                txn_params.load_data_base
                    + match loaded {
                        Some(num_bytes) => txn_params.load_data_per_byte * num_bytes,
                        None => txn_params.load_data_failure,
                    }
            }
            _ => {
                let storage_params = self.storage_gas_params.as_ref().unwrap();

                // TODO(Gas): Rewrite this in a better way.
                storage_params.per_item_read * (NumArgs::from(1))
                    + match loaded {
                        Some(num_bytes) => storage_params.per_byte_read * num_bytes,
                        None => 0.into(),
                    }
            }
        };

        self.charge(cost)
    }

    #[inline]
    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        self.charge(params.call_base + params.call_per_arg * NumArgs::new(args.len() as u64))
    }

    #[inline]
    fn charge_call_generic(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        self.charge(
            params.call_generic_base
                + params.call_generic_per_ty_arg * NumArgs::new(ty_args.len() as u64)
                + params.call_generic_per_arg * NumArgs::new(args.len() as u64),
        )
    }

    #[inline]
    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        self.charge(params.ld_const_base + params.ld_const_per_byte * size)
    }

    #[inline]
    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        let instr_params = &self.gas_params.instr;
        let cost = instr_params.copy_loc_base
            + instr_params.copy_loc_per_abs_val_unit
                * self.gas_params.misc.abs_val.abstract_value_size(val);

        self.charge(cost)
    }

    #[inline]
    fn charge_move_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.move_loc_base)
    }

    #[inline]
    fn charge_store_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.st_loc_base)
    }

    #[inline]
    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.pack_base + params.pack_per_field * NumArgs::new(args.len() as u64),
            true => {
                params.pack_generic_base
                    + params.pack_generic_per_field * NumArgs::new(args.len() as u64)
            }
        };
        self.charge(cost)
    }

    #[inline]
    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost = match is_generic {
            false => params.unpack_base + params.unpack_per_field * NumArgs::new(args.len() as u64),
            true => {
                params.unpack_generic_base
                    + params.unpack_generic_per_field * NumArgs::new(args.len() as u64)
            }
        };
        self.charge(cost)
    }

    #[inline]
    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        let instr_params = &self.gas_params.instr;
        let cost = instr_params.read_ref_base
            + instr_params.read_ref_per_abs_val_unit
                * self.gas_params.misc.abs_val.abstract_value_size(val);
        self.charge(cost)
    }

    #[inline]
    fn charge_write_ref(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.write_ref_base)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let instr_params = &self.gas_params.instr;
        let abs_val_params = &self.gas_params.misc.abs_val;
        let per_unit = instr_params.eq_per_abs_val_unit;

        let cost = instr_params.eq_base
            + per_unit
                * (abs_val_params.abstract_value_size_dereferenced(lhs)
                    + abs_val_params.abstract_value_size_dereferenced(rhs));

        self.charge(cost)
    }

    #[inline]
    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let instr_params = &self.gas_params.instr;
        let abs_val_params = &self.gas_params.misc.abs_val;
        let per_unit = instr_params.neq_per_abs_val_unit;

        let cost = instr_params.neq_base
            + per_unit
                * (abs_val_params.abstract_value_size_dereferenced(lhs)
                    + abs_val_params.abstract_value_size_dereferenced(rhs));

        self.charge(cost)
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
        self.charge(cost)
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
        self.charge(cost)
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
        self.charge(cost)
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
        self.charge(cost)
    }

    #[inline]
    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost =
            params.vec_pack_base + params.vec_pack_per_elem * NumArgs::new(args.len() as u64);
        self.charge(cost)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
    ) -> PartialVMResult<()> {
        let params = &self.gas_params.instr;
        let cost =
            params.vec_unpack_base + params.vec_unpack_per_expected_elem * expect_num_elements;
        self.charge(cost)
    }

    #[inline]
    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.vec_len_base)
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
        self.charge(cost)
    }

    #[inline]
    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.vec_push_back_base)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.vec_pop_back_base)
    }

    #[inline]
    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge(self.gas_params.instr.vec_swap_base)
    }
}

impl AptosGasMeter {
    pub fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let cost = self.gas_params.txn.calculate_intrinsic_gas(txn_size);
        self.charge(cost).map_err(|e| e.finish(Location::Undefined))
    }

    pub fn charge_write_set_gas<'a>(
        &mut self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> VMResult<()> {
        let cost = match self.feature_version {
            0 => self.gas_params.txn.calculate_write_set_gas(ops),
            _ => self
                .storage_gas_params
                .as_ref()
                .unwrap()
                .calculate_write_set_gas(ops),
        };
        self.charge(cost).map_err(|e| e.finish(Location::Undefined))
    }
}
