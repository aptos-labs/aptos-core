// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Minimal [`ExecutionContext`] + [`DescriptorProvider`] impl for tests
//! and benchmarks that don't go through the full loader stack.

use crate::{ExecutionContext, LocalExecutionContext};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::ProductionNativeRegistry,
    types::{InternedType, InternedTypeList},
    ConstantPoolIndex, DescriptorId, DescriptorProvider, FunctionPtr, LayoutId, LayoutProvider,
    ObjectDescriptor, ObjectDescriptorTable, ResourceProvider, ValueLayout, ValueLayoutTable,
};
use mono_move_gas::{GasMeter, NoOpGasMeter, SimpleGasMeter};
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
pub struct LocalRuntimeContext<'r, G: GasMeter = NoOpGasMeter> {
    inner: LocalExecutionContext<'r, G>,
    descriptors: ObjectDescriptorTable,
    layouts: ValueLayoutTable,
}

impl LocalRuntimeContext<'static, NoOpGasMeter> {
    /// No gas accounting, no descriptors. Suitable for tests that exercise only
    /// descriptor-less micro-ops.
    pub fn unmetered() -> Self {
        Self {
            inner: LocalExecutionContext::unmetered(),
            descriptors: ObjectDescriptorTable::new(),
            layouts: ValueLayoutTable::new(),
        }
    }

    /// No gas accounting; uses the supplied descriptor table.
    pub fn unmetered_with_descriptors(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            inner: LocalExecutionContext::unmetered(),
            descriptors,
            layouts: ValueLayoutTable::new(),
        }
    }
}

impl<'r, G: GasMeter> LocalRuntimeContext<'r, G> {
    /// General constructor: the gas meter, the resource provider, and
    /// the descriptor table. Used by tests that exercise global storage
    /// (which need a non-trivial resource provider).
    pub fn new(
        gas_meter: G,
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

impl LocalRuntimeContext<'static, SimpleGasMeter> {
    /// [`SimpleGasMeter`] with `u64::MAX` budget and the supplied
    /// descriptor table.
    pub fn with_max_budget(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            inner: LocalExecutionContext::with_max_budget(),
            descriptors,
            layouts: ValueLayoutTable::new(),
        }
    }

    /// [`SimpleGasMeter`] with `u64::MAX` budget and no user
    /// descriptors (just the reserved Trivial/Closure entries).
    pub fn with_max_budget_no_descriptors() -> Self {
        Self::with_max_budget(ObjectDescriptorTable::new())
    }

    /// [`SimpleGasMeter`] with a custom budget. Useful for gas-exhaustion
    /// tests. No user descriptors.
    pub fn with_budget(amount: u64) -> Self {
        Self {
            inner: LocalExecutionContext::with_budget(amount),
            descriptors: ObjectDescriptorTable::new(),
            layouts: ValueLayoutTable::new(),
        }
    }
}

impl<'r, G: GasMeter> LocalRuntimeContext<'r, G> {
    /// Install a populated native registry. Replaces the empty default
    /// installed by the constructors above.
    pub fn with_natives(mut self, natives: ProductionNativeRegistry<G>) -> Self {
        self.inner = self.inner.with_natives(natives);
        self
    }
}

impl<'r, G: GasMeter> ExecutionContext for LocalRuntimeContext<'r, G> {
    type GasMeter = G;

    fn gas_meter(&mut self) -> &mut G {
        self.inner.gas_meter()
    }

    fn natives(&self) -> &ProductionNativeRegistry<G> {
        self.inner.natives()
    }

    fn natives_and_gas_meter(&mut self) -> (&ProductionNativeRegistry<G>, &mut G) {
        self.inner.natives_and_gas_meter()
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
    ) -> Option<(InternedType, &[u8])> {
        self.inner.load_constant(module_id, idx)
    }

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.inner.resource_provider()
    }
}

impl<'r, G: GasMeter> DescriptorProvider for LocalRuntimeContext<'r, G> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.descriptors.descriptor(id)
    }
}

impl<'r, G: GasMeter> LayoutProvider for LocalRuntimeContext<'r, G> {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.layouts.layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.layouts.layout_id(ty)
    }
}
