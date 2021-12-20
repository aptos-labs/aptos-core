// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{data_cache::TransactionDataCache, runtime::VMRuntime};
use move_binary_format::errors::*;
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Event},
    gas_schedule::GasAlgebra,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    resolver::MoveResolver,
    value::MoveTypeLayout,
};
use move_vm_types::gas_schedule::GasStatus;

pub struct Session<'r, 'l, S> {
    pub(crate) runtime: &'l VMRuntime,
    pub(crate) data_cache: TransactionDataCache<'r, 'l, S>,
}

/// Result of executing a function in the VM
pub enum ExecutionResult {
    /// Execution completed successfully and changed global state
    Success {
        /// Changes to global state that occurred during execution
        change_set: ChangeSet,
        /// Events emitted during execution
        events: Vec<Event>,
        /// Values returned by the function
        return_values: Vec<Vec<u8>>,
        /// Final value of inputs passed in to the entrypoint via a mutable reference
        mutable_ref_values: Vec<Vec<u8>>,
        /// Gas used during execution
        gas_used: u64,
    },
    /// Execution failed and had no side effects
    Fail {
        /// The reason execution failed
        error: VMError,
        /// Gas used during execution
        gas_used: u64,
    },
}

impl<'r, 'l, S: MoveResolver> Session<'r, 'l, S> {
    /// Execute a Move function with the given arguments. This is mainly designed for an external
    /// environment to invoke system logic written in Move.
    ///
    /// The caller MUST ensure
    ///   - All types and modules referred to by the type arguments exist.
    ///
    /// The Move VM MUST return an invariant violation if the caller fails to follow any of the
    /// rules above.
    ///
    /// Currently if any other error occurs during execution, the Move VM will simply propagate that
    /// error back to the outer environment without handling/translating it. This behavior may be
    /// revised in the future.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn execute_function(
        &mut self,
        module: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_status: &mut GasStatus,
    ) -> VMResult<Vec<Vec<u8>>> {
        self.runtime.execute_function(
            module,
            function_name,
            ty_args,
            args,
            &mut self.data_cache,
            gas_status,
        )
    }

    /// Execute `module`::`fuction_name`<`ty_args`>(`args`) and return the effects in
    /// an `ExecutionResult`, including
    /// * the write set and events
    /// * return values of the function
    /// * changes to values passes by mutable reference to the function
    /// Arguments to the function in `args` can be any type--ground types, user-defined struct
    /// types, and references (including mutable references).
    /// A reference argument in `args[i]` with type `&T` or `&mut T` will be deserialized as a `T`.
    /// Pure arguments are deserialized in the obvious way.
    ///
    /// NOTE: The ability to deserialize `args` into arbitrary types is very powerful--e.g., it can
    /// used to manufacture `signer`'s or `Coin`'s from raw bytes. It is the respmsibility of the
    /// caller (e.g. adapter) to ensure that this power is useed responsibility/securely for its use-case.
    pub fn execute_function_for_effects(
        mut self,
        module: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_status: &mut GasStatus,
    ) -> ExecutionResult {
        let gas_budget = gas_status.remaining_gas().get();
        let execution_res = self.runtime.execute_function_for_effects(
            module,
            function_name,
            ty_args,
            args,
            &mut self.data_cache,
            gas_status,
        );
        let gas_used = gas_budget - gas_status.remaining_gas().get();
        match execution_res {
            Ok((return_values, mutable_ref_values)) => match self.finish() {
                Ok((change_set, events)) => ExecutionResult::Success {
                    change_set,
                    events,
                    return_values,
                    mutable_ref_values,
                    gas_used,
                },
                Err(error) => ExecutionResult::Fail { error, gas_used },
            },
            Err(error) => ExecutionResult::Fail { error, gas_used },
        }
    }

    /// Execute a Move script function with the given arguments.
    ///
    /// Unlike `execute_function` which is designed for system logic, `execute_script_function` is
    /// mainly designed to call a script function in an existing module. It similar to
    /// `execute_script` except that execution of the "script" begins with the specified function
    ///
    /// The Move VM MUST return a user error (in other words, an error that's not an invariant
    /// violation) if
    ///   - The function does not exist.
    ///   - The function does not have script visibility.
    ///   - The signature is not valid for a script. Not all script-visible module functions can
    ///     be invoked from this entry point. See `move_bytecode_verifier::script_signature` for the
    ///     rules.
    ///   - Type arguments refer to a non-existent type.
    ///   - Arguments (senders included) fail to deserialize or fail to match the signature of the
    ///     script function.

    ///
    /// If any other error occurs during execution, the Move VM MUST propagate that error back to
    /// the caller.
    /// Besides, no user input should cause the Move VM to return an invariant violation.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn execute_script_function(
        &mut self,
        module: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        senders: Vec<AccountAddress>,
        gas_status: &mut GasStatus,
    ) -> VMResult<()> {
        self.runtime.execute_script_function(
            module,
            function_name,
            ty_args,
            args,
            senders,
            &mut self.data_cache,
            gas_status,
        )
    }

    /// Execute a transaction script.
    ///
    /// The Move VM MUST return a user error (in other words, an error that's not an invariant
    /// violation) if
    ///   - The script fails to deserialize or verify. Not all expressible signatures are valid.
    ///     See `move_bytecode_verifier::script_signature` for the rules.
    ///   - Type arguments refer to a non-existent type.
    ///   - Arguments (senders included) fail to deserialize or fail to match the signature of the
    ///     script function.
    ///
    /// If any other error occurs during execution, the Move VM MUST propagate that error back to
    /// the caller.
    /// Besides, no user input should cause the Move VM to return an invariant violation.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn execute_script(
        &mut self,
        script: Vec<u8>,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        senders: Vec<AccountAddress>,
        gas_status: &mut GasStatus,
    ) -> VMResult<()> {
        self.runtime.execute_script(
            script,
            ty_args,
            args,
            senders,
            &mut self.data_cache,
            gas_status,
        )
    }

    /// Publish the given module.
    ///
    /// The Move VM MUST return a user error, i.e., an error that's not an invariant violation, if
    ///   - The module fails to deserialize or verify.
    ///   - The sender address does not match that of the module.
    ///   - (Republishing-only) the module to be updated is not backward compatible with the old module.
    ///   - (Republishing-only) the module to be updated introduces cyclic dependencies.
    ///
    /// The Move VM should not be able to produce other user errors.
    /// Besides, no user input should cause the Move VM to return an invariant violation.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn publish_module(
        &mut self,
        module: Vec<u8>,
        sender: AccountAddress,
        gas_status: &mut GasStatus,
    ) -> VMResult<()> {
        self.publish_module_bundle(vec![module], sender, gas_status)
    }

    /// Publish a series of modules.
    ///
    /// The Move VM MUST return a user error, i.e., an error that's not an invariant violation, if
    /// any module fails to deserialize or verify (see the full list of  failing conditions in the
    /// `publish_module` API). The publishing of the module series is an all-or-nothing action:
    /// either all modules are published to the data store or none is.
    ///
    /// Similar to the `publish_module` API, the Move VM should not be able to produce other user
    /// errors. Besides, no user input should cause the Move VM to return an invariant violation.
    ///
    /// In case an invariant violation occurs, the whole Session should be considered corrupted and
    /// one shall not proceed with effect generation.
    pub fn publish_module_bundle(
        &mut self,
        modules: Vec<Vec<u8>>,
        sender: AccountAddress,
        gas_status: &mut GasStatus,
    ) -> VMResult<()> {
        self.runtime
            .publish_module_bundle(modules, sender, &mut self.data_cache, gas_status)
    }

    pub fn num_mutated_accounts(&self, sender: &AccountAddress) -> u64 {
        self.data_cache.num_mutated_accounts(sender)
    }

    /// Finish up the session and produce the side effects.
    ///
    /// This function should always succeed with no user errors returned, barring invariant violations.
    ///
    /// This MUST NOT be called if there is a previous invocation that failed with an invariant violation.
    pub fn finish(self) -> VMResult<(ChangeSet, Vec<Event>)> {
        self.data_cache
            .into_effects()
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub fn get_type_layout(&self, type_tag: &TypeTag) -> VMResult<MoveTypeLayout> {
        self.runtime
            .loader()
            .get_type_layout(type_tag, &self.data_cache)
    }
}
