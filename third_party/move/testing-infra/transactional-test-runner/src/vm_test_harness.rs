// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::{either_or_no_modules, run_test_impl, CompiledState, MoveTestAdapter},
    tasks::{EmptyCommand, InitCommand, SyntaxChoice, TaskInput},
};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use move_binary_format::{
    compatibility::Compatibility, deserializer::DeserializerConfig, file_format::CompiledScript,
    file_format_common, CompiledModule,
};
use move_bytecode_verifier::VerifierConfig;
use move_command_line_common::{
    address::ParsedAddress,
    env::{get_move_compiler_block_v1_from_env, get_move_compiler_v2_from_env, read_bool_env_var},
    files::verify_and_create_named_address_mapping,
    testing::{EXP_EXT, EXP_EXT_V2},
};
use move_compiler::{
    compiled_unit::AnnotatedCompiledUnit,
    shared::{
        known_attributes::KnownAttribute, string_packagepath_to_symbol_packagepath, Flags,
        NumericalAddress, PackagePaths,
    },
    FullyCompiledProgram,
};
use move_core_types::{
    account_address::AccountAddress,
    effects::ChangeSet,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::MoveValue,
};
use move_model::metadata::LanguageVersion;
use move_resource_viewer::MoveValueAnnotator;
use move_stdlib::move_stdlib_named_addresses;
use move_symbol_pool::Symbol;
use move_vm_runtime::{
    config::VMConfig, module_traversal::*, move_vm::MoveVM, session::SerializedReturnValues,
    should_use_loader_v2, TestModuleStorage, TestScriptStorage,
};
use move_vm_test_utils::{
    gas_schedule::{CostTable, Gas, GasStatus},
    InMemoryStorage,
};
use move_vm_types::resolver::ResourceResolver;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, BTreeSet},
    iter::Iterator,
    path::Path,
};

const STD_ADDR: AccountAddress = AccountAddress::ONE;

struct SimpleVMTestAdapter<'a> {
    compiled_state: CompiledState<'a>,

    // VM to be shared by all tasks. If we use V1 loader, we store None here.
    vm: Option<MoveVM>,

    // Different storages for a task: resources, modules, and scripts. Module
    // and script storages are only used if loader V2 implementation is enabled.
    resource_storage: InMemoryStorage,
    module_storage: TestModuleStorage,
    script_storage: TestScriptStorage,

    default_syntax: SyntaxChoice,
    comparison_mode: bool,
    run_config: TestRunConfig,
}

#[derive(Debug, Parser)]
pub struct AdapterPublishArgs {
    #[clap(long)]
    pub skip_check_struct_and_pub_function_linking: bool,
    #[clap(long)]
    /// is skip the struct_layout compatibility check
    pub skip_check_struct_layout: bool,
    #[clap(long)]
    /// is skip the check friend link, if true, treat `friend` as `private`
    pub skip_check_friend_linking: bool,
    /// print more complete information for VMErrors on publish
    #[clap(long)]
    pub verbose: bool,
}

#[derive(Debug, Parser)]
pub struct AdapterExecuteArgs {
    /// print more complete information for VMErrors on run
    #[clap(long)]
    pub verbose: bool,
}

fn move_test_debug() -> bool {
    static MOVE_TEST_DEBUG: Lazy<bool> = Lazy::new(|| read_bool_env_var("MOVE_TEST_DEBUG"));
    *MOVE_TEST_DEBUG
}

impl<'a> MoveTestAdapter<'a> for SimpleVMTestAdapter<'a> {
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
        comparison_mode: bool,
        run_config: TestRunConfig,
        pre_compiled_deps_v1: Option<&'a (FullyCompiledProgram, Vec<PackagePaths>)>,
        pre_compiled_deps_v2: Option<&'a PrecompiledFilesModules>,
        task_opt: Option<TaskInput<(InitCommand, EmptyCommand)>>,
    ) -> (Self, Option<String>) {
        let additional_mapping = match task_opt.map(|t| t.command) {
            Some((InitCommand { named_addresses }, _)) => {
                verify_and_create_named_address_mapping(named_addresses).unwrap()
            },
            None => BTreeMap::new(),
        };

        let mut named_address_mapping = move_stdlib_named_addresses();
        for (name, addr) in additional_mapping {
            if named_address_mapping.contains_key(&name) {
                panic!(
                    "Invalid init. The named address '{}' is reserved by the move-stdlib",
                    name
                )
            }
            named_address_mapping.insert(name, addr);
        }

        // Create the VM and storages to initialize.
        let vm = Self::create_vm();
        let mut resource_storage = InMemoryStorage::new();
        let module_storage = TestModuleStorage::empty(&DeserializerConfig::default());
        let script_storage = TestScriptStorage::empty(&DeserializerConfig::default());

        // Initialize module storages.
        for module in either_or_no_modules(pre_compiled_deps_v1, pre_compiled_deps_v2)
            .into_iter()
            .map(|tmod| &tmod.named_module.module)
        {
            let mut session = vm.new_session(&resource_storage);
            session
                .verify_module_bundle_before_publishing(
                    &[module.clone()],
                    module.self_addr(),
                    &module_storage,
                )
                .unwrap();
            drop(session);

            let mut module_bytes = vec![];
            module
                .serialize_for_version(Some(file_format_common::VERSION_MAX), &mut module_bytes)
                .unwrap();

            // We need to store both in resource and module storage for preserving
            // V1 and V2 flows.
            resource_storage.publish_or_overwrite_module(module.self_id(), module_bytes.clone());
            module_storage.add_module_bytes(
                module.self_addr(),
                module.self_name(),
                module_bytes.into(),
            );
        }

        let mut adapter = Self {
            compiled_state: CompiledState::new(
                named_address_mapping,
                pre_compiled_deps_v1,
                pre_compiled_deps_v2,
                None,
            ),
            default_syntax,
            // If we use V2 loader, we should share the VM with other tasks.
            vm: should_use_loader_v2().then_some(vm),
            resource_storage,
            module_storage,
            script_storage,
            comparison_mode,
            run_config,
        };

        let mut addr_to_name_mapping = BTreeMap::new();
        for (name, addr) in move_stdlib_named_addresses() {
            let prev = addr_to_name_mapping.insert(addr, Symbol::from(name));
            assert!(prev.is_none());
        }
        let missing_modules: Vec<_> =
            either_or_no_modules(pre_compiled_deps_v1, pre_compiled_deps_v2)
                .into_iter()
                .map(|tmod| &tmod.named_module.module)
                .filter(|module| !adapter.compiled_state.is_precompiled_dep(&module.self_id()))
                .collect();
        for module in missing_modules {
            adapter
                .compiled_state
                .add_and_generate_interface_file(module.clone())
        }
        (adapter, None)
    }

    fn publish_module(
        &mut self,
        module: CompiledModule,
        _named_addr_opt: Option<Identifier>,
        _gas_budget: Option<u64>,
        extra_args: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)> {
        let compat = if extra_args.skip_check_struct_and_pub_function_linking {
            Compatibility::no_check()
        } else {
            Compatibility::new(
                !extra_args.skip_check_struct_layout,
                !extra_args.skip_check_friend_linking,
            )
        };

        // Run verification as a separate session.
        let result = {
            if let Some(vm) = &self.vm {
                let mut session = vm.new_session(&self.resource_storage);
                session.verify_module_bundle_before_publishing_with_compat_config(
                    &[module.clone()],
                    module.self_addr(),
                    &self.module_storage,
                    compat,
                )
            } else {
                let vm = Self::create_vm();
                let mut session = vm.new_session(&self.resource_storage);
                session.verify_module_bundle_before_publishing_with_compat_config(
                    &[module.clone()],
                    module.self_addr(),
                    &self.module_storage,
                    compat,
                )
            }
        };

        match result {
            Ok(()) => {
                let mut module_bytes = vec![];
                module.serialize_for_version(
                    Some(file_format_common::VERSION_MAX),
                    &mut module_bytes,
                )?;

                self.module_storage.add_module_bytes(
                    module.self_addr(),
                    module.self_name(),
                    module_bytes.clone().into(),
                );
                self.resource_storage
                    .publish_or_overwrite_module(module.self_id(), module_bytes);
                Ok((None, module))
            },
            Err(vm_error) => Err(anyhow!(
                "Unable to publish module '{}'. Got VMError: {}",
                module.self_id(),
                vm_error.format_test_output(
                    move_test_debug() || extra_args.verbose,
                    !move_test_debug() && self.comparison_mode
                )
            )),
        }
    }

    fn execute_script(
        &mut self,
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        txn_args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<Option<String>> {
        let signers: Vec<_> = signers
            .into_iter()
            .map(|addr| self.compiled_state().resolve_address(&addr))
            .collect();

        let mut script_bytes = vec![];
        script.serialize_for_version(Some(file_format_common::VERSION_MAX), &mut script_bytes)?;

        let args = txn_args
            .iter()
            .map(|arg| arg.simple_serialize().unwrap())
            .collect::<Vec<_>>();
        // TODO rethink testing signer args
        let args = signers
            .iter()
            .map(|a| MoveValue::Signer(*a).simple_serialize().unwrap())
            .chain(args)
            .collect();

        let change_set = if let Some(vm) = &self.vm {
            self.execute_script_impl(
                vm,
                &script_bytes,
                type_args,
                args,
                gas_budget,
                extra_args.verbose,
            )?
        } else {
            let vm = Self::create_vm();
            self.execute_script_impl(
                &vm,
                &script_bytes,
                type_args,
                args,
                gas_budget,
                extra_args.verbose,
            )?
        };
        self.resource_storage.apply(change_set).unwrap();
        Ok(None)
    }

    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        txn_args: Vec<MoveValue>,
        gas_budget: Option<u64>,
        extra_args: Self::ExtraRunArgs,
    ) -> Result<(Option<String>, SerializedReturnValues)> {
        let signers: Vec<_> = signers
            .into_iter()
            .map(|addr| self.compiled_state().resolve_address(&addr))
            .collect();

        let args = txn_args
            .iter()
            .map(|arg| arg.simple_serialize().unwrap())
            .collect::<Vec<_>>();
        // TODO rethink testing signer args
        let args = signers
            .iter()
            .map(|a| MoveValue::Signer(*a).simple_serialize().unwrap())
            .chain(args)
            .collect();

        let (serialized_return_values, change_set) = if let Some(vm) = &self.vm {
            self.call_function_impl(
                vm,
                module,
                function,
                type_args,
                args,
                gas_budget,
                extra_args.verbose,
            )?
        } else {
            let vm = Self::create_vm();
            self.call_function_impl(
                &vm,
                module,
                function,
                type_args,
                args,
                gas_budget,
                extra_args.verbose,
            )?
        };
        self.resource_storage.apply(change_set).unwrap();
        Ok((None, serialized_return_values))
    }

    fn view_data(
        &mut self,
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>,
    ) -> Result<String> {
        let tag = StructTag {
            address: *module.address(),
            module: module.name().to_owned(),
            name: resource.to_owned(),
            type_args,
        };
        match self
            .resource_storage
            .get_resource_bytes_with_metadata_and_layout(&address, &tag, &[], None)
            .unwrap()
            .0
        {
            None => Ok("[No Resource Exists]".to_owned()),
            Some(data) => {
                let annotated = MoveValueAnnotator::new(self.resource_storage.clone())
                    .view_resource(&tag, &data)?;
                Ok(format!("{}", annotated))
            },
        }
    }

    fn handle_subcommand(&mut self, _: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        unreachable!()
    }
}

impl<'a> SimpleVMTestAdapter<'a> {
    fn create_vm() -> MoveVM {
        let vm_config = VMConfig {
            verifier_config: VerifierConfig::production(),
            paranoid_type_checks: true,
            ..VMConfig::default()
        };
        MoveVM::new_with_config(
            move_stdlib::natives::all_natives(
                STD_ADDR,
                // TODO: come up with a suitable gas schedule
                move_stdlib::natives::GasParameters::zeros(),
            ),
            vm_config,
        )
    }

    fn get_gas_status(&self, gas_budget: Option<u64>) -> GasStatus {
        get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            gas_budget,
        )
        .unwrap()
    }

    fn execute_script_impl(
        &self,
        vm: &MoveVM,
        script_bytes: &[u8],
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_budget: Option<u64>,
        verbose: bool,
    ) -> anyhow::Result<ChangeSet> {
        let traversal_storage = TraversalStorage::new();
        let mut gas_status = self.get_gas_status(gas_budget);

        let mut session = vm.new_session(&self.resource_storage);
        session
            .execute_script(
                script_bytes,
                ty_args,
                args,
                &mut gas_status,
                &mut TraversalContext::new(&traversal_storage),
                &self.module_storage,
                &self.script_storage,
            )
            .map_err(|vm_error| {
                anyhow!(
                    "Script execution failed with VMError: {}",
                    vm_error.format_test_output(
                        move_test_debug() || verbose,
                        !move_test_debug() && self.comparison_mode
                    )
                )
            })?;
        let change_set = session.finish()?;
        Ok(change_set)
    }

    fn call_function_impl(
        &self,
        vm: &MoveVM,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<TypeTag>,
        args: Vec<Vec<u8>>,
        gas_budget: Option<u64>,
        verbose: bool,
    ) -> anyhow::Result<(SerializedReturnValues, ChangeSet)> {
        let traversal_storage = TraversalStorage::new();
        let mut gas_status = self.get_gas_status(gas_budget);

        let mut session = vm.new_session(&self.resource_storage);
        let results = session
            .execute_function_bypass_visibility(
                module_id,
                function_name,
                ty_args,
                args,
                &mut gas_status,
                &mut TraversalContext::new(&traversal_storage),
                &self.module_storage,
            )
            .map_err(|vm_error| {
                anyhow!(
                    "Function execution failed with VMError: {}",
                    vm_error.format_test_output(
                        move_test_debug() || verbose,
                        !move_test_debug() && self.comparison_mode
                    )
                )
            })?;
        let change_set = session.finish()?;
        Ok((results, change_set))
    }
}

fn get_gas_status(cost_table: &CostTable, gas_budget: Option<u64>) -> Result<GasStatus> {
    let gas_status = if let Some(gas_budget) = gas_budget {
        // TODO(Gas): This should not be hardcoded.
        let max_gas_budget = u64::MAX.checked_div(1000).unwrap();
        if gas_budget >= max_gas_budget {
            bail!("Gas budget set too high; maximum is {}", max_gas_budget)
        }
        GasStatus::new(cost_table.clone(), Gas::new(gas_budget))
    } else {
        // no budget specified. Disable gas metering
        GasStatus::new_unmetered()
    };
    Ok(gas_status)
}

static PRECOMPILED_MOVE_STDLIB: Lazy<Option<(FullyCompiledProgram, Vec<PackagePaths>)>> =
    Lazy::new(|| {
        if get_move_compiler_block_v1_from_env() {
            return None;
        }
        let lib_paths = PackagePaths {
            name: None,
            paths: move_stdlib::move_stdlib_files(),
            named_address_map: move_stdlib::move_stdlib_named_addresses(),
        };
        let lib_paths_movesym =
            string_packagepath_to_symbol_packagepath::<NumericalAddress>(&lib_paths);
        let program_res = move_compiler::construct_pre_compiled_lib(
            vec![lib_paths],
            None,
            Flags::empty().set_skip_attribute_checks(true), // no point in checking.
            KnownAttribute::get_all_attribute_names(),
        )
        .unwrap();
        match program_res {
            Ok(stdlib) => Some((stdlib, vec![lib_paths_movesym])),
            Err((files, errors)) => {
                eprintln!("!!!Standard library failed to compile!!!");
                move_compiler::diagnostics::report_diagnostics(&files, errors)
            },
        }
    });

pub struct PrecompiledFilesModules(Vec<String>, Vec<AnnotatedCompiledUnit>);

impl PrecompiledFilesModules {
    pub fn new(files: Vec<String>, modules: Vec<AnnotatedCompiledUnit>) -> Self {
        PrecompiledFilesModules(files, modules)
    }

    pub fn filenames(&self) -> &Vec<String> {
        &self.0
    }

    pub fn units(&self) -> &Vec<AnnotatedCompiledUnit> {
        &self.1
    }
}

static PRECOMPILED_MOVE_STDLIB_V2: Lazy<PrecompiledFilesModules> = Lazy::new(|| {
    let options = move_compiler_v2::Options {
        sources: move_stdlib::move_stdlib_files(),
        sources_deps: vec![],
        dependencies: vec![],
        named_address_mapping: move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: None,
        ..move_compiler_v2::Options::default()
    };

    let (_global_env, modules) = move_compiler_v2::run_move_compiler_to_stderr(options)
        .expect("stdlib compilation succeeds");
    PrecompiledFilesModules::new(move_stdlib::move_stdlib_files(), modules)
});

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum TestRunConfig {
    CompilerV1,
    CompilerV2 {
        language_version: LanguageVersion,
        v2_experiments: Vec<(String, bool)>,
    },
    ComparisonV1V2 {
        language_version: LanguageVersion,
        v2_experiments: Vec<(String, bool)>,
    },
}

pub fn run_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_test_with_config(TestRunConfig::CompilerV1, path)
}

fn precompiled_v1_stdlib_if_needed(
    config: &TestRunConfig,
) -> Option<&'static (FullyCompiledProgram, Vec<PackagePaths>)> {
    match config {
        TestRunConfig::CompilerV1 { .. } => PRECOMPILED_MOVE_STDLIB.as_ref(),
        TestRunConfig::ComparisonV1V2 { .. } => PRECOMPILED_MOVE_STDLIB.as_ref(),
        TestRunConfig::CompilerV2 { .. } => None,
    }
}

fn precompiled_v2_stdlib_if_needed(
    config: &TestRunConfig,
) -> Option<&'static PrecompiledFilesModules> {
    match config {
        TestRunConfig::CompilerV1 { .. } => None,
        TestRunConfig::ComparisonV1V2 { .. } => Some(&*PRECOMPILED_MOVE_STDLIB_V2),
        TestRunConfig::CompilerV2 { .. } => Some(&*PRECOMPILED_MOVE_STDLIB_V2),
    }
}

pub fn run_test_with_config(
    config: TestRunConfig,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (suffix, config) =
        if get_move_compiler_v2_from_env() && !matches!(config, TestRunConfig::CompilerV2 { .. }) {
            (Some(EXP_EXT_V2.to_owned()), TestRunConfig::CompilerV2 {
                language_version: LanguageVersion::default(),
                v2_experiments: vec![],
            })
        } else {
            (Some(EXP_EXT.to_owned()), config)
        };
    let v1_lib = precompiled_v1_stdlib_if_needed(&config);
    let v2_lib = precompiled_v2_stdlib_if_needed(&config);
    run_test_impl::<SimpleVMTestAdapter>(config, path, v1_lib, v2_lib, &suffix)
}

pub fn run_test_with_config_and_exp_suffix(
    config: TestRunConfig,
    path: &Path,
    exp_suffix: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config =
        if get_move_compiler_v2_from_env() && !matches!(config, TestRunConfig::CompilerV2 { .. }) {
            TestRunConfig::CompilerV2 {
                language_version: LanguageVersion::default(),
                v2_experiments: vec![],
            }
        } else {
            config
        };
    let v1_lib = precompiled_v1_stdlib_if_needed(&config);
    let v2_lib = precompiled_v2_stdlib_if_needed(&config);
    run_test_impl::<SimpleVMTestAdapter>(config, path, v1_lib, v2_lib, exp_suffix)
}
