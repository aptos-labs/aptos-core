// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Represents the state machine managing resource access control in VM execution.

use crate::{interpreter::ACCESS_STACK_SIZE_LIMIT, loader::Function};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::runtime_access_specifier::{
    AccessInstance, AccessSpecifier, AccessSpecifierEnv,
};

/// The state of access control. Maintains a stack of active access specifiers.
///
/// The top-most element is the active access control policy, the values below are restore points
/// to return to when the current context is exited.
///
/// At any time, in the current semantics, the top-most element of the stack represents the join
/// of all elements beneath + some restrictions. Notice that technically it is possible to change
/// this, for example, when privileged code is called.
#[derive(Clone, Debug)]
pub struct AccessControlState {
    specifier_stack: Vec<AccessSpecifier>,
}

impl Default for AccessControlState {
    fn default() -> Self {
        Self {
            specifier_stack: vec![AccessSpecifier::Any],
        }
    }
}

impl AccessControlState {
    /// Enters a function, applying its access specifier to the state.
    pub(crate) fn enter_function(
        &mut self,
        env: &impl AccessSpecifierEnv,
        fun: &Function,
    ) -> PartialVMResult<()> {
        if matches!(fun.access_specifier, AccessSpecifier::Any) {
            // Shortcut case that no access is specified
            return Ok(());
        }
        // Specialize the functions access specifier
        let mut fun_specifier = fun.access_specifier.clone();
        fun_specifier.specialize(env)?;
        // Join with top of stack
        let current = self.check_stack_and_peek()?;
        let new_specifier = current.join(&fun_specifier);
        if let Some(false) = new_specifier.subsumes(&fun_specifier) {
            // We don't allow to call this function, even if in some code paths access would be ok.
            // This ensures that static analysis results are compatible.
            return Err(PartialVMError::new(StatusCode::ACCESS_DENIED)
                .with_message(format!("not allowed to call `{}`", fun.pretty_string())));
        }
        if self.specifier_stack.len() >= ACCESS_STACK_SIZE_LIMIT {
            Err(
                PartialVMError::new(StatusCode::ACCESS_STACK_LIMIT_EXCEEDED).with_message(format!(
                    "access specifier stack overflow (limit = {})",
                    ACCESS_STACK_SIZE_LIMIT
                )),
            )
        } else {
            self.specifier_stack.push(new_specifier);
            Ok(())
        }
    }

    /// Exit function, restoring access state before entering.
    pub(crate) fn exit_function(&mut self, fun: &Function) -> PartialVMResult<()> {
        if !matches!(fun.access_specifier, AccessSpecifier::Any) {
            self.check_stack_and_peek()?;
            self.specifier_stack.pop();
        }
        Ok(())
    }

    fn check_stack_and_peek(&self) -> PartialVMResult<&AccessSpecifier> {
        self.specifier_stack.last().ok_or_else(|| {
            PartialVMError::new(StatusCode::ACCESS_CONTROL_INVARIANT_VIOLATION)
                .with_message("unbalanced access specifier stack".to_owned())
        })
    }

    /// Check whether the given access is allowed in the current state.
    pub(crate) fn check_access(&self, access: AccessInstance) -> PartialVMResult<()> {
        if let Some(active) = self.specifier_stack.last() {
            if !active.enables(&access) {
                return Err(PartialVMError::new(StatusCode::ACCESS_DENIED)
                    .with_message(format!("not allowed to perform `{}`", access)));
            }
        }
        Ok(())
    }
}
