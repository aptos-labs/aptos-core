// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transaction context that wires the [`Loader`] into the interpreter's
//! cross-module dispatch path.

use crate::{read_set::ExecutableReadSet, Loader};
use anyhow::anyhow;
use mono_move_alloc::{ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{ExecutableId, ExecutionContext, Function};
use mono_move_gas::GasMeter;
use mono_move_global_context::ExecutionGuard;

/// Per-transaction execution context. Maintains per-transaction state
/// (gas meter, read-set of loaded modules) and serves the interpreter's
/// runtime queries against it.
pub struct TransactionContext<'guard, 'ctx, G: GasMeter> {
    guard: &'guard ExecutionGuard<'ctx>,
    loader: Loader<'guard, 'ctx>,
    read_set: ExecutableReadSet<'guard>,
    gas_meter: G,
}

impl<'guard, 'ctx, G: GasMeter> TransactionContext<'guard, 'ctx, G> {
    pub fn new(
        guard: &'guard ExecutionGuard<'ctx>,
        loader: Loader<'guard, 'ctx>,
        gas_meter: G,
    ) -> Self {
        Self {
            guard,
            loader,
            read_set: ExecutableReadSet::new(),
            gas_meter,
        }
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ExecutableReadSet<'guard> {
        &self.read_set
    }
}

impl<'guard, 'ctx, G: GasMeter> ExecutionContext for TransactionContext<'guard, 'ctx, G> {
    fn gas_meter(&mut self) -> &mut impl GasMeter {
        &mut self.gas_meter
    }

    /// Looks up cross-module targets in the read-set, falling back to the [`Loader`] on cache miss.
    fn load_function(
        &mut self,
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
    ) -> anyhow::Result<ExecutableArenaPtr<Function>> {
        let id = self.guard.arena_ref_for_executable_id(executable_id);

        let loaded = match self.read_set.get(id) {
            Some(loaded) => loaded,
            None => self
                .loader
                .load(&mut self.read_set, &mut self.gas_meter, id)?,
        };

        loaded.executable().get_function(name).ok_or_else(|| {
            // SAFETY: `name` is an interned arena pointer, valid for the duration
            // of the execution phase.
            let name = unsafe { name.as_ref_unchecked() };
            anyhow!(
                "load_function: function `{}` not found in loaded module",
                name
            )
        })
    }
}
