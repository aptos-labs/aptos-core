// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines the [`ExecutionContext`] trait the interpreter calls into,
//! and a minimal [`LocalExecutionContext`] impl for tests and benchmarks.

use crate::error::RuntimeResult;
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::{NativeRegistry, ProductionNativeRegistry},
    storage::{ResourceProvider, NO_RESOURCE_PROVIDER},
    types::{InternedType, InternedTypeList},
    ConstantPoolIndex, FunctionPtr,
};
use mono_move_gas::{GasMeter, NoOpGasMeter, SimpleGasMeter};
use mono_move_loader::LoaderResult;

/// Runtime context consulted by the interpreter during execution: gas
/// charging, cross-module function or resource resolution.
pub trait ExecutionContext {
    /// Concrete gas meter type for this execution context.
    type GasMeter: GasMeter;

    /// Access the gas meter.
    fn gas_meter(&mut self) -> &mut Self::GasMeter;

    /// Read-only access to the native function registry.
    fn natives(&self) -> &ProductionNativeRegistry<Self::GasMeter>;

    /// Disjoint borrow of the native registry and the gas meter. The
    /// interpreter needs both simultaneously at times.
    fn natives_and_gas_meter(
        &mut self,
    ) -> (
        &ProductionNativeRegistry<Self::GasMeter>,
        &mut Self::GasMeter,
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
pub struct LocalExecutionContext<'r, G: GasMeter = NoOpGasMeter> {
    gas_meter: G,
    natives: ProductionNativeRegistry<G>,
    resource_provider: &'r dyn ResourceProvider,
}

impl LocalExecutionContext<'static, NoOpGasMeter> {
    /// No gas accounting at all (`charge` is a no-op).
    pub fn unmetered() -> Self {
        Self {
            gas_meter: NoOpGasMeter,
            natives: NativeRegistry::new(),
            resource_provider: &NO_RESOURCE_PROVIDER,
        }
    }
}

impl LocalExecutionContext<'static, SimpleGasMeter> {
    /// [`SimpleGasMeter`] with `u64::MAX` budget.
    pub fn with_max_budget() -> Self {
        Self::with_budget(u64::MAX)
    }

    /// [`SimpleGasMeter`] with the given starting budget.
    pub fn with_budget(amount: u64) -> Self {
        Self {
            gas_meter: SimpleGasMeter::new(amount),
            natives: NativeRegistry::new(),
            resource_provider: &NO_RESOURCE_PROVIDER,
        }
    }
}

impl<'r, G: GasMeter> LocalExecutionContext<'r, G> {
    /// Builds a context with the gas meter and resource provider.
    pub fn new(gas_meter: G, resource_provider: &'r dyn ResourceProvider) -> Self {
        Self {
            gas_meter,
            natives: NativeRegistry::new(),
            resource_provider,
        }
    }

    /// Install a populated native registry on this context. Replaces any
    /// previously-installed registry.
    pub fn with_natives(mut self, natives: ProductionNativeRegistry<G>) -> Self {
        self.natives = natives;
        self
    }
}

impl<'r, G: GasMeter> ExecutionContext for LocalExecutionContext<'r, G> {
    type GasMeter = G;

    fn gas_meter(&mut self) -> &mut G {
        &mut self.gas_meter
    }

    fn natives(&self) -> &ProductionNativeRegistry<G> {
        &self.natives
    }

    fn natives_and_gas_meter(&mut self) -> (&ProductionNativeRegistry<G>, &mut G) {
        (&self.natives, &mut self.gas_meter)
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
