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
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{Location, VMResult},
    file_format::CompiledScript,
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
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::MoveValue,
};
use move_model::metadata::LanguageVersion;
use move_resource_viewer::MoveValueAnnotator;
use move_stdlib::move_stdlib_named_addresses;
use move_symbol_pool::Symbol;
use move_vm_runtime::{
    config::VMConfig,
    module_traversal::*,
    move_vm::MoveVM,
    session::{SerializedReturnValues, Session},
    IntoUnsyncCodeStorage, IntoUnsyncModuleStorage, LocalModuleBytesStorage,
    TemporaryModuleStorage,
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
    rc::Rc,
};

const STD_ADDR: AccountAddress = AccountAddress::ONE;

struct SimpleVMTestAdapter<'a> {
    compiled_state: CompiledState<'a>,

    // VM to be shared by all tasks. If we use V1 loader, we store None here.
    vm: Option<Rc<MoveVM>>,

    // Different storages for a task: resources, modules, and scripts. Module
    // and script storages are only used if loader V2 implementation is enabled.
    resource_storage: InMemoryStorage,
    module_bytes_storage: LocalModuleBytesStorage,

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

        let vm_config = vm_config();
        let vm = vm_config
            .use_loader_v2
            .then_some(Rc::new(create_vm(vm_config)));

        let mut adapter = Self {
            compiled_state: CompiledState::new(
                named_address_mapping,
                pre_compiled_deps_v1,
                pre_compiled_deps_v2,
                None,
            ),
            default_syntax,
            comparison_mode,
            run_config,
            vm,
            module_bytes_storage: LocalModuleBytesStorage::empty(),
            resource_storage: InMemoryStorage::new(),
        };

        let vm = adapter.vm();
        let mut modules_to_publish = vec![];

        for module in either_or_no_modules(pre_compiled_deps_v1, pre_compiled_deps_v2)
            .into_iter()
            .map(|tmod| &tmod.named_module.module)
        {
            let mut module_bytes = vec![];
            module
                .serialize_for_version(Some(file_format_common::VERSION_MAX), &mut module_bytes)
                .unwrap();
            let id = module.self_id();
            let sender = *id.address();

            let module_storage = adapter
                .module_bytes_storage
                .clone()
                .into_unsync_module_storage(vm.runtime_environment());

            let additional_modules_to_publish = adapter
                .perform_session_action(None, |session, gas_status| {
                    if vm.vm_config().use_loader_v2 {
                        let tmp_storage = TemporaryModuleStorage::new(
                            &sender,
                            vm.runtime_environment(),
                            &module_storage,
                            vec![module_bytes.into()],
                        )
                        .expect("All modules for initialization should publish");
                        Ok(tmp_storage.release_verified_module_bundle().collect())
                    } else {
                        #[allow(deprecated)]
                        session
                            .publish_module(module_bytes, sender, gas_status)
                            .unwrap();
                        Ok(vec![])
                    }
                })
                .unwrap();
            modules_to_publish.extend(additional_modules_to_publish);
        }

        for (bytes, module) in modules_to_publish {
            adapter
                .module_bytes_storage
                .add_module_bytes(module.address(), module.name(), bytes);
        }

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
        gas_budget: Option<u64>,
        extra_args: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)> {
        let vm = self.vm();
        let module_storage = self
            .module_bytes_storage
            .clone()
            .into_unsync_module_storage(vm.runtime_environment());

        let mut module_bytes = vec![];
        module.serialize_for_version(Some(file_format_common::VERSION_MAX), &mut module_bytes)?;

        let id = module.self_id();
        let sender = *id.address();
        let verbose = extra_args.verbose;
        let result = self.perform_session_action(gas_budget, |session, gas_status| {
            let compat = if extra_args.skip_check_struct_and_pub_function_linking {
                Compatibility::no_check()
            } else {
                Compatibility::new(
                    !extra_args.skip_check_struct_layout,
                    !extra_args.skip_check_friend_linking,
                )
            };
            if vm.vm_config().use_loader_v2 {
                let tmp_module_storage = TemporaryModuleStorage::new_with_compat_config(
                    &sender,
                    vm.runtime_environment(),
                    compat,
                    &module_storage,
                    vec![module_bytes.into()],
                )
                .map_err(|e| e.finish(Location::Undefined))?;
                Ok(tmp_module_storage
                    .release_verified_module_bundle()
                    .collect())
            } else {
                #[allow(deprecated)]
                session.publish_module_bundle_with_compat_config(
                    vec![module_bytes],
                    sender,
                    gas_status,
                    compat,
                )?;
                Ok(vec![])
            }
        });
        match result {
            Ok(modules_to_publish) => {
                for (bytes, module) in modules_to_publish {
                    self.module_bytes_storage.add_module_bytes(
                        module.address(),
                        module.name(),
                        bytes,
                    );
                }
                Ok((None, module))
            },
            Err(vm_error) => Err(anyhow!(
                "Unable to publish module '{}'. Got VMError: {}",
                module.self_id(),
                vm_error.format_test_output(
                    move_test_debug() || verbose,
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
        let vm = self.vm();
        let module_and_script_storage = self
            .module_bytes_storage
            .clone()
            .into_unsync_code_storage(vm.runtime_environment());

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
        let verbose = extra_args.verbose;
        let traversal_storage = TraversalStorage::new();
        self.perform_session_action(gas_budget, |session, gas_status| {
            session.execute_script(
                script_bytes,
                type_args,
                args,
                gas_status,
                &mut TraversalContext::new(&traversal_storage),
                &module_and_script_storage,
                &module_and_script_storage,
            )
        })
        .map_err(|vm_error| {
            anyhow!(
                "Script execution failed with VMError: {}",
                vm_error.format_test_output(
                    move_test_debug() || verbose,
                    !move_test_debug() && self.comparison_mode
                )
            )
        })?;
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
        let vm = self.vm();
        let module_storage = self
            .module_bytes_storage
            .clone()
            .into_unsync_module_storage(vm.runtime_environment());

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
        let verbose = extra_args.verbose;
        let traversal_storage = TraversalStorage::new();
        let serialized_return_values = self
            .perform_session_action(gas_budget, |session, gas_status| {
                session.execute_function_bypass_visibility(
                    module,
                    function,
                    type_args,
                    args,
                    gas_status,
                    &mut TraversalContext::new(&traversal_storage),
                    &module_storage,
                )
            })
            .map_err(|vm_error| {
                anyhow!(
                    "Function execution failed with VMError: {}",
                    vm_error.format_test_output(
                        move_test_debug() || verbose,
                        !move_test_debug() && self.comparison_mode
                    )
                )
            })?;
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
    fn vm(&self) -> Rc<MoveVM> {
        match &self.vm {
            Some(vm) => vm.clone(),
            None => Rc::new(create_vm(vm_config())),
        }
    }

    fn perform_session_action<Ret>(
        &mut self,
        gas_budget: Option<u64>,
        f: impl FnOnce(&mut Session, &mut GasStatus) -> VMResult<Ret>,
    ) -> VMResult<Ret> {
        let vm = self.vm();
        let (mut session, mut gas_status) = {
            let gas_status = get_gas_status(
                &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
                gas_budget,
            )
            .unwrap();
            let session = vm.new_session(&self.resource_storage);
            (session, gas_status)
        };

        // perform op
        let res = f(&mut session, &mut gas_status)?;

        // save changeset
        let changeset = session.finish()?;
        self.resource_storage.apply(changeset).unwrap();
        Ok(res)
    }
}

fn vm_config() -> VMConfig {
    VMConfig {
        verifier_config: VerifierConfig::production(),
        paranoid_type_checks: true,
        ..VMConfig::default()
    }
}

fn create_vm(vm_config: VMConfig) -> MoveVM {
    MoveVM::new_with_config(
        move_stdlib::natives::all_natives(
            STD_ADDR,
            // TODO: come up with a suitable gas schedule
            move_stdlib::natives::GasParameters::zeros(),
        ),
        vm_config,
    )
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
