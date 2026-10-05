// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reentrancy checker for resource access and `#[module_lock]`.
//!
//! A module is active while any of its functions is on the call stack. Calling
//! an active module from another module, or through a closure, re-enters it.
//! Regular same-module calls do not cause reentry.
//!
//! A reentrant call and any regular same-module calls it makes cannot access
//! their module's resources.
//!
//! A `#[module_lock]` function forbids cross-module reentry into any active
//! module until it returns. A cross-module call to the locking function is
//! rejected if its module is already active, even if no other lock is held.
//!
//! Cross-module calls, closure calls, and calls to `#[module_lock]` functions
//! save enough state to undo their changes on return: remove the module if
//! this call added it, release any lock it acquired, and restore the caller's
//! resource-access flag.

use crate::error::{RuntimeError, RuntimeInvariantViolation};
use mono_move_core::{
    interner::{identifier_of, module_id_of, InternedModuleId},
    types::{view_type, InternedType, Type},
    Function, VMInternalError, VMResult,
};
use shared_dsa::UnorderedSet;

/// State that [`ReentrancyState::exit`] cannot derive from the callee.
struct CheckedEntry {
    /// Whether this call added the callee's module to `active` and must remove
    /// it on return.
    removes_module: bool,
    /// Whether the caller was forbidden from accessing its module's resources.
    /// Restored when this call returns.
    prev_resources_locked: bool,
}

pub(crate) struct ReentrancyState {
    /// Modules with at least one active entry.
    active: UnorderedSet<InternedModuleId>,
    /// Entries of checked calls, most recent last. The root frame has no entry
    /// because its return does not call [`Self::exit`].
    // TODO(perf): consider keeping each entry in spare bits of the callee's
    // tagged saved pc and drop this stack.
    entries: Vec<CheckedEntry>,
    /// Number of active calls to `#[module_lock]` functions.
    lock_count: u32,
    /// Whether the current frame is forbidden from accessing its module's
    /// resources because of reentry. Regular same-module calls inherit this.
    top_resources_locked: bool,
}

impl ReentrancyState {
    pub(crate) fn new() -> Self {
        Self {
            active: UnorderedSet::new(),
            entries: Vec::new(),
            lock_count: 0,
            top_resources_locked: false,
        }
    }

    /// Starts a session with the root module active and its resources accessible.
    /// Acquires a module lock if `root` has `#[module_lock]`.
    pub(crate) fn reset(&mut self, root: &Function) {
        self.active.clear();
        self.entries.clear();
        self.active.insert(root.module_id);
        self.lock_count = u32::from(root.has_module_lock);
        self.top_resources_locked = false;
    }

    /// Rejects resource access during reentry. Lowering guarantees that `ty`
    /// is defined in the current function's module.
    #[inline(always)]
    pub(crate) fn check_resource_access(&self, ty: InternedType) -> VMResult<()> {
        if self.top_resources_locked {
            return Err(resource_access_during_reentrancy(ty));
        }
        Ok(())
    }

    /// Checks reentrancy and records a call from `caller_module` to `callee`.
    ///
    /// The caller must tag the callee frame's `saved_pc` so its return reaches
    /// [`Self::exit`].
    #[inline(never)]
    pub(crate) fn enter(
        &mut self,
        caller_module: InternedModuleId,
        callee: &Function,
        is_closure: bool,
    ) -> VMResult<()> {
        let module = callee.module_id;
        if callee.has_module_lock {
            // Cannot overflow: at most one increment per live frame, and each
            // frame occupies at least `FRAME_METADATA_SIZE` bytes of a stack
            // far smaller than `u32::MAX` frames.
            self.lock_count += 1;
        }
        let cross = module != caller_module;
        let counted = cross || is_closure;
        let mut prev_member = false;
        if counted {
            prev_member = !self.active.insert(module);
            // Same-module closure calls create entries and lock resources,
            // but the module-lock check applies only to cross-module calls.
            if cross && prev_member && self.lock_count > 0 {
                return Err(reentrancy_under_module_lock(callee));
            }
        }
        self.entries.push(CheckedEntry {
            removes_module: counted && !prev_member,
            prev_resources_locked: self.top_resources_locked,
        });
        if counted {
            self.top_resources_locked = prev_member;
        }
        Ok(())
    }

    /// Undoes the innermost checked frame's [`Self::enter`]. Called from a
    /// tagged `Return` of `callee`.
    #[inline(never)]
    pub(crate) fn exit(&mut self, callee: &Function) -> VMResult<()> {
        let Some(entry) = self.entries.pop() else {
            return Err(invariant_violation(
                RuntimeInvariantViolation::ReentrancyExitWithoutRecord,
            ));
        };
        if entry.removes_module && !self.active.remove(&callee.module_id) {
            return Err(invariant_violation(
                RuntimeInvariantViolation::ReentrancyModuleNotActive,
            ));
        }
        if callee.has_module_lock {
            let Some(count) = self.lock_count.checked_sub(1) else {
                return Err(invariant_violation(
                    RuntimeInvariantViolation::ReentrancyLockUnderflow,
                ));
            };
            self.lock_count = count;
        }
        self.top_resources_locked = entry.prev_resources_locked;
        Ok(())
    }
}

// These non-inlined helpers keep error construction and allocation code out
// of the interpreter's dispatch loop.

#[cold]
#[inline(never)]
fn invariant_violation(violation: RuntimeInvariantViolation) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(violation))
}

#[cold]
#[inline(never)]
fn reentrancy_under_module_lock(callee: &Function) -> VMInternalError {
    VMInternalError::new(RuntimeError::ReentrancyUnderModuleLock {
        module: Box::new(module_id_of(callee.module_id)),
        function: identifier_of(callee.name),
    })
}

#[cold]
#[inline(never)]
fn resource_access_during_reentrancy(ty: InternedType) -> VMInternalError {
    let Type::Nominal {
        module_id, name, ..
    } = view_type(ty)
    else {
        return invariant_violation(RuntimeInvariantViolation::Unreachable(
            "resource type must be a nominal type".to_string(),
        ));
    };
    VMInternalError::new(RuntimeError::ResourceAccessDuringReentrancy {
        module: Box::new(module_id_of(*module_id)),
        name: identifier_of(*name),
    })
}
