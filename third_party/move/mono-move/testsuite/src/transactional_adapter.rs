// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM adapter for the shared transactional test framework.
//!
//! Task parsing, compilation, and publishing checks use V1's infrastructure.
//! Publish tasks also apply MonoVM loading checks before updating storage.
//! Run tasks execute on MonoVM and render outcomes in V1's baseline format.

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
        precompiled_v2_stdlib, publish_error, serialize_module, stage_precompiled_stdlib,
        type_layout, view_resource, AdapterExecuteArgs, AdapterPublishArgs,
        PrecompiledFilesModules, TestRunConfig,
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
/// that name a module function. Script runs are unsupported, and so are runs
/// with a `--gas-budget`: the adapter runs unmetered, so a run that V1 ends
/// with `OUT_OF_GAS` would not terminate.
pub fn supports_source(path: &Path) -> Result<bool> {
    Ok(taskify::<MonoVMTaskCommand>(path)?
        .iter()
        .all(|task| match &task.command {
            TaskCommand::Init(..)
            | TaskCommand::PrintBytecode(..)
            | TaskCommand::Publish(..)
            | TaskCommand::View(..) => true,
            TaskCommand::Run(command, _) => command.name.is_some() && command.gas_budget.is_none(),
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

    fn execute_script(
        &mut self,
        _script: CompiledScript,
        _type_args: Vec<TypeTag>,
        _signers: Vec<ParsedAddress>,
        _args: Vec<MoveValue>,
        _gas_budget: Option<u64>,
        _extra_args: Self::ExtraRunArgs,
    ) -> Option<String> {
        panic!(
            "the MonoVM adapter does not support script tasks yet; filter sources with \
             `supports_source`"
        )
    }

    /// Runs the function unmetered; `supports_source` keeps `--gas-budget`
    /// runs out.
    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        args: Vec<MoveValue>,
        _gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Option<String> {
        let signers = signers
            .into_iter()
            .map(|addr| self.compiled_state().resolve_address(&addr))
            .collect::<Vec<_>>();
        let args = serialize_values(&args);
        let debugging = self.run_config.vm_config.enable_debugging;
        let failed = |vm_error: VMError| function_execution_error(&vm_error, extra_args.verbose);

        let error = match self
            .session
            .run(module, function, &type_args, &signers, &args)
        {
            Ok(RunOutcome::Success { return_values }) => {
                match self.serialized_return_values(return_values) {
                    Ok(values) => return self.display_return_values(values),
                    Err(err) => format!("{err:#}"),
                }
            },
            Ok(RunOutcome::Aborted {
                code,
                message,
                location,
                offset,
            }) => failed(abort_error(code, message, location, offset, debugging)).to_string(),
            Err(RunError::Arguments(err)) => failed(argument_error(err)).to_string(),
            Err(RunError::Vm(err)) => failed(run_vm_error(&err, debugging)).to_string(),
            Err(
                err @ (RunError::VmUnsupported(_) | RunError::Unsupported(_) | RunError::Commit(_)),
            ) => err.to_string(),
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
