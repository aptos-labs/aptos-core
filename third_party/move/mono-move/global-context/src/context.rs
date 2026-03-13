// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
//!
//! ## Global Allocation Race Window
//!
//! When interning, allocation happens **outside the [`dashmap::DashMap`]
//! lock** to reduce contention. This creates a race window where multiple
//! threads may allocate the same interned data. This is intentional and safe:
//!
//!   - Only one pointer is stored in the interner's map.
//!   - Duplicate allocations leak but are bounded (interning converges).
//!   - Trade-off: minor memory waste for lower lock contention.

mod identifiers;

pub use identifiers::ExecutableId;
use std::hash::{Hash, Hasher};
mod interner;
mod types;
pub(crate) use crate::{
    alloc::{GlobalArenaPtr, GlobalArenaShard},
    context::{
        interner::DashMapInterner,
        types::{
            ADDRESS, BOOL, I128, I16, I256, I32, I64, I8, SIGNER, U128, U16, U256, U32, U64, U8,
        },
    },
    maintenance_config::MaintenanceConfig,
    GlobalArenaPool,
};
use dashmap::{DashMap, Entry};
use fxhash::FxBuildHasher;
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{FunctionParamOrReturnTag, ModuleId, TypeTag},
};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData;
pub use types::Type;

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
    /// Shared caches storing interned data, executables.
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
    identifiers: DashMapInterner<str>,
    executable_ids: DashMapInterner<ExecutableId>,
    types: DashMapInterner<Type>,
    type_lists: DashMapInterner<[GlobalArenaPtr<Type>]>,
    /// Cache for type substitution. Maps (generic type, type argument list)
    /// to its canonical fully-instantiated type. Both keys use interned
    /// pointers, so the lookup is O(1) after the first computation.
    type_subst_cache: DashMap<
        (GlobalArenaPtr<Type>, GlobalArenaPtr<[GlobalArenaPtr<Type>]>),
        GlobalArenaPtr<Type>,
        FxBuildHasher,
    >,
    /// Cache for type list substitution. Maps (input list ptr, ty_args ptr)
    /// to the result list ptr. Result equals input when no element changed,
    /// so callers detect no-change via pointer identity.
    type_list_subst_cache: DashMap<
        (
            GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
            GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        ),
        GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        FxBuildHasher,
    >,
}

/// RAII guard for the maintenance phase providing exclusive write access.
///
/// Only one maintenance context can exist at a time, ensuring exclusive
/// access to the internal state for maintenance operations. The write lock
/// is held for the lifetime of this guard and automatically released when
/// dropped.
pub struct MaintenanceGuard<'a> {
    /// Reference to the caches stored in context.
    ctx: &'a Context,
    /// Pool of all arenas managing global allocations.
    global_arena: &'a GlobalArenaPool,
    /// Configuration controlling maintenance behavior.
    #[allow(dead_code)]
    maintenance_config: &'a MaintenanceConfig,

    /// Write guard that disallows obtaining concurrent execution
    /// guard. **Must** be dropped last.
    _guard: RwLockWriteGuard<'a, ()>,
}

/// RAII guard for the execution phase providing concurrent read access
/// to shared interned data and exclusive access to a dedicated arena.
/// Multiple execution contexts can exist simultaneously across threads.
pub struct ExecutionGuard<'a> {
    /// Reference to the caches stored in context.
    #[allow(dead_code)]
    ctx: &'a Context,
    /// Arena dedicated for this execution guard with exclusive access.
    /// During execution, data can be allocated here without contention.
    #[allow(dead_code)]
    global_arena: GlobalArenaShard<'a>,

    /// Read guard preventing maintenance phase, but allowing concurrent
    /// execution phases. **Must** be dropped last.
    _guard: RwLockReadGuard<'a, ()>,
}

/// Scoped reference returned by public [`ExecutionGuard`] APIs. The lifetime
/// enforces compile-time guarantee that the execution guard is alive when
/// holding the reference. Hence, there is no way to invalidate the underlying
/// pointer because only [`MaintenanceGuard`] can deallocate, but it cannot be
/// acquired as the [`ExecutionGuard`] is held.
pub struct Ref<'a, T: ?Sized> {
    ptr: GlobalArenaPtr<T>,
    _guard: PhantomData<&'a ()>,
}

impl<'a, T: ?Sized> Ref<'a, T> {
    /// Casts this reference to a raw pointer.
    pub fn as_raw_ptr(&self) -> *const T {
        self.ptr.as_raw_ptr()
    }
}

impl<'a, T: ?Sized> Hash for Ref<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a, T: ?Sized> PartialEq for Ref<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a, T: ?Sized> Eq for Ref<'a, T> {}

impl<'a, T: ?Sized> Copy for Ref<'a, T> {}

impl<'a, T: ?Sized> Clone for Ref<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// Scoped reference to a list of data returned by public [`ExecutionGuard`]
/// APIs. The lifetime enforces compile-time guarantee that the execution guard
/// is alive when holding the reference, similar to [`Ref`].
pub struct ListRef<'a, T> {
    ptr: GlobalArenaPtr<[GlobalArenaPtr<T>]>,
    _guard: PhantomData<&'a ()>,
}

impl<'a, T> Hash for ListRef<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a, T> PartialEq for ListRef<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a, T> Eq for ListRef<'a, T> {}

impl<'a, T> Copy for ListRef<'a, T> {}

impl<'a, T> Clone for ListRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: 'a> ListRef<'a, T> {
    fn as_slice(&self) -> &'a [GlobalArenaPtr<T>] {
        // SAFETY:
        //
        // We already hold the reference to the list, so it guarantees that the
        // execution guard is alive, and the arena has not been reset. This
        // implies that the pointer is still valid.
        unsafe { self.ptr.as_ref_unchecked() }
    }

    /// Returns the number of elements in the list.
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Returns true if the list contains no elements.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Returns a reference to the element at the specified index, or [`None`]
    /// if out of bounds.
    pub fn get(&self, idx: usize) -> Option<Ref<'a, T>> {
        self.as_slice().get(idx).map(|ptr| Ref {
            ptr: *ptr,
            _guard: PhantomData,
        })
    }

    /// Returns an iterator over the elements in the list.
    pub fn iter(&self) -> impl Iterator<Item = Ref<'a, T>> + 'a {
        self.as_slice().iter().map(|ptr| Ref {
            ptr: *ptr,
            _guard: PhantomData,
        })
    }
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
                identifiers: DashMapInterner::default(),
                executable_ids: DashMapInterner::default(),
                types: DashMapInterner::default(),
                type_lists: DashMapInterner::default(),
                type_subst_cache: DashMap::with_hasher(FxBuildHasher::default()),
                type_list_subst_cache: DashMap::with_hasher(FxBuildHasher::default()),
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
    pub fn try_execution_context(&self, worker_id: usize) -> Option<ExecutionGuard<'_>> {
        let _guard = self.phase.try_read()?;

        Some(ExecutionGuard {
            ctx: &self.ctx,
            global_arena: self.global_arena.try_lock_arena(worker_id)?,
            _guard,
        })
    }
}

impl<'a> MaintenanceGuard<'a> {
    /// Returns the total number of bytes used across all arenas in the global
    /// arena pool. Note that resetting the arena does not mean that this
    /// number goes to zero - while data is cleared the allocation can still
    /// be kept alive.
    pub fn global_arena_allocated_bytes_sum(&self) -> usize {
        (0..self.global_arena.num_arenas())
            .map(|idx| self.global_arena.allocated_bytes(idx))
            .sum()
    }

    /// Returns the number of entries in interner's map for identifiers.
    pub fn interned_identifiers_count(&self) -> usize {
        self.ctx.identifiers.len()
    }

    /// Returns the number of entries in interner's map for executable IDs.
    pub fn interned_executable_ids_count(&self) -> usize {
        self.ctx.executable_ids.len()
    }

    /// Returns the number of entries in the types interner.
    pub fn interned_types_count(&self) -> usize {
        self.ctx.types.len()
    }

    /// Returns the number of entries in the type lists interner.
    pub fn interned_type_lists_count(&self) -> usize {
        self.ctx.type_lists.len()
    }

    /// Resets all caches that store pointers to the arenas, and then resets
    /// the arenas as well.
    pub fn reset_arena_pool(&mut self) {
        // SAFETY: Arena is only reset **after**, so clearing all caches is
        // safe.
        unsafe {
            self.reset_all_caches();
        }

        // SAFETY: We are in maintenance phase, so there are no concurrent
        // execution contexts and therefore no live pointers to arena other
        // than ones that were stored in caches. All caches were cleared (see
        // above), and so there are no live pointers making reset safe.
        unsafe {
            self.global_arena.reset_unchecked();
        }
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns Move identifier as a string and returns a reference to it. The
    /// reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_identifier<'b>(&'b self, identifier: &IdentStr) -> Ref<'b, str>
    where
        'a: 'b,
    {
        // TODO:
        //   Consider checking that the identifier size is within bounds. While
        //   CompiledModule / CompiledScript deserializer enforces 256 byte
        //   limit (in new config), when coming from deserialized TypeTag from
        //   transaction payload there is no bound. It is not a big problem,
        //   but just makes spam attacks easier to intern some dummy data in the
        //   pool. In general, for type tag interning we might want to enforce
        //   that the modules which are specified actually exist on-chain. In
        //   existing VM we already do that to get ability information, but not
        //   here (for now), so that we ensure that there is no spam that can
        //   get in. However, there still can be a problem with speculative
        //   module publishing: if we speculatively intern new names, but the
        //   publish actually fails, we end up with spam on-chain.
        //   Note: this DoS is only possible via `init_module`. If we remove it
        //   or ensure no speculative data even for names ever get on-chain, we
        //   limit interned set to the on-chain data, so for DoS one actually
        //   has to publish modules (expensive).
        let str = identifier.as_str();

        if let Some(ptr) = self.ctx.identifiers.get(str) {
            // SAFETY: We read the pointer from the interner's map, so it must
            // have been allocated and is still valid **provided** global arena
            // has not been flushed. The maintenance guard ensures all caches
            // are flushed. Hence, we can use the guard's lifetime for it.
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        };

        let Ref { ptr, _guard } = self.alloc_str(str);
        Ref {
            // SAFETY: We have just allocated this pointer. Hence, it is safe
            // to dereference its contents when inserting into the interner.
            ptr: unsafe { self.ctx.identifiers.insert(ptr) },
            _guard,
        }
    }

    /// Interns [`ModuleId`] as [`ExecutableId`] and returns a reference to it.
    /// The reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_module_id<'b>(&'b self, module_id: &ModuleId) -> Ref<'b, ExecutableId>
    where
        'a: 'b,
    {
        self.intern_address_name(&module_id.address, &module_id.name)
    }

    /// Interns [`AccountAddress`]-[`Identifier`] pair as [`ExecutableId`] and
    /// returns a reference to it. The reference is valid for the lifetime of
    /// the [`ExecutionGuard`].
    pub fn intern_address_name<'b>(
        &'b self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> Ref<'b, ExecutableId>
    where
        'a: 'b,
    {
        if let Some(ptr) = self.ctx.executable_ids.get(&(address, name)) {
            // SAFETY: We read the pointer from the interner's map, so it must
            // have been allocated and is still valid **provided** global arena
            // has not been flushed. The maintenance guard ensures all caches
            // are flushed. Hence, we can use the guard's lifetime for it.
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        };

        let name = self.intern_identifier(name);
        let Ref { ptr, _guard } = self.alloc_executable_id(*address, name);
        Ref {
            // SAFETY: We have just allocated this pointer. Hence, it is safe
            // to dereference its contents when inserting into the interner.
            ptr: unsafe { self.ctx.executable_ids.insert(ptr) },
            _guard,
        }
    }

    /// Interns a list of [`TypeTag`]s and returns a reference to it. The
    /// reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_type_tags<'b>(&'b self, tags: &[TypeTag]) -> ListRef<'b, Type>
    where
        'a: 'b,
    {
        if let Some(ptr) = self.ctx.type_lists.get(tags) {
            return ListRef {
                ptr,
                _guard: PhantomData,
            };
        }

        let types = tags
            .iter()
            .map(|tag| {
                let Ref { ptr, .. } = self.intern_type_tag(tag);
                ptr
            })
            .collect::<Vec<_>>();
        let ptr = self.global_arena.alloc_slice_copy(&types);
        ListRef {
            ptr: unsafe { self.ctx.type_lists.insert(ptr) },
            _guard: PhantomData,
        }
    }

    /// Interns a [`TypeTag`] and returns a reference to it. The reference is
    /// valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_type_tag<'b>(&'b self, type_tag: &TypeTag) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        match type_tag {
            TypeTag::Bool => return static_type_ref(&BOOL),
            TypeTag::U8 => return static_type_ref(&U8),
            TypeTag::U16 => return static_type_ref(&U16),
            TypeTag::U32 => return static_type_ref(&U32),
            TypeTag::U64 => return static_type_ref(&U64),
            TypeTag::U128 => return static_type_ref(&U128),
            TypeTag::U256 => return static_type_ref(&U256),
            TypeTag::I8 => return static_type_ref(&I8),
            TypeTag::I16 => return static_type_ref(&I16),
            TypeTag::I32 => return static_type_ref(&I32),
            TypeTag::I64 => return static_type_ref(&I64),
            TypeTag::I128 => return static_type_ref(&I128),
            TypeTag::I256 => return static_type_ref(&I256),
            TypeTag::Address => return static_type_ref(&ADDRESS),
            TypeTag::Signer => return static_type_ref(&SIGNER),
            // Composites types; handle below.
            TypeTag::Vector(_) | TypeTag::Struct(_) | TypeTag::Function(_) => {},
        }

        if let Some(ptr) = self.ctx.types.get(type_tag) {
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        }

        let Ref { ptr, _guard } = match type_tag {
            TypeTag::Vector(elem_tag) => {
                let elem_type = self.intern_type_tag(elem_tag.as_ref());
                self.alloc_vector_type(elem_type)
            },

            TypeTag::Struct(struct_tag) => {
                let executable_id =
                    self.intern_address_name(&struct_tag.address, &struct_tag.module);
                let name = self.intern_identifier(&struct_tag.name);
                let type_args = self.intern_type_tags(&struct_tag.type_args);
                self.alloc_struct_type(executable_id, name, type_args)
            },

            TypeTag::Function(function_tag) => {
                let args = self.intern_function_param_or_return_tags(function_tag.args.as_ref());
                let results =
                    self.intern_function_param_or_return_tags(function_tag.results.as_ref());
                self.alloc_function_type(args, results, function_tag.abilities)
            },

            // TODO: exhaustive match
            _ => unreachable!("Primitives are already handled"),
        };

        Ref {
            ptr: unsafe { self.ctx.types.insert(ptr) },
            _guard,
        }
    }

    /// Interns a slice of [`SignatureToken`]s and returns a scoped list
    /// reference valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_signature_tokens<'b>(
        &'b self,
        tokens: &[SignatureToken],
        module: &CompiledModule,
    ) -> ListRef<'b, Type>
    where
        'a: 'b,
    {
        if let Some(ptr) = self.ctx.type_lists.get(&(tokens, module)) {
            return ListRef {
                ptr,
                _guard: PhantomData,
            };
        }

        let types = tokens
            .iter()
            .map(|token| {
                let Ref { ptr, .. } = self.intern_signature_token(token, module);
                ptr
            })
            .collect::<Vec<_>>();
        let ptr = self.global_arena.alloc_slice_copy(&types);
        ListRef {
            ptr: unsafe { self.ctx.type_lists.insert(ptr) },
            _guard: PhantomData,
        }
    }

    /// Interns a single [`SignatureToken`] and returns a scoped reference
    /// valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_signature_token<'b>(
        &'b self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        match token {
            SignatureToken::Bool => return static_type_ref(&BOOL),
            SignatureToken::U8 => return static_type_ref(&U8),
            SignatureToken::U16 => return static_type_ref(&U16),
            SignatureToken::U32 => return static_type_ref(&U32),
            SignatureToken::U64 => return static_type_ref(&U64),
            SignatureToken::U128 => return static_type_ref(&U128),
            SignatureToken::U256 => return static_type_ref(&U256),
            SignatureToken::I8 => return static_type_ref(&I8),
            SignatureToken::I16 => return static_type_ref(&I16),
            SignatureToken::I32 => return static_type_ref(&I32),
            SignatureToken::I64 => return static_type_ref(&I64),
            SignatureToken::I128 => return static_type_ref(&I128),
            SignatureToken::I256 => return static_type_ref(&I256),
            SignatureToken::Address => return static_type_ref(&ADDRESS),
            SignatureToken::Signer => return static_type_ref(&SIGNER),
            // Composites types; handle below.
            // TODO; exhaustive match
            _ => {},
        }

        if let Some(ptr) = self.ctx.types.get(&(token, module)) {
            return Ref {
                ptr,
                _guard: PhantomData,
            };
        }

        let Ref { ptr, _guard } = match token {
            SignatureToken::Vector(tok) => {
                let elem_type = self.intern_signature_token(tok.as_ref(), module);
                self.alloc_vector_type(elem_type)
            },

            SignatureToken::Reference(tok) => {
                let inner = self.intern_signature_token(tok.as_ref(), module);
                self.alloc_ref_type(inner)
            },

            SignatureToken::MutableReference(tok) => {
                let inner = self.intern_signature_token(tok.as_ref(), module);
                self.alloc_ref_mut_type(inner)
            },

            SignatureToken::Struct(idx) => {
                let struct_handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(struct_handle.module);

                let module_addr = module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name);
                let executable_id = self.intern_address_name(module_addr, module_name);
                let name = self.intern_identifier(module.identifier_at(struct_handle.name));
                let type_args = self.intern_signature_tokens(&[], module);

                self.alloc_struct_type(executable_id, name, type_args)
            },

            SignatureToken::StructInstantiation(idx, type_args) => {
                let struct_handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(struct_handle.module);

                let module_addr = module.address_identifier_at(module_handle.address);
                let module_name = module.identifier_at(module_handle.name);
                let executable_id = self.intern_address_name(module_addr, module_name);
                let name = self.intern_identifier(module.identifier_at(struct_handle.name));
                let type_args = self.intern_signature_tokens(type_args, module);

                self.alloc_struct_type(executable_id, name, type_args)
            },

            SignatureToken::Function(args, results, abilities) => {
                let args = self.intern_signature_tokens(args, module);
                let results = self.intern_signature_tokens(results, module);
                self.alloc_function_type(args, results, *abilities)
            },

            SignatureToken::TypeParameter(idx) => Ref {
                ptr: self.global_arena.alloc(Type::TypeParam(*idx)),
                _guard: PhantomData,
            },

            // TODO: exhaustive match
            _ => unreachable!("Primitives are already handled"),
        };

        Ref {
            ptr: unsafe { self.ctx.types.insert(ptr) },
            _guard,
        }
    }

    /// Substitutes type parameters in each element of `tys`. Returns `tys`
    /// unchanged when no element changed, or a new interned list otherwise.
    /// Callers detect no-change via pointer identity (`result.ptr == tys.ptr`).
    ///
    /// Caches the result under `(tys.ptr, ty_args.ptr)` so repeated calls for
    /// the same (list, ty_args) pair — common when many struct/function types
    /// share the same interned type_args list — are O(1).
    ///
    /// Uses a two-phase approach to avoid any heap allocation on the unchanged
    /// path and any extra branching in the build phase:
    ///   Phase 1 — scan without allocating until the first changed element.
    ///   Phase 2 — build the output Vec starting from the first change; all
    ///             remaining elements are pushed directly with no Option
    ///             overhead.
    fn substitute_type_list<'b>(
        &'b self,
        tys: ListRef<'b, Type>,
        ty_args: ListRef<'b, Type>,
    ) -> ListRef<'b, Type>
    where
        'a: 'b,
    {
        if let Some(cached) = self.ctx.type_list_subst_cache.get(&(tys.ptr, ty_args.ptr)) {
            return ListRef {
                ptr: *cached.value(),
                _guard: PhantomData,
            };
        }

        // Phase 1: scan without allocating until the first changed element.
        let result_ptr = 'scan: {
            let mut iter = tys.iter().enumerate();
            let (first_idx, first_new) = loop {
                let Some((i, ty)) = iter.next() else {
                    // No element changed.
                    break 'scan tys.ptr;
                };
                let new_ty = self.substitute_type(ty, ty_args);
                if new_ty != ty {
                    break (i, new_ty);
                }
            };

            // Phase 2: at least one element changed — build a new list.
            let mut new_tys = Vec::with_capacity(tys.len());
            new_tys.extend_from_slice(&tys.as_slice()[..first_idx]);
            let Ref { ptr, .. } = first_new;
            new_tys.push(ptr);
            for (_, ty) in iter {
                let Ref { ptr, .. } = self.substitute_type(ty, ty_args);
                new_tys.push(ptr);
            }

            let new_tys = self.global_arena.alloc_slice_copy(&new_tys);
            unsafe { self.ctx.type_lists.insert(new_tys) }
        };

        match self.ctx.type_list_subst_cache.entry((tys.ptr, ty_args.ptr)) {
            Entry::Occupied(e) => {
                assert!(*e.get() == result_ptr);
            },
            Entry::Vacant(e) => {
                assert!(*e.insert(result_ptr) == result_ptr);
            },
        };

        ListRef {
            ptr: result_ptr,
            _guard: PhantomData,
        }
    }

    /// Substitutes type parameters in a type, returning a canonicalized type
    /// pointer. For internal usage only.
    pub fn substitute_type<'b>(
        &'b self,
        ty: Ref<'b, Type>,
        ty_args: ListRef<'b, Type>,
    ) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        let Ref { ptr, _guard } = ty;
        let original_ptr = ptr;
        let ty_ref = unsafe { ptr.as_ref_unchecked() };
        match ty_ref {
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
            | Type::Signer => {
                // Fast path: primitives have no type parameters.
                return ty;
            },

            // Substitution: replace type parameter with the concrete type.
            Type::TypeParam(idx) => {
                let ty_args_ref = unsafe { ty_args.ptr.as_ref_unchecked() };
                return Ref {
                    ptr: ty_args_ref[*idx as usize],
                    _guard,
                };
            },
            Type::Struct { type_args, .. } => {
                let type_args = unsafe { type_args.as_ref_unchecked() };
                if type_args.is_empty() {
                    // Non-generic struct, return.
                    return ty;
                }
            },
            Type::Vector(_) | Type::Ref(_) | Type::RefMut(_) | Type::Function { .. } => {
                // Handle composite types below.
            },
        }

        if let Some(cached) = self.ctx.type_subst_cache.get(&(ptr, ty_args.ptr)) {
            return Ref {
                ptr: *cached.value(),
                _guard,
            };
        }

        let Ref { ptr, _guard } = match ty_ref {
            Type::Vector(inner_ty) => {
                let new_inner = self.substitute_type(
                    Ref {
                        ptr: *inner_ty,
                        _guard,
                    },
                    ty_args,
                );
                if new_inner.ptr == *inner_ty {
                    return ty;
                }
                self.alloc_vector_type(new_inner)
            },

            Type::Ref(inner_ty) => {
                let new_inner = self.substitute_type(
                    Ref {
                        ptr: *inner_ty,
                        _guard,
                    },
                    ty_args,
                );
                if new_inner.ptr == *inner_ty {
                    return ty;
                }
                self.alloc_ref_type(new_inner)
            },

            Type::RefMut(inner_ty) => {
                let new_inner = self.substitute_type(
                    Ref {
                        ptr: *inner_ty,
                        _guard,
                    },
                    ty_args,
                );
                if new_inner.ptr == *inner_ty {
                    return ty;
                }
                self.alloc_ref_mut_type(new_inner)
            },

            Type::Struct {
                executable_id,
                name,
                type_args: generic_type_args,
            } => {
                let new_type_args = self.substitute_type_list(
                    ListRef {
                        ptr: *generic_type_args,
                        _guard,
                    },
                    ty_args,
                );
                if new_type_args.ptr == *generic_type_args {
                    return ty;
                }
                self.alloc_struct_type(
                    Ref {
                        ptr: *executable_id,
                        _guard,
                    },
                    Ref { ptr: *name, _guard },
                    new_type_args,
                )
            },

            Type::Function {
                args,
                results,
                abilities,
            } => {
                let new_args = self.substitute_type_list(ListRef { ptr: *args, _guard }, ty_args);
                let new_results = self.substitute_type_list(
                    ListRef {
                        ptr: *results,
                        _guard,
                    },
                    ty_args,
                );
                if new_args.ptr == *args && new_results.ptr == *results {
                    return ty;
                }

                self.alloc_function_type(new_args, new_results, *abilities)
            },
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
            | Type::TypeParam(_) => unreachable!("Must be already handled above"),
        };

        // Re-canonicalize. This can allocate more memory.
        let ptr = unsafe { self.ctx.types.insert(ptr) };
        match self.ctx.type_subst_cache.entry((original_ptr, ty_args.ptr)) {
            Entry::Occupied(e) => {
                assert!(*e.get() == ptr);
            },
            Entry::Vacant(e) => {
                assert!(*e.insert(ptr) == ptr);
            },
        };
        Ref { ptr, _guard }
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a> MaintenanceGuard<'a> {
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
        // CRITICAL:
        //   - Enforce that the reset order is enforced for any new cache.
        let Context {
            identifiers,
            executable_ids,
            types,
            type_lists,
            type_subst_cache,
            type_list_subst_cache,
        } = self.ctx;

        type_list_subst_cache.clear();
        type_subst_cache.clear();
        type_lists.reset();
        types.reset();
        executable_ids.reset();
        identifiers.reset();
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns a slice of function argument or return type tags. For internal use only.
    fn intern_function_param_or_return_tags<'b>(
        &'b self,
        tags: &[FunctionParamOrReturnTag],
    ) -> ListRef<'b, Type>
    where
        'a: 'b,
    {
        if let Some(ptr) = self.ctx.type_lists.get(tags) {
            return ListRef {
                ptr,
                _guard: PhantomData,
            };
        }

        let types = tags
            .iter()
            .map(|arg| match arg {
                FunctionParamOrReturnTag::Reference(tag) => {
                    if let Some(ptr) = self.ctx.types.get(arg) {
                        ptr
                    } else {
                        let inner = self.intern_type_tag(tag);
                        let Ref { ptr, .. } = self.alloc_ref_type(inner);
                        unsafe { self.ctx.types.insert(ptr) }
                    }
                },
                FunctionParamOrReturnTag::MutableReference(tag) => {
                    if let Some(ptr) = self.ctx.types.get(arg) {
                        ptr
                    } else {
                        let inner = self.intern_type_tag(tag);
                        let Ref { ptr, .. } = self.alloc_ref_mut_type(inner);
                        unsafe { self.ctx.types.insert(ptr) }
                    }
                },
                FunctionParamOrReturnTag::Value(tag) => {
                    let Ref { ptr, .. } = self.intern_type_tag(tag);
                    ptr
                },
            })
            .collect::<Vec<_>>();
        let ptr = self.global_arena.alloc_slice_copy(&types);
        ListRef {
            ptr: unsafe { self.ctx.type_lists.insert(ptr) },
            _guard: PhantomData,
        }
    }
}

/// Returns a [`Ref`] backed by a `'static` [`Type`] value. Used for the 15
/// primitive types that are interned as program-lifetime statics.
#[inline]
fn static_type_ref(ty: &'static Type) -> Ref<'static, Type> {
    Ref {
        ptr: GlobalArenaPtr::from_static(ty),
        _guard: PhantomData,
    }
}
