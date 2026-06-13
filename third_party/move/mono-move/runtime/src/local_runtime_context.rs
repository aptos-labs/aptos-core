// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Minimal [`ExecutionContext`] + [`DescriptorProvider`] impl for tests
//! and benchmarks that don't go through the full loader stack.

use crate::{
    error::RuntimeResult, native_context::ProductionNativeRegistry, ExecutionContext,
    LocalExecutionContext,
};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::NativeExtensions,
    types::{InternedType, InternedTypeList},
    ConstantPoolIndex, DescriptorId, DescriptorProvider, FunctionPtr, GasMeter, LayoutId,
    LayoutProvider, ObjectDescriptor, ObjectDescriptorTable, ResourceProvider, ValueLayout,
    ValueLayoutTable,
};
use mono_move_loader::LoaderResult;

/// Combines a [`LocalExecutionContext`] with an owned
/// [`ObjectDescriptorTable`]. Used by tests and benches that need vector/object
/// descriptors but don't go through the full loader stack.
///
/// Like `LocalExecutionContext`, `load_function` always errors — this is not a
/// substitute for the production `TransactionContext`.
//
// TODO: migrate to a real impl and remove this (mirrors the TODO on
// `LocalExecutionContext` in `mono_move_core`).
pub struct LocalRuntimeContext<'r> {
    inner: LocalExecutionContext<'r>,
    descriptors: ObjectDescriptorTable,
    layouts: ValueLayoutTable,
}

impl<'r> LocalRuntimeContext<'r> {
    /// General constructor: the gas meter, the resource provider, and
    /// the descriptor table. Used by tests that exercise global storage
    /// (which need a non-trivial resource provider).
    pub fn new(
        gas_meter: GasMeter,
        resource_provider: &'r dyn ResourceProvider,
        descriptors: ObjectDescriptorTable,
    ) -> Self {
        Self {
            inner: LocalExecutionContext::new(gas_meter, resource_provider),
            descriptors,
            layouts: ValueLayoutTable::new(),
        }
    }
}

impl LocalRuntimeContext<'static> {
    /// [`GasMeter`] with `u64::MAX` budget and the supplied
    /// descriptor table.
    pub fn with_max_budget(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            inner: LocalExecutionContext::with_max_budget(),
            descriptors,
            layouts: ValueLayoutTable::new(),
        }
    }

    /// [`GasMeter`] with `u64::MAX` budget and no user
    /// descriptors (just the reserved Trivial/Closure entries).
    pub fn with_max_budget_no_descriptors() -> Self {
        Self::with_max_budget(ObjectDescriptorTable::new())
    }

    /// [`GasMeter`] with a custom budget. Useful for gas-exhaustion
    /// tests. No user descriptors.
    pub fn with_budget(amount: u64) -> Self {
        Self {
            inner: LocalExecutionContext::with_budget(amount),
            descriptors: ObjectDescriptorTable::new(),
            layouts: ValueLayoutTable::new(),
        }
    }
}

impl LocalRuntimeContext<'_> {
    /// Install a populated native registry. Replaces the empty default
    /// installed by the constructors above.
    pub fn with_natives(mut self, natives: ProductionNativeRegistry) -> Self {
        self.inner = self.inner.with_natives(natives);
        self
    }

    /// Install the per-transaction native extensions on the inner context.
    pub fn with_extensions(mut self, extensions: NativeExtensions) -> Self {
        self.inner = self.inner.with_extensions(extensions);
        self
    }
}

impl ExecutionContext for LocalRuntimeContext<'_> {
    fn gas_meter(&mut self) -> &mut GasMeter {
        self.inner.gas_meter()
    }

    fn natives(&self) -> &ProductionNativeRegistry {
        self.inner.natives()
    }

    fn extensions(&self) -> &NativeExtensions {
        self.inner.extensions()
    }

    fn native_call_borrows(
        &mut self,
    ) -> (
        &ProductionNativeRegistry,
        &dyn DescriptorProvider,
        &dyn LayoutProvider,
        &dyn ResourceProvider,
        &mut GasMeter,
        &NativeExtensions,
    ) {
        let (natives, _, _, resource_provider, gas_meter, extensions) =
            self.inner.native_call_borrows();
        (
            natives,
            &self.descriptors,
            &self.layouts,
            resource_provider,
            gas_meter,
            extensions,
        )
    }

    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr> {
        self.inner.load_function(module_id, name, ty_args)
    }

    fn load_constant(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> RuntimeResult<(InternedType, &[u8])> {
        self.inner.load_constant(module_id, idx)
    }

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.inner.resource_provider()
    }
}

impl DescriptorProvider for LocalRuntimeContext<'_> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.descriptors.descriptor(id)
    }
}

impl LayoutProvider for LocalRuntimeContext<'_> {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.layouts.layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.layouts.layout_id(ty)
    }
}
