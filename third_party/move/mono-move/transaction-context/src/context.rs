// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_core::ExecutableId;
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::{ArenaRef, Executable};
use mono_move_linker::Linker;
use mono_move_loader::{
    read_set::{ExecutableRead, ExecutableReadSet},
    LoadedExecutable, Loader,
};

/// Per-transaction state implementation.
#[allow(dead_code)]
pub struct TransactionLocalContext<'guard, 'ctx> {
    /// Loads executables from global storage into cache and local context's read-set.
    loader: Loader<'guard, 'ctx>,
    /// Links loaded executables.
    linker: Linker,

    /// Charges gas for this transaction.
    gas_meter: SimpleGasMeter,

    /// Tracks every executable this transaction reads. Used for:
    ///   - Block-STM read-set for validation. On code upgrade, this read-set
    ///     checked and transaction is re-executed if there is a conflict.
    ///   - Executable local cache to avoid lookups in concurrent global cache.
    ///     Ensures there is a consistent view for every executable.
    ///   - Set of executables used for gas metering and already accounted for.
    executable_read_set: ExecutableReadSet<'guard>,
}

impl<'guard, 'ctx> TransactionLocalContext<'guard, 'ctx> {
    /// Creates a new empty transaction context.
    #[allow(dead_code)]
    pub fn new(loader: Loader<'guard, 'ctx>) -> Self {
        Self {
            loader,
            linker: Linker::new(),
            gas_meter: SimpleGasMeter::new(u64::MAX),
            executable_read_set: ExecutableReadSet::default(),
        }
    }
}

impl<'guard, 'ctx> TransactionLocalContext<'guard, 'ctx> {
    #[allow(dead_code)]
    fn resolve_executable(
        &mut self,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<&'guard Executable> {
        if let Some(ExecutableRead::Visited(executable)) = self.executable_read_set.get(id) {
            return Ok(executable);
        }

        match self
            .loader
            .load(&mut self.executable_read_set, &mut self.gas_meter, id)?
        {
            LoadedExecutable::CacheHit { executable } => Ok(executable),
            LoadedExecutable::CacheMiss { .. } => {
                // TODO:
                //   1. Convert compiled modules to IR.
                //   2. Create executables.
                //   3. linker.link();
                //   4. for executable in executables, insert into cache
                //   5. return executables[idx];
                unimplemented!("Cache miss processing is not yet implemented");
            },
        }
    }
}
