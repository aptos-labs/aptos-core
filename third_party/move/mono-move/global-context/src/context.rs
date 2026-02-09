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
//! 1. **Execution Phase**: Multiple [`ExecutionContext`] guards held concurrently
//!    - Read-only access to interners via RwLockReadGuard
//!    - Arena allocations are stable (no flush possible)
//!    - Returned pointers valid for guard lifetime
//!
//! 2. **Maintenance Phase**: Single exclusive [`MaintenanceContext`] guard
//!    - Write access via RwLockWriteGuard
//!    - Can flush arena and clear interners
//!    - No execution contexts can coexist
//!
//! ## Allocation Race Window (Intentional Design)
//!
//! When interning, allocation happens **outside the DashMap lock** to reduce
//! contention. This creates a race window where multiple threads may allocate
//! the same identifier. This is intentional and safe:
//!
//! - Only one pointer is stored in the map (Entry API ensures this)
//! - Duplicate allocations leak but are bounded (interning converges)
//! - Trade-off: minor memory waste for lower lock contention
//!
//! ## Why This is Safe
//!
//! - Arena pointers remain stable until flush (guaranteed by allocator)
//! - Flush only happens during maintenance phase (no concurrent execution)
//! - Returned pointer lifetimes tied to guard lifetime (cannot outlive context)
//! - Use-after-flush prevented by Rust's lifetime system
//!
//! ## Critical Safety Invariants
//!
//! ### DashMap with 'static Keys
//!
//! The interner uses `DashMap<&'static T, ArenaPtr<T>>` which stores keys with
//! `'static` lifetime even though the actual arena allocations are not static.
//!
//! **Why this is sound:**
//! 1. `MaintenanceContext::check_memory_usage()` (lines 371-410) clears all DashMaps
//!    BEFORE calling `arena.flush()` (line 391)
//! 2. The RwLock ensures no execution contexts can hold arena references during flush
//! 3. The public API returns lifetime-bound types (`TypePtr<'ctx>`) preventing escape
//!
//! ### Hash/Eq Implementations
//!
//! `TypeKey` and `TypeListKey` hash/compare by dereferencing `ArenaPtr`. This is safe
//! because:
//! 1. These types are only constructed within ExecutionContext
//! 2. Hash/Eq are called synchronously within the same guard scope
//! 3. No suspension points exist between construction and hash/eq
//!
//! ### Public API Safety via Lifetime Bounds
//!
//! ArenaPtr is never exposed publicly. The public API uses wrapper types like
//! `TypePtr<'a>`, `ExecutableIdPtr<'a>`, etc., which contain `ArenaPtr` internally
//! but are lifetime-bound to ExecutionContext.
//!
//! **Safety Argument:** If we have a reference `&'a SomeType` where `SomeType`
//! internally contains `ArenaPtr<T>`, then it is safe to cast the inner `ArenaPtr`
//! to lifetime `'a` because:
//! 1. The outer reference `&'a SomeType` proves the data is valid for lifetime `'a`
//! 2. The lifetime `'a` is tied to the ExecutionContext guard
//! 3. The ExecutionContext guard holds `RwLockReadGuard` preventing flush
//! 4. Therefore, the arena remains valid for the entire lifetime `'a`
//!
//! ### Allocation Race Window
//!
//! Multiple threads may allocate duplicate values during concurrent interning (see
//! lines 22-30). This is intentional: reduces lock contention at the cost of bounded
//! memory waste. Only one pointer wins the DashMap race.

use crate::{
    arena::{ArenaAllocator, ArenaGuard, ArenaPool, ArenaPtr},
    counters,
    interner::{
        DashMapInterner, ExecutableIdKey, IdentifierKey, SignatureTokenKey, SignatureTokenListKey,
        TypeKey, TypeListKey, TypeTagKey, TypeTagListKey,
    },
    types::{ExecutableId, ExecutableIdInternal, FunctionId, StructId, Type, TypeInternal},
    TypeList,
};
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{FunctionParamOrReturnTag, ModuleId, TypeTag},
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Shared context containing interned data structures. This is separate from
/// arena storage to allow independent locking of arenas per worker.
struct Context {
    /// Deduplication map for string identifiers (function names, struct names).
    /// Shared across all identifier types for maximum memory efficiency.
    identifiers: DashMapInterner<IdentifierKey, str>,
    /// Deduplication map for [`ExecutableIdInternal`] pointers. Keys are addresses and
    /// module names pairs that uniquely identify Move modules.
    executable_ids: DashMapInterner<ExecutableIdKey, ExecutableIdInternal>,
    /// Deduplication map for [`TypeInternal`] pointers. Enables cross-format deduplication
    /// of TypeTag and SignatureToken to the same canonical Type.
    types: DashMapInterner<TypeKey, TypeInternal>,
    /// Deduplication map for [`TypeList`] pointers. Enables cross-format deduplication
    /// of type lists from TypeTag and SignatureToken.
    type_lists: DashMapInterner<TypeListKey, [ArenaPtr<TypeInternal>]>,
    /// Configuration parameters controlling memory usage thresholds and flush
    /// behavior.
    config: GlobalContextConfig,
}

/// Configuration for [`GlobalContext`].
#[derive(Clone)]
pub struct GlobalContextConfig {
    pub memory_threshold_bytes: usize,
}

impl Default for GlobalContextConfig {
    fn default() -> Self {
        Self {
            memory_threshold_bytes: 100 * 1024 * 1024,
        }
    }
}

/// Global execution context with a two-phase state machine.
///
/// # Phases
///
/// 1. **Execution Phase**: Multiple [`ExecutionContext`] guards can be obtained
///    concurrently across threads. Each worker gets a dedicated arena for
///    lock-free allocation. This allows parallel transaction execution where
///    each thread can read from shared caches and allocate without contention.
///
/// 2. **Maintenance Phase**: A single [`MaintenanceContext`] guard provides
///    exclusive write access for inter-block maintenance operations such as
///    cache cleanup or data de-allocation.
pub struct GlobalContext {
    /// Shared interned data structures protected by read-write lock.
    shared: RwLock<Context>,
    /// Per-worker arenas. Each worker gets exclusive access to its arena.
    arenas: ArenaPool,
}

impl Default for GlobalContext {
    fn default() -> Self {
        Self::with_config(GlobalContextConfig::default())
    }
}

impl GlobalContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: GlobalContextConfig) -> Self {
        Self::with_num_workers(8, config)
    }

    /// Creates a new GlobalContext with a specific number of worker arenas.
    ///
    /// # Arguments
    ///
    /// * `num_workers` - Number of worker arenas to create (1-128).
    /// * `config` - Configuration for memory thresholds.
    ///
    /// # Panics
    ///
    /// Panics if `num_workers` is 0 or greater than 128.
    pub fn with_num_workers(num_workers: usize, config: GlobalContextConfig) -> Self {
        assert!(
            num_workers > 0 && num_workers <= 128,
            "num_workers must be between 1 and 128, got {num_workers}"
        );
        assert!(
            num_workers.is_power_of_two(),
            "num_workers must be a power of two, got {num_workers}"
        );

        Self {
            shared: RwLock::new(Context {
                identifiers: DashMapInterner::default(),
                executable_ids: DashMapInterner::default(),
                types: DashMapInterner::default(),
                type_lists: DashMapInterner::default(),
                config,
            }),
            arenas: ArenaPool::with_num_arenas(num_workers),
        }
    }

    /// Transitions to execution mode by obtaining a read guard and locking
    /// the arena for the given worker_id.
    ///
    /// Multiple execution contexts can be held concurrently across threads,
    /// each with a dedicated arena for lock-free allocation.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The worker ID (0 to num_arenas-1).
    ///
    /// # Returns
    ///
    /// `None` if:
    /// - `worker_id >= num_arenas`
    /// - Arena for this worker_id is already in use
    /// - A [`MaintenanceContext`] is currently held
    pub fn execution_context(
        &self,
        worker_id: usize,
    ) -> Option<ExecutionContext<'_, ArenaGuard<'_>>> {
        if worker_id >= self.arenas.num_arenas() {
            return None;
        }

        let shared_guard = self.shared.try_read()?;
        let arena = self.arenas.lock_arena(worker_id)?;

        Some(ExecutionContext {
            shared_guard,
            arena,
        })
    }

    /// Transitions to maintenance mode by obtaining a write guard.
    ///
    /// Only one maintenance context can be held at a time, providing
    /// exclusive access to the internal state for maintenance operations.
    ///
    /// # Returns
    ///
    /// `None` if any [`ExecutionContext`] is currently held.
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
pub struct ExecutionContext<'a, A: ArenaAllocator> {
    /// Read guard on shared interned data (prevents maintenance phase).
    shared_guard: RwLockReadGuard<'a, Context>,
    /// Arena where context can allocate data.
    arena: A,
}

impl<'a, A: ArenaAllocator> ExecutionContext<'a, A> {
    /// Interns a [`ModuleId`] and returns a stable pointer [`ExecutableId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_module_id<'b>(&'b self, module_id: &ModuleId) -> ExecutableId<'b>
    where
        'a: 'b,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns a [`AccountAddress`]-[`Identifier`] pair corresponding to Move module
    /// and returns a stable pointer [`ExecutableId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_address_name<'b>(
        &'b self,
        addr: &AccountAddress,
        name: &Identifier,
    ) -> ExecutableId<'b>
    where
        'a: 'b,
    {
        let id = self.intern_address_name_internal(addr, name);
        // SAFETY: We're in ExecutionContext with lifetime 'b. The 'b lifetime is tied
        // to our guard, which prevents flush. Therefore it's safe to cast ptr to lifetime 'b.
        ExecutableId::new_internal(unsafe { id.as_ref_unchecked() })
    }

    /// Interns a function name (as a string) and returns a stable pointer [`FunctionId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_function_name<'b>(&'b self, name: &Identifier) -> FunctionId<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_str_internal(name.as_str());
        // SAFETY: We're in ExecutionContext with lifetime 'b. The 'b lifetime is tied
        // to our guard, which prevents flush. Therefore it's safe to cast ptr to lifetime 'b.
        FunctionId::new_internal(unsafe { ptr.as_ref_unchecked() })
    }

    /// Interns a struct name (as an Identifier) and returns a stable pointer [`StructId`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_struct_name<'b>(&'b self, name: &Identifier) -> StructId<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_str_internal(name.as_str());
        // SAFETY: We're in ExecutionContext with lifetime 'b. The 'b lifetime is tied
        // to our guard, which prevents flush. Therefore it's safe to cast ptr to lifetime 'b.
        StructId::new_internal(unsafe { ptr.as_ref_unchecked() })
    }

    /// Interns a [`TypeTag`] and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_type_tag<'b>(&'b self, type_tag: &TypeTag) -> Type<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_type_tag_internal(type_tag);

        // SAFETY:
        //   1. Arena keeps allocation alive until flush (which happens only in
        //      maintenance mode).
        //   2. We are in execution context, preventing maintenance mode and flush.
        //   3. Returning the lifetime 'b (guard lifetime) is therefore safe.
        let ty = unsafe { ptr.as_ref_unchecked() };
        Type::new_internal(ty)
    }

    /// Interns a list of [`TypeTag`]s and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_type_tags<'b>(&'b self, type_tags: &[TypeTag]) -> TypeList<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_type_tags_internal(type_tags);

        // SAFETY:
        //   1. Arena keeps allocation alive until flush (which happens only in
        //      maintenance mode).
        //   2. We are in execution context, preventing maintenance mode and flush.
        //   3. Returning the lifetime 'b (guard lifetime) is therefore safe.
        let ty = unsafe { ptr.as_ref_unchecked() };
        TypeList::new_internal(ty)
    }

    /// Interns a [`SignatureToken`] and returns a stable pointer [`Type`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    pub fn intern_signature_token<'b>(
        &'b self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> Type<'b>
    where
        'a: 'b,
    {
        let ptr = self.intern_signature_token_internal(token, module);

        // SAFETY:
        //   1. Arena keeps allocation alive until flush (which happens only in
        //      maintenance mode).
        //   2. We are in execution context, preventing maintenance mode and flush.
        //   3. Returning the lifetime 'b (guard lifetime) is therefore safe.
        let ty = unsafe { ptr.as_ref_unchecked() };
        Type::new_internal(ty)
    }
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when dropped.
pub struct MaintenanceContext<'a> {
    /// Write guard on shared interned data (prevents execution contexts).
    shared_guard: RwLockWriteGuard<'a, Context>,
    /// Reference to all arenas for flushing and metrics.
    arenas: &'a ArenaPool,
}

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

    /// Checks if interner's memory consumption is within the limits specified
    /// in [`GlobalContextConfig`]. If limits are exceeded, all caches and the
    /// arenas are flushed. Returns true if there was a flush, and false otherwise.
    pub fn check_memory_usage(&mut self) -> bool {
        let interner_arena_allocated_bytes = self.interner_arena_allocated_bytes();
        counters::set_interner_arena_allocated_bytes(interner_arena_allocated_bytes);
        counters::set_interned_executable_id_count(self.interned_executable_ids_count());
        counters::set_interned_identifier_count(self.interned_identifiers_count());
        counters::set_interned_type_count(self.interned_type_count());
        counters::set_interned_type_list_count(self.interned_type_list_count());

        if interner_arena_allocated_bytes >= self.shared_guard.config.memory_threshold_bytes {
            self.shared_guard.identifiers.clear();
            self.shared_guard.executable_ids.clear();
            self.shared_guard.types.clear();
            self.shared_guard.type_lists.clear();

            // SAFETY:
            //   1. While in maintenance context, there can be no execution contexts
            //      alive at the same time, hence, there are no pointers to the arenas
            //      other than stored in interner's maps.
            //   2. Interner's maps have been flushed.
            unsafe {
                self.arenas.flush();
            }
            counters::set_interner_arena_allocated_bytes(self.interner_arena_allocated_bytes());
            counters::set_interned_executable_id_count(0);
            counters::set_interned_identifier_count(0);
            counters::set_interned_type_count(0);
            counters::set_interned_type_list_count(0);
            counters::inc_interner_flush_count();

            // Also reset number of cache misses for interner.
            counters::reset_executable_id_interner_cache_miss();
            counters::reset_identifier_interner_cache_miss();
            counters::reset_type_interner_cache_miss();
            counters::reset_type_list_interner_cache_miss();

            return true;
        }

        false
    }
}

// Private APIs.
impl<'a, A: ArenaAllocator> ExecutionContext<'a, A> {
    /// Interns a string and returns a stable pointer to it. For internal usage only.
    fn intern_str_internal(&self, name: &str) -> ArenaPtr<str> {
        if let Some(ptr) = self.shared_guard.identifiers.get(name) {
            return ptr;
        }
        counters::log_identifier_interner_cache_miss();
        let ptr = self.arena.alloc_str(name);
        self.shared_guard.identifiers.insert(ptr)
    }

    fn intern_address_name_internal(
        &self,
        addr: &AccountAddress,
        name: &IdentStr,
    ) -> ArenaPtr<ExecutableIdInternal> {
        if let Some(ptr) = self.shared_guard.executable_ids.get(&(addr, name)) {
            return ptr;
        }
        counters::log_executable_id_interner_cache_miss();
        let name = self.intern_str_internal(name.as_str());
        let ptr = self.arena.alloc(ExecutableIdInternal {
            address: *addr,
            name,
        });
        self.shared_guard.executable_ids.insert(ptr)
    }

    // SAFETY: This method is private. Callers ensure the returned ArenaPtr is only
    // dereferenced within ExecutionContext lifetime. The public API (intern_type_tag)
    // returns TypePtr<'ctx> which is lifetime-bound to the ExecutionContext guard.
    fn intern_type_tag_internal(&self, type_tag: &TypeTag) -> ArenaPtr<TypeInternal> {
        if let Some(ptr) = self.shared_guard.types.get(&TypeTagKey(type_tag)) {
            return ptr;
        }

        counters::log_type_interner_cache_miss();

        let ptr = match type_tag {
            TypeTag::Bool => self.arena.alloc(TypeInternal::Bool),
            TypeTag::U8 => self.arena.alloc(TypeInternal::U8),
            TypeTag::U16 => self.arena.alloc(TypeInternal::U16),
            TypeTag::U32 => self.arena.alloc(TypeInternal::U32),
            TypeTag::U64 => self.arena.alloc(TypeInternal::U64),
            TypeTag::U128 => self.arena.alloc(TypeInternal::U128),
            TypeTag::U256 => self.arena.alloc(TypeInternal::U256),
            TypeTag::I8 => self.arena.alloc(TypeInternal::I8),
            TypeTag::I16 => self.arena.alloc(TypeInternal::I16),
            TypeTag::I32 => self.arena.alloc(TypeInternal::I32),
            TypeTag::I64 => self.arena.alloc(TypeInternal::I64),
            TypeTag::I128 => self.arena.alloc(TypeInternal::I128),
            TypeTag::I256 => self.arena.alloc(TypeInternal::I256),
            TypeTag::Address => self.arena.alloc(TypeInternal::Address),
            TypeTag::Signer => self.arena.alloc(TypeInternal::Signer),

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
                let results =
                    self.intern_function_param_or_return_tags_internal(function_tag.args.as_ref());
                self.arena.alloc(TypeInternal::Function {
                    args,
                    results,
                    abilities: function_tag.abilities,
                })
            },
        };

        self.shared_guard.types.insert(ptr)
    }

    fn intern_function_param_or_return_tags_internal(
        &self,
        tags: &[FunctionParamOrReturnTag],
    ) -> ArenaPtr<[ArenaPtr<TypeInternal>]> {
        let tags = tags
            .iter()
            .map(|arg| match arg {
                FunctionParamOrReturnTag::Reference(tag) => {
                    let ptr = self
                        .arena
                        .alloc(TypeInternal::Ref(self.intern_type_tag_internal(tag)));
                    self.arena.alloc(TypeInternal::Ref(ptr))
                },
                FunctionParamOrReturnTag::MutableReference(tag) => {
                    let ptr = self
                        .arena
                        .alloc(TypeInternal::Ref(self.intern_type_tag_internal(tag)));
                    self.arena.alloc(TypeInternal::RefMut(ptr))
                },
                FunctionParamOrReturnTag::Value(tag) => self.intern_type_tag_internal(tag),
            })
            .collect::<Vec<_>>();
        self.arena.alloc_slice_copy(&tags)
    }

    // SAFETY: This method is private. Callers ensure the returned ArenaPtr is only
    // dereferenced within ExecutionContext lifetime. The public API (intern_type_tags)
    // returns TypeListPtr<'ctx> which is lifetime-bound to the ExecutionContext guard.
    fn intern_type_tags_internal(&self, tags: &[TypeTag]) -> ArenaPtr<[ArenaPtr<TypeInternal>]> {
        if let Some(ptr) = self.shared_guard.type_lists.get(&TypeTagListKey(tags)) {
            return ptr;
        }

        counters::log_type_list_interner_cache_miss();

        let types = tags
            .iter()
            .map(|tag| self.intern_type_tag_internal(tag))
            .collect::<Vec<_>>();
        let ptr = self.arena.alloc_slice_copy(&types);
        self.shared_guard.type_lists.insert(ptr)
    }

    // SAFETY: This method is private. Callers ensure the returned ArenaPtr is only
    // dereferenced within ExecutionContext lifetime. The public API (intern_signature_token)
    // returns TypePtr<'ctx> which is lifetime-bound to the ExecutionContext guard.
    fn intern_signature_token_internal(
        &self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> ArenaPtr<TypeInternal> {
        if let Some(ptr) = self
            .shared_guard
            .types
            .get(&SignatureTokenKey(token, module))
        {
            return ptr;
        }

        counters::log_type_interner_cache_miss();

        let ptr = match token {
            SignatureToken::Bool => self.arena.alloc(TypeInternal::Bool),
            SignatureToken::U8 => self.arena.alloc(TypeInternal::U8),
            SignatureToken::U16 => self.arena.alloc(TypeInternal::U16),
            SignatureToken::U32 => self.arena.alloc(TypeInternal::U32),
            SignatureToken::U64 => self.arena.alloc(TypeInternal::U64),
            SignatureToken::U128 => self.arena.alloc(TypeInternal::U128),
            SignatureToken::U256 => self.arena.alloc(TypeInternal::U256),
            SignatureToken::I8 => self.arena.alloc(TypeInternal::I8),
            SignatureToken::I16 => self.arena.alloc(TypeInternal::I16),
            SignatureToken::I32 => self.arena.alloc(TypeInternal::I32),
            SignatureToken::I64 => self.arena.alloc(TypeInternal::I64),
            SignatureToken::I128 => self.arena.alloc(TypeInternal::I128),
            SignatureToken::I256 => self.arena.alloc(TypeInternal::I256),
            SignatureToken::Address => self.arena.alloc(TypeInternal::Address),
            SignatureToken::Signer => self.arena.alloc(TypeInternal::Signer),

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

            SignatureToken::TypeParameter(_) => {
                panic!("Type parameters cannot be interned")
            },
        };

        self.shared_guard.types.insert(ptr)
    }

    /// Interns a list of [`SignatureToken`]s and returns a stable pointer [`TypeList`].
    ///
    /// The returned pointer's lifetime is tied to this execution context's lifetime.
    ///
    /// # Panics
    ///
    /// Panics if any token is a TypeParameter.
    ///
    /// # Safety
    ///
    /// This method is private. Callers ensure the returned ArenaPtr is only dereferenced
    /// within ExecutionContext lifetime.
    fn intern_signature_tokens_internal(
        &self,
        tokens: &[SignatureToken],
        module: &CompiledModule,
    ) -> ArenaPtr<[ArenaPtr<TypeInternal>]> {
        if let Some(ptr) = self
            .shared_guard
            .type_lists
            .get(&SignatureTokenListKey(tokens, module))
        {
            return ptr;
        }

        counters::log_type_list_interner_cache_miss();

        let types = tokens
            .iter()
            .map(|token| self.intern_signature_token_internal(token, module))
            .collect::<Vec<_>>();
        let ptr = self.arena.alloc_slice_copy(&types);
        self.shared_guard.type_lists.insert(ptr)
    }
}
