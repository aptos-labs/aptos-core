// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines the [`ExecutionContext`] trait the interpreter calls into,
//! and a minimal [`LocalExecutionContext`] impl for tests and benchmarks.

use crate::{ExecutableId, Function};
use mono_move_alloc::{ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_gas::{GasMeter, NoOpGasMeter, SimpleGasMeter};

pub trait FunctionResolver {
    /// Resolves a cross-module function reference. Returns [`None`] if
    /// the module or function isn't loaded.
    fn resolve_function(
        &self,
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
    ) -> Option<ExecutableArenaPtr<Function>>;
}

/// Runtime context consulted by the interpreter during execution: gas
/// charging and cross-module function resolution.
pub trait ExecutionContext {
    /// Access the gas meter.
    fn gas_meter(&mut self) -> &mut impl GasMeter;

    /// Resolve a runtime function call.
    /// May trigger lazy module loading and gas charge on a cache miss.
    fn load_function(
        &mut self,
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
    ) -> anyhow::Result<ExecutableArenaPtr<Function>>;
}

/// A [`ExecutionContext`] that supports only local execution within a
/// single executable: `load_function` always errors. Carries a real gas
/// meter so the interpreter can charge `MicroOp::Charge` costs.
///
/// Intended for tests and benches that don't exercise cross-module dispatch.
///
// TODO: migrate to a real impl and remove this.
pub struct LocalExecutionContext<G: GasMeter = NoOpGasMeter> {
    gas_meter: G,
}

impl LocalExecutionContext<NoOpGasMeter> {
    /// No gas accounting at all (`charge` is a no-op).
    pub fn unmetered() -> Self {
        Self {
            gas_meter: NoOpGasMeter,
        }
    }
}

impl LocalExecutionContext<SimpleGasMeter> {
    /// [`SimpleGasMeter`] with `u64::MAX` budget.
    pub fn with_max_budget() -> Self {
        Self::with_budget(u64::MAX)
    }

    /// [`SimpleGasMeter`] with the given starting budget.
    pub fn with_budget(amount: u64) -> Self {
        Self {
            gas_meter: SimpleGasMeter::new(amount),
        }
    }
}

impl<G: GasMeter> ExecutionContext for LocalExecutionContext<G> {
    fn gas_meter(&mut self) -> &mut impl GasMeter {
        &mut self.gas_meter
    }

    fn load_function(
        &mut self,
        _executable_id: GlobalArenaPtr<ExecutableId>,
        _name: GlobalArenaPtr<str>,
    ) -> anyhow::Result<ExecutableArenaPtr<Function>> {
        anyhow::bail!("LocalExecutionContext: load_function not supported")
    }
}
