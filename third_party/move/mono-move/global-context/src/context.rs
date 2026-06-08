// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types and implementation for the global execution context.
//!
//! # Rationale
//!
//! The block executor runs many transactions in parallel within a block and
//! sequentially across blocks. Code-derived data (interned identifiers,
//! interned types, loaded modules) is long-lived: it survives across
//! transactions and is shared between worker threads. Ideally, the following
//! requirements are satisfied:
//!   - allocations are cheap and lock-free on the hot path,
//!   - references to data can be handed to workers and stay valid for the
//!     duration of their work without per-reference counting.
//!
//! Concurrent deallocation against many readers is the hard problem. To avoid
//! it, memory is only reclaimed at certain epochs (e.g., between blocks). The
//! two-phase state machine below turns this observation safety contract
//! enforced at compile-time.
//!
//! # Safety Contract
//!
//! ## Two-Phase State Machine
//!
//! The global context operates in two exclusive phases:
//!
//! 1. **Execution Phase**
//!
//!    Multiple [`ExecutionGuard`] guards can be held concurrently. Guards
//!    provide read-only access to caches to obtain or allocate data, but never
//!    deallocate, making arena allocations stable (no reset or drop possible).
//!    Pointers returned from the guard are valid for the guard's lifetime.
//!
//! 2. **Maintenance Phase**
//!    A single exclusive [`MaintenanceGuard`] guard exists with write access
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

use crate::maintenance_config::MaintenanceConfig;
use anyhow::{bail, Result};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use mono_move_alloc::{GlobalArenaPool, GlobalArenaPtr, GlobalArenaShard};
use mono_move_core::{
    types::NominalLayout, DescriptorId, DescriptorProvider, FrameOffset, FunctionRef, Interner,
    ModuleId, ObjectDescriptor, TRIVIAL_DESCRIPTOR_ID,
};
use move_binary_format::{file_format::SignatureToken, CompiledModule};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

// Submodules: to split implementation into smaller pieces.
mod identifiers;
use identifiers::IdentifierInternerKey;
mod module_ids;
use module_ids::ModuleIdInternerKey;
mod loaded_module;
pub use loaded_module::{
    FunctionSlot, LoadedModule, LoadedModuleSlot, ModuleMandatoryDependencies, ModuleSlot,
};
mod module_cache;
use module_cache::ModuleCache;
use mono_move_core::interner::{InternedFunctionRef, InternedIdentifier, InternedModuleId};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

mod types;
pub use types::{
    struct_info_at, try_as_primitive_type, view_name, view_type, view_type_list, FieldLayout,
    InternedType, InternedTypeList, Type,
};
use types::{TypeInternerKey, TypeListInternerKey};

/// Global execution context with a two-phase state machine.
///
/// # Phases
///
/// 1. **Execution Phase**: Multiple [`ExecutionGuard`] guards can be
///    obtained concurrently across threads. Each worker gets access to global
///    arena. This allows parallel execution where each thread can read from
///    the shared caches, allocate data, and safely use raw pointers (addresses
///    are guaranteed to be stable).
///
/// 2. **Maintenance Phase**: A single [`MaintenanceGuard`] guard provides
///    exclusive write access for maintenance operations (scheduled between
///    execution phases, e.g., between blocks of transactions) such as cache
///    cleanup or data deallocation.
pub struct GlobalContext {
    /// Shared caches storing interned data, modules.
    ctx: Context,
    /// Pool of arenas (assigned per execution worker). Each worker gets
    /// exclusive access to their arena to avoid contention.
    global_arena: GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    maintenance_config: MaintenanceConfig,
    /// Lock to switch between execution and maintenance modes:
    ///   - Read lock: execution phase.
    ///   - Write lock: maintenance phase.
    phase: RwLock<()>,
}

/// Shared context containing interned data structures. Global arena where the
/// data is allocated is kept separately.
struct Context {
    identifiers: DashMap<IdentifierInternerKey, InternedIdentifier, ahash::RandomState>,
    module_ids: DashMap<ModuleIdInternerKey, InternedModuleId, ahash::RandomState>,
    types: DashMap<TypeInternerKey, InternedType, ahash::RandomState>,
    type_lists: DashMap<TypeListInternerKey, InternedTypeList, ahash::RandomState>,
    // TODO(perf): reconsider whether this indirection earns its keep. The
    // alternative is to widen the closure's `Unresolved` func_ref payload to
    // store the 24-byte triple inline, dropping both this map and the
    // `InternedFunctionRef` arena allocation at the cost of a larger closure
    // object.
    function_refs: DashMap<
        (InternedModuleId, InternedIdentifier, InternedTypeList),
        InternedFunctionRef,
        ahash::RandomState,
    >,
    module_cache: ModuleCache,
    /// Published object descriptors.
    descriptors: Descriptors,
}

/// Storage for the published object-descriptor set.
///
/// # Invariants
///
/// - `table[id.as_usize()]` is the descriptor for [`DescriptorId`] `id`.
/// - Slot 0 is [`ObjectDescriptor::trivial`] and slot 1 is
///   [`ObjectDescriptor::closure`]; user descriptors start at slot 2.
/// - For every `(elem_ty, id)` in `vector_by_elem`, `table[id]` is a
///   `Vector` descriptor with that element type.
/// - Entries are appended but never removed or reordered during the
///   execution phase; only [`MaintenanceGuard::reset_arena_pool`] clears
///   the table.
/// - Descriptors are held behind `Arc` so the table's `store` on reset
///   drops their heap-owning payloads (e.g. `Vec<u32>` offset lists).
//
// TODO(perf): if the per-lookup `Arc` deref or the per-append `Vec` clone
// shows up in profiles, switch to arena-allocated descriptors with POD
// payloads (`&'arena [u32]` instead of `Vec<u32>`). Eliminates the `Arc`
// indirection and the clone-per-append at the cost of changing
// `ObjectDescriptorInner`'s payload shape.
struct Descriptors {
    /// Vector-descriptor idempotency cache: `elem_ty -> id`. Lock-free reads
    /// via DashMap; the first publisher for a given `elem_ty` takes a shard
    /// write-lock once. Future descriptor kinds can share this cache by
    /// keying on the full `InternedType` (e.g. `vector<T>`, `struct<...>`).
    vector_by_elem: DashMap<InternedType, DescriptorId, ahash::RandomState>,
    /// Captured-data idempotency cache: pointer-offset shape -> id. Captures
    /// sharing a pointer shape share one descriptor. Pointer-free captures
    /// bypass this cache for `TRIVIAL_DESCRIPTOR_ID`.
    captured_data_by_pointer_offsets: DashMap<Vec<u32>, DescriptorId, ahash::RandomState>,
    /// All descriptors (reserved + user) in id order. Replaced atomically
    /// on append via `ArcSwap::rcu` (CAS loop). Readers `load()` without
    /// locking.
    table: ArcSwap<Vec<Arc<ObjectDescriptor>>>,
}

impl Default for Descriptors {
    fn default() -> Self {
        Self {
            vector_by_elem: DashMap::default(),
            captured_data_by_pointer_offsets: DashMap::default(),
            table: ArcSwap::from_pointee(initial_descriptors()),
        }
    }
}

impl Descriptors {
    /// Drop user descriptors and idempotency caches; reinstall the
    /// reserved-slot table.
    fn reset(&self) {
        // Exhaustive destructuring so that adding a new field forces a
        // compile-time error here.
        let Self {
            vector_by_elem,
            captured_data_by_pointer_offsets,
            table,
        } = self;
        vector_by_elem.clear();
        captured_data_by_pointer_offsets.clear();
        table.store(Arc::new(initial_descriptors()));
    }
}

/// Initial descriptor table: the two reserved entries.
fn initial_descriptors() -> Vec<Arc<ObjectDescriptor>> {
    vec![
        Arc::new(ObjectDescriptor::trivial()),
        Arc::new(ObjectDescriptor::closure()),
    ]
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceGuard<'ctx> {
    /// Reference to the caches stored in context.
    ctx: &'ctx Context,
    /// Pool of all arenas managing global allocations.
    global_arena: &'ctx GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    #[allow(dead_code)]
    maintenance_config: &'ctx MaintenanceConfig,

    /// Write guard that disallows obtaining concurrent execution
    /// guard. **Must** be dropped last.
    _guard: RwLockWriteGuard<'ctx, ()>,
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionGuard<'ctx> {
    /// Reference to the caches stored in context.
    ctx: &'ctx Context,
    /// Arena dedicated for this execution guard with exclusive access.
    /// During execution, data can be allocated here without contention.
    global_arena: GlobalArenaShard<'ctx>,

    /// Read guard preventing maintenance phase, but allowing concurrent
    /// execution phases. **Must** be dropped last.
    _guard: RwLockReadGuard<'ctx, ()>,
}

/// A scoped reference to data obtained from [`ExecutionGuard`] and is guaranteed
/// to be alive until the guard is dropped.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
///
/// The pointer stored behind the reference is guaranteed to be valid and
/// safe to dereference.
pub struct ArenaRef<'guard, T: ?Sized> {
    ptr: GlobalArenaPtr<T>,
    _guard: PhantomData<&'guard ()>,
}

impl GlobalContext {
    /// Creates a new global context with the specified number of workers that
    /// can acquire [`ExecutionGuard`] and default maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_execution_workers(num_workers: usize) -> Self {
        Self::with_num_execution_workers_and_config(num_workers, MaintenanceConfig::default())
    }

    /// Creates a new global context with the specified number of execution
    /// workers that can acquire [`ExecutionGuard`] and the maintenance config.
    ///
    /// # Panics
    ///
    /// Panics if the number of workers is 0, greater than 128 or is not a
    /// power of two.
    pub fn with_num_execution_workers_and_config(
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
            ctx: Context {
                identifiers: DashMap::default(),
                module_ids: DashMap::default(),
                types: DashMap::default(),
                type_lists: DashMap::default(),
                function_refs: DashMap::default(),
                module_cache: ModuleCache::new(),
                descriptors: Descriptors::default(),
            },
            global_arena: GlobalArenaPool::with_num_arenas(num_workers),
            maintenance_config,
            phase: RwLock::new(()),
        }
    }

    /// Transitions to maintenance mode by obtaining a [`MaintenanceGuard`]
    /// guard. Only one maintenance context can be held at a time, providing
    /// exclusive access to the internal state for maintenance operations. No
    /// execution context can be held concurrently.
    ///
    /// Returns [`None`] if [`ExecutionGuard`] is currently held or there is
    /// an ongoing maintenance.
    #[must_use]
    pub fn try_maintenance_context(&self) -> Option<MaintenanceGuard<'_>> {
        let _guard = self.phase.try_write()?;

        Some(MaintenanceGuard {
            ctx: &self.ctx,
            global_arena: &self.global_arena,
            maintenance_config: &self.maintenance_config,
            _guard,
        })
    }

    /// Transitions to execution mode by obtaining an [`ExecutionGuard`] guard
    /// and locking the arena for the given worker. Multiple execution contexts
    /// can be held concurrently across threads for different workers.
    ///
    /// Returns [`None`] if
    ///   - there is an ongoing maintenance phase,
    ///   - the arena for this worker has already been locked.
    ///
    /// # Panics
    ///
    /// Panics if the worker ID is out of bounds when trying to get an arena
    /// from the pool.
    #[must_use]
    pub fn try_execution_context(&self, worker_id: usize) -> Option<ExecutionGuard<'_>> {
        let _guard = self.phase.try_read()?;

        Some(ExecutionGuard {
            ctx: &self.ctx,
            global_arena: self.global_arena.try_lock_arena(worker_id)?,
            _guard,
        })
    }
}

impl<'ctx> MaintenanceGuard<'ctx> {
    /// Returns the total number of bytes used across all arenas in the global
    /// arena pool.
    pub fn global_arena_allocated_bytes_sum(&self) -> usize {
        (0..self.global_arena.num_arenas())
            .map(|idx| self.global_arena.allocated_bytes(idx))
            .sum()
    }

    /// Returns the number of entries in interner's map for identifiers.
    pub fn interned_identifiers_count(&self) -> usize {
        self.ctx.identifiers.len()
    }

    /// Returns the number of entries in interner's map for module IDs.
    pub fn interned_module_ids_count(&self) -> usize {
        self.ctx.module_ids.len()
    }

    /// Returns the number of entries in interner's map for types.
    pub fn interned_types_count(&self) -> usize {
        self.ctx.types.len()
    }

    /// Returns the number of entries in interner's map for type lists.
    pub fn interned_type_lists_count(&self) -> usize {
        self.ctx.type_lists.len()
    }

    /// Resets all caches that store pointers to the arenas, and then resets
    /// the arenas as well.
    pub fn reset_arena_pool(&mut self) {
        // SAFETY: Arena is only reset **after** caches are cleared.
        unsafe {
            self.reset_all_caches();
        }

        // SAFETY: We are in maintenance phase, so there are no concurrent
        // execution contexts and therefore no live pointers to arena other
        // than ones that were stored in caches. All caches were cleared (see
        // above), and so there are no live pointers making reset safe.
        unsafe {
            self.global_arena.reset_all_arenas_unchecked();
        }
    }
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Inserts a loaded module into the cache, keyed by its interned ID.
    ///
    /// Returns an error only if the cache detects an invariant violation
    /// during install. Under normal operation this method always returns
    /// `Ok`.
    pub fn insert_module(&self, module: Box<LoadedModule>) -> Result<&LoadedModule> {
        let ptr = self.ctx.module_cache.insert(module)?;

        // SAFETY: The pointer is valid since it was created by leaking a box,
        // and can only be freed during the maintenance phase, while we are in
        // the execution phase (guard is alive). If the loaded module was
        // already in the cache, it is also alive (maintenance has not reset
        // caches).
        Ok(unsafe { ptr.as_ref_unchecked() })
    }

    /// Looks up a cached loaded module by its interned ID and returns a
    /// reference tied to the guard's lifetime, if found.
    pub fn get_module<'guard>(
        &'guard self,
        key: ArenaRef<'guard, ModuleId>,
    ) -> Option<&'guard LoadedModule> {
        let ptr = self.ctx.module_cache.get(key.into_global_arena_ptr())?;

        // SAFETY: The pointer is valid since it was created by leaking a box,
        // and can only be freed during the maintenance phase, while we are in
        // the execution phase (guard is alive).
        Some(unsafe { ptr.as_ref_unchecked() })
    }

    /// Returns the stable slot for `key`, creating an empty one if absent.
    /// The returned pointer is valid for the cache's lifetime. Takes a
    /// shard write lock on the create path.
    pub fn get_or_create_module_slot<'guard>(
        &'guard self,
        key: ArenaRef<'guard, ModuleId>,
    ) -> LoadedModuleSlot {
        self.ctx
            .module_cache
            .get_or_create_slot(key.into_global_arena_ptr())
    }

    /// Wraps module ID pointer in a guard-scoped [`ArenaRef`], matching the
    /// key shape used by the module cache.
    pub fn arena_ref_for_module_id<'guard>(
        &'guard self,
        ptr: InternedModuleId,
    ) -> ArenaRef<'guard, ModuleId>
    where
        'ctx: 'guard,
    {
        // SAFETY: interned ids are alive for the entire execution phase.
        unsafe { self.arena_ref(ptr) }
    }

    // ====================================================================
    // Public type construction helpers
    // ====================================================================

    /// Safely dereferences a type pointer. Returns a reference to the
    /// underlying [`Type`] data. The reference is valid for the guard's
    /// lifetime.
    ///
    /// This localizes `unsafe` to this method.
    pub fn type_data(&self, ptr: InternedType) -> &Type {
        // SAFETY: All type pointers are valid during the execution phase
        // because the arena is not reset while any ExecutionGuard is alive.
        unsafe { ptr.as_ref_unchecked() }
    }

    /// Interns a nominal (struct or enum) identity without populating its
    /// layout. The returned pointer can be used immediately in IR, but
    /// [`Type::size_and_align`] and [`Type::layout`] will return [`None`]
    /// until [`Self::set_nominal_layout`] is called for this type. Callers
    /// using this in cross-module contexts must tolerate the missing layout
    /// until a layout-population pass runs.
    pub fn intern_nominal(
        &self,
        module_id: InternedModuleId,
        name: GlobalArenaPtr<str>,
        ty_args: InternedTypeList,
    ) -> InternedType {
        let ty = self.global_arena.alloc(Type::Nominal {
            module_id,
            name,
            ty_args,
            layout: OnceLock::new(),
        });
        self.insert_allocated_type_pointer_internal(ty)
    }

    /// Allocates the field-layout slice in the arena (when `fields` is
    /// `Some`), builds a [`NominalLayout`] and installs it into the type's
    /// layout slot. Pass `Some(&fields)` for structs and `None` for enums.
    /// Returns an error if `ty` is not a nominal type.
    ///
    /// Setting layouts concurrently is safe: the layout stores canonical
    /// field type pointers and so is structurally identical across threads.
    pub fn set_nominal_layout(
        &self,
        ty: InternedType,
        size: u32,
        align: u32,
        fields: Option<&[FieldLayout]>,
    ) -> Result<()> {
        let slot = match view_type(ty) {
            Type::Nominal { layout: slot, .. } => slot,
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Vector { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => {
                bail!("set_nominal_layout called on a non-nominal type")
            },
        };

        // Fast path: if the layout is already installed, skip the field-slice
        // allocation entirely. A race can still leak one allocation between
        // this check and `slot.set` below, but that is bounded and consistent
        // with the documented arena race trade-off.
        if slot.get().is_some() {
            return Ok(());
        }

        let layout = match fields {
            Some(fields) => {
                let fields_ptr = self.global_arena.alloc_slice_copy(fields);
                NominalLayout::new_struct(size, align, fields_ptr)
            },
            None => NominalLayout::new_enum(size, align),
        };
        if let Err(other_layout) = slot.set(layout) {
            let installed_layout = slot.get().expect("Layout was just installed");
            debug_assert_eq!(installed_layout.size, other_layout.size);
            debug_assert_eq!(installed_layout.align, other_layout.align);
            debug_assert_eq!(
                installed_layout.field_layouts().is_some(),
                other_layout.field_layouts().is_some()
            );
            // Layout computation is deterministic given the type identity,
            // so per-field offsets must match too.
            if let (Some(installed), Some(other)) = (
                installed_layout.field_layouts(),
                other_layout.field_layouts(),
            ) {
                debug_assert_eq!(installed.len(), other.len());
                for (installed, other) in installed.iter().zip(other.iter()) {
                    debug_assert_eq!(installed.offset, other.offset);
                }
            }
        }
        Ok(())
    }

    /// Returns the already-published vector-descriptor id for `elem_ty`,
    /// or `None` if no descriptor has been published yet. Lock-free.
    pub fn vec_descriptor_for(&self, elem_ty: InternedType) -> Option<DescriptorId> {
        self.ctx
            .descriptors
            .vector_by_elem
            .get(&elem_ty)
            .map(|r| *r)
    }

    /// Materializes a vector-object descriptor for `elem_ty` into the
    /// shared arena and returns its assigned [`DescriptorId`]. Idempotent:
    /// subsequent calls with the same `elem_ty` return the same id without
    /// re-allocating.
    //
    // TODO(perf): the slow path takes a DashMap shard write-lock (`entry`)
    // and clones the descriptor table on each append; the `rcu` loop
    // additionally re-clones on conflict. Profile, then revisit. Two
    // candidates:
    //   1. Preallocate the table in chunks so most appends are O(1) and
    //      only chunk-boundary crossings clone (a small Vec of chunk
    //      pointers).
    //   2. Replace `ArcSwap<Vec<_>>` with `DashMap<DescriptorId, Arc<_>>`
    //      + `AtomicU32` counter — O(1) appends, but reads (hot path)
    //      pay a hashed lookup instead of array indexing.
    pub fn publish_vec_descriptor(
        &self,
        elem_ty: InternedType,
        elem_size: u32,
        elem_ptr_offsets: &[FrameOffset],
    ) -> DescriptorId {
        // Fast path: existing entry returns without touching the shard
        // write-lock.
        if let Some(id) = self.ctx.descriptors.vector_by_elem.get(&elem_ty) {
            return *id;
        }
        *self
            .ctx
            .descriptors
            .vector_by_elem
            .entry(elem_ty)
            .or_insert_with(|| {
                let offsets: Vec<u32> = elem_ptr_offsets.iter().map(|o| o.0).collect();
                let desc = Arc::new(
                    ObjectDescriptor::new_vector(elem_size, offsets)
                        .unwrap_or_else(|e| panic!("publish_vec_descriptor: {e}")),
                );
                self.append_descriptor(desc)
            })
    }

    /// Appends `desc` to the shared descriptor table and returns its assigned
    /// [`DescriptorId`]. `rcu` retries on CAS conflict; the closure re-reads
    /// `next.len()` so the id always matches the table state at the store.
    fn append_descriptor(&self, desc: Arc<ObjectDescriptor>) -> DescriptorId {
        let mut assigned_id = DescriptorId(0);
        self.ctx.descriptors.table.rcu(|cur| {
            let mut next = cur.as_ref().clone();
            assigned_id = DescriptorId(
                u32::try_from(next.len()).expect("published descriptor count exceeds u32::MAX"),
            );
            next.push(desc.clone());
            Arc::new(next)
        });
        assigned_id
    }

    /// Returns the GC trace descriptor for a closure's captured-data object.
    /// `values_size` is the byte width of the packed values region;
    /// `pointer_offsets` are intra-values heap-pointer offsets.
    ///
    /// A pointer-free capture (no offsets) returns the reserved
    /// [`TRIVIAL_DESCRIPTOR_ID`]. A pointer-bearing capture materializes (or
    /// reuses) a `CapturedData` descriptor, idempotent on the pointer-offset
    /// shape.
    pub fn publish_captured_data_descriptor(
        &self,
        values_size: u32,
        pointer_offsets: &[FrameOffset],
    ) -> DescriptorId {
        if pointer_offsets.is_empty() {
            return TRIVIAL_DESCRIPTOR_ID;
        }
        let offsets: Vec<u32> = pointer_offsets.iter().map(|o| o.0).collect();
        if let Some(id) = self
            .ctx
            .descriptors
            .captured_data_by_pointer_offsets
            .get(&offsets)
        {
            return *id;
        }
        *self
            .ctx
            .descriptors
            .captured_data_by_pointer_offsets
            .entry(offsets.clone())
            .or_insert_with(move || {
                let desc = Arc::new(
                    ObjectDescriptor::new_captured_data(values_size, offsets)
                        .unwrap_or_else(|e| panic!("publish_captured_data_descriptor: {e}")),
                );
                self.append_descriptor(desc)
            })
    }

    /// Looks up a type previously interned from a signature token of `module`.
    /// Returns `None` if the token has not yet been interned in this module's
    /// context.
    ///
    /// For nominal types, the returned pointer carries identity but its
    /// layout slot may still be empty — `set_nominal_layout` runs in a
    /// separate pass after all field types are interned. Callers consuming
    /// this in cross-module contexts must tolerate `None` from
    /// [`Type::size_and_align`] and [`Type::layout`] until the layout-
    /// population pass completes.
    pub fn try_intern_for_module(
        &self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> Option<InternedType> {
        self.get_interned_type_pointer_internal(token, module)
    }
}

impl<'ctx> DescriptorProvider for ExecutionGuard<'ctx> {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        let guard = self.ctx.descriptors.table.load();
        let arc = guard.get(id.as_usize())?;
        let ptr: *const ObjectDescriptor = Arc::as_ptr(arc);
        drop(guard);
        // SAFETY: Descriptor `Arc`s are dropped only when the table is
        // replaced on maintenance reset, which requires the phase write-lock
        // and therefore the absence of any live `ExecutionGuard`. This
        // `ExecutionGuard` holds the phase read-lock, so no maintenance can
        // run while `&self` lives — the `Arc<ObjectDescriptor>` for any id
        // that resolved here stays alive for the returned reference's
        // lifetime, which is tied to `&self`.
        Some(unsafe { &*ptr })
    }
}

impl<'ctx> Interner for ExecutionGuard<'ctx> {
    fn type_param_of(&self, idx: u16) -> InternedType {
        let ty = self.global_arena.alloc(Type::TypeParam { idx });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn vector_of(&self, elem: InternedType) -> InternedType {
        let ty = self.global_arena.alloc(Type::Vector { elem });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn immut_ref_of(&self, inner: InternedType) -> InternedType {
        let ty = self.global_arena.alloc(Type::ImmutRef { inner });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn mut_ref_of(&self, inner: InternedType) -> InternedType {
        let ty = self.global_arena.alloc(Type::MutRef { inner });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn function_of(
        &self,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: move_core_types::ability::AbilitySet,
    ) -> InternedType {
        let ty = self.global_arena.alloc(Type::Function {
            args,
            results,
            abilities,
        });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn type_list_of(&self, types: &[InternedType]) -> InternedTypeList {
        if types.is_empty() {
            return types::EMPTY_TYPE_LIST;
        }
        let ptr = self.global_arena.alloc_slice_copy(types);
        self.insert_allocated_type_list_internal(InternedTypeList::new(ptr))
    }

    fn nominal_of(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedType {
        let ty = self.global_arena.alloc(Type::Nominal {
            module_id,
            name,
            ty_args,
            layout: OnceLock::new(),
        });
        self.insert_allocated_type_pointer_internal(ty)
    }

    fn module_id_of(&self, address: &AccountAddress, name: &IdentStr) -> InternedModuleId {
        self.intern_address_name_internal(*address, name)
    }

    fn function_ref_of(
        &self,
        module_id: InternedModuleId,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedFunctionRef {
        // SAFETY: all three components are canonical (already-interned)
        // pointers, so the tuple's pointer-based hash/eq is structural. The
        // map is cleared on arena reset, so stored pointers stay valid.
        let key = (module_id, func_name, ty_args);
        if let Some(entry) = self.ctx.function_refs.get(&key) {
            return *entry.value();
        }
        let ptr = self.global_arena.alloc(FunctionRef {
            module_id,
            func_name,
            ty_args,
        });
        *self.ctx.function_refs.entry(key).or_insert(ptr)
    }

    fn identifier_of(&self, identifier: &IdentStr) -> InternedIdentifier {
        self.intern_identifier_internal(identifier)
    }

    // TODO:
    //   1. Non-recursive implementation.
    //   2. Current implementation is O(N^2) because hashes of inner types are
    //      not cached, and have to be recomputed on insertion.
    fn subst_type(
        &self,
        ty: InternedType,
        ty_args: InternedTypeList,
    ) -> anyhow::Result<InternedType> {
        if ty_args.is_empty() {
            return Ok(ty);
        }

        Ok(match view_type(ty) {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer => ty,

            Type::Vector { elem } => {
                let new_elem = self.subst_type(*elem, ty_args)?;
                if new_elem == *elem {
                    ty
                } else {
                    self.vector_of(new_elem)
                }
            },
            Type::ImmutRef { inner } => {
                let new_inner = self.subst_type(*inner, ty_args)?;
                if new_inner == *inner {
                    ty
                } else {
                    self.immut_ref_of(new_inner)
                }
            },
            Type::MutRef { inner } => {
                let new_inner = self.subst_type(*inner, ty_args)?;
                if new_inner == *inner {
                    ty
                } else {
                    self.mut_ref_of(new_inner)
                }
            },
            Type::Nominal {
                module_id,
                name,
                ty_args: inner_args,
                layout,
            } => {
                let new_inner_args = self.subst_type_list(*inner_args, ty_args)?;
                if new_inner_args == *inner_args {
                    ty
                } else {
                    debug_assert!(layout.get().is_none(), "Layout cannot be set for generics");
                    self.nominal_of(*module_id, *name, new_inner_args)
                }
            },
            Type::Function {
                args,
                results,
                abilities,
            } => {
                let new_args = self.subst_type_list(*args, ty_args)?;
                let new_results = self.subst_type_list(*results, ty_args)?;
                if new_args == *args && new_results == *results {
                    ty
                } else {
                    self.function_of(new_args, new_results, *abilities)
                }
            },
            Type::TypeParam { idx } => {
                let table = view_type_list(ty_args);
                *table.get(*idx as usize).ok_or_else(|| {
                    anyhow::anyhow!(
                        "type parameter index {idx} out of bounds: substitution table has {} entries",
                        table.len(),
                    )
                })?
            },
        })
    }

    fn subst_type_list(
        &self,
        tys: InternedTypeList,
        ty_args: InternedTypeList,
    ) -> anyhow::Result<InternedTypeList> {
        if ty_args.is_empty() || tys.is_empty() {
            return Ok(tys);
        }

        let slice = view_type_list(tys);
        let mut changed = false;
        let mut new_tys = Vec::with_capacity(slice.len());
        for &ty in slice {
            let new_ty = self.subst_type(ty, ty_args)?;
            if new_ty != ty {
                changed = true;
            }
            new_tys.push(new_ty);
        }
        if !changed {
            return Ok(tys);
        }
        Ok(self.type_list_of(&new_tys))
    }
}

//
// Only private APIs below.
// ------------------------

impl<'ctx> MaintenanceGuard<'ctx> {
    /// Clears all caches stored in [`Context`]. Triggered when the global
    /// arena requires a full reset (and thus, any cache that stores pointers
    /// to that arena must be invalidated).
    ///
    /// # Safety
    ///
    /// Should be called **before** arena backing allocations is reset or
    /// dropped.
    unsafe fn reset_all_caches(&mut self) {
        // Exhaustive destructuring so that there is a compile-time error if a
        // new field is added without being explicitly handled here.
        //
        // CRITICAL: caches can store pointers to arenas which can be reset, it
        // is important to ensure these caches are cleared before that.
        let Context {
            identifiers,
            module_ids,
            types,
            type_lists,
            function_refs,
            module_cache,
            descriptors,
        } = self.ctx;

        identifiers.clear();
        module_ids.clear();
        types.clear();
        type_lists.clear();
        function_refs.clear();
        descriptors.reset();

        // SAFETY: We are in maintenance phase, and therefore there are no
        // execution guards alive. Hence, there are no pointers to modules
        // alive, and it is safe to free the allocation behind the box.
        unsafe {
            module_cache.clear();
        }
    }
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Returns a reference scoped to the lifetime of the guard.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer points to stable data and will not
    /// be deallocated during guard's lifetime.
    unsafe fn arena_ref<'guard, T: ?Sized>(
        &'guard self,
        ptr: GlobalArenaPtr<T>,
    ) -> ArenaRef<'guard, T>
    where
        'ctx: 'guard,
    {
        ArenaRef {
            ptr,
            _guard: PhantomData,
        }
    }
}

impl<'guard, T: ?Sized> ArenaRef<'guard, T> {
    /// Returns the underlying [`GlobalArenaPtr`] for this arena reference.
    pub fn into_global_arena_ptr(self) -> GlobalArenaPtr<T> {
        self.ptr
    }

    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_raw_ptr().addr()
    }
}

// Arena reference uses pointer hash. Because of interning, pointer hash
// equality implies structural hash equality (ignoring hash collisions).
impl<'guard, T: ?Sized> Hash for ArenaRef<'guard, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
    }
}

// Arena reference uses pointer equality. Because of interning, pointer
// equality implies structural equality.
impl<'guard, T: ?Sized> PartialEq for ArenaRef<'guard, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'guard, T: ?Sized> Eq for ArenaRef<'guard, T> {}

// Arena reference can be duplicated with bitwise copy.
impl<'guard, T: ?Sized> Copy for ArenaRef<'guard, T> {}

impl<'guard, T: ?Sized> Clone for ArenaRef<'guard, T> {
    fn clone(&self) -> Self {
        *self
    }
}
