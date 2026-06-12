// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction context that wires the [`Loader`] into the interpreter's
//! cross-module dispatch path.
//
// TODO: move out of the runtime once a layer above it exists.

use crate::{error::RuntimeResult, native_context::ProductionNativeRegistry, ExecutionContext};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::NativeExtensions,
    types::{InternedType, InternedTypeList},
    ConstantPoolIndex, DescriptorId, DescriptorProvider, FunctionPtr, GasMeter, LayoutId,
    LayoutProvider, ObjectDescriptor, ResourceProvider, ValueLayout,
};
use mono_move_loader::{Loader, LoaderResult, ModuleReadSet};

/// Per-transaction execution context. Maintains per-transaction state
/// (gas meter, read-set of loaded modules) and serves the interpreter's
/// runtime queries against it.
pub struct TransactionContext<'guard, 'ctx> {
    loader: Loader<'guard, 'ctx>,
    read_set: ModuleReadSet<'guard>,
    gas_meter: GasMeter,
    // TODO(refactor):
    //   We need to move resource read-set here, as well as heap and some
    //   other fields from interpreter context which should live longer than
    //   a single interpreter session.
    resource_provider: &'guard dyn ResourceProvider,
    // TODO: Move the native registry off the per-transaction context and
    // onto a long-lived owner (e.g. the global context).
    //
    // TODO: Enforce that `natives` here and the `NativeResolver` passed
    // to `loader` are the same instance.
    natives: &'guard ProductionNativeRegistry,
    /// Per-transaction native extensions, shared across native calls.
    extensions: NativeExtensions,
}

impl<'guard, 'ctx> TransactionContext<'guard, 'ctx> {
    pub fn new(
        loader: Loader<'guard, 'ctx>,
        gas_meter: GasMeter,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry,
    ) -> Self {
        Self {
            loader,
            read_set: ModuleReadSet::new(),
            gas_meter,
            resource_provider,
            natives,
            extensions: NativeExtensions::new(),
        }
    }

    /// Install the per-transaction native extensions. Replaces any previously
    /// installed set.
    pub fn with_extensions(mut self, extensions: NativeExtensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }
}

impl ExecutionContext for TransactionContext<'_, '_> {
    fn gas_meter(&mut self) -> &mut GasMeter {
        &mut self.gas_meter
    }

    fn natives(&self) -> &ProductionNativeRegistry {
        self.natives
    }

    fn extensions(&self) -> &NativeExtensions {
        &self.extensions
    }

    fn native_call_borrows(
        &mut self,
    ) -> (
        &ProductionNativeRegistry,
        &dyn DescriptorProvider,
        &mut GasMeter,
        &NativeExtensions,
    ) {
        (
            self.natives,
            self.loader.guard(),
            &mut self.gas_meter,
            &self.extensions,
        )
    }

    /// Looks up cross-module targets in the read-set, falling back to
    /// the [`Loader`] on cache miss.
    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr> {
        self.loader.load_function(
            &mut self.read_set,
            &mut self.gas_meter,
            module_id,
            name,
            ty_args,
        )
    }

    fn load_constant(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> RuntimeResult<(InternedType, &[u8])> {
        let arena_ref = self.loader.guard().arena_ref_for_module_id(module_id);
        let module = &self.read_set.get_loaded(arena_ref)?.ir().module;
        Ok((
            module.interned_constant_type_at(idx),
            module.constant_data_at(idx),
        ))
    }

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.resource_provider
    }
}

impl DescriptorProvider for TransactionContext<'_, '_> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.loader.guard().descriptor(id)
    }
}

impl LayoutProvider for TransactionContext<'_, '_> {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.loader.guard().layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.loader.guard().layout_id(ty)
    }
}
