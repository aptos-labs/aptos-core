// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Placeholder types for executables (compiled modules / scripts).

use crate::{ArenaRef, ExecutableId, ExecutionGuard};
use bumpalo::Bump;
use fxhash::FxBuildHasher;
use mono_move_alloc::GlobalArenaPtr;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use parking_lot::Mutex;
use std::{collections::HashMap, ptr::NonNull};

/// A pointer into an executable's private [`Bump`] arena. The executable owns
/// the arena, so the pointer is valid for the lifetime of the executable.
///
/// # Safety model
///
/// Dereferencing is **unsafe** - the caller must ensure the executable that
/// owns the area allocation has not been dropped.
#[repr(transparent)]
pub struct ExecutableArenaPtr<T>(NonNull<T>);

impl<T> ExecutableArenaPtr<T> {
    /// Returns a shared reference to the pointee with the same lifetime as
    /// executable.
    ///
    /// # Safety
    ///
    /// The caller must ensure the executable's arena that owns the allocation
    /// is alive and has not been reset or dropped.
    #[allow(dead_code)]
    pub unsafe fn as_ref_unchecked(&self) -> &T {
        // SAFETY: The caller ensures the arena is still alive / not dropped.
        unsafe { self.0.as_ref() }
    }
}

// This type can be duplicated using bitwise copy.
impl<T> Copy for ExecutableArenaPtr<T> {}

impl<T> Clone for ExecutableArenaPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY:
//
// Pointer acts as a shared reference when sent to other threads. It is
// allocated in the executable's arena, and therefore is guaranteed to be
// alive while the executable is alive. Because executables are never dropped
// during concurrent execution, there is no need to require T to be `Send`.
// However, T has to be `Sync` because global pointer does expose a shared
// reference to T.
unsafe impl<T: Sync> Send for ExecutableArenaPtr<T> {}

// SAFETY:
//
// Pointer is `Sync` because it provides read-only access to pointee type when
// shared between threads, which is safe if pointee is also `Sync`.
unsafe impl<T: Sync> Sync for ExecutableArenaPtr<T> {}

/// Loaded function placeholder.
pub struct Function {
    #[allow(dead_code)]
    name: GlobalArenaPtr<str>,
    // TODO:
    //   Need to move micro-ops to same crate or move global pointer out
    //   to avoid circular dependency. Also cannot use function in micro
    //   ops because of the same issue.
}

impl Function {
    /// Returns the name of this function.
    pub fn name(&self) -> &str {
        // SAFETY: Function name is a pointer to data allocated in global
        // arena. It must still be valid because:
        //   - This function allocation is alive.
        //   - Executable storing function pointer is not dropped.
        //   - Global arena has not been reset and therefore the pointer is
        //     valid since it was created.
        unsafe { self.name.as_ref_unchecked() }
    }
}

/// A loaded executable (from module or script).
pub struct Executable {
    data: ExecutableData,

    /// Arena where data is allocated for this executable. **Must** be the
    /// last field so that it is dropped after any data structure that holds
    /// pointers into it.
    #[allow(dead_code)]
    arena: Mutex<Bump>,
}

struct ExecutableData {
    /// Executable ID which uniquely identifies this executable.
    #[allow(dead_code)]
    id: GlobalArenaPtr<ExecutableId>,

    /// Non-generic functions.
    functions: HashMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>, FxBuildHasher>,
}

impl Executable {
    /// Returns a non-generic function from this executable. Returns [`None`]
    /// if such function does not exist.
    pub fn get_function(&self, name: ArenaRef<'_, str>) -> Option<&Function> {
        self.data
            .functions
            .get(&name.into_global_arena_ptr())
            .map(|ptr| {
                // SAFETY: Because executable is alive, all its allocations are
                // still valid.
                unsafe { ptr.as_ref_unchecked() }
            })
    }
}

// TODO: this is likely to change. Placeholder.
#[allow(dead_code)]
pub struct ExecutableBuilder<'a> {
    // TODO: support scripts.
    module: &'a CompiledModule,
    arena: Bump,
}

#[allow(dead_code)]
impl<'a> ExecutableBuilder<'a> {
    pub fn new_for_module(module: &'a CompiledModule) -> Self {
        // TODO: run verifier here?
        Self {
            module,
            arena: Bump::new(),
        }
    }

    /// Builds an executable from the provided compiled module.
    pub fn build(self, guard: &ExecutionGuard<'_>) -> anyhow::Result<Box<Executable>> {
        let address = self.module.self_addr();
        let module_name = self.module.self_name();
        let id = guard.intern_address_name_internal(*address, module_name);

        let mut data = ExecutableData {
            id,
            functions: HashMap::with_hasher(FxBuildHasher::default()),
        };

        for function in self.module.function_defs() {
            let handle = self.module.function_handle_at(function.function);
            if !handle.type_parameters.is_empty() {
                todo!("Not yet implemented");
            }

            let identifier = self.module.identifier_at(handle.name);
            let name = guard.intern_identifier_internal(identifier);
            let function = Function { name };
            let ptr = ExecutableArenaPtr(NonNull::from(self.arena.alloc(function)));
            data.functions.insert(name, ptr);
        }

        Ok(Box::new(Executable {
            data,
            arena: Mutex::new(self.arena),
        }))
    }
}
