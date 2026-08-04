// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction set of modules recorded by the loader, pinning one
//! version per module for the duration of the transaction.

use crate::invariant_violation;
use mono_move_core::{ModuleId, VMResult};
use mono_move_global_context::{ArenaRef, LoadedModule};
use shared_dsa::UnorderedMap;

/// Represents different states of a loaded module in a read-set. Allowed
/// state transitions:
///   1. [`ModuleState::Unmetered`] can become [`ModuleState::Metered`] if gas has been
///      charged for the module.
///   2. [`ModuleState::Metered`] can become [`ModuleState::ReadyForLowering`] if
///      the module became ready for lowering.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ModuleState {
    /// This module has been loaded, charged gas and its mandatory dependency
    /// set is known. Function IR can be lowered.
    ReadyForLowering,
    /// This module has been loaded and is about to be charged gas. Lowering of
    /// functions is not yet possible because module's mandatory dependency set
    /// has not yet been computed.
    ///
    /// Note that this state is recorded before gas is charged to make sure
    /// that running out-of-gas still has a read in the read-set.
    Metered,
    /// This module has not been metered yet and is only used for caching to
    /// ensure that transactions always sees the same module version during
    /// execution.
    Unmetered,
}

/// Tracks how this read depends on a particular loaded module.
#[derive(Copy, Clone)]
pub enum ModuleRead<'guard> {
    /// The module is about to be loaded. Used to ensure reads for modules
    /// that fail to load are still present in the read-set.
    Pending,
    /// The module is loaded for this transaction. The [`ModuleState`] of the
    /// module may be updated.
    Loaded {
        module: &'guard LoadedModule,
        state: ModuleState,
    },
}

impl<'guard> ModuleRead<'guard> {
    /// Returns the loaded module. For test only.
    pub fn loaded_module_for_test(&self) -> &'guard LoadedModule {
        match self {
            ModuleRead::Pending => unreachable!(),
            ModuleRead::Loaded { module, .. } => module,
        }
    }

    /// Returns the deterministic load cost of the loaded module. For test only.
    pub fn cost_for_test(&self) -> u64 {
        self.loaded_module_for_test().cost()
    }
}

/// Maps from module ID to the version the transaction is using for the
/// duration of this transaction.
#[derive(Default)]
pub struct ModuleReadSet<'guard> {
    inner: UnorderedMap<ArenaRef<'guard, ModuleId>, ModuleRead<'guard>>,
}

impl<'guard> ModuleReadSet<'guard> {
    /// Creates an empty read-set.
    pub fn new() -> Self {
        Self {
            inner: UnorderedMap::new(),
        }
    }

    /// Returns the recorded read, or [`None`] if absent.
    pub fn get(&self, id: ArenaRef<'guard, ModuleId>) -> Option<ModuleRead<'guard>> {
        self.inner.get(&id).copied()
    }

    /// Returns the loaded module for `id`. Callers that resolve a module they
    /// are already executing from rely on it being present and loaded, so a
    /// missing or still-pending entry is an invariant violation.
    pub fn get_loaded(&self, id: ArenaRef<'guard, ModuleId>) -> VMResult<&'guard LoadedModule> {
        match self.get(id) {
            Some(ModuleRead::Loaded { module, .. }) => Ok(module),
            Some(ModuleRead::Pending) => invariant_violation!(ReadSetEntryNotLoaded),
            None => invariant_violation!(UnexpectedReadSetMiss),
        }
    }

    /// Records a module that is about to be loaded. Used so a load that fails
    /// due to deserialization / verification still leaves the read in the set.
    pub fn record_pending_loading(&mut self, id: ArenaRef<'guard, ModuleId>) -> VMResult<()> {
        if self.inner.insert(id, ModuleRead::Pending).is_some() {
            invariant_violation!(EntryAlreadyExists);
        }
        Ok(())
    }

    /// Records loaded module in the read-set as unmetered.
    pub fn record_unmetered(
        &mut self,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> VMResult<()> {
        let read = ModuleRead::Loaded {
            module,
            state: ModuleState::Unmetered,
        };
        let prev = self.inner.insert(id, read);
        match prev {
            Some(ModuleRead::Pending) => Ok(()),
            Some(ModuleRead::Loaded { .. }) | None => invariant_violation!(ModuleExpectedPending),
        }
    }

    /// Records loaded module in the read-set as metered.
    pub fn record_metered(
        &mut self,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> VMResult<()> {
        let read = ModuleRead::Loaded {
            module,
            state: ModuleState::Metered,
        };
        let prev = self.inner.insert(id, read);
        match prev {
            Some(ModuleRead::Pending) => Ok(()),
            Some(ModuleRead::Loaded { .. }) | None => invariant_violation!(ModuleExpectedPending),
        }
    }

    /// Records that existing loaded module has been metered and its functions
    /// are ready for lowering (i.e., its mandatory dependency is known).
    pub fn record_ready_for_lowering(
        &mut self,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> VMResult<()> {
        match self.inner.get(&id) {
            Some(ModuleRead::Pending) => {
                self.inner.insert(id, ModuleRead::Loaded {
                    module,
                    state: ModuleState::ReadyForLowering,
                });
                Ok(())
            },
            None => invariant_violation!(ModuleExpectedPending),
            Some(ModuleRead::Loaded { .. }) => invariant_violation!(ModuleAlreadyLoaded),
        }
    }

    /// Transitions an existing loaded module from unmetered to metered state.
    pub fn mark_metered(&mut self, id: ArenaRef<'guard, ModuleId>) -> VMResult<()> {
        match self.inner.get_mut(&id) {
            Some(ModuleRead::Loaded { state, .. }) => match state {
                ModuleState::Unmetered => {
                    *state = ModuleState::Metered;
                    Ok(())
                },
                ModuleState::Metered | ModuleState::ReadyForLowering => {
                    invariant_violation!(ModuleAlreadyMetered);
                },
            },
            Some(ModuleRead::Pending) | None => invariant_violation!(ModuleExpectedLoaded),
        }
    }

    /// Records that existing loaded module has satisfied the lowering
    /// requirements (i.e., its mandatory dependency set has been computed).
    pub fn mark_ready_for_lowering(&mut self, id: ArenaRef<'guard, ModuleId>) -> VMResult<()> {
        match self.inner.get_mut(&id) {
            Some(ModuleRead::Loaded { state, .. }) => match state {
                ModuleState::Unmetered => invariant_violation!(ModuleExpectedMetered),
                ModuleState::ReadyForLowering => invariant_violation!(ModuleAlreadyReady),
                ModuleState::Metered => {
                    *state = ModuleState::ReadyForLowering;
                    Ok(())
                },
            },
            Some(ModuleRead::Pending) | None => invariant_violation!(ModuleExpectedAtLeastLoaded),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the read-set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
