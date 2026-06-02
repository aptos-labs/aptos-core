// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines the [`ExecutionContext`] trait the interpreter calls into,
//! and a minimal [`LocalExecutionContext`] impl for tests and benchmarks.

use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    storage::{ResourceProvider, NO_RESOURCE_PROVIDER},
    types::InternedTypeList,
    FunctionPtr,
};
use mono_move_gas::{GasMeter, NoOpGasMeter, SimpleGasMeter};
use mono_move_loader::LoaderResult;

/// Runtime context consulted by the interpreter during execution: gas
/// charging, cross-module function or resource resolution.
pub trait ExecutionContext {
    /// Access the gas meter.
    fn gas_meter(&mut self) -> &mut impl GasMeter;

    /// Resolve a runtime function call.
    /// May trigger lazy module loading, gas charge on a cache miss, and
    /// lowering of the function's code.
    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr>;

    /// Access the resource provider to fetch resource from storage on read-set
    /// cache miss.
    fn resource_provider(&self) -> &dyn ResourceProvider;
}

/// A [`ExecutionContext`] that supports only local execution within a
/// single executable: `load_function` always errors. Carries a real gas
/// meter so the interpreter can charge `MicroOp::Charge` costs.
///
/// Intended for tests and benches that don't exercise cross-module dispatch.
///
// TODO: migrate to a real impl and remove this.
pub struct LocalExecutionContext<'r, G: GasMeter = NoOpGasMeter> {
    gas_meter: G,
    resource_provider: &'r dyn ResourceProvider,
}

impl LocalExecutionContext<'static, NoOpGasMeter> {
    /// No gas accounting at all (`charge` is a no-op).
    pub fn unmetered() -> Self {
        Self {
            gas_meter: NoOpGasMeter,
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
            resource_provider: &NO_RESOURCE_PROVIDER,
        }
    }
}

impl<'r, G: GasMeter> LocalExecutionContext<'r, G> {
    /// Builds a context with the gas meter and resource provider.
    pub fn new(gas_meter: G, resource_provider: &'r dyn ResourceProvider) -> Self {
        Self {
            gas_meter,
            resource_provider,
        }
    }
}

impl<'r, G: GasMeter> ExecutionContext for LocalExecutionContext<'r, G> {
    fn gas_meter(&mut self) -> &mut impl GasMeter {
        &mut self.gas_meter
    }

    fn load_function(
        &mut self,
        _module_id: InternedModuleId,
        _name: InternedIdentifier,
        _ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr> {
        panic!("LocalExecutionContext: load_function not supported")
    }

    fn resource_provider(&self) -> &dyn ResourceProvider {
        self.resource_provider
    }
}
