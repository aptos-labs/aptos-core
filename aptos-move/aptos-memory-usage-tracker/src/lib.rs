// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_gas_algebra::{
    AbstractValueSize, Fee, FeePerGasUnit, InternalGas, NumArgs, NumBytes, NumTypeNodes,
};
use aptos_gas_meter::{AptosGasMeter, CacheValueSizes, PeakMemoryUsage};
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
    gas::{DependencyGasMeter, DependencyKind, GasMeter, NativeGasMeter, SimpleInstruction},
    views::{TypeView, ValueView},
};

pub trait MemoryAlgebra {
    fn new(memory_quota: AbstractValueSize, feature_version: u64) -> Self;
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()>;
    fn release_heap_memory(&mut self, amount: AbstractValueSize);
    fn current_memory_usage(&self) -> AbstractValueSize;
}

pub struct StandardMemoryAlgebra {
    initial_memory_quota: AbstractValueSize,
    remaining_memory_quota: AbstractValueSize,
    feature_version: u64,
}

impl MemoryAlgebra for StandardMemoryAlgebra {
    fn new(memory_quota: AbstractValueSize, feature_version: u64) -> Self {
        Self {
            initial_memory_quota: memory_quota,
            remaining_memory_quota: memory_quota,
            feature_version,
        }
    }

    #[inline]
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()> {
        if self.feature_version >= 3 {
            match self.remaining_memory_quota.checked_sub(amount) {
                Some(remaining_quota) => {
                    self.remaining_memory_quota = remaining_quota;
                    Ok(())
                },
                None => {
                    self.remaining_memory_quota = 0.into();
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
            self.remaining_memory_quota += amount;
        }
    }

    #[inline]
    fn current_memory_usage(&self) -> AbstractValueSize {
        match self
            .initial_memory_quota
            .checked_sub(self.remaining_memory_quota)
        {
            Some(usage) => usage,
            None => AbstractValueSize::zero(),
            // Note: It's possible for the available memory quota to rise above the initial quota
            //       under rare circumstances (e.g. values not being tracked initially).
            //       In such cases, just treat the usage as 0.
        }
    }
}

pub struct ExtendedMemoryAlgebra {
    base: StandardMemoryAlgebra,
    peak_memory_usage: AbstractValueSize,
}

impl ExtendedMemoryAlgebra {
    #[inline]
    fn update_peak_memory_usage(&mut self) {
        self.peak_memory_usage = self.base.current_memory_usage().max(self.peak_memory_usage);
    }
}

impl MemoryAlgebra for ExtendedMemoryAlgebra {
    fn new(memory_quota: AbstractValueSize, feature_version: u64) -> Self {
        Self {
            base: StandardMemoryAlgebra::new(memory_quota, feature_version),
            peak_memory_usage: AbstractValueSize::zero(),
        }
    }

    #[inline]
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()> {
        let res = self.base.use_heap_memory(amount);
        self.update_peak_memory_usage();
        res
    }

    #[inline]
    fn release_heap_memory(&mut self, amount: AbstractValueSize) {
        self.base.release_heap_memory(amount);
        self.update_peak_memory_usage();
    }

    #[inline]
    fn current_memory_usage(&self) -> AbstractValueSize {
        self.base.current_memory_usage()
    }
}

/// Special gas meter implementation that tracks the VM's memory usage based on the operations
/// executed.
///
/// Must be composed with a base gas meter.
pub struct MemoryTrackedGasMeterImpl<G, A> {
    base: G,

    algebra: A,
    should_leak_memory_for_native: bool,
}

pub type MemoryTrackedGasMeter<G> = MemoryTrackedGasMeterImpl<G, StandardMemoryAlgebra>;
pub type ExtendedMemoryTrackedGasMeter<G> = MemoryTrackedGasMeterImpl<G, ExtendedMemoryAlgebra>;

impl<G> PeakMemoryUsage for ExtendedMemoryTrackedGasMeter<G> {
    fn peak_memory_usage(&self) -> AbstractValueSize {
        self.algebra.peak_memory_usage
    }
}

impl<G, A> MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter + CacheValueSizes,
    A: MemoryAlgebra,
{
    pub fn new(base: G) -> Self {
        let memory_quota = base.vm_gas_params().txn.memory_quota;
        let feature_version = base.feature_version();

        Self {
            base,
            algebra: A::new(memory_quota, feature_version),
            should_leak_memory_for_native: false,
        }
    }

    #[inline]
    fn use_heap_memory(&mut self, amount: AbstractValueSize) -> PartialVMResult<()> {
        self.algebra.use_heap_memory(amount)
    }

    #[inline]
    fn release_heap_memory(&mut self, amount: AbstractValueSize) {
        self.algebra.release_heap_memory(amount);
    }
}

// TODO: consider switching to a library like https://docs.rs/delegate/latest/delegate/.
macro_rules! delegate {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        #[inline(always)]
        $(fn $fn $(<$($lt)*>)? (&self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

macro_rules! delegate_mut {
    ($(
        fn $fn: ident $(<$($lt: lifetime),*>)? (&mut self $(, $arg: ident : $ty: ty)* $(,)?) -> $ret_ty: ty;
    )*) => {
        #[inline(always)]
        $(fn $fn $(<$($lt)*>)? (&mut self, $($arg: $ty),*) -> $ret_ty {
            self.base.$fn($($arg),*)
        })*
    };
}

impl<G, A> DependencyGasMeter for MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter,
    A: MemoryAlgebra,
{
    delegate_mut! {
        fn charge_dependency(&mut self, kind: DependencyKind, addr: &AccountAddress, name: &IdentStr, size: NumBytes) -> PartialVMResult<()>;
    }
}

impl<G, A> NativeGasMeter for MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter + CacheValueSizes,
    A: MemoryAlgebra,
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

impl<G, A> GasMeter for MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter + CacheValueSizes,
    A: MemoryAlgebra,
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

        fn charge_vec_len(&mut self) -> PartialVMResult<()>;

        fn charge_vec_borrow(
            &mut self,
            is_mut: bool,
        ) -> PartialVMResult<()>;

        fn charge_vec_swap(&mut self) -> PartialVMResult<()>;

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

    fn charge_abort_message(&mut self, bytes: &[u8]) -> PartialVMResult<()> {
        self.release_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_heap_size(bytes, self.feature_version())?,
        );

        self.base.charge_abort_message(bytes)
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
        let (stack_size, heap_size) = self
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(&val, self.feature_version())?;

        self.charge_copy_loc_cached(stack_size, heap_size)
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
        let (stack_size, heap_size) = self
            .vm_gas_params()
            .misc
            .abs_val
            .abstract_value_size_stack_and_heap(val, self.feature_version())?;

        self.charge_read_ref_cached(stack_size, heap_size)
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
    fn charge_vec_pack(
        &mut self,
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

        self.base.charge_vec_pack(args)
    }

    #[inline]
    fn charge_vec_unpack(
        &mut self,
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

        self.base.charge_vec_unpack(expect_num_elements, elems)
    }

    #[inline]
    fn charge_vec_push_back(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        self.use_heap_memory(
            self.vm_gas_params()
                .misc
                .abs_val
                .abstract_packed_size(&val)?,
        )?;

        self.base.charge_vec_push_back(val)
    }

    #[inline]
    fn charge_vec_pop_back(&mut self, val: Option<impl ValueView>) -> PartialVMResult<()> {
        if let Some(val) = &val {
            self.release_heap_memory(
                self.vm_gas_params()
                    .misc
                    .abs_val
                    .abstract_packed_size(val)?,
            );
        }

        self.base.charge_vec_pop_back(val)
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

impl<G, A> CacheValueSizes for MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter + CacheValueSizes,
    A: MemoryAlgebra,
{
    #[inline]
    fn charge_read_ref_cached(
        &mut self,
        stack_size: AbstractValueSize,
        heap_size: AbstractValueSize,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(heap_size)?;

        self.base.charge_read_ref_cached(stack_size, heap_size)
    }

    #[inline]
    fn charge_copy_loc_cached(
        &mut self,
        stack_size: AbstractValueSize,
        heap_size: AbstractValueSize,
    ) -> PartialVMResult<()> {
        self.use_heap_memory(heap_size)?;

        self.base.charge_copy_loc_cached(stack_size, heap_size)
    }
}

impl<G, A> AptosGasMeter for MemoryTrackedGasMeterImpl<G, A>
where
    G: AptosGasMeter + CacheValueSizes,
    A: MemoryAlgebra,
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
