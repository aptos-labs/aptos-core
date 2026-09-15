// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM adapter for the shared transactional test framework.
//!
//! Task parsing, compilation, and publishing checks use V1's infrastructure.
//! Publish tasks also apply MonoVM loading checks before updating storage.
//! Function and script run tasks execute on MonoVM and render
//! outcomes in V1's baseline format.

use crate::transactional_session::{
    ArgumentError, PublishError, RunError, RunOutcome, TransactionalSession,
};
use anyhow::{bail, Result};
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use mono_move_core::{BytecodeOffset, FunctionDefinitionIndex, VMInternalError};
use mono_move_output::v1_error::{describe_or_fallback, v1_location, V1Message};
use move_binary_format::{
    errors::{set_stable_test_display, ExecutionState, Location, PartialVMError, VMError},
    file_format::CompiledScript,
    CompiledModule,
};
use move_command_line_common::address::ParsedAddress;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    value::{serialize_values, MoveTypeLayout, MoveValue},
    vm_status::{AbortLocation, StatusCode},
};
use move_transactional_test_runner::{
    framework::{run_test_impl_with_baseline, BaselineTarget, CompiledState, MoveTestAdapter},
    tasks::{taskify, EmptyCommand, InitCommand, SyntaxChoice, TaskCommand, TaskInput},
    vm_test_harness::{
        compiled_state_with_stdlib, deserialize_value, function_execution_error,
        precompiled_v2_stdlib, publish_error, script_execution_error, serialize_module,
        serialize_script, stage_precompiled_stdlib, type_layout, view_resource, AdapterExecuteArgs,
        AdapterPublishArgs, PrecompiledFilesModules, TestRunConfig,
    },
};
use move_vm_runtime::move_vm::SerializedReturnValues;
use move_vm_types::values::Value;
use std::{collections::BTreeSet, path::Path};

/// Task syntax and arguments parsed for [`MonoVMTestAdapter`].
type MonoVMTaskCommand =
    TaskCommand<EmptyCommand, AdapterPublishArgs, (), AdapterExecuteArgs, EmptyCommand>;

/// Returns whether the source contains only supported tasks: initialization,
/// bytecode printing, module publishing, resource viewing, and unmetered runs
/// of a module function or script.
pub fn supports_source(path: &Path) -> Result<bool> {
    Ok(taskify::<MonoVMTaskCommand>(path)?
        .iter()
        .all(|task| match &task.command {
            TaskCommand::Init(..)
            | TaskCommand::PrintBytecode(..)
            | TaskCommand::Publish(..)
            | TaskCommand::View(..) => true,
            TaskCommand::Run(command, _) => command.gas_budget.is_none(),
            TaskCommand::Subcommand(..) => false,
        }))
}

pub struct MonoVMTestAdapter<'a> {
    compiled_state: CompiledState<'a>,
    session: TransactionalSession,
    default_syntax: SyntaxChoice,
    run_config: TestRunConfig,
}

impl<'a> MoveTestAdapter<'a> for MonoVMTestAdapter<'a> {
    type ExtraInitArgs = EmptyCommand;
    type ExtraPublishArgs = AdapterPublishArgs;
    type ExtraRunArgs = AdapterExecuteArgs;
    type ExtraValueArgs = ();
    type Subcommand = EmptyCommand;

    fn compiled_state(&mut self) -> &mut CompiledState<'a> {
        &mut self.compiled_state
    }

    fn default_syntax(&self) -> SyntaxChoice {
        self.default_syntax
    }

    fn known_attributes(&self) -> &BTreeSet<String> {
        KnownAttribute::get_all_attribute_names()
    }

    fn run_config(&self) -> TestRunConfig {
        self.run_config.clone()
    }

    fn init(
        default_syntax: SyntaxChoice,
        run_config: TestRunConfig,
        pre_compiled_deps_v2: &'a PrecompiledFilesModules,
        task_opt: Option<TaskInput<(InitCommand, EmptyCommand)>>,
    ) -> (Self, Option<String>) {
        set_stable_test_display();
        let mut session = TransactionalSession::new(&run_config.vm_config);
        session.commit(stage_precompiled_stdlib(
            session.storage(),
            pre_compiled_deps_v2,
        ));
        let adapter = Self {
            compiled_state: compiled_state_with_stdlib(
                pre_compiled_deps_v2,
                task_opt.map(|task| task.command.0),
            ),
            session,
            default_syntax,
            run_config,
        };
        (adapter, None)
    }

    fn publish_module(
        &mut self,
        module: CompiledModule,
        _named_addr_opt: Option<Identifier>,
        _gas_budget: Option<u64>,
        extra_args: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)> {
        let module_bytes = serialize_module(self.session.storage(), &module)?;
        let id = module.self_id();
        self.session
            .publish(
                id.address(),
                self.run_config.publish_compatibility(&extra_args),
                vec![module_bytes],
            )
            .map_err(|err| match err {
                PublishError::Staging(vm_error) => {
                    publish_error(&id, &vm_error, extra_args.verbose)
                },
                mono_load @ PublishError::MonoLoad { .. } => anyhow::Error::new(mono_load),
            })?;
        Ok((None, module))
    }

    /// Runs the script unmetered and returns diagnostic text only on failure.
    fn execute_script(
        &mut self,
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Option<String> {
        if let Some(error) = gas_budget_error(gas_budget) {
            return Some(error);
        }
        let script_bytes = match serialize_script(self.session.storage(), &script) {
            Ok(script_bytes) => script_bytes,
            Err(err) => return Some(format!("Error: {err}")),
        };
        let signers = self.compiled_state.resolve_signers(signers);
        let args = serialize_values(&args);
        let result = self
            .session
            .run_script(&script_bytes, &type_args, &signers, &args);
        self.run_result(result, script_execution_error, extra_args.verbose)
            .err()
            .map(|error| format!("Error: {error}"))
    }

    /// Runs the function unmetered.
    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Option<String> {
        if let Some(error) = gas_budget_error(gas_budget) {
            return Some(error);
        }
        let signers = self.compiled_state.resolve_signers(signers);
        let args = serialize_values(&args);
        let result = self
            .session
            .run(module, function, &type_args, &signers, &args);
        let error = match self.run_result(result, function_execution_error, extra_args.verbose) {
            Ok(return_values) => match self.serialized_return_values(return_values) {
                Ok(values) => return self.display_return_values(values),
                Err(err) => format!("{err:#}"),
            },
            Err(error) => error,
        };
        Some(format!("Error: {error}"))
    }

    fn view_data(
        &mut self,
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>,
    ) -> Result<String> {
        view_resource(self.session.storage(), address, module, resource, type_args)
    }

    fn handle_subcommand(&mut self, _: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        bail!("the MonoVM adapter defines no subcommands")
    }

    fn deserialize(&self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
        deserialize_value(self.session.storage(), bytes, layout)
    }
}

impl MonoVMTestAdapter<'_> {
    /// Returns successful results or formats failures, using `execution_error`
    /// for the task kind's VM diagnostics.
    fn run_result(
        &self,
        result: Result<RunOutcome, RunError>,
        execution_error: fn(&VMError, bool) -> anyhow::Error,
        verbose: bool,
    ) -> Result<Vec<(TypeTag, Vec<u8>)>, String> {
        let debugging = self.run_config.vm_config.enable_debugging;
        let vm_error = match result {
            Ok(RunOutcome::Success { return_values }) => return Ok(return_values),
            Ok(RunOutcome::Aborted {
                code,
                message,
                location,
                offset,
            }) => abort_error(code, message, location, offset, debugging),
            Err(RunError::Arguments(err)) => argument_error(err),
            Err(RunError::Vm(err)) => run_vm_error(&err, debugging),
            Err(
                err @ (RunError::VmUnsupported(_) | RunError::Unsupported(_) | RunError::Commit(_)),
            ) => return Err(err.to_string()),
        };
        Err(execution_error(&vm_error, verbose).to_string())
    }

    /// Pairs each BCS return value with the layout V1's renderer needs.
    fn serialized_return_values(
        &self,
        return_values: Vec<(TypeTag, Vec<u8>)>,
    ) -> Result<SerializedReturnValues> {
        let return_values = return_values
            .into_iter()
            .map(|(tag, bytes)| Ok((bytes, type_layout(self.session.storage(), &tag)?)))
            .collect::<Result<Vec<_>>>()?;
        Ok(SerializedReturnValues {
            mutable_reference_outputs: vec![],
            return_values,
        })
    }
}

/// Rejects gas budgets when callers bypass `supports_source`. Unmetered
/// execution may not terminate where V1 stops with `OUT_OF_GAS`.
fn gas_budget_error(gas_budget: Option<u64>) -> Option<String> {
    gas_budget.map(|budget| {
        format!(
            "Error: the MonoVM adapter runs unmetered and cannot apply a gas budget of {budget}"
        )
    })
}

/// V1's `VMError` for an abort. V1 attributes a native abort to the native's
/// own definition at offset zero; MonoVM records no instruction for it, so
/// that case carries no offset.
fn abort_error(
    code: u64,
    message: Option<String>,
    location: AbortLocation,
    offset: Option<(FunctionDefinitionIndex, BytecodeOffset)>,
    debugging: bool,
) -> VMError {
    let mut error = PartialVMError::new(StatusCode::ABORTED).with_sub_status(code);
    if let Some(message) = message {
        error = error.with_message(message);
    }
    if let Some((function, code_offset)) = offset {
        error = error.at_code_offset(function, code_offset);
    }
    if debugging {
        error = error.with_exec_state(ExecutionState::new(vec![]));
    }
    error.finish(match location {
        AbortLocation::Module(module) => Location::Module(module),
        AbortLocation::Script => Location::Script,
    })
}

/// V1's `VMError` for an argument that does not fit the function.
fn argument_error(err: ArgumentError) -> VMError {
    let (status, message) = match &err {
        ArgumentError::CountMismatch { .. } => {
            (StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH, err.to_string())
        },
        ArgumentError::Undecodable => (
            StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT,
            "[VM] failed to deserialize argument".to_string(),
        ),
    };
    PartialVMError::new(status)
        .with_message(message)
        .finish(Location::Undefined)
}

/// Converts a MonoVM failure to a `VMError` using the V1 mapping and attached
/// location. With debugging enabled, errors with instruction offsets include
/// an empty execution state. MonoVM records no stack trace, so this matches
/// V1's execution state for failures in the entry function.
fn run_vm_error(err: &VMInternalError, debugging: bool) -> VMError {
    let info = describe_or_fallback(err);
    let mut error = PartialVMError::new(info.status);
    if let Some(sub_status) = info.sub_status.known() {
        error = error.with_sub_status(sub_status);
    }
    match info.message {
        V1Message::Verbatim(text) | V1Message::MonoText(text) => error = error.with_message(text),
        V1Message::Absent => {},
    }
    let (location, offset) = v1_location(err.location());
    if let Some((function, code_offset)) = offset {
        error = error.at_code_offset(function, code_offset);
        if debugging {
            error = error.with_exec_state(ExecutionState::new(vec![]));
        }
    }
    error.finish(location)
}

/// Runs the transactional test at `path` on MonoVM against `baseline`.
pub fn run_transactional_test(
    config: TestRunConfig,
    path: &Path,
    baseline: &BaselineTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    run_test_impl_with_baseline::<MonoVMTestAdapter>(
        config,
        path,
        precompiled_v2_stdlib(),
        baseline,
    )
}
