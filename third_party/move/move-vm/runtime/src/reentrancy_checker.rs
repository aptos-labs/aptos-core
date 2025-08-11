// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a reentrancy checker for dynamic dispatch. Two types of checks
//! are implemented:
//!
//! (1) The resource lock mechanism for closure dispatch, as described in
//!     [AIP-122](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-112.md).
//!     In summary, for this mechanism access to any resource is disallowed on reentrancy.
//! (2) The module lock mechanism for native dispatch as implemented for
//!     [AIP-73](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-73.md).
//!     For this mechanism reentrancy via any kind of function call is disallowed.
//!     This entails (2), so every check failing for (1), also fails in (2). This
//!     is by the property that resources can only be accessed inside the module
//!     they are defined in.
//!
//! The checker by default operates in mode (1), but allows to enter code
//! which operates in mode (2), which will override the more relaxed behavior of (1)
//! until it is exited.

use crate::LoadedFunction;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::ModuleId, vm_status::StatusCode};
use move_vm_types::loaded_data::runtime_types::StructIdentifier;
use std::collections::{btree_map::Entry, BTreeMap};

/// The reentrancy checker's state
#[derive(Default)]
pub(crate) struct ReentrancyChecker {
    /// A multiset (bag) of active modules. This is not a set because the same
    /// module can be entered multiple times on closure dispatch.
    active_modules: BTreeMap<ModuleId, usize>,
    /// Whether we are in module lock mode. This happens if we enter a function which is locking:
    ///   - call via [CallType::NativeDynamicDispatch],
    ///   - function has `#[module_lock]` attribute.
    module_lock_count: usize,
}

/// Ways how functions are called
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallType {
    /// Regular static function call.
    Regular,
    /// Dynamic dispatch via the NativeDispatch feature.
    NativeDynamicDispatch,
    /// Dynamic dispatch via closure call.
    ClosureDynamicDispatch,
}

impl CallType {
    /// Returns true of the call to callee needs to lock the module. This is the case if:
    ///   1. we are dispatching via native,
    ///   2. the callee has `#[module_lock]` attribute.
    fn is_locking(&self, callee: &LoadedFunction) -> bool {
        match self {
            Self::NativeDynamicDispatch => true,
            Self::Regular | Self::ClosureDynamicDispatch => callee.function.has_module_lock(),
        }
    }
}

impl ReentrancyChecker {
    pub fn enter_function(
        &mut self,
        caller_module: Option<&ModuleId>,
        callee: &LoadedFunction,
        call_type: CallType,
    ) -> PartialVMResult<()> {
        if call_type.is_locking(callee) {
            self.enter_module_lock();
        }

        let callee_module = callee.module_or_script_id();
        if Some(callee_module) != caller_module {
            // Cross module call.
            // When module lock is active, and we have already called into this module, this
            // reentry is disallowed
            match self.active_modules.entry(callee_module.clone()) {
                Entry::Occupied(mut e) => {
                    if self.module_lock_count > 0 {
                        return Err(PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR)
                            .with_message(format!(
                                "Reentrancy disallowed: reentering `{}` via function `{}` \
                     (module lock is active)",
                                callee_module,
                                callee.name()
                            )));
                    }
                    *e.get_mut() += 1
                },
                Entry::Vacant(e) => {
                    e.insert(1);
                },
            }
        } else if call_type == CallType::ClosureDynamicDispatch || caller_module.is_none() {
            // If this is closure dispatch, or we have no caller module (i.e. top-level entry).
            // Count the intra-module call like an inter-module call, as reentrance.
            // A static local call is governed by Move's `acquire` static semantics; however,
            // a dynamic dispatched local call has accesses not known at the caller side, so needs
            // the runtime reentrancy check. Note that this doesn't apply to NativeDynamicDispatch
            // which already has a check in place preventing a dispatch into the same module.
            *self
                .active_modules
                .entry(callee_module.clone())
                .or_default() += 1;
        }
        Ok(())
    }

    pub fn exit_function(
        &mut self,
        caller_module: &ModuleId,
        callee: &LoadedFunction,
        call_type: CallType,
    ) -> PartialVMResult<()> {
        let callee_module = callee.module_or_script_id();
        if caller_module != callee_module || call_type == CallType::ClosureDynamicDispatch {
            // If this is an exit from cross-module call, or exit from closure dispatch,
            // decrement counter.
            match self.active_modules.entry(callee_module.clone()) {
                Entry::Occupied(mut e) => {
                    let val = e.get_mut();
                    if *val == 1 {
                        e.remove_entry();
                    } else {
                        *val -= 1;
                    }
                },
                Entry::Vacant(_) => {
                    return Err(PartialVMError::new_invariant_violation(
                        "Unbalanced reentrancy stack operation",
                    ))
                },
            }
        }

        if call_type.is_locking(callee) {
            self.exit_module_lock()?;
        }
        Ok(())
    }

    pub fn enter_module_lock(&mut self) {
        self.module_lock_count += 1
    }

    pub fn exit_module_lock(&mut self) -> PartialVMResult<()> {
        if self.module_lock_count > 0 {
            self.module_lock_count -= 1;
            Ok(())
        } else {
            Err(PartialVMError::new_invariant_violation(
                "Unbalanced module lock counter",
            ))
        }
    }

    pub fn check_resource_access(&self, struct_id: &StructIdentifier) -> PartialVMResult<()> {
        if self
            .active_modules
            .get(&struct_id.module)
            .copied()
            .unwrap_or_default()
            > 1
        {
            // If the count is greater one, we have reentered this module, and all
            // resources it defines are locked.
            Err(
                PartialVMError::new(StatusCode::RUNTIME_DISPATCH_ERROR).with_message(format!(
                    "Resource `{}` cannot be accessed because of active reentrancy of defining \
                    module.",
                    struct_id,
                )),
            )
        } else {
            Ok(())
        }
    }
}
