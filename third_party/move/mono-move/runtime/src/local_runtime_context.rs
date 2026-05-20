// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Minimal [`ExecutionContext`] + [`DescriptorProvider`] impl for tests
//! and benchmarks that don't go through the full loader stack.

use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    types::InternedTypeList,
    DescriptorId, DescriptorProvider, ExecutionContext, FunctionPtr, LocalExecutionContext,
    ObjectDescriptor, ObjectDescriptorTable,
};
use mono_move_gas::{GasMeter, NoOpGasMeter, SimpleGasMeter};

/// Combines a [`LocalExecutionContext`] with an owned
/// [`ObjectDescriptorTable`]. Used by tests and benches that need vector/object
/// descriptors but don't go through the full loader stack.
///
/// Like `LocalExecutionContext`, `load_function` always errors — this is not a
/// substitute for the production `TransactionContext`.
//
// TODO: migrate to a real impl and remove this (mirrors the TODO on
// `LocalExecutionContext` in `mono_move_core`).
pub struct LocalRuntimeContext<G: GasMeter = NoOpGasMeter> {
    inner: LocalExecutionContext<G>,
    descriptors: ObjectDescriptorTable,
}

impl LocalRuntimeContext<NoOpGasMeter> {
    /// No gas accounting, no descriptors. Suitable for tests that exercise only
    /// descriptor-less micro-ops.
    pub fn unmetered() -> Self {
        Self {
            inner: LocalExecutionContext::unmetered(),
            descriptors: ObjectDescriptorTable::new(),
        }
    }

    /// No gas accounting; uses the supplied descriptor table.
    pub fn unmetered_with_descriptors(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            inner: LocalExecutionContext::unmetered(),
            descriptors,
        }
    }
}

impl LocalRuntimeContext<SimpleGasMeter> {
    /// [`SimpleGasMeter`] with `u64::MAX` budget and the supplied
    /// descriptor table.
    pub fn with_max_budget(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            inner: LocalExecutionContext::with_max_budget(),
            descriptors,
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
        }
    }
}

impl<G: GasMeter> ExecutionContext for LocalRuntimeContext<G> {
    fn gas_meter(&mut self) -> &mut impl GasMeter {
        self.inner.gas_meter()
    }

    fn load_function(
        &mut self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> anyhow::Result<FunctionPtr> {
        self.inner.load_function(module_id, name, ty_args)
    }
}

impl<G: GasMeter> DescriptorProvider for LocalRuntimeContext<G> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.descriptors.descriptor(id)
    }
}
