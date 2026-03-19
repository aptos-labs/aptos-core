// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Placeholder types for executables (compiled modules / scripts).

use crate::{alloc::GlobalArenaPtr, ArenaRef};
use bumpalo::Bump;
use fxhash::FxHashMap;
use parking_lot::Mutex;
use std::ptr::NonNull;

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
    /// Non-generic functions.
    #[allow(dead_code)]
    functions: FxHashMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>>,

    /// Arena where data is allocated for this executable. **Must** be the
    /// last field so that it is dropped after any data structure that holds
    /// pointers into it.
    #[allow(dead_code)]
    arena: Mutex<Bump>,
}

impl Executable {
    /// Returns a non-generic function from this executable. Returns [`None`]
    /// if such function does not exist.
    pub fn get_function(&self, name: ArenaRef<'_, str>) -> Option<&Function> {
        self.functions
            .get(&name.into_global_arena_ptr())
            .map(|ptr| {
                // SAFETY: Because executable is alive, all its allocations are
                // still valid.
                unsafe { ptr.as_ref_unchecked() }
            })
    }
}
