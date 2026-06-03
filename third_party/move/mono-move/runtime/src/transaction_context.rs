// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction context that wires the [`Loader`] into the interpreter's
//! cross-module dispatch path.
//
// TODO: move out of the runtime once a layer above it exists.

use crate::ExecutionContext;
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::ProductionNativeRegistry,
    types::{InternedType, InternedTypeList},
    DescriptorId, DescriptorProvider, FunctionPtr, LayoutId, LayoutProvider, ObjectDescriptor,
    ResourceProvider, TypeLayout,
};
use mono_move_gas::GasMeter;
use mono_move_loader::{Loader, LoaderResult, ModuleReadSet};

/// Per-transaction execution context. Maintains per-transaction state
/// (gas meter, read-set of loaded modules) and serves the interpreter's
/// runtime queries against it.
pub struct TransactionContext<'guard, 'ctx, G: GasMeter> {
    loader: Loader<'guard, 'ctx>,
    read_set: ModuleReadSet<'guard>,
    gas_meter: G,
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
    natives: &'guard ProductionNativeRegistry<G>,
}

impl<'guard, 'ctx, G: GasMeter> TransactionContext<'guard, 'ctx, G> {
    pub fn new(
        loader: Loader<'guard, 'ctx>,
        gas_meter: G,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry<G>,
    ) -> Self {
        Self {
            loader,
            read_set: ModuleReadSet::new(),
            gas_meter,
            resource_provider,
            natives,
        }
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }
}

impl<'guard, 'ctx, G: GasMeter> ExecutionContext for TransactionContext<'guard, 'ctx, G> {
    type GasMeter = G;

    fn gas_meter(&mut self) -> &mut G {
        &mut self.gas_meter
    }

    fn natives(&self) -> &ProductionNativeRegistry<G> {
        self.natives
    }

    fn natives_and_gas_meter(&mut self) -> (&ProductionNativeRegistry<G>, &mut G) {
        (self.natives, &mut self.gas_meter)
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

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.resource_provider
    }
}

impl<'guard, 'ctx, G: GasMeter> DescriptorProvider for TransactionContext<'guard, 'ctx, G> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.loader.guard().descriptor(id)
    }
}

impl<'guard, 'ctx, G: GasMeter> LayoutProvider for TransactionContext<'guard, 'ctx, G> {
    fn layout(&self, id: LayoutId) -> Option<&TypeLayout> {
        self.loader.guard().layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.loader.guard().layout_id(ty)
    }
}
