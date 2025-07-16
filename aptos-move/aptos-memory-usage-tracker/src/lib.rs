// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{
    AbstractValueSize, Fee, FeePerGasUnit, InternalGas, NumArgs, NumBytes, NumTypeNodes,
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS, contract_event::ContractEvent,
    state_store::state_key::StateKey, write_set::WriteOpSize,
};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult, VMResult},
    file_format::CodeOffset,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::{DependencyGasMeter, GasMeter, NativeGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};

/// Special gas meter implementation that tracks the VM's memory usage based on the operations
/// executed.
///
/// Must be composed with a base gas meter.
pub struct MemoryTrackedGasMeter<G> {
    base: G,

    memory_quota: AbstractValueSize,
    should_leak_memory_for_native: bool,
}

impl<G> MemoryTrackedGasMeter<G>
where
    G: AptosGasMeter,
{
    pub fn new(base: G) -> Self {
        let memory_quota = base.vm_gas_params().txn.memory_quota;

        Self {
            base,
            memory_quota,
            should_leak_memory_for_native: false,
        }
    }

    #[inline]
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()> {
        if self.feature_version() >= 3 {
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
        if self.feature_version() >= 3 {
            self.memory_quota += amount;
        }
    }
}

// TODO: consider switching to a library like https://docs.rs/delegate/latest/delegate/.
macro_rules! delegate {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        $(fn $fn $(<$($lt)*>)? (&self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

macro_rules! delegate_mut {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&mut self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        $(fn $fn $(<$($lt)*>)? (&mut self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

impl<G> DependencyGasMeter for MemoryTrackedGasMeter<G>
where
    G: AptosGasMeter,
{
    delegate_mut! {
        fn charge_dependency(&mut self, is_new: bool, addr: &AccountAddress, name: &IdentStr, size: NumBytes) -> PartialVMResult<()>;
    }
}

impl<G> NativeGasMeter for MemoryTrackedGasMeter<G>
where
    G: AptosGasMeter,
{
    delegate! {
        fn legacy_gas_budget_in_native_context(&self) -> InternalGas;
    }

    delegate_mut! {
        fn charge_native_execution(&mut self, amount: InternalGas) -> PartialVMResult<()>;
    }

    #[inline]
    fn use_heap_memory_in_native_context(&mut self, amount: u64) -> PartialVMResult<()> {
        self.use_heap_memory(amount.into())
    }
}

impl<G> GasMeter for MemoryTrackedGasMeter<G>
where
    G: AptosGasMeter,
{
    delegate_mut! {
        fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()>;

        fn charge_br_true(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()>;

        fn charge_br_false(&mut self, target_offset: Option<CodeOffset>) -> PartialVMResult<()>;

        fn charge_branch(&mut self, target_offset: CodeOffset) -> PartialVMResult<()>;

        fn charge_call(
            &mut self,
            module_id: &ModuleId,
            func_name: &str,
            args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
            num_locals: NumArgs,
        ) -> PartialVMResult<()>;

        fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()>;

        fn charge_move_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

        fn charge_store_loc(&mut self, val: impl ValueView) -> PartialVMResult<()>;

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

        fn charge_vec_len(&mut self, ty: impl TypeView) -> PartialVMResult<()>;

        fn charge_vec_borrow(
            &mut self,
            is_mut: bool,
            ty: impl TypeView,
            is_success: bool,
        ) -> PartialVMResult<()>;

        fn charge_vec_swap(&mut self, ty: impl TypeView) -> PartialVMResult<()>;

        fn charge_create_ty(&mut self, num_nodes: NumTypeNodes) -> PartialVMResult<()>;
    }

    #[inline]
    fn balance_internal(&self) -> InternalGas {
        self.base.balance_internal()
    }

    #[inline]
    fn charge_call_generic(
        &mut self,
        module_id: &ModuleId,
        func_name: &str,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
        num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        // Save the info for charge_native_function_before_execution.
        self.should_leak_memory_for_native = (*module_id.address() == CORE_CODE_ADDRESS
            && module_id.name().as_str() == "table")
            || (self.feature_version() >= 4
                && *module_id.address() == CORE_CODE_ADDRESS
                && module_id.name().as_str() == "event");

        self.base
            .charge_call_generic(module_id, func_name, ty_args, args, num_locals)
    }

    #[inline]
    fn charge_native_function_before_execution(
        &mut self,
        ty_args: impl ExactSizeIterator<Item = impl TypeView> + Clone,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        // TODO(Gas): https://github.com/aptos-labs/aptos-core/issues/5485
        if !self.should_leak_memory_for_native {
            self.release_heap_memory(args.clone().try_fold(
                AbstractValueSize::zero(),
                |acc, val| {
                    let heap_size = self
                        .vm_gas_params()
                        .misc
                        .abs_val
                        .abstract_heap_size(val, self.feature_version())?;
                    Ok::<_, PartialVMError>(acc + heap_size)
                },
            )?);
        }

        self.base
            .charge_native_function_before_execution(ty_args, args)
    }

    #[inline]
    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView> + Clone>,
    ) -> PartialVMResult<()> {
        if let Some(mut ret_vals) = ret_vals.clone() {
            self.use_heap_memory(ret_vals.try_fold(AbstractValueSize::zero(), |acc, val| {
                let heap_size = self
                    .vm_gas_params()
                    .misc
                    .abs_val
                    .abstract_heap_size(val, self.feature_version())?;
                Ok::<_, PartialVMError>(acc + heap_size)
            })?)?;
        }

        self.base.charge_native_function(amount, ret_vals)
    }

    #[inline]
    fn charge_load_resource(
        &mut self,
        addr: AccountAddress,
        ty: impl TypeView,
        val: Option<impl ValueView>,
        bytes_loaded: NumBytes,
    ) -> PartialVMResult<()> {
        if self.feature_version() != 0 {
            // TODO(Gas): Rewrite this in a better way.
            if let Some(val) = &val {
                self.use_heap_memory(
                    self.vm_gas_params()
                        .misc
                        .abs_val
                        .abstract_heap_size(val, self.feature_version())?,
                )?;
            }
        }

        self.base.charge_load_resource(addr, ty, val, bytes_loaded)
    }

    #[inline]
    fn charge_pop(&mut self, popped_val: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&popped_val, self.feature_version())?,
        );

        self.base.charge_pop(popped_val)
    }

    #[inline]
    fn charge_ld_const_after_deserialization(
        &mut self,
        val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&val, self.feature_version())?,
        )?;

        self.base.charge_ld_const_after_deserialization(val)
    }

    #[inline]
    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let heap_size = self
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_heap_size(&val, self.feature_version())?;

        self.use_heap_memory(heap_size)?;

        self.base.charge_copy_loc(val)
    }

    #[inline]
    fn charge_pack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            args.clone()
                .try_fold(AbstractValueSize::zero(), |acc, val| {
                    let stack_size = self
                        .vm_gas_params()
                        .misc
                        .abs_val
                        .abstract_stack_size(val, self.feature_version())?;
                    Ok::<_, PartialVMError>(acc + stack_size)
                })?,
        )?;

        self.base.charge_pack(is_generic, args)
    }

    #[inline]
    fn charge_unpack(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(args.clone().try_fold(
            AbstractValueSize::zero(),
            |acc, val| {
                let stack_size = self
                    .vm_gas_params()
                    .misc
                    .abs_val
                    .abstract_stack_size(val, self.feature_version())?;
                Ok::<_, PartialVMError>(acc + stack_size)
            },
        )?);

        self.base.charge_unpack(is_generic, args)
    }

    #[inline]
    fn charge_pack_closure(
        &mut self,
        is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            args.clone()
                .try_fold(AbstractValueSize::zero(), |acc, val| {
                    let stack_size = self
                        .vm_gas_params()
                        .misc
                        .abs_val
                        .abstract_stack_size(val, self.feature_version())?;
                    Ok::<_, PartialVMError>(acc + stack_size)
                })?,
        )?;

        self.base.charge_pack_closure(is_generic, args)
    }

    #[inline]
    fn charge_read_ref(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        let heap_size = self
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_heap_size(&val, self.feature_version())?;

        self.use_heap_memory(heap_size)?;

        self.base.charge_read_ref(val)
    }

    #[inline]
    fn charge_write_ref(
        &mut self,
        new_val: impl ValueView,
        old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&old_val, self.feature_version())?,
        );

        self.base.charge_write_ref(new_val, old_val)
    }

    #[inline]
    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&lhs, self.feature_version())?,
        );
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&rhs, self.feature_version())?,
        );

        self.base.charge_eq(lhs, rhs)
    }

    #[inline]
    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&lhs, self.feature_version())?,
        );
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(&rhs, self.feature_version())?,
        );

        self.base.charge_neq(lhs, rhs)
    }

    #[inline]
    fn charge_vec_pack<'a>(
        &mut self,
        ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            args.clone()
                .try_fold(AbstractValueSize::zero(), |acc, val| {
                    Ok::<_, PartialVMError>(
                        acc + self
                            .vm_gas_params()
                            .misc
                            .abs_val
                            .abstract_packed_size(val)?,
                    )
                })?,
        )?;

        self.base.charge_vec_pack(ty, args)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
        ty: impl TypeView,
        expect_num_elements: NumArgs,
        elems: impl ExactSizeIterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(elems.clone().try_fold(
            AbstractValueSize::zero(),
            |acc, val| {
                Ok::<_, PartialVMError>(
                    acc + self
                        .vm_gas_params()
                        .misc
                        .abs_val
                        .abstract_packed_size(val)?,
                )
            },
        )?);

        self.base.charge_vec_unpack(ty, expect_num_elements, elems)
    }

    #[inline]
    fn charge_vec_push_back(
        &mut self,
        ty: impl TypeView,
        val: impl ValueView,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_packed_size(&val)?,
        )?;

        self.base.charge_vec_push_back(ty, val)
    }

    #[inline]
    fn charge_vec_pop_back(
        &mut self,
        ty: impl TypeView,
        val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        if let Some(val) = &val {
            self.release_heap_memory(
                self.vm_gas_params()
                    .misc
                    .abs_val
                    .abstract_packed_size(val)?,
            );
        }

        self.base.charge_vec_pop_back(ty, val)
    }

    #[inline]
    fn charge_drop_frame(
        &mut self,
        locals: impl Iterator<Item = impl ValueView> + Clone,
    ) -> PartialVMResult<()> {
        self.release_heap_memory(locals.clone().try_fold(
            AbstractValueSize::zero(),
            |acc, val| {
                let heap_size = self
                    .vm_gas_params()
                    .misc
                    .abs_val
                    .abstract_heap_size(val, self.feature_version())?;
                Ok::<_, PartialVMError>(acc + heap_size)
            },
        )?);

        self.base.charge_drop_frame(locals)
    }
}

impl<G> AptosGasMeter for MemoryTrackedGasMeter<G>
where
    G: AptosGasMeter,
{
    type Algebra = G::Algebra;

    delegate! {
        fn algebra(&self) -> &Self::Algebra;
    }

    delegate_mut! {
        fn algebra_mut(&mut self) -> &mut Self::Algebra;

        fn charge_io_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()>;

        fn charge_io_gas_for_event(&mut self, event: &ContractEvent) -> VMResult<()>;

        fn charge_io_gas_for_write(&mut self, key: &StateKey, op: &WriteOpSize) -> VMResult<()>;

        fn charge_storage_fee(
            &mut self,
            amount: Fee,
            gas_unit_price: FeePerGasUnit,
        ) -> PartialVMResult<()>;

        fn charge_intrinsic_gas_for_transaction(&mut self, txn_size: NumBytes) -> VMResult<()>;

        fn charge_keyless(&mut self) -> VMResult<()>;
    }
}
