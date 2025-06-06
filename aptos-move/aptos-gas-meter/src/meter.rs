// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{AptosGasMeter, GasAlgebra};
use aptos_gas_algebra::{Fee, FeePerGasUnit, NumTypeNodes};
use aptos_gas_schedule::{
    gas_feature_versions::*,
    gas_params::{instr::*, txn::*},
};
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
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
    gas::{DependencyGasMeter, GasMeter, NativeGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};
use std::collections::BTreeSet;

/// The official gas meter used inside the Aptos VM.
/// It maintains an internal gas counter, measured in internal gas units, and carries an environment
/// consisting all the gas parameters, which it can lookup when performing gas calculations.
pub struct StandardGasMeter<A> {
    algebra: A,
    metered_modules: BTreeSet<ModuleId>,
}

impl<A> StandardGasMeter<A>
where
    A: GasAlgebra,
{
    pub fn new(algebra: A) -> Self {
        Self {
            algebra,
            metered_modules: BTreeSet::new(),
        }
    }

    pub fn feature_version(&self) -> u64 {
        self.algebra.feature_version()
    }
}

impl<A> DependencyGasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    fn is_existing_dependency_metered(&self, module_id: &ModuleId) -> bool {
        self.metered_modules.contains(module_id)
    }

    fn charge_new_dependency(
        &mut self,
        module_id: &ModuleId,
        size: NumBytes,
    ) -> PartialVMResult<()> {
        // Modules under special addresses are considered system modules that should always
        // be loaded, and are therefore excluded from gas charging.
        //
        // TODO: 0xA550C18 is a legacy system address we used, but it is currently not covered by
        //       `.is_special()`. We should double check if this address still needs special
        //       treatment.
        if self.feature_version() >= 15 && !module_id.address().is_special() {
            self.algebra
                .charge_execution(DEPENDENCY_PER_MODULE + DEPENDENCY_PER_BYTE * size)?;
            self.algebra.count_dependency(size)?;
        }
        Ok(())
    }

    fn charge_existing_dependency(
        &mut self,
        module_id: &ModuleId,
        size: NumBytes,
    ) -> PartialVMResult<()> {
        // Modules under special addresses are considered system modules that should always
        // be loaded, and are therefore excluded from gas charging.
        //
        // TODO: 0xA550C18 is a legacy system address we used, but it is currently not covered by
        //       `.is_special()`. We should double check if this address still needs special
        //       treatment.
        if self.feature_version() < 15
            || module_id.address().is_special()
            || self.metered_modules.contains(module_id)
        {
            return Ok(());
        }

        self.metered_modules.insert(module_id.clone());
        self.algebra
            .charge_execution(DEPENDENCY_PER_MODULE + DEPENDENCY_PER_BYTE * size)?;
        self.algebra.count_dependency(size)?;
        Ok(())
    }
}

impl<A> NativeGasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    fn legacy_gas_budget_in_native_context(&self) -> InternalGas {
        self.algebra.balance_internal()
    }

    fn charge_native_execution(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        self.algebra.charge_execution(amount)
    }

    #[inline]
    fn use_heap_memory_in_native_context(&mut self, _amount: u64) -> PartialVMResult<()> {
        Ok(())
    }
}

impl<A> GasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    #[inline]
    fn balance_internal(&self) -> InternalGas {
        self.algebra.balance_internal()
    }

    #[inline]
    fn charge_br_false(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.algebra.charge_execution(BR_FALSE)
    }

    #[inline]
    fn charge_br_true(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        self.algebra.charge_execution(BR_TRUE)
    }

    #[inline]
    fn charge_branch(&mut self, _target_offset: CodeOffset) -> PartialVMResult<()> {
        self.algebra.charge_execution(BRANCH)
    }

    #[inline]
    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        macro_rules! dispatch {
            ($($name: ident => $cost: expr),* $(,)?) => {
                match instr {
                    $(SimpleInstruction::$name => self.algebra.charge_execution($cost)),*
                }
            };
        }

        dispatch! {
            Nop => NOP,

            Abort => ABORT,
            Ret => RET,

            LdU8 => LD_U8,
            LdU16 => LD_U16,
            LdU32 => LD_U32,
            LdU64 => LD_U64,
            LdU128 => LD_U128,
            LdU256 => LD_U256,
            LdTrue => LD_TRUE,
            LdFalse => LD_FALSE,

            ImmBorrowLoc => IMM_BORROW_LOC,
            MutBorrowLoc => MUT_BORROW_LOC,
            ImmBorrowField => IMM_BORROW_FIELD,
            MutBorrowField => MUT_BORROW_FIELD,
            ImmBorrowFieldGeneric => IMM_BORROW_FIELD_GENERIC,
            MutBorrowFieldGeneric => MUT_BORROW_FIELD_GENERIC,
            ImmBorrowVariantField => IMM_BORROW_VARIANT_FIELD,
            MutBorrowVariantField => MUT_BORROW_VARIANT_FIELD,
            ImmBorrowVariantFieldGeneric => IMM_BORROW_VARIANT_FIELD_GENERIC,
            MutBorrowVariantFieldGeneric => MUT_BORROW_VARIANT_FIELD_GENERIC,
            TestVariant => TEST_VARIANT,
            TestVariantGeneric => TEST_VARIANT_GENERIC,

            FreezeRef => FREEZE_REF,

            CastU8 => CAST_U8,
            CastU16 => CAST_U16,
            CastU32 => CAST_U32,
            CastU64 => CAST_U64,
            CastU128 => CAST_U128,
            CastU256 => CAST_U256,

            Add => ADD,
            Sub => SUB,
            Mul => MUL,
            Mod => MOD_,
            Div => DIV,

            BitOr => BIT_OR,
            BitAnd => BIT_AND,
            Xor => XOR,
            Shl => SHL,
            Shr => SHR,

            Or => OR,
            And => AND,
            Not => NOT,

            Lt => LT,
            Gt => GT,
            Le => LE,
            Ge => GE,
        }
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
        // TODO(Gas): Is this correct???
        self.algebra.charge_execution(amount)
    }

    #[inline]
    fn charge_load_resource(
        &mut self,
        _addr: AccountAddress,
        _ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        // TODO(Gas): check if this is correct.
        if self.feature_version() <= 8 && val.is_none() && bytes_loaded != 0.into() {
            return Err(PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message("in legacy versions, number of bytes loaded must be zero when the resource does not exist ".to_string()));
        }
        let cost = self
            .io_pricing()
            .calculate_read_gas(val.is_some(), bytes_loaded);
        self.algebra.charge_io(cost)
    }

    #[inline]
    fn charge_pop(&mut self, _popped_val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(POP)
    }

    #[inline]
    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        let cost = CALL_BASE + CALL_PER_ARG * NumArgs::new(args.len() as u64);

        match self.feature_version() {
            0..=2 => self.algebra.charge_execution(cost),
            3.. => self
                .algebra
                .charge_execution(cost + CALL_PER_LOCAL * num_locals),
        }
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
        let cost = CALL_GENERIC_BASE
            + CALL_GENERIC_PER_TY_ARG * NumArgs::new(ty_args.len() as u64)
            + CALL_GENERIC_PER_ARG * NumArgs::new(args.len() as u64);

        match self.feature_version() {
            0..=2 => self.algebra.charge_execution(cost),
            3.. => self
                .algebra
                .charge_execution(cost + CALL_GENERIC_PER_LOCAL * num_locals),
        }
    }

    #[inline]
    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()> {
        self.algebra
            .charge_execution(LD_CONST_BASE + LD_CONST_PER_BYTE * size)
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
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version());

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        self.algebra
            .charge_execution(COPY_LOC_BASE + COPY_LOC_PER_ABS_VAL_UNIT * (stack_size + heap_size))
    }

    #[inline]
    fn charge_move_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(MOVE_LOC_BASE)
    }

    #[inline]
    fn charge_store_loc(&mut self, _val: impl ValueView) -> PartialVMResult<()> {
        self.algebra.charge_execution(ST_LOC_BASE)
    }

    #[inline]
    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        match is_generic {
            false => self
                .algebra
                .charge_execution(PACK_BASE + PACK_PER_FIELD * num_args),
            true => self
                .algebra
                .charge_execution(PACK_GENERIC_BASE + PACK_GENERIC_PER_FIELD * num_args),
        }
    }

    #[inline]
    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        match is_generic {
            false => self
                .algebra
                .charge_execution(UNPACK_BASE + UNPACK_PER_FIELD * num_args),
            true => self
                .algebra
                .charge_execution(UNPACK_GENERIC_BASE + UNPACK_GENERIC_PER_FIELD * num_args),
        }
    }

    #[inline]
    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let (stack_size, heap_size) = self
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version());

        // Note(Gas): this makes a deep copy so we need to charge for the full value size
        self.algebra
            .charge_execution(READ_REF_BASE + READ_REF_PER_ABS_VAL_UNIT * (stack_size + heap_size))
    }

    #[inline]
    fn charge_write_ref(
        &mut self,
        _new_val: impl ValueView,
        _old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(WRITE_REF_BASE)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let abs_val_params = &self.vm_gas_params().misc.abs_val;

        let cost = EQ_BASE
            + EQ_PER_ABS_VAL_UNIT
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version())
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version()));

        self.algebra.charge_execution(cost)
    }

    #[inline]
    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let abs_val_params = &self.vm_gas_params().misc.abs_val;

        let cost = NEQ_BASE
            + NEQ_PER_ABS_VAL_UNIT
                * (abs_val_params.abstract_value_size_dereferenced(lhs, self.feature_version())
                    + abs_val_params.abstract_value_size_dereferenced(rhs, self.feature_version()));

        self.algebra.charge_execution(cost)
    }

    #[inline]
    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match (is_mut, is_generic) {
            (false, false) => self.algebra.charge_execution(IMM_BORROW_GLOBAL_BASE),
            (false, true) => self
                .algebra
                .charge_execution(IMM_BORROW_GLOBAL_GENERIC_BASE),
            (true, false) => self.algebra.charge_execution(MUT_BORROW_GLOBAL_BASE),
            (true, true) => self
                .algebra
                .charge_execution(MUT_BORROW_GLOBAL_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_exists(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _exists: bool,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(EXISTS_BASE),
            true => self.algebra.charge_execution(EXISTS_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_move_from(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(MOVE_FROM_BASE),
            true => self.algebra.charge_execution(MOVE_FROM_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_move_to(
        &mut self,
        is_generic: bool,
        _ty: impl TypeView,
        _val: impl ValueView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match is_generic {
            false => self.algebra.charge_execution(MOVE_TO_BASE),
            true => self.algebra.charge_execution(MOVE_TO_GENERIC_BASE),
        }
    }

    #[inline]
    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        let num_args = NumArgs::new(args.len() as u64);

        self.algebra
            .charge_execution(VEC_PACK_BASE + VEC_PACK_PER_ELEM * num_args)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
        _elems: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        self.algebra
            .charge_execution(VEC_UNPACK_BASE + VEC_UNPACK_PER_EXPECTED_ELEM * expect_num_elements)
    }

    #[inline]
    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_LEN_BASE)
    }

    #[inline]
    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        match is_mut {
            false => self.algebra.charge_execution(VEC_IMM_BORROW_BASE),
            true => self.algebra.charge_execution(VEC_MUT_BORROW_BASE),
        }
    }

    #[inline]
    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_PUSH_BACK_BASE)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_POP_BACK_BASE)
    }

    #[inline]
    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.algebra.charge_execution(VEC_SWAP_BASE)
    }

    #[inline]
    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    #[inline]
    fn charge_create_ty(&mut self, num_nodes: NumTypeNodes) -> PartialVMResult<()> {
        if self.feature_version() < 14 {
            return Ok(());
        }

        let cost = SUBST_TY_PER_NODE * num_nodes;

        self.algebra.charge_execution(cost)
    }
}

impl<A> AptosGasMeter for StandardGasMeter<A>
where
    A: GasAlgebra,
{
    type Algebra = A;

    #[inline]
    fn algebra(&self) -> &Self::Algebra {
        &self.algebra
    }

    #[inline]
    fn algebra_mut(&mut self) -> &mut Self::Algebra {
        &mut self.algebra
    }

    #[inline]
    fn charge_storage_fee(
        &mut self,
        amount: Fee,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        self.algebra.charge_storage_fee(amount, gas_unit_price)
    }

    fn charge_io_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let cost = self.io_pricing().io_gas_per_transaction(txn_size);

        self.algebra
            .charge_io(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn charge_io_gas_for_event(&mut self, event: &ContractEvent) -> VMResult<()> {
        let cost = self.io_pricing().io_gas_per_event(event);

        self.algebra
            .charge_io(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn charge_io_gas_for_write(&mut self, key: &StateKey, op_size: &WriteOpSize) -> VMResult<()> {
        let cost = self.io_pricing().io_gas_per_write(key, op_size);

        self.algebra
            .charge_io(cost)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()> {
        let excess = txn_size
            .checked_sub(self.vm_gas_params().txn.large_transaction_cutoff)
            .unwrap_or_else(|| 0.into());

        self.algebra
            .charge_execution(MIN_TRANSACTION_GAS_UNITS + INTRINSIC_GAS_PER_BYTE * excess)
            .map_err(|e| e.finish(Location::Undefined))
    }

    fn charge_keyless(&mut self) -> VMResult<()> {
        if self.feature_version() < RELEASE_V1_12 {
            return Ok(());
        }

        self.algebra
            .charge_execution(KEYLESS_BASE_COST)
            .map_err(|e| e.finish(Location::Undefined))
    }
}
