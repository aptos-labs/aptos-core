// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines the [`ExecutionContext`] trait the interpreter calls into,
//! and a minimal [`LocalExecutionContext`] impl for tests and benchmarks.

use crate::{error::RuntimeResult, native_context::ProductionNativeRegistry};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::{NativeExtensions, NativeRegistry},
    storage::{ResourceProvider, NO_RESOURCE_PROVIDER},
    types::{InternedType, InternedTypeList},
    ConstantPoolIndex, DescriptorProvider, FunctionPtr, GasMeter, LayoutProvider,
    NO_DESCRIPTOR_PROVIDER, NO_LAYOUT_PROVIDER,
};
use mono_move_loader::LoaderResult;

/// Runtime context consulted by the interpreter during execution: gas
/// charging, cross-module function or resource resolution.
pub trait ExecutionContext {
    /// Access the gas meter.
    fn gas_meter(&mut self) -> &mut GasMeter;

    /// Read-only access to the native function registry.
    fn natives(&self) -> &ProductionNativeRegistry;

    /// The per-transaction native extensions.
    fn extensions(&self) -> &NativeExtensions;

    /// Disjoint borrows all the sub-components needed for a native call.
    /// Needed for avoiding borrow conflicts downstream.
    fn native_call_borrows(
        &mut self,
    ) -> (
        &ProductionNativeRegistry,
        &dyn DescriptorProvider,
        &dyn LayoutProvider,
        &mut GasMeter,
        &NativeExtensions,
    );

    /// Resolve a runtime function call.
    /// May trigger lazy module loading, gas charge on a cache miss, and
    /// lowering of the function's code.
    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr>;

    /// Resolve a constant from `module_id`'s constant pool, returning its
    /// interned type and BCS bytes. The calling function was loaded from
    /// `module_id`, so the module is always present and loaded in the read
    /// set; a missing or not-yet-loaded entry is an invariant violation.
    fn load_constant(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> RuntimeResult<(InternedType, &[u8])>;

    /// Access the resource provider to fetch resource from storage on read-set
    /// cache miss.
    fn resource_provider(&self) -> &dyn ResourceProvider;
}

/// A [`ExecutionContext`] that supports only local execution within a
/// single executable: `load_function` always errors. Carries a real gas
/// meter so the interpreter can charge per-block costs.
///
/// Intended for tests and benches that don't exercise cross-module dispatch.
///
// TODO: migrate to a real impl and remove this.
pub struct LocalExecutionContext<'r> {
    gas_meter: GasMeter,
    natives: ProductionNativeRegistry,
    resource_provider: &'r dyn ResourceProvider,
    extensions: NativeExtensions,
}

impl LocalExecutionContext<'static> {
    /// [`GasMeter`] with `u64::MAX` budget.
    pub fn with_max_budget() -> Self {
        Self::with_budget(u64::MAX)
    }

    /// [`GasMeter`] with the given starting budget.
    pub fn with_budget(amount: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(amount),
            natives: NativeRegistry::new(),
            extensions: NativeExtensions::new(),
            resource_provider: &NO_RESOURCE_PROVIDER,
        }
    }
}

impl<'r> LocalExecutionContext<'r> {
    /// Builds a context with the gas meter and resource provider.
    pub fn new(gas_meter: GasMeter, resource_provider: &'r dyn ResourceProvider) -> Self {
        Self {
            gas_meter,
            natives: NativeRegistry::new(),
            extensions: NativeExtensions::new(),
            resource_provider,
        }
    }

    /// Install a populated native registry on this context. Replaces any
    /// previously-installed registry.
    pub fn with_natives(mut self, natives: ProductionNativeRegistry) -> Self {
        self.natives = natives;
        self
    }

    /// Install the per-transaction native extensions. Replaces any previously
    /// installed set.
    pub fn with_extensions(mut self, extensions: NativeExtensions) -> Self {
        self.extensions = extensions;
        self
    }
}

impl ExecutionContext for LocalExecutionContext<'_> {
    fn gas_meter(&mut self) -> &mut GasMeter {
        &mut self.gas_meter
    }

    fn natives(&self) -> &ProductionNativeRegistry {
        &self.natives
    }

    fn extensions(&self) -> &NativeExtensions {
        &self.extensions
    }

    fn native_call_borrows(
        &mut self,
    ) -> (
        &ProductionNativeRegistry,
        &dyn DescriptorProvider,
        &dyn LayoutProvider,
        &mut GasMeter,
        &NativeExtensions,
    ) {
        (
            &self.natives,
            &NO_DESCRIPTOR_PROVIDER,
            &NO_LAYOUT_PROVIDER,
            &mut self.gas_meter,
            &self.extensions,
        )
    }

    fn load_function(
        &mut self,
        _module_id: InternedModuleId,
        _name: InternedIdentifier,
        _ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr> {
        panic!("LocalExecutionContext: load_function not supported")
    }

    fn load_constant(
        &self,
        _module_id: InternedModuleId,
        _idx: ConstantPoolIndex,
    ) -> RuntimeResult<(InternedType, &[u8])> {
        panic!("LocalExecutionContext: load_constant not supported")
    }

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.resource_provider
    }
}
