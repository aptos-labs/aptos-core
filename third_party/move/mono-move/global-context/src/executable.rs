// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Executable data structures for compiled Move modules or scripts.
//!
//! This module defines:
//! - [`Executable`]: interned compiled module or script representation,
//! - [`Function`]: interned function representation, stored in [`Executable`].

use crate::{
    alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArena, GlobalArenaPtr, LeakedBoxPtr},
    context::ExecutionContext,
    counters,
    types::{FunctionCacheKey, TypeInternal},
    version::BlockIndex,
    FunctionId, TypeList,
};
use ahash::{HashMap, HashMapExt};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use std::{
    mem::ManuallyDrop,
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

pub enum Code {
    // TODO: connect to micro-ops, or other representation here.
    Dummy,
}

/// Function definition stored in an [`Executable`]. May or may not be
/// monomorphized - the code that the function stores identifies that.
pub struct Function {
    /// Name of this function.
    #[allow(dead_code)]
    name: GlobalArenaPtr<str>,
    /// Fully-instantiated parameter types of this function.
    #[allow(dead_code)]
    param_types: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    /// Fully-instantiated return types of this function.
    #[allow(dead_code)]
    return_types: GlobalArenaPtr<[GlobalArenaPtr<TypeInternal>]>,
    /// Code for this function.
    #[allow(dead_code)]
    code: ArcSwap<Code>,
}

impl Function {
    /// Returns the name of this function.
    pub fn name(&self) -> &str {
        // SAFETY:
        //   Function can only be obtained from executable, which is therefore
        //   alive. Hence, we must be in execution context and the pointer to
        //   interned data is still valid.
        unsafe { self.name.as_ref_unchecked() }
    }

    /// Returns the parameter types of this function.
    pub fn param_types(&'_ self) -> TypeList<'_> {
        // SAFETY:
        //   Function can only be obtained from executable, which is therefore
        //   alive. Hence, we must be in execution context and the pointer to
        //   interned data is still valid.
        let tys = unsafe { self.param_types.as_ref_unchecked() };
        TypeList::new_internal(tys)
    }

    /// Returns the return types of this function.
    pub fn return_types(&'_ self) -> TypeList<'_> {
        // SAFETY:
        //   Function can only be obtained from executable, which is therefore
        //   alive. Hence, we must be in execution context and the pointer to
        //   interned data is still valid.
        let tys = unsafe { self.return_types.as_ref_unchecked() };
        TypeList::new_internal(tys)
    }
}

/// Cache key for the per-executable monomorphized function map. Encodes both
/// the generic function pointer and the type-argument list pointer by address,
/// enabling O(1) lookup when the same `(function, ty_args)` pair is seen again.
///
/// Valid for the lifetime of the executable that owns the map; both pointers
/// are interned (stable addresses) for the duration of any live
/// [`ExecutionContext`].
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct MonoCacheKey {
    /// Pointer address of the interned function name string.
    function_ptr: usize,
    /// Pointer address of the interned type-argument list slice.
    type_list_ptr: usize,
}

impl MonoCacheKey {
    fn new(id: FunctionId<'_>, type_list: TypeList<'_>) -> Self {
        Self {
            function_ptr: id.as_usize(),
            type_list_ptr: type_list.as_usize(),
        }
    }
}

/// Value stored in the per-executable monomorphized function cache.
struct MonomorphizedEntry {
    /// Heap-allocated monomorphized function.
    ptr: LeakedBoxPtr<Function>,
    /// Block index at which this entry was last accessed (read or created).
    /// Updated with Relaxed ordering on every hit; used for TTL eviction.
    /// `BlockIndex = u32`, so `AtomicU32` is the right width (saves 4 bytes
    /// per entry vs `AtomicU64`).
    last_used_block: AtomicU32,
}

/// All data stored in [`Executable`], separate from the arena where it is
/// allocated.
struct ExecutableData {
    /// Non-generic functions.
    functions: HashMap<FunctionCacheKey, ExecutableArenaPtr<Function>>,
    /// Generic (non-monomorphized) functions.
    generic_functions: HashMap<FunctionCacheKey, ExecutableArenaPtr<Function>>,
    /// Lazily monomorphized generic functions. Keyed by `(interned
    /// function-name ptr, interned type-list ptr)`. Values are Box-allocated
    /// for individual eviction.
    ///
    /// # Safety
    ///
    /// Every entry must be freed via [`LeakedBoxPtr::free_unchecked`] before
    /// this map is dropped. See [`Executable::drop`].
    monomorphized_functions: DashMap<MonoCacheKey, MonomorphizedEntry, ahash::RandomState>,
}

/// Interned compiled module or script.
pub struct Executable {
    /// Data fields containing pointers to arena-allocated objects (e.g.,
    /// functions or constants).
    ///
    /// # Safety
    ///
    /// Must be dropped **first**, before the arena because all pointers it
    /// stores reference memory owned by the bump allocator. Monomorphized
    /// functions in `data.monomorphized_functions` must be explicitly freed
    /// (via `LeakedBoxPtr::free_unchecked`) before `ManuallyDrop::drop` is
    /// called on `data`.
    data: ManuallyDrop<ExecutableData>,

    /// Stores all allocations for this executable.
    ///
    /// # Safety
    ///
    /// Must be dropped **last**. This arena owns the memory that all pointers
    /// in `data.functions` reference.
    bump: ManuallyDrop<ExecutableArena>,
}

impl Drop for Executable {
    fn drop(&mut self) {
        // SAFETY:
        //
        // Explicit drop order to avoid dangling pointers (CRITICAL):
        //   1. Drain monomorphized_functions: free each Box-allocated function.
        //      LeakedBoxPtr is Copy, so we copy each value and call
        //      free_unchecked on the copy. The retain call removes entries
        //      (returning false), preventing any double-free.
        //   2. Drop data: drops the HashMap and DashMap containers. The NonNull
        //      values inside (ExecutableArenaPtr) have no Drop impl, so this is
        //      safe.
        //   3. Drop bump arena: frees all bump-allocated non-generic functions.
        unsafe {
            self.data.monomorphized_functions.retain(|_, entry| {
                let to_free = entry.ptr;
                // SAFETY:
                //   Drop has exclusive access (&mut self). No references to
                //   monomorphized functions can exist past this point.
                to_free.free_unchecked();
                false
            });
            ManuallyDrop::drop(&mut self.data);
            ManuallyDrop::drop(&mut self.bump);
        }
    }
}

impl Executable {
    /// Returns a non-generic function by its ID, if it exists.
    pub fn get_function<'a>(&'a self, id: FunctionId<'a>) -> Option<&'a Function> {
        self.data
            .functions
            .get(&FunctionCacheKey::new(id))
            .map(|ptr| {
                // SAFETY:
                //   Because this executable is alive, the pointer is valid. The
                //   address is also stable after construction (there is no
                //   deallocation until executable is freed).
                unsafe { ptr.as_ref_unchecked() }
            })
    }

    /// Returns a monomorphized generic function by its ID and type arguments,
    /// monomorphizing it on demand if not already cached.
    ///
    /// On the first call for a given `(id, type_list)` pair, the generic
    /// template is looked up, monomorphized, and the result stored in
    /// `monomorphized_functions`. Concurrent calls for the same key may both
    /// monomorphize, but only one result is kept; the duplicate is freed
    /// immediately via [`LeakedBoxPtr::free_unchecked`].
    ///
    /// `current_block` is the block index of the calling execution context,
    /// used to update the TTL timestamp on every access.
    ///
    /// `mono_total` is the global count of cached monomorphized functions,
    /// incremented atomically when a new entry is committed to the cache.
    /// Prefer calling this via [`ExecutionContext::get_monomorphized_function`]
    /// to ensure the counter is wired correctly.
    pub fn get_monomorphized_function<'a>(
        &'a self,
        id: FunctionId<'a>,
        type_list: TypeList<'a>,
        current_block: BlockIndex,
        mono_total: &AtomicUsize,
    ) -> Option<&'a Function> {
        let key = MonoCacheKey::new(id, type_list);

        // Fast path: already monomorphized.
        if let Some(entry) = self.data.monomorphized_functions.get(&key) {
            entry
                .last_used_block
                .store(current_block, Ordering::Relaxed);
            // SAFETY:
            //   The pointer lives in the cache for the lifetime of this
            //   executable, which is alive for 'a.
            return Some(unsafe { entry.ptr.as_ref_unchecked() });
        }

        // Look up the generic template.
        let template_ptr = self
            .data
            .generic_functions
            .get(&FunctionCacheKey::new(id))?;
        // SAFETY:
        //   The executable (and its bump arena) is alive for 'a, so the
        //   arena-allocated template pointer is valid.
        let template = unsafe { template_ptr.as_ref_unchecked() };

        // Monomorphize (Phase 1: stub returning a dummy).
        let mono = self.monomorphize(template, type_list);

        // Insert via Entry API. If another thread already inserted for this
        // key, free our duplicate and use the winner's pointer instead.
        let ptr = match self.data.monomorphized_functions.entry(key) {
            dashmap::Entry::Occupied(entry) => {
                // SAFETY:
                //   `mono` is exclusively owned by this thread; no other thread
                //   has a reference to it. Freeing it here is safe.
                unsafe { mono.free_unchecked() };
                entry.get().ptr
            },
            dashmap::Entry::Vacant(entry) => {
                // Increment global count and record the miss only when we
                // actually commit a new entry (not on races that lose).
                mono_total.fetch_add(1, Ordering::Relaxed);
                counters::log_monomorphized_function_cache_miss();
                entry
                    .insert(MonomorphizedEntry {
                        ptr: mono,
                        last_used_block: AtomicU32::new(current_block),
                    })
                    .ptr
            },
        };

        // SAFETY:
        //   The winning pointer lives in the cache for the lifetime of this
        //   executable, which is proven alive by the 'a lifetime on `self`.
        Some(unsafe { ptr.as_ref_unchecked() })
    }

    /// Evicts all monomorphized functions whose `last_used_block` is `<=
    /// cutoff`. Uses `DashMap::retain` for a single O(N) pass with no
    /// intermediate allocations. Returns the number of entries evicted.
    ///
    /// # Safety
    ///
    /// Maintenance exclusive access is required; no live references to
    /// monomorphized functions may exist at call time.
    pub(crate) unsafe fn evict_stale_entries(&self, cutoff: BlockIndex) -> usize {
        let mut evicted = 0;
        self.data.monomorphized_functions.retain(|_, entry| {
            if entry.last_used_block.load(Ordering::Relaxed) <= cutoff {
                // SAFETY: Caller guarantees exclusive access; no live refs.
                unsafe { entry.ptr.free_unchecked() };
                evicted += 1;
                false
            } else {
                true
            }
        });
        evicted
    }

    /// Monomorphizes a generic function template with the given type arguments,
    /// returning a heap-allocated [`Function`].
    ///
    /// The caller must either insert the returned pointer into
    /// `monomorphized_functions` (transferring ownership to the cache) or free
    /// it via [`LeakedBoxPtr::free_unchecked`] to avoid a memory leak.
    ///
    /// TODO: implement actual type substitution.
    fn monomorphize(
        &self,
        template: &Function,
        _type_list: TypeList<'_>,
    ) -> LeakedBoxPtr<Function> {
        LeakedBoxPtr::from_box(Box::new(Function {
            name: template.name,
            param_types: template.param_types,
            return_types: template.return_types,
            code: ArcSwap::from_pointee(Code::Dummy),
        }))
    }
}

/// Builder for constructing an [`Executable`] from a [`CompiledModule`].
///
/// This encapsulates all the logic for iterating through a module's functions,
/// interning types, and building the executable data structure.
pub struct ExecutableBuilder<'a, 'b, A: GlobalArena> {
    context: &'a ExecutionContext<'b, A>,
    module: &'a CompiledModule,
}

impl<'a, 'b, A: GlobalArena> ExecutableBuilder<'a, 'b, A> {
    /// Creates a new builder for the given module and execution context.
    pub fn new(context: &'a ExecutionContext<'b, A>, module: &'a CompiledModule) -> Self {
        Self { context, module }
    }

    /// Builds the executable by iterating through all function definitions,
    /// interning their signatures, and allocating function metadata.
    pub fn build(self) -> Box<Executable> {
        let mut executable = Box::new(Executable {
            data: ManuallyDrop::new(ExecutableData {
                functions: HashMap::new(),
                generic_functions: HashMap::new(),
                monomorphized_functions: DashMap::with_hasher(ahash::RandomState::new()),
            }),
            bump: ManuallyDrop::new(ExecutableArena::new()),
        });

        for _struct_def in self.module.struct_defs() {
            // TODO
        }
        for _struct_handle in self.module.struct_handles() {
            // TODO
        }

        for func_def in self.module.function_defs() {
            let func_handle = self.module.function_handle_at(func_def.function);
            let name = self
                .context
                .intern_str_internal(self.module.identifier_at(func_handle.name).as_str());

            // SAFETY:
            //   We have just interned `name`; it will not be freed until maintenance.
            let key =
                FunctionCacheKey::new(FunctionId::new_internal(unsafe { name.as_ref_unchecked() }));

            let param_types = self.context.intern_signature_tokens_internal(
                &self.module.signature_at(func_handle.parameters).0,
                self.module,
            );
            let return_types = self.context.intern_signature_tokens_internal(
                &self.module.signature_at(func_handle.return_).0,
                self.module,
            );
            let func = Function {
                name,
                param_types,
                return_types,
                code: ArcSwap::from_pointee(Code::Dummy),
            };

            // SAFETY:
            //   The arena is alive for the lifetime of the executable.
            let func = unsafe { executable.bump.alloc(func) };
            if func_handle.type_parameters.is_empty() {
                executable.data.functions.insert(key, func);
            } else {
                // TODO:
                //   Optimize for phantom type arguments or when they can be erased.
                executable.data.generic_functions.insert(key, func);
            }
        }

        executable
    }
}
