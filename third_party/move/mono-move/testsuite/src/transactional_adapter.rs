// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM adapter for the shared transactional test framework.
//!
//! Task parsing, compilation, and publishing checks use V1's infrastructure.
//! Publish tasks also apply MonoVM loading checks before updating storage.

use crate::transactional_session::{PublishError, TransactionalSession};
use anyhow::{bail, Result};
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use move_binary_format::{
    errors::set_stable_test_display, file_format::CompiledScript, CompiledModule,
};
use move_command_line_common::address::ParsedAddress;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use move_transactional_test_runner::{
    framework::{run_test_impl_with_baseline, BaselineTarget, CompiledState, MoveTestAdapter},
    tasks::{taskify, EmptyCommand, InitCommand, SyntaxChoice, TaskCommand, TaskInput},
    vm_test_harness::{
        compiled_state_with_stdlib, deserialize_value, precompiled_v2_stdlib, publish_error,
        serialize_module, stage_precompiled_stdlib, AdapterExecuteArgs, AdapterPublishArgs,
        PrecompiledFilesModules, TestRunConfig,
    },
};
use move_vm_types::values::Value;
use std::{collections::BTreeSet, path::Path};

/// Task syntax and arguments parsed for [`MonoVMTestAdapter`].
type MonoVMTaskCommand =
    TaskCommand<EmptyCommand, AdapterPublishArgs, (), AdapterExecuteArgs, EmptyCommand>;

/// Returns whether the source contains only supported tasks: initialization,
/// bytecode printing, and module publishing.
pub fn supports_source(path: &Path) -> Result<bool> {
    Ok(taskify::<MonoVMTaskCommand>(path)?
        .iter()
        .all(|task| match &task.command {
            TaskCommand::Init(..) | TaskCommand::PrintBytecode(..) | TaskCommand::Publish(..) => {
                true
            },
            TaskCommand::Run(..) | TaskCommand::View(..) | TaskCommand::Subcommand(..) => false,
        }))
}

/// Panics on an unsupported task that should have been filtered out by
/// [`supports_source`].
fn unsupported(task: &str) -> ! {
    panic!(
        "the MonoVM adapter does not support `{task}` tasks yet; filter sources with \
         `supports_source`"
    )
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
        unsupported("run")
    }

    fn call_function(
        &mut self,
        _module: &ModuleId,
        _function: &IdentStr,
        _type_args: Vec<TypeTag>,
        _signers: Vec<ParsedAddress>,
        _args: Vec<MoveValue>,
        _gas_budget: Option<u64>,
        _extra_args: Self::ExtraRunArgs,
    ) -> Option<String> {
        unsupported("run")
    }

    fn view_data(
        &mut self,
        _address: AccountAddress,
        _module: &ModuleId,
        _resource: &IdentStr,
        _type_args: Vec<TypeTag>,
    ) -> Result<String> {
        unsupported("view")
    }

    fn handle_subcommand(&mut self, _: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        bail!("the MonoVM adapter defines no subcommands")
    }

    fn deserialize(&self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
        deserialize_value(self.session.storage(), bytes, layout)
    }
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
