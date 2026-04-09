// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction module loader.

use crate::{
    hooks::LoaderHooks,
    read_set::{ExecutableRead, ExecutableReadSet},
};
use mono_move_core::ExecutableId;
use mono_move_gas::GasMeter;
use mono_move_global_context::{ArenaRef, Executable, ExecutionGuard};
use move_binary_format::CompiledModule;

/// Describes how modules are loaded from storage.
pub enum LoadingPolicy {
    /// Loads every module lazily.
    Lazy,
}

/// Result of module loading.
pub enum LoadedExecutable<'a> {
    /// This module's executable has already been cached, returned directly.
    CacheHit { executable: &'a Executable },
    /// This module's executable not yet exists. Returned together with other
    /// cache misses in the cache for subsequent lowering and linking.
    CacheMiss {
        // Index into cache misses vector corresponding to the module being
        // loaded.
        idx: usize,
        cache_misses: Vec<CompiledModule>,
    },
}

/// Per-transaction module loader.
///
/// Used within a single transaction to load modules and record them in per-
/// transaction state. Note that the only job of the loader is load and charge
/// gas for the set of modules needed for later lowering to micro-ops,
/// monomorphization or optimized linking.
pub struct Loader<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    hooks: &'guard dyn LoaderHooks,
    policy: LoadingPolicy,
}

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Creates a new loader with hooks.
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, hooks: &'guard dyn LoaderHooks) -> Self {
        Self {
            guard,
            hooks,
            policy: LoadingPolicy::Lazy,
        }
    }

    /// Loads the specified executable. If cache hit, adds it to transaction
    /// context and returns the read executable. If cache miss, uses hooks
    /// to obtain the corresponding verified [`CompiledModule`]. Returns any
    /// other modules that were not cached but visited during loading phase.
    ///
    /// # Precondition
    ///
    /// The executable has not been ([`ExecutableRead::Visited`]) and does not
    /// exist in the read-set.
    pub fn load(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<LoadedExecutable<'guard>> {
        match self.policy {
            LoadingPolicy::Lazy => self.load_lazy(read_set, gas_meter, id),
        }
    }
}

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Implementation of lazy loading policy. Loads only the current module
    /// and nothing else. Charges gas for that module.
    ///
    /// # Precondition
    ///
    /// The executable has not been ([`ExecutableRead::Visited`]) and does not
    /// exist in the read-set.
    fn load_lazy(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<LoadedExecutable<'guard>> {
        Ok(match self.guard.get_executable(id) {
            None => {
                let bytes = self
                    .hooks
                    .get_module_bytes(id.address(), id.name())?
                    .ok_or_else(|| anyhow::anyhow!("Linker error"))?;

                let charge = bytes.len() as u64;
                gas_meter.charge(charge)?;
                read_set.insert(id, ExecutableRead::Charged(charge));

                let module = self.hooks.deserialize_module(&bytes)?;
                self.hooks.verify_module(&module)?;
                LoadedExecutable::CacheMiss {
                    idx: 0,
                    cache_misses: vec![module],
                }
            },
            Some(executable) => {
                gas_meter.charge(executable.cost())?;
                read_set.insert(id, ExecutableRead::Visited(executable));
                LoadedExecutable::CacheHit { executable }
            },
        })
    }
}
