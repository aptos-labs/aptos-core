// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Represents the state machine managing resource access control in VM execution.

use crate::{interpreter::ACCESS_STACK_SIZE_LIMIT, LoadedFunction};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::runtime_access_specifier::{
    AccessInstance, AccessSpecifier, AccessSpecifierEnv,
};

/// The state of access control. Maintains a stack of active access specifiers.
///
/// Every access to a resource must satisfy every specifier on the stack.
#[derive(Clone, Debug, Default)]
pub struct AccessControlState {
    specifier_stack: Vec<AccessSpecifier>,
}

impl AccessControlState {
    /// Enters a function, applying its access specifier to the state.
    pub(crate) fn enter_function(
        &mut self,
        env: &impl AccessSpecifierEnv,
        fun: &LoadedFunction,
    ) -> PartialVMResult<()> {
        if matches!(fun.access_specifier(), AccessSpecifier::Any) {
            // Shortcut case that no access is specified
            return Ok(());
        }
        if self.specifier_stack.len() >= ACCESS_STACK_SIZE_LIMIT {
            Err(
                PartialVMError::new(StatusCode::ACCESS_STACK_LIMIT_EXCEEDED).with_message(format!(
                    "access specifier stack overflow (limit = {})",
                    ACCESS_STACK_SIZE_LIMIT
                )),
            )
        } else {
            // Specialize the functions access specifier and push it on the stack.
            let mut fun_specifier = fun.access_specifier().clone();
            fun_specifier.specialize(env)?;
            self.specifier_stack.push(fun_specifier);
            Ok(())
        }
    }

    /// Exit function, restoring access state before entering.
    pub(crate) fn exit_function(&mut self, fun: &LoadedFunction) -> PartialVMResult<()> {
        if !matches!(fun.access_specifier(), AccessSpecifier::Any) {
            if self.specifier_stack.is_empty() {
                return Err(
                    PartialVMError::new(StatusCode::ACCESS_CONTROL_INVARIANT_VIOLATION)
                        .with_message("unbalanced access specifier stack".to_owned()),
                );
            }
            self.specifier_stack.pop();
        }
        Ok(())
    }

    /// Check whether the given access is allowed in the current state.
    pub(crate) fn check_access(&self, access: AccessInstance) -> PartialVMResult<()> {
        for specifier in self.specifier_stack.iter().rev() {
            if !specifier.enables(&access) {
                return Err(PartialVMError::new(StatusCode::ACCESS_DENIED)
                    .with_message(format!("not allowed to perform `{}`", access)));
            }
        }
        Ok(())
    }
}
