// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::views::{TypeView, ValueView};
use move_binary_format::{
    errors::PartialVMResult, file_format::CodeOffset, file_format_common::Opcodes,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{InternalGas, NumArgs, NumBytes, NumTypeNodes},
    identifier::IdentStr,
    language_storage::ModuleId,
};

/// Enum of instructions that do not need extra information for gas metering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimpleInstruction {
    Nop,
    Ret,

    LdU8,
    LdU64,
    LdU128,
    LdTrue,
    LdFalse,

    FreezeRef,
    MutBorrowLoc,
    ImmBorrowLoc,
    ImmBorrowField,
    MutBorrowField,
    ImmBorrowFieldGeneric,
    MutBorrowFieldGeneric,
    ImmBorrowVariantField,
    MutBorrowVariantField,
    ImmBorrowVariantFieldGeneric,
    MutBorrowVariantFieldGeneric,
    TestVariant,
    TestVariantGeneric,

    CastU8,
    CastU64,
    CastU128,

    Add,
    Sub,
    Mul,
    Mod,
    Div,

    BitOr,
    BitAnd,
    Xor,
    Shl,
    Shr,

    Or,
    And,
    Not,

    Lt,
    Gt,
    Le,
    Ge,

    Abort,

    LdU16,
    LdU32,
    LdU256,
    CastU16,
    CastU32,
    CastU256,
}

impl SimpleInstruction {
    pub fn to_opcode(&self) -> Opcodes {
        use Opcodes::*;
        use SimpleInstruction::*;

        match self {
            Nop => NOP,
            Ret => RET,

            LdU8 => LD_U8,
            LdU64 => LD_U64,
            LdU128 => LD_U128,
            LdTrue => LD_TRUE,
            LdFalse => LD_FALSE,

            FreezeRef => FREEZE_REF,
            MutBorrowLoc => MUT_BORROW_LOC,
            ImmBorrowLoc => IMM_BORROW_LOC,
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

            CastU8 => CAST_U8,
            CastU64 => CAST_U64,
            CastU128 => CAST_U128,

            Add => ADD,
            Sub => SUB,
            Mul => MUL,
            Mod => MOD,
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

            Abort => ABORT,
            LdU16 => LD_U16,
            LdU32 => LD_U32,
            LdU256 => LD_U256,
            CastU16 => CAST_U16,
            CastU32 => CAST_U32,
            CastU256 => CAST_U256,
        }
    }
}

/// Metering API for module or script dependencies. Defined as a stand-alone trait so it can be
/// used in native context.
///
/// Note: because native functions are trait objects, it is not possible to make [GasMeter] a trait
/// object as well as it has APIs that are generic.
pub trait DependencyGasMeter {
    fn charge_dependency(
        &mut self,
        is_new: bool,
        addr: &AccountAddress,
        name: &IdentStr,
        size: NumBytes,
    ) -> PartialVMResult<()>;
}

/// Gas meter to use to meter native function.
pub trait NativeGasMeter: DependencyGasMeter {
    /// Returns the remaining gas budget of the meter. Same as [GasMeter::balance_internal] and
    /// will be removed in the future.
    fn legacy_gas_budget_in_native_context(&self) -> InternalGas;

    /// Charges the given gas amount. Only used if metering in native context is enabled.
    fn charge_native_execution(&mut self, amount: InternalGas) -> PartialVMResult<()>;

    /// Tracks heap memory usage.
    fn use_heap_memory_in_native_context(&mut self, amount: u64) -> PartialVMResult<()>;
}

/// Trait that defines a generic gas meter interface, allowing clients of the Move VM to implement
/// their own metering scheme.
pub trait GasMeter: NativeGasMeter {
    fn balance_internal(&self) -> InternalGas;

    /// Charge an instruction and fail if not enough gas units are left.
    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()>;

    fn charge_br_true(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()>;

    fn charge_br_false(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()>;

    fn charge_branch(&mut self, target_offset: CodeOffset) -> PartialVMResult<()>;

    fn charge_pop(&mut self, popped_val: impl ValueView) -> PartialVMResult<()>;

    fn charge_call(
        &mut self,
        module_id: &ModuleId,
        func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: NumArgs,
    ) -> PartialVMResult<()>;

    fn charge_call_generic(
        &mut self,
        module_id: &ModuleId,
        func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: NumArgs,
    ) -> PartialVMResult<()>;

    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()>;

    fn charge_ld_const_after_deserialization(&mut self, val: impl ValueView)
        -> PartialVMResult<()>;

    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

    fn charge_move_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

    fn charge_store_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_pack_variant(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        // Currently mapped to pack, can be specialized if needed
        self.charge_pack(is_generic, args)
    }

    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_unpack_variant(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        // Currently mapped to pack, can be specialized if needed
        self.charge_unpack(is_generic, args)
    }

    fn charge_pack_closure(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()>;

    fn charge_write_ref(
        &mut self,
        new_val: impl ValueView,
        old_val: impl ValueView,
    ) -> PartialVMResult<()>;

    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()>;

    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()>;

    fn charge_borrow_global(
        &mut self,
        is_mut: bool,
        is_generic: bool,
        ty: impl TypeView,
        is_success: bool,
    ) -> PartialVMResult<()>;

    fn charge_exists(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        // TODO(Gas): see if we can get rid of this param
        exists: bool,
    ) -> PartialVMResult<()>;

    fn charge_move_from(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: Option<impl ValueView>,
    ) -> PartialVMResult<()>;

    fn charge_move_to(
        &mut self,
        is_generic: bool,
        ty: impl TypeView,
        val: impl ValueView,
        is_success: bool,
    ) -> PartialVMResult<()>;

    fn charge_vec_pack<'a>(
        &mut self,
        ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_vec_len(&mut self, ty: impl TypeView) -> PartialVMResult<()>;

    fn charge_vec_borrow(
        &mut self,
        is_mut: bool,
        ty: impl TypeView,
        is_success: bool,
    ) -> PartialVMResult<()>;

    fn charge_vec_push_back(
        &mut self,
        ty: impl TypeView,
        val: impl ValueView,
    ) -> PartialVMResult<()>;

    fn charge_vec_pop_back(
        &mut self,
        ty: impl TypeView,
        val: Option<impl ValueView>,
    ) -> PartialVMResult<()>;

    // TODO(Gas): Expose the elements
    fn charge_vec_unpack(
        &mut self,
        ty: impl TypeView,
        expect_num_elements: NumArgs,
        elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    // TODO(Gas): Expose the two elements
    fn charge_vec_swap(&mut self, ty: impl TypeView) -> PartialVMResult<()>;

    /// Charges for loading a resource from storage. This is only called when the resource is not
    /// cached.
    ///
    /// WARNING: This can be dangerous if you execute multiple user transactions in the same
    /// session -- identical transactions can have different gas costs. Use at your own risk.
    fn charge_load_resource(
        &mut self,
        addr: AccountAddress,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()>;

    /// Charge for executing a native function.
    /// The cost is calculated returned by the native function implementation.
    /// Should fail if not enough gas units are left.
    ///
    /// In the future, we may want to remove this and directly pass a reference to the GasMeter
    /// instance to the native functions to allow gas to be deducted during computation.
    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView> + Clone>,
    ) -> PartialVMResult<()>;

    fn charge_native_function_before_execution(
        &mut self,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_drop_frame(
        &mut self,
        locals: impl Iterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()>;

    fn charge_create_ty(&mut self, num_nodes: NumTypeNodes) -> PartialVMResult<()>;
}

/// A dummy gas meter that does not meter anything.
/// Charge operations will always succeed.
pub struct UnmeteredGasMeter;

impl DependencyGasMeter for UnmeteredGasMeter {
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

impl NativeGasMeter for UnmeteredGasMeter {
    fn legacy_gas_budget_in_native_context(&self) -> InternalGas {
        u64::MAX.into()
    }

    fn charge_native_execution(&mut self, _amount: InternalGas) -> PartialVMResult<()> {
        Ok(())
    }

    fn use_heap_memory_in_native_context(&mut self, _amount: u64) -> PartialVMResult<()> {
        Ok(())
    }
}

impl GasMeter for UnmeteredGasMeter {
    fn balance_internal(&self) -> InternalGas {
        u64::MAX.into()
    }

    fn charge_simple_instr(&mut self, _instr: SimpleInstruction) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_br_false(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_br_true(&mut self, _target_offset: Option<CodeOffset>) -> PartialVMResult<()> {
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
        _args: impl IntoIterator<Item = impl ValueView>,
        _num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_call_generic(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        _args: impl ExactSizeIterator<Item = impl ValueView>,
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
        _args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_unpack(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_pack_closure(
        &mut self,
        _is_generic: bool,
        _args: impl ExactSizeIterator<Item = impl ValueView>,
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
        _args: impl ExactSizeIterator<Item = impl ValueView>,
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
        _elems: impl ExactSizeIterator<Item = impl ValueView>,
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
        _ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView>>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        _args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }

    fn charge_create_ty(&mut self, _num_nodes: NumTypeNodes) -> PartialVMResult<()> {
        Ok(())
    }
}
