// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Core types and implementation for the global execution context.
//!
//! # Safety Contract & Design Principles
//!
//! ## Two-Phase State Machine
//!
//! The global context operates in two exclusive phases:
//!
//! 1. **Execution Phase**
//!
//!    Multiple [`ExecutionContext`] guards can be held concurrently. Guards
//!    provide read-only access to interners via [`RwLockReadGuard`] that may
//!    allocate new data but never deallocates, making arena allocations
//!    stable (no reset or drop possible). Pointers returned from the guard are
//!    valid for the guard's lifetime.
//!
//! 2. **Maintenance Phase**
//!    A single exclusive [`MaintenanceContext`] guard exists with write access
//!    via [`RwLockWriteGuard`]. During this phase caches can be reset. Because
//!    no execution contexts can co-exist, there can be no dangling pointers,
//!    making deallocation safe.
//!
//! ## Global Allocation Race Window
//!
//! When interning, allocation happens **outside the [`DashMap`] lock** to
//! reduce contention. This creates a race window where multiple threads may
//! allocate the same interned data. This is intentional and safe:
//!
//!   - Only one pointer is stored in the interner's map.
//!   - Duplicate allocations leak but are bounded (interning converges).
//!   - Trade-off: minor memory waste for lower lock contention.

use crate::{
    alloc::{ArenaGuard, ArenaPool, GlobalArena, GlobalArenaPtr},
    configs::MaintenanceConfig,
    counters,
    executable::{Executable, ExecutableBuilder, Function},
    executable_cache::ExecutableCache,
    interner::{
        DashMapInterner, ExecutableIdKey, IdentifierKey, SignatureTokenKey, SignatureTokenListKey,
        TypeKey, TypeListKey, TypeTagKey, TypeTagListKey,
    },
    types::{
        ExecutableCacheKey, ExecutableId, ExecutableIdInternal, FunctionId, StructId,
        SubstitutionKey, Type, TypeInternal, ADDRESS_INTERNAL, BOOL_INTERNAL, I128_INTERNAL,
        I16_INTERNAL, I256_INTERNAL, I32_INTERNAL, I64_INTERNAL, I8_INTERNAL, SIGNER_INTERNAL,
        U128_INTERNAL, U16_INTERNAL, U256_INTERNAL, U32_INTERNAL, U64_INTERNAL, U8_INTERNAL,
    },
    version::{BlockIndex, TxnIndex, Version},
    TypeList,
};
use dashmap::DashMap;
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{FunctionParamOrReturnTag, ModuleId, TypeTag},
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData;

/// Global execution context with a two-phase state machine.
///
/// # Phases
///
/// 1. **Execution Phase**: Multiple [`ExecutionContext`] guards can be
///    obtained concurrently across threads. Each worker gets access to global
///    arena. This allows parallel execution where each thread can read from
///    the shared caches, allocate data, and safely use raw pointers (addresses
///    are guaranteed to be stable).
///
/// 2. **Maintenance Phase**: A single [`MaintenanceContext`] guard provides
///    exclusive write access for maintenance operations (scheduled between
///    execution phases, e.g., between blocks of transactions) such as cache
///    cleanup or data deallocation.
pub struct GlobalContext {
    /// Shared caches protected by read-write lock.
    shared: RwLock<Context>,
    /// Pool of per-worker arenas. Each worker gets exclusive access to its
    /// arena to reduce contention.
    arenas: ArenaPool,
}

/// Shared context containing interned data structures. Global arena where the
/// data is allocated is kept separately.
struct Context {
    /// Deduplication map for function and struct names. Shared across all
    /// identifier types for maximum memory efficiency.
    identifiers: DashMapInterner<IdentifierKey, str>,
    /// Deduplication map for executable IDs (address-name pairs). Each ID
    /// uniquely identifies a Move module.
    executable_ids: DashMapInterner<ExecutableIdKey, ExecutableIdInternal>,
    /// Deduplication map for types (both fully-instantiated and containing
    /// type parameters).
    types: DashMapInterner<TypeKey, TypeInternal>,
    /// Deduplication map for lists of types (both fully-instantiated and
    /// containing type parameters).
    type_lists: DashMapInterner<TypeListKey, [GlobalArenaPtr<TypeInternal>]>,

    /// Cache for type substitution. Maps generic type (with type parameters)
    /// and a type argument list to its canonical fully-instantiated type
    /// allocated in the global arena.
    type_subst_cache: DashMap<SubstitutionKey, GlobalArenaPtr<TypeInternal>>,

    /// Cache for executables - loaded Move modules.
    executables: ExecutableCache,

    /// Index of the current block, incremented at each maintenance phase
    /// (block boundary). Used for versioning of executables across multiple
    /// blocks.
    block_idx: BlockIndex,

    /// Configuration controlling memory usage thresholds and maintenance
    /// behavior.
    maintenance_config: MaintenanceConfig,
}

impl GlobalContext {
    /// Creates a new global context with the specified number of workers and
    /// default maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_workers(num_workers: usize) -> Self {
        Self::with_num_workers_and_config(num_workers, MaintenanceConfig::default())
    }

    /// Creates a new global context with the specified number of workers and
    /// the maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_workers_and_config(
        num_workers: usize,
        maintenance_config: MaintenanceConfig,
    ) -> Self {
        assert!(
            num_workers > 0 && num_workers <= 128,
            "Number of workers must be between 1 and 128, got {num_workers}"
        );
        assert!(
            num_workers.is_power_of_two(),
            "Number of workers must be a power of two, got {num_workers}"
        );

        Self {
            shared: RwLock::new(Context {
                identifiers: DashMapInterner::default(),
                executable_ids: DashMapInterner::default(),
                types: DashMapInterner::default(),
                type_lists: DashMapInterner::default(),
                type_subst_cache: DashMap::new(),
                executables: ExecutableCache::new(),
                block_idx: 0,
                maintenance_config,
            }),
            arenas: ArenaPool::with_num_arenas(num_workers),
        }
    }

    /// Transitions to execution mode by obtaining a [`ExecutionContext`] guard
    /// and locking the arena for the given worker. Multiple execution contexts
    /// can be held concurrently across threads for different workers.
    ///
    /// Returns [`None`] if
    ///   - there is an ongoing maintenance phase,
    ///   - the worker ID is out of bounds,
    ///   - the arena has already been locked for the same worker.
    pub fn execution_context(
        &self,
        worker_id: usize,
    ) -> Option<ExecutionContext<'_, ArenaGuard<'_>>> {
        let shared_guard = self.shared.try_read()?;

        if worker_id >= self.arenas.num_arenas() {
            return None;
        }
        let arena = self.arenas.lock_arena(worker_id)?;

        Some(ExecutionContext {
            shared_guard,
            arena,
        })
    }

    /// Transitions to maintenance mode by obtaining a [`MaintenanceContext`]
    /// guard. Only one maintenance context can be held at a time, providing
    /// exclusive access to the internal state for maintenance operations. No
    /// execution context can be held concurrently.
    ///
    /// Returns [`None`] if [`ExecutionContext`] is currently held or there is
    /// an ongoing maintenance.
    pub fn maintenance_context(&self) -> Option<MaintenanceContext<'_>> {
        Some(MaintenanceContext {
            shared_guard: self.shared.try_write()?,
            arenas: &self.arenas,
        })
    }
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionContext<'a, A: GlobalArena> {
    /// Read guard on shared interned data (prevents maintenance phase).
    shared_guard: RwLockReadGuard<'a, Context>,
    /// Arena where context can allocate data.
    arena: A,
}

/// Zero-cost compile-time proof that the current scope holds an active
/// [`ExecutionContext`] guard, ensuring arena pointers are stable.
pub(crate) struct ExecutionContextScope<'a>(PhantomData<&'a Context>);

impl<'a, A: GlobalArena> ExecutionContext<'a, A> {
    /// Interns a [`ModuleId`] and returns a stable pointer [`ExecutableId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_module_id<'b>(&'b self, module_id: &ModuleId) -> ExecutableId<'b>
    where
        'a: 'b,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns a [`AccountAddress`]-[`Identifier`] pair corresponding to a
    /// Move module and returns a stable pointer [`ExecutableId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_address_name<'b>(
        &'b self,
        addr: &AccountAddress,
        name: &Identifier,
    ) -> ExecutableId<'b>
    where
        'a: 'b,
    {
        let id = self.intern_address_name_internal(addr, name);
        let scope = self.scope();
        ExecutableId::new_internal(id.as_ref(&scope))
    }

    /// Interns a function name (as a string) and returns a stable pointer
    /// [`FunctionId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_function_name<'b>(&'b self, name: &Identifier) -> FunctionId<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_str_internal(name.as_str());
        let scope = self.scope();
        FunctionId::new_internal(ptr.as_ref(&scope))
    }

    /// Interns a struct name (as an Identifier) and returns a stable pointer
    /// [`StructId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_struct_name<'b>(&'b self, name: &Identifier) -> StructId<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_str_internal(name.as_str());
        let scope = self.scope();
        StructId::new_internal(ptr.as_ref(&scope))
    }

    /// Interns a [`TypeTag`] and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_type_tag<'b>(&'b self, type_tag: &TypeTag) -> Type<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_type_tag_internal(type_tag);
        let scope = self.scope();
        Type::new_internal(ptr.as_ref(&scope))
    }

    /// Interns a list of [`TypeTag`]s and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_type_tags<'b>(&'b self, type_tags: &[TypeTag]) -> TypeList<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_type_tags_internal(type_tags);
        let scope = self.scope();
        TypeList::new_internal(ptr.as_ref(&scope))
    }

    /// Interns a [`SignatureToken`] and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's
    /// lifetime.
    pub fn intern_signature_token<'b>(
        &'b self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> Type<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_signature_token_internal(token, module);
        let scope = self.scope();
        Type::new_internal(ptr.as_ref(&scope))
    }

    /// Returns the current block index.
    ///
    /// Pass the returned value to [`Executable::get_monomorphized_function`]
    /// so the monomorphized function cache can track last-used timestamps for
    /// LRU eviction.
    pub fn block_idx(&self) -> BlockIndex {
        self.shared_guard.block_idx
    }

    /// Returns an executable from the cache, if it exists.
    ///
    /// The returned reference's lifetime is tied to this execution context.
    pub fn get_executable<'b>(&'b self, executable_id: ExecutableId<'_>) -> Option<&'b Executable>
    where
        'a: 'b,
    {
        self.shared_guard
            .executables
            .get_latest(ExecutableCacheKey::new(executable_id))
    }

    /// Returns an executable from the cache at a specific version, if it
    /// exists.
    ///
    /// The returned reference's lifetime is tied to this execution context.
    pub fn get_executable_at<'b>(
        &'b self,
        executable_id: ExecutableId<'_>,
        version: Version,
    ) -> Option<&'b Executable>
    where
        'a: 'b,
    {
        self.shared_guard
            .executables
            .get_at_version(ExecutableCacheKey::new(executable_id), version)
    }

    /// Inserts executable into cache, returns the reference to the inserted
    /// entry. If executable already exists at the same version, no-op and
    /// returns existing reference.
    ///
    /// The returned reference's lifetime is tied to this execution context.
    pub fn insert_executable<'b>(
        &'b self,
        executable_id: ExecutableId<'_>,
        executable: Box<Executable>,
        txn_idx: TxnIndex,
    ) -> &'b Executable
    where
        'a: 'b,
    {
        let version = Version::from_txn_idx(self.shared_guard.block_idx, txn_idx);
        self.shared_guard.executables.insert_cold(
            ExecutableCacheKey::new(executable_id),
            executable,
            version,
        )
    }

    /// Interns a compiled module, creating an [`Executable`] with all
    /// function metadata.
    ///
    /// The returned reference's lifetime is tied to this execution context.
    pub fn intern_compiled_module<'b>(
        &'b self,
        module: &CompiledModule,
        txn_idx: TxnIndex,
    ) -> &'b Executable
    where
        'a: 'b,
    {
        let module_id = self.intern_module_id(&module.self_id());

        // Check if already cached.
        if let Some(exec) = self.get_executable(module_id) {
            return exec;
        }

        // Build the executable.
        let executable = self.intern_compiled_module_internal(module);
        self.insert_executable(module_id, executable, txn_idx)
    }

    /// Returns a monomorphized function from the given executable, caching it
    /// on the first call for a given `(id, type_list)` pair.
    ///
    /// Prefer this over calling [`Executable::get_monomorphized_function`]
    /// directly so the global `mono_total` counter is wired correctly.
    ///
    /// The returned reference's lifetime is tied to this execution context.
    pub fn get_monomorphized_function<'b>(
        &'b self,
        exec: &'b Executable,
        id: FunctionId<'b>,
        type_list: TypeList<'b>,
    ) -> Option<&'b Function>
    where
        'a: 'b,
    {
        let current_block = self.shared_guard.block_idx;
        let mono_total = self.shared_guard.executables.mono_total_ref();
        exec.get_monomorphized_function(id, type_list, current_block, mono_total)
    }
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceContext<'a> {
    /// Write guard on shared interned data (prevents execution contexts).
    shared_guard: RwLockWriteGuard<'a, Context>,
    /// Reference to all arenas for flushing and metrics.
    arenas: &'a ArenaPool,
}

/// Zero-cost compile-time proof that the current scope holds an active
/// [`MaintenanceContext`] guard, ensuring exclusive write access and that
/// no concurrent [`ExecutionContext`] guards exist.
pub(crate) struct MaintenanceContextScope<'a>(PhantomData<&'a Context>);

impl<'a> MaintenanceContext<'a> {
    /// Returns the total number of bytes used across all arenas.
    pub fn interner_arena_allocated_bytes(&self) -> usize {
        self.arenas.allocated_bytes_sum()
    }

    /// Returns the number of bytes used by a specific worker's arena.
    pub fn interner_arena_allocated_bytes_by_worker(&self, worker_id: usize) -> usize {
        self.arenas.allocated_bytes(worker_id)
    }

    /// Returns the number of entries in interner's map for executable IDs.
    pub fn interned_executable_ids_count(&self) -> usize {
        self.shared_guard.executable_ids.len()
    }

    /// Returns the number of entries in interner's map for identifiers.
    pub fn interned_identifiers_count(&self) -> usize {
        self.shared_guard.identifiers.len()
    }

    /// Returns the number of entries in interner's map for types.
    pub fn interned_type_count(&self) -> usize {
        self.shared_guard.types.len()
    }

    /// Returns the number of entries in interner's map for type lists.
    pub fn interned_type_list_count(&self) -> usize {
        self.shared_guard.type_lists.len()
    }

    /// Returns the total number of cached monomorphized functions across all
    /// hot executables. Reads the O(1) atomic counter.
    pub fn monomorphized_function_count(&self) -> usize {
        self.shared_guard
            .executables
            .total_monomorphized_function_count()
    }

    /// Called at the end of each epoch (block boundary) to perform maintenance
    /// operations.
    pub fn on_epoch_end(&mut self) {
        // 1. Promote cold executables to hot, free stale versions.
        // SAFETY:
        //   Maintenance context guarantees there is exclusive access.
        let (promoted, freed) = unsafe { self.shared_guard.executables.compact_and_promote() };
        counters::set_executables_promoted(promoted);
        counters::set_executables_freed(freed);

        // 2. Evict stale monomorphized functions if over threshold.
        self.check_monomorphized_cache();

        // 3. Full arena flush if interner memory exceeds threshold.
        self.check_memory_usage();

        // 4. Advance block index.
        self.shared_guard.block_idx += 1;
        counters::set_block_idx(self.shared_guard.block_idx);
    }

    /// Checks the monomorphized function cache size and performs TTL-based
    /// eviction if the count exceeds
    /// [`MaintenanceConfig::max_monomorphized_functions`].
    fn check_monomorphized_cache(&mut self) {
        let max = self
            .shared_guard
            .maintenance_config
            .max_monomorphized_functions;
        // O(1) atomic read — no traversal of all executables.
        let total = self
            .shared_guard
            .executables
            .total_monomorphized_function_count();
        counters::set_monomorphized_function_count(total);

        if total <= max {
            // No eviction needed; skip the gauge write entirely (C1).
            return;
        }

        let ttl = self
            .shared_guard
            .maintenance_config
            .mono_eviction_ttl_blocks;
        let cutoff = self.shared_guard.block_idx.saturating_sub(ttl);

        // SAFETY:
        //   MaintenanceContext holds exclusive write guard; no execution
        //   contexts can exist, so no live references to individual functions.
        let evicted = unsafe {
            self.shared_guard
                .executables
                .evict_stale_monomorphized(cutoff)
        };

        counters::set_monomorphized_functions_evicted(evicted);
    }

    /// Checks if interner's memory consumption is within the limits specified
    /// in [`MaintenanceConfig`]. If limits are exceeded, all caches and the
    /// arenas are flushed. Returns true if there was a flush, and false
    /// otherwise.
    ///
    /// CRITICAL: Executables must be flushed BEFORE types because
    /// Function.param_types and Function.return_types point to global type
    /// arenas via ArenaPtr.
    pub fn check_memory_usage(&mut self) -> bool {
        let interner_arena_allocated_bytes = self.interner_arena_allocated_bytes();
        counters::set_global_arena_allocated_bytes(interner_arena_allocated_bytes);
        counters::set_interned_executable_id_count(self.interned_executable_ids_count());
        counters::set_interned_identifier_count(self.interned_identifiers_count());
        counters::set_interned_type_count(self.interned_type_count());
        counters::set_interned_type_list_count(self.interned_type_list_count());

        if interner_arena_allocated_bytes
            >= self
                .shared_guard
                .maintenance_config
                .max_global_arena_allocated_bytes
        {
            // SAFETY:
            //   Maintenance context guarantees there is exclusive access.
            //   CRITICAL: Flush executables BEFORE types since executables
            //   store pointers to global type arenas.
            unsafe {
                self.shared_guard.executables.flush();
            }
            debug_assert!(
                self.shared_guard.executables.is_empty(),
                "executable cache must be drained before arena flush"
            );

            let scope = self.scope();

            // Exhaustive destructuring — compile error if a new field is
            // added to Context without being explicitly handled here.
            let Context {
                identifiers,
                executable_ids,
                types,
                type_lists,
                type_subst_cache,
                executables: _,        // flushed above via executables.flush()
                block_idx: _,          // not a cache
                maintenance_config: _, // not a cache
            } = &*self.shared_guard;

            identifiers.reset(&scope);
            executable_ids.reset(&scope);
            types.reset(&scope);
            type_lists.reset(&scope);
            type_subst_cache.clear();

            // SAFETY:
            //   1. Maintenance context: no concurrent execution contexts.
            //   2. All interner maps cleared above (exhaustive destructuring
            //      guarantees this).
            //   3. Executables flushed before this call.
            unsafe {
                self.arenas.reset(&scope);
            }
            counters::set_global_arena_allocated_bytes(self.interner_arena_allocated_bytes());
            counters::set_interned_executable_id_count(0);
            counters::set_interned_identifier_count(0);
            counters::set_interned_type_count(0);
            counters::set_interned_type_list_count(0);
            counters::inc_global_arena_reset_count();

            // Also reset number of cache misses for interner and mono cache.
            counters::reset_executable_id_interner_cache_miss();
            counters::reset_identifier_interner_cache_miss();
            counters::reset_type_interner_cache_miss();
            counters::reset_type_list_interner_cache_miss();
            counters::reset_monomorphized_function_cache_misses();

            return true;
        }

        false
    }
}

// Private APIs.
impl<'a, A: GlobalArena> ExecutionContext<'a, A> {
    /// Returns a scope token proving this execution context guard is held.
    fn scope<'b>(&'b self) -> ExecutionContextScope<'b>
    where
        'a: 'b,
    {
        ExecutionContextScope(PhantomData)
    }

    /// Interns a string and returns a stable pointer to it. For internal
    /// usage only.
    pub(crate) fn intern_str_internal(&self, name: &str) -> GlobalArenaPtr<str> {
        let scope = self.scope();
        if let Some(ptr) = self.shared_guard.identifiers.get(name, &scope) {
            return ptr;
        }
        let ptr = self.arena.alloc_str(name);
        counters::log_identifier_interner_cache_miss();
        self.shared_guard.identifiers.insert(ptr, &scope)
    }

    /// Interns address-name pair and returns a stable pointer to the
    /// corresponding [`ExecutableIdInternal`]. For internal usage only.
    fn intern_address_name_internal(
        &self,
        addr: &AccountAddress,
        name: &IdentStr,
    ) -> GlobalArenaPtr<ExecutableIdInternal> {
        let scope = self.scope();
        if let Some(ptr) = self.shared_guard.executable_ids.get(&(addr, name), &scope) {
            return ptr;
        }
        let name = self.intern_str_internal(name.as_str());
        let ptr = self.arena.alloc(ExecutableIdInternal {
            address: *addr,
            name,
        });
        counters::log_executable_id_interner_cache_miss();
        self.shared_guard.executable_ids.insert(ptr, &scope)
    }

    /// Interns a type tag. For internal usage only.
    fn intern_type_tag_internal(&self, type_tag: &TypeTag) -> GlobalArenaPtr<TypeInternal> {
        // Primitives: return static canonical pointer — no DashMap, no alloc.
        match type_tag {
            TypeTag::Bool => return GlobalArenaPtr::from_static(&BOOL_INTERNAL),
            TypeTag::U8 => return GlobalArenaPtr::from_static(&U8_INTERNAL),
            TypeTag::U16 => return GlobalArenaPtr::from_static(&U16_INTERNAL),
            TypeTag::U32 => return GlobalArenaPtr::from_static(&U32_INTERNAL),
            TypeTag::U64 => return GlobalArenaPtr::from_static(&U64_INTERNAL),
            TypeTag::U128 => return GlobalArenaPtr::from_static(&U128_INTERNAL),
            TypeTag::U256 => return GlobalArenaPtr::from_static(&U256_INTERNAL),
            TypeTag::I8 => return GlobalArenaPtr::from_static(&I8_INTERNAL),
            TypeTag::I16 => return GlobalArenaPtr::from_static(&I16_INTERNAL),
            TypeTag::I32 => return GlobalArenaPtr::from_static(&I32_INTERNAL),
            TypeTag::I64 => return GlobalArenaPtr::from_static(&I64_INTERNAL),
            TypeTag::I128 => return GlobalArenaPtr::from_static(&I128_INTERNAL),
            TypeTag::I256 => return GlobalArenaPtr::from_static(&I256_INTERNAL),
            TypeTag::Address => return GlobalArenaPtr::from_static(&ADDRESS_INTERNAL),
            TypeTag::Signer => return GlobalArenaPtr::from_static(&SIGNER_INTERNAL),
            // Composites fall through to the DashMap path below.
            TypeTag::Vector(_) | TypeTag::Struct(_) | TypeTag::Function(_) => {},
        }

        // Only composite types reach here.
        let scope = self.scope();
        if let Some(ptr) = self.shared_guard.types.get(&TypeTagKey(type_tag), &scope) {
            return ptr;
        }

        let ptr = match type_tag {
            TypeTag::Vector(inner) => {
                let ptr = self.intern_type_tag_internal(inner.as_ref());
                self.arena.alloc(TypeInternal::Vector(ptr))
            },

            TypeTag::Struct(struct_tag) => {
                let module_id =
                    self.intern_address_name_internal(&struct_tag.address, &struct_tag.module);
                let name = self.intern_str_internal(struct_tag.name.as_str());
                let type_args = self.intern_type_tags_internal(&struct_tag.type_args);
                self.arena.alloc(TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                })
            },

            TypeTag::Function(function_tag) => {
                let args =
                    self.intern_function_param_or_return_tags_internal(function_tag.args.as_ref());
                let results = self
                    .intern_function_param_or_return_tags_internal(function_tag.results.as_ref());
                self.arena.alloc(TypeInternal::Function {
                    args,
                    results,
                    abilities: function_tag.abilities,
                })
            },

            // Primitives are handled at the top of this function.
            _ => unreachable!(),
        };

        counters::log_type_interner_cache_miss();
        self.shared_guard.types.insert(ptr, &scope)
    }

    /// Interns a function type tag arguments or results. For internal
    /// usage only.
    fn intern_function_param_or_return_tags_internal(
        &self,
        tags: &[FunctionParamOrReturnTag],
    ) -> GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]> {
        let scope = self.scope();
        let types = tags
            .iter()
            .map(|arg| match arg {
                FunctionParamOrReturnTag::Reference(tag) => {
                    let inner = self.intern_type_tag_internal(tag);
                    let ptr = self.arena.alloc(TypeInternal::Ref(inner));
                    self.shared_guard.types.insert(ptr, &scope)
                },
                FunctionParamOrReturnTag::MutableReference(tag) => {
                    let inner = self.intern_type_tag_internal(tag);
                    let ptr = self.arena.alloc(TypeInternal::RefMut(inner));
                    self.shared_guard.types.insert(ptr, &scope)
                },
                FunctionParamOrReturnTag::Value(tag) => self.intern_type_tag_internal(tag),
            })
            .collect::<Vec<_>>();
        let ptr = self.arena.alloc_slice_copy(&types);
        self.shared_guard.type_lists.insert(ptr, &scope)
    }

    /// Interns a list of type tags. For internal usage only.
    fn intern_type_tags_internal(
        &self,
        tags: &[TypeTag],
    ) -> GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]> {
        let scope = self.scope();
        if let Some(ptr) = self
            .shared_guard
            .type_lists
            .get(&TypeTagListKey(tags), &scope)
        {
            return ptr;
        }

        let types = tags
            .iter()
            .map(|tag| self.intern_type_tag_internal(tag))
            .collect::<Vec<_>>();
        let ptr = self.arena.alloc_slice_copy(&types);
        counters::log_type_list_interner_cache_miss();
        self.shared_guard.type_lists.insert(ptr, &scope)
    }

    /// Interns a signature token. For internal usage only.
    fn intern_signature_token_internal(
        &self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> GlobalArenaPtr<TypeInternal> {
        // Primitives: return static canonical pointer — no DashMap, no alloc.
        match token {
            SignatureToken::Bool => return GlobalArenaPtr::from_static(&BOOL_INTERNAL),
            SignatureToken::U8 => return GlobalArenaPtr::from_static(&U8_INTERNAL),
            SignatureToken::U16 => return GlobalArenaPtr::from_static(&U16_INTERNAL),
            SignatureToken::U32 => return GlobalArenaPtr::from_static(&U32_INTERNAL),
            SignatureToken::U64 => return GlobalArenaPtr::from_static(&U64_INTERNAL),
            SignatureToken::U128 => return GlobalArenaPtr::from_static(&U128_INTERNAL),
            SignatureToken::U256 => return GlobalArenaPtr::from_static(&U256_INTERNAL),
            SignatureToken::I8 => return GlobalArenaPtr::from_static(&I8_INTERNAL),
            SignatureToken::I16 => return GlobalArenaPtr::from_static(&I16_INTERNAL),
            SignatureToken::I32 => return GlobalArenaPtr::from_static(&I32_INTERNAL),
            SignatureToken::I64 => return GlobalArenaPtr::from_static(&I64_INTERNAL),
            SignatureToken::I128 => return GlobalArenaPtr::from_static(&I128_INTERNAL),
            SignatureToken::I256 => return GlobalArenaPtr::from_static(&I256_INTERNAL),
            SignatureToken::Address => return GlobalArenaPtr::from_static(&ADDRESS_INTERNAL),
            SignatureToken::Signer => return GlobalArenaPtr::from_static(&SIGNER_INTERNAL),
            // Composites and TypeParameter fall through to the DashMap path
            // below.
            SignatureToken::Vector(_)
            | SignatureToken::Reference(_)
            | SignatureToken::MutableReference(_)
            | SignatureToken::Struct(_)
            | SignatureToken::StructInstantiation(_, _)
            | SignatureToken::Function(_, _, _)
            | SignatureToken::TypeParameter(_) => {},
        }

        // Only composite / TypeParameter tokens reach here.
        let scope = self.scope();
        if let Some(ptr) = self
            .shared_guard
            .types
            .get(&SignatureTokenKey(token, module), &scope)
        {
            return ptr;
        }

        let ptr = match token {
            SignatureToken::Vector(tok) => {
                let ptr = self.intern_signature_token_internal(tok.as_ref(), module);
                self.arena.alloc(TypeInternal::Vector(ptr))
            },

            SignatureToken::Reference(tok) => {
                let ptr = self.intern_signature_token_internal(tok.as_ref(), module);
                self.arena.alloc(TypeInternal::Ref(ptr))
            },

            SignatureToken::MutableReference(tok) => {
                let ptr = self.intern_signature_token_internal(tok.as_ref(), module);
                self.arena.alloc(TypeInternal::RefMut(ptr))
            },

            SignatureToken::Struct(idx) => {
                let struct_handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(struct_handle.module);

                let module_addr = module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name);
                let module_id = self.intern_address_name_internal(module_addr, module_name);
                let name =
                    self.intern_str_internal(module.identifier_at(struct_handle.name).as_str());
                let type_args = self.intern_signature_tokens_internal(&[], module);

                self.arena.alloc(TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                })
            },

            SignatureToken::StructInstantiation(idx, type_args) => {
                let handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(handle.module);

                let module_addr = module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name);
                let module_id = self.intern_address_name_internal(module_addr, module_name);
                let name = self.intern_str_internal(module.identifier_at(handle.name).as_str());
                let type_args = self.intern_signature_tokens_internal(type_args, module);

                self.arena.alloc(TypeInternal::Struct {
                    module_id,
                    name,
                    type_args,
                })
            },

            SignatureToken::Function(args, results, abilities) => {
                let args = self.intern_signature_tokens_internal(args, module);
                let results = self.intern_signature_tokens_internal(results, module);
                self.arena.alloc(TypeInternal::Function {
                    args,
                    results,
                    abilities: *abilities,
                })
            },

            SignatureToken::TypeParameter(idx) => {
                return self.arena.alloc(TypeInternal::TyParam(*idx));
            },

            // Primitives are handled at the top of this function.
            _ => unreachable!(),
        };

        counters::log_type_interner_cache_miss();
        self.shared_guard.types.insert(ptr, &scope)
    }

    /// Interns a list of signature tokens. For internal usage only.
    pub(crate) fn intern_signature_tokens_internal(
        &self,
        tokens: &[SignatureToken],
        module: &CompiledModule,
    ) -> GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]> {
        let scope = self.scope();
        if let Some(ptr) = self
            .shared_guard
            .type_lists
            .get(&SignatureTokenListKey(tokens, module), &scope)
        {
            return ptr;
        }

        let types = tokens
            .iter()
            .map(|token| self.intern_signature_token_internal(token, module))
            .collect::<Vec<_>>();
        let ptr = self.arena.alloc_slice_copy(&types);
        counters::log_type_list_interner_cache_miss();
        self.shared_guard.type_lists.insert(ptr, &scope)
    }

    /// Interns a compiled module's function signatures, creating an
    /// [`Executable`]. For internal usage only.
    fn intern_compiled_module_internal(&self, module: &CompiledModule) -> Box<Executable> {
        ExecutableBuilder::new(self, module).build()
    }

    /// Substitutes type parameters in each element of `tys`, returning `None`
    /// if no element changed (caller can reuse the original canonical pointer),
    /// or `Some(vec)` with all substituted elements if at least one changed.
    ///
    /// Uses a two-phase approach to avoid any heap allocation on the unchanged
    /// path and any extra branching in the build phase:
    ///   Phase 1 — scan without allocating until the first changed element.
    ///   Phase 2 — build the output Vec starting from the first change; all
    ///             remaining elements are pushed directly with no Option
    ///             overhead.
    fn substitute_type_list_internal(
        &self,
        tys: &[GlobalArenaPtr<TypeInternal>],
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    ) -> Option<Vec<GlobalArenaPtr<TypeInternal>>> {
        // Phase 1: scan for the first changed element without allocating.
        let mut iter = tys.iter().enumerate();
        let (first_idx, first_new) = loop {
            match iter.next() {
                None => return None, // All elements unchanged.
                Some((i, &ty)) => {
                    let new_ty = self.substitute_type_internal(ty, ty_args);
                    if new_ty != ty {
                        break (i, new_ty);
                    }
                },
            }
        };

        // Phase 2: at least one element changed — build the output Vec.
        // Copy the unchanged prefix in one shot, then push the rest.
        let mut v = Vec::with_capacity(tys.len());
        v.extend_from_slice(&tys[..first_idx]);
        v.push(first_new);
        for (_, &ty) in iter {
            v.push(self.substitute_type_internal(ty, ty_args));
        }
        Some(v)
    }

    /// Substitutes type parameters in a type, returning a canonicalized type
    /// pointer. For internal usage only.
    #[allow(dead_code)]
    pub(crate) fn substitute_type_internal(
        &self,
        ty: GlobalArenaPtr<TypeInternal>,
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    ) -> GlobalArenaPtr<TypeInternal> {
        let scope = self.scope();
        let ty_ref = ty.as_ref(&scope);
        match ty_ref {
            // Fast path: primitives have no type parameters.
            TypeInternal::Bool
            | TypeInternal::U8
            | TypeInternal::U16
            | TypeInternal::U32
            | TypeInternal::U64
            | TypeInternal::U128
            | TypeInternal::U256
            | TypeInternal::I8
            | TypeInternal::I16
            | TypeInternal::I32
            | TypeInternal::I64
            | TypeInternal::I128
            | TypeInternal::I256
            | TypeInternal::Address
            | TypeInternal::Signer => ty,
            // Substitution: replace type parameter with the concrete type.
            TypeInternal::TyParam(idx) => {
                let ty_args_ref = ty_args.as_ref(&scope);
                ty_args_ref[*idx as usize]
            },
            // Composite types: cache check + recurse + rebuild if changed.
            _ => {
                // Non-generic structs have no type parameters and always
                // return `ty` unchanged. Detect them here, before paying
                // for the DashMap shard lock in the cache lookup below.
                if let TypeInternal::Struct {
                    type_args: generic_type_args,
                    ..
                } = ty_ref
                {
                    if generic_type_args.as_ref(&scope).is_empty() {
                        return ty;
                    }
                }

                let key = SubstitutionKey::new(ty, ty_args);
                if let Some(cached) = self.shared_guard.type_subst_cache.get(&key) {
                    return *cached.value();
                }

                let result = match ty_ref {
                    TypeInternal::Vector(inner_ty) => {
                        let new_inner = self.substitute_type_internal(*inner_ty, ty_args);
                        if new_inner == *inner_ty {
                            return ty;
                        }
                        let new_ptr = self.arena.alloc(TypeInternal::Vector(new_inner));
                        self.shared_guard.types.insert(new_ptr, &scope)
                    },

                    TypeInternal::Ref(inner_ty) => {
                        let new_inner = self.substitute_type_internal(*inner_ty, ty_args);
                        if new_inner == *inner_ty {
                            return ty;
                        }
                        let new_ptr = self.arena.alloc(TypeInternal::Ref(new_inner));
                        self.shared_guard.types.insert(new_ptr, &scope)
                    },

                    TypeInternal::RefMut(inner_ty) => {
                        let new_inner = self.substitute_type_internal(*inner_ty, ty_args);
                        if new_inner == *inner_ty {
                            return ty;
                        }
                        let new_ptr = self.arena.alloc(TypeInternal::RefMut(new_inner));
                        self.shared_guard.types.insert(new_ptr, &scope)
                    },

                    TypeInternal::Struct {
                        module_id,
                        name,
                        type_args: generic_type_args,
                    } => {
                        // Non-generic structs (empty type_args) are already
                        // handled by the early return above, so `orig` is
                        // non-empty here.
                        let orig = generic_type_args.as_ref(&scope);

                        // Single-pass lazy substitution: no heap allocation
                        // when nothing changes.
                        let new_args = match self.substitute_type_list_internal(orig, ty_args) {
                            None => return ty,
                            Some(v) => v,
                        };

                        // Canonicalize the child slice through the type_lists
                        // interner so that every path to this type agrees on
                        // the same type_args pointer.
                        let raw = self.arena.alloc_slice_copy(&new_args);
                        let canonical_args = self.shared_guard.type_lists.insert(raw, &scope);
                        let new_ptr = self.arena.alloc(TypeInternal::Struct {
                            module_id: *module_id,
                            name: *name,
                            type_args: canonical_args,
                        });
                        self.shared_guard.types.insert(new_ptr, &scope)
                    },

                    TypeInternal::Function {
                        args,
                        results,
                        abilities,
                    } => {
                        let orig_args = args.as_ref(&scope);
                        let orig_results = results.as_ref(&scope);

                        // Single-pass lazy substitution for each slice.
                        // Returns None when the slice is fully concrete
                        // (nothing changed), avoiding heap allocation.
                        let new_args = self.substitute_type_list_internal(orig_args, ty_args);
                        let new_results = self.substitute_type_list_internal(orig_results, ty_args);

                        if new_args.is_none() && new_results.is_none() {
                            return ty;
                        }

                        // Canonicalize both slices. If a slice was unchanged,
                        // reuse its original canonical pointer directly —
                        // no extra allocation.
                        let canonical_args = match new_args {
                            Some(v) => {
                                let raw = self.arena.alloc_slice_copy(&v);
                                self.shared_guard.type_lists.insert(raw, &scope)
                            },
                            None => *args,
                        };
                        let canonical_results = match new_results {
                            Some(v) => {
                                let raw = self.arena.alloc_slice_copy(&v);
                                self.shared_guard.type_lists.insert(raw, &scope)
                            },
                            None => *results,
                        };
                        let new_ptr = self.arena.alloc(TypeInternal::Function {
                            args: canonical_args,
                            results: canonical_results,
                            abilities: *abilities,
                        });
                        self.shared_guard.types.insert(new_ptr, &scope)
                    },

                    // Handled by earlier arms; unreachable here.
                    _ => unreachable!(),
                };

                match self.shared_guard.type_subst_cache.entry(key) {
                    dashmap::mapref::entry::Entry::Occupied(e) => *e.get(),
                    dashmap::mapref::entry::Entry::Vacant(e) => *e.insert(result),
                }
            },
        }
    }
}

impl<'a> MaintenanceContext<'a> {
    /// Returns a scope token proving this maintenance context guard is held.
    fn scope<'b>(&'b self) -> MaintenanceContextScope<'b>
    where
        'a: 'b,
    {
        MaintenanceContextScope(PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::GlobalContext;
    use crate::{
        alloc::GlobalArena,
        types::{SubstitutionKey, TypeInternal},
    };
    use move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };

    fn new_ctx() -> GlobalContext {
        GlobalContext::with_num_workers(1)
    }

    /// Substitute `Vec<T>[T=u64]` first, then `intern(Vec<u64>)` — must
    /// produce the same canonical pointer.
    #[test]
    fn test_substitute_then_intern_vector() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        // Build canonical Vec<TyParam(0)>.
        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(tparam_ptr));
        let vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_t_raw, &exec_ctx.scope());

        // ty_args = [u64].
        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(vec_t, ty_args);
        let intern_ptr =
            exec_ctx.intern_type_tag_internal(&TypeTag::Vector(Box::new(TypeTag::U64)));

        assert!(
            subst_ptr == intern_ptr,
            "substitute(Vec<T>, [u64]) must equal intern(Vec<u64>)"
        );
    }

    /// `intern(Vec<u64>)` first, then substitute `Vec<T>[T=u64]` — must
    /// produce the same canonical pointer.
    #[test]
    fn test_intern_then_substitute_vector() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        // Intern Vec<u64> first.
        let intern_ptr =
            exec_ctx.intern_type_tag_internal(&TypeTag::Vector(Box::new(TypeTag::U64)));

        // Build canonical Vec<TyParam(0)> and substitute.
        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(tparam_ptr));
        let vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_t_raw, &exec_ctx.scope());

        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(vec_t, ty_args);

        assert!(
            intern_ptr == subst_ptr,
            "intern(Vec<u64>) then substitute(Vec<T>, [u64]) must yield the same canonical pointer"
        );
    }

    /// Substituting `Vec<T>[T=TyParam(0)]` (identity) must return the
    /// original canonical `Vec<T>` pointer without allocating a new node.
    #[test]
    fn test_substitute_no_change_returns_canonical() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        // Build canonical Vec<TyParam(0)>.
        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(tparam_ptr));
        let vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_t_raw, &exec_ctx.scope());

        // ty_args = [TyParam(0)] — every type parameter maps to itself.
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[tparam_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let result = exec_ctx.substitute_type_internal(vec_t, ty_args);

        assert!(
            result == vec_t,
            "identity substitution must return the original canonical pointer"
        );
    }

    /// Calling substitute with the same arguments twice must return the same
    /// pointer on both calls, and the result must be present in the
    /// substitution cache.
    #[test]
    fn test_substitute_cache_idempotent() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(tparam_ptr));
        let vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_t_raw, &exec_ctx.scope());

        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let ptr1 = exec_ctx.substitute_type_internal(vec_t, ty_args);
        let ptr2 = exec_ctx.substitute_type_internal(vec_t, ty_args);

        assert!(
            ptr1 == ptr2,
            "two substitutions with the same args must return the same pointer"
        );

        // The result must be present in the substitution cache after the
        // first call.
        let key = SubstitutionKey::new(vec_t, ty_args);
        assert!(
            exec_ctx.shared_guard.type_subst_cache.contains_key(&key),
            "substitution result must be stored in the cache"
        );
    }

    /// `Ref(TyParam(0))[T=u64]` must produce the same canonical pointer as a
    /// manually interned `Ref(u64)`.
    #[test]
    fn test_substitute_ref_consistent() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let ref_t_raw = exec_ctx.arena.alloc(TypeInternal::Ref(tparam_ptr));
        let ref_t = exec_ctx
            .shared_guard
            .types
            .insert(ref_t_raw, &exec_ctx.scope());

        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(ref_t, ty_args);

        // Build the canonical &u64 independently via the interner.
        let ref_u64_raw = exec_ctx.arena.alloc(TypeInternal::Ref(u64_ptr));
        let ref_u64 = exec_ctx
            .shared_guard
            .types
            .insert(ref_u64_raw, &exec_ctx.scope());

        assert!(
            subst_ptr == ref_u64,
            "substitute(&T, [u64]) must equal canonical &u64"
        );
    }

    /// `RefMut(TyParam(0))[T=u64]` must produce the same canonical pointer as
    /// a manually interned `RefMut(u64)`.
    #[test]
    fn test_substitute_refmut_consistent() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let refmut_t_raw = exec_ctx.arena.alloc(TypeInternal::RefMut(tparam_ptr));
        let refmut_t = exec_ctx
            .shared_guard
            .types
            .insert(refmut_t_raw, &exec_ctx.scope());

        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(refmut_t, ty_args);

        // Build the canonical &mut u64 independently via the interner.
        let refmut_u64_raw = exec_ctx.arena.alloc(TypeInternal::RefMut(u64_ptr));
        let refmut_u64 = exec_ctx
            .shared_guard
            .types
            .insert(refmut_u64_raw, &exec_ctx.scope());

        assert!(
            subst_ptr == refmut_u64,
            "substitute(&mut T, [u64]) must equal canonical &mut u64"
        );
    }

    /// `Vec<Vec<T>>[T=u64]` must produce the same canonical pointer as
    /// `intern(Vec<Vec<u64>>)`.
    #[test]
    fn test_substitute_nested_vector() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        // Build canonical Vec<TyParam(0)>.
        let tparam_ptr = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(tparam_ptr));
        let vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_t_raw, &exec_ctx.scope());

        // Build canonical Vec<Vec<TyParam(0)>>.
        let vec_vec_t_raw = exec_ctx.arena.alloc(TypeInternal::Vector(vec_t));
        let vec_vec_t = exec_ctx
            .shared_guard
            .types
            .insert(vec_vec_t_raw, &exec_ctx.scope());

        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(vec_vec_t, ty_args);
        let intern_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::Vector(Box::new(
            TypeTag::Vector(Box::new(TypeTag::U64)),
        )));

        assert!(
            subst_ptr == intern_ptr,
            "substitute(Vec<Vec<T>>, [u64]) must equal intern(Vec<Vec<u64>>)"
        );
    }

    /// `Struct<T, U>[T=u64, U=bool]` must produce the same canonical pointer as
    /// `intern(Struct<u64, bool>)` built via `TypeTag::Struct`.
    #[test]
    fn test_substitute_struct_type_args() {
        let ctx = new_ctx();
        let exec_ctx = ctx.execution_context(0).unwrap();

        let addr = AccountAddress::ZERO;
        let module_name = Identifier::new("test_module").unwrap();
        let struct_name = Identifier::new("TestStruct").unwrap();

        // Intern module_id and struct name.
        let module_id = exec_ctx.intern_address_name_internal(&addr, &module_name);
        let name = exec_ctx.intern_str_internal(struct_name.as_str());

        // Build canonical Struct<TyParam(0), TyParam(1)>.
        let tparam0 = exec_ctx.arena.alloc(TypeInternal::TyParam(0));
        let tparam1 = exec_ctx.arena.alloc(TypeInternal::TyParam(1));
        let generic_args_raw = exec_ctx.arena.alloc_slice_copy(&[tparam0, tparam1]);
        let generic_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(generic_args_raw, &exec_ctx.scope());
        let generic_struct_raw = exec_ctx.arena.alloc(TypeInternal::Struct {
            module_id,
            name,
            type_args: generic_args,
        });
        let generic_struct = exec_ctx
            .shared_guard
            .types
            .insert(generic_struct_raw, &exec_ctx.scope());

        // ty_args = [u64, bool].
        let u64_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::U64);
        let bool_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::Bool);
        let ty_args_raw = exec_ctx.arena.alloc_slice_copy(&[u64_ptr, bool_ptr]);
        let ty_args = exec_ctx
            .shared_guard
            .type_lists
            .insert(ty_args_raw, &exec_ctx.scope());

        let subst_ptr = exec_ctx.substitute_type_internal(generic_struct, ty_args);

        // Build the expected fully-instantiated type via TypeTag.
        let struct_tag = StructTag {
            address: addr,
            module: module_name,
            name: struct_name,
            type_args: vec![TypeTag::U64, TypeTag::Bool],
        };
        let intern_ptr = exec_ctx.intern_type_tag_internal(&TypeTag::Struct(Box::new(struct_tag)));

        assert!(
            subst_ptr == intern_ptr,
            "substitute(Struct<T,U>, [u64, bool]) must equal intern(Struct<u64, bool>)"
        );
    }
}
