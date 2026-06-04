// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Concrete per-transaction execution context. Wires the [`Loader`] into the
//! interpreter's cross-module dispatch path and owns the long-lived
//! per-transaction state: gas meter, read-set of loaded modules, GC-managed
//! heap, global-storage read/write set, and RNG. A single interpreter session
//! borrows it; the state outlives any one session.
//
// TODO: move out of the runtime once a layer above it exists.

use crate::{
    error::RuntimeResult,
    global_storage::ResourceReadWriteSet,
    heap::{pinned_roots::PinnedRoots, Heap},
    types::DEFAULT_HEAP_SIZE,
};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::ProductionNativeRegistry,
    types::InternedTypeList,
    DescriptorId, DescriptorProvider, FunctionPtr, ObjectDescriptor, ResourceProvider,
};
use mono_move_gas::GasMeter;
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::{Loader, LoaderResult, ModuleReadSet};
use rand::{rngs::StdRng, SeedableRng};

/// Per-transaction execution context. Owns per-transaction state (gas meter,
/// module read-set, heap, global-storage working set, RNG) and serves the
/// interpreter's runtime queries against it.
pub struct ExecutionContext<'guard, 'ctx, G: GasMeter> {
    loader: Loader<'guard, 'ctx>,
    read_set: ModuleReadSet<'guard>,
    gas_meter: G,
    resource_provider: &'guard dyn ResourceProvider,
    // TODO: Move the native registry off the per-transaction context and
    // onto a long-lived owner (e.g. the global context).
    //
    // TODO: Enforce that `natives` here and the `NativeResolver` passed
    // to `loader` are the same instance.
    natives: &'guard ProductionNativeRegistry<G>,

    /// GC-managed heap for objects allocated during execution.
    pub(crate) heap: Heap,
    /// Auxiliary GC root set for temporarily-live heap pointers that are not
    /// yet stored in any frame slot (e.g. between two allocations in a fused
    /// micro-op, or in native functions).
    pub(crate) pinned_roots: PinnedRoots,
    /// Per-transaction global-storage state: working map of cached reads /
    /// pending writes, linear journal for rollback, and checkpoint stack.
    pub(crate) read_write_set: ResourceReadWriteSet,
    pub(crate) rng: StdRng,
}

impl<'guard, 'ctx, G: GasMeter> ExecutionContext<'guard, 'ctx, G> {
    pub fn new(
        loader: Loader<'guard, 'ctx>,
        gas_meter: G,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry<G>,
    ) -> Self {
        Self::with_heap_size(
            loader,
            gas_meter,
            resource_provider,
            natives,
            DEFAULT_HEAP_SIZE,
        )
    }

    /// Builds a context with a custom heap size (for testing GC pressure).
    pub fn with_heap_size(
        loader: Loader<'guard, 'ctx>,
        gas_meter: G,
        resource_provider: &'guard dyn ResourceProvider,
        natives: &'guard ProductionNativeRegistry<G>,
        heap_size: usize,
    ) -> Self {
        Self {
            loader,
            read_set: ModuleReadSet::new(),
            gas_meter,
            resource_provider,
            natives,
            heap: Heap::new(heap_size),
            pinned_roots: PinnedRoots::new(),
            read_write_set: ResourceReadWriteSet::new(),
            rng: StdRng::seed_from_u64(0),
        }
    }

    /// Returns the transaction's read-set.
    pub fn read_set(&self) -> &ModuleReadSet<'guard> {
        &self.read_set
    }

    /// The execution guard backing the loader. Serves descriptor lookups and
    /// keeps the global executable cache alive. Returns a `'guard`-lifetime
    /// reference, decoupled from `&self`, so callers can hold it while
    /// mutably borrowing the heap or read-write set.
    pub fn guard(&self) -> &'guard ExecutionGuard<'ctx> {
        self.loader.guard()
    }

    /// Access the gas meter.
    pub fn gas_meter(&mut self) -> &mut G {
        &mut self.gas_meter
    }

    /// Read-only access to the native function registry.
    pub fn natives(&self) -> &ProductionNativeRegistry<G> {
        self.natives
    }

    /// Disjoint borrow of the native registry and the gas meter. The
    /// interpreter needs both simultaneously at times.
    pub fn natives_and_gas_meter(&mut self) -> (&ProductionNativeRegistry<G>, &mut G) {
        (self.natives, &mut self.gas_meter)
    }

    /// Resolve a runtime function call. Looks up cross-module targets in the
    /// read-set, falling back to the [`Loader`] on cache miss. May trigger
    /// lazy module loading, gas charge on a cache miss, and lowering of the
    /// function's code.
    pub fn load_function(
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

    /// Access the resource provider to fetch resource from storage on read-set
    /// cache miss. Returns a `'guard`-lifetime reference, decoupled from
    /// `&self`, so callers can hold it while mutably borrowing the read-write
    /// set.
    pub fn resource_provider(&self) -> &'guard dyn ResourceProvider {
        self.resource_provider
    }

    /// Number of GC cycles run on the heap so far.
    pub fn gc_count(&self) -> usize {
        self.heap.gc_count
    }

    /// Seed the RNG that backs `StoreRandomU64`.
    pub fn set_rng_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Open a global-storage rollback checkpoint.
    pub fn checkpoint(&mut self) {
        self.read_write_set.checkpoint();
    }

    /// Roll back the last `n` global-storage checkpoints.
    pub fn rollback(&mut self, n: usize) -> RuntimeResult<()> {
        self.read_write_set.rollback(n)
    }

    /// Number of open global-storage checkpoints.
    pub fn checkpoint_depth(&self) -> usize {
        self.read_write_set.checkpoint_depth()
    }

    /// Current global-storage epoch.
    pub fn current_epoch(&self) -> u64 {
        self.read_write_set.current_epoch()
    }

    /// Length of the global-storage journal.
    pub fn journal_len(&self) -> usize {
        self.read_write_set.journal_len()
    }
}

impl<'guard, 'ctx, G: GasMeter> DescriptorProvider for ExecutionContext<'guard, 'ctx, G> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.loader.guard().descriptor(id)
    }
}
