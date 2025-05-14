// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::{run_test_impl, CompiledState, MoveTestAdapter},
    tasks::{EmptyCommand, InitCommand, SyntaxChoice, TaskInput},
};
use anyhow::{anyhow, bail, Result};
use clap::Parser;
use legacy_move_compiler::{
    compiled_unit::{AnnotatedCompiledModule, AnnotatedCompiledUnit},
    shared::known_attributes::KnownAttribute,
};
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{Location, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_bytecode_verifier::VerifierConfig;
use move_command_line_common::{
    address::ParsedAddress, env::read_bool_env_var, files::verify_and_create_named_address_mapping,
    testing::EXP_EXT,
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
    data_cache::TransactionDataCache,
    module_traversal::*,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    AsUnsyncCodeStorage, AsUnsyncModuleStorage, CodeStorage, LoadedFunction, ModuleStorage,
    RuntimeEnvironment, StagingModuleStorage,
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
    storage: InMemoryStorage,
    default_syntax: SyntaxChoice,
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
        run_config: TestRunConfig,
        pre_compiled_deps_v2: &'a PrecompiledFilesModules,
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

        let vm_config = match &run_config {
            TestRunConfig::CompilerV2 { vm_config, .. } => vm_config.clone(),
        };
        let runtime_environment = create_runtime_environment(vm_config);
        let storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);
        let max_binary_format_version = storage.max_binary_format_version();

        let mut adapter = Self {
            compiled_state: CompiledState::new(named_address_mapping, pre_compiled_deps_v2, None),
            default_syntax,
            run_config,
            storage,
        };

        let module_storage = adapter.storage.clone().into_unsync_module_storage();

        let addresses = pre_compiled_deps_v2
            .get_pre_compiled_modules()
            .iter()
            .map(|tmod| *tmod.named_module.module.self_addr())
            .collect::<BTreeSet<_>>();
        assert_eq!(addresses.len(), 1);

        let sender = *addresses.first().unwrap();
        let module_bundle = pre_compiled_deps_v2
            .get_pre_compiled_modules()
            .into_iter()
            .map(|tmod| {
                let mut module_bytes = vec![];
                tmod.named_module
                    .module
                    .serialize_for_version(Some(max_binary_format_version), &mut module_bytes)
                    .unwrap();
                module_bytes.into()
            })
            .collect();

        StagingModuleStorage::create(&sender, &module_storage, module_bundle)
            .expect("All modules should publish")
            .release_verified_module_bundle()
            .into_iter()
            .for_each(|(module_id, bytes)| {
                adapter
                    .storage
                    .add_module_bytes(module_id.address(), module_id.name(), bytes);
            });

        let mut addr_to_name_mapping = BTreeMap::new();
        for (name, addr) in move_stdlib_named_addresses() {
            let prev = addr_to_name_mapping.insert(addr, Symbol::from(name));
            assert!(prev.is_none());
        }
        let missing_modules: Vec<_> = pre_compiled_deps_v2
            .get_pre_compiled_modules()
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
        let module_storage = self.storage.clone().into_unsync_module_storage();

        let mut module_bytes = vec![];
        module.serialize_for_version(
            Some(self.storage.max_binary_format_version()),
            &mut module_bytes,
        )?;

        let id = module.self_id();
        let sender = *id.address();
        let verbose = extra_args.verbose;

        let compat = if extra_args.skip_check_struct_and_pub_function_linking {
            Compatibility::no_check()
        } else {
            Compatibility::new(
                !extra_args.skip_check_struct_layout,
                !extra_args.skip_check_friend_linking,
                false,
            )
        };
        let staging_module_storage = StagingModuleStorage::create_with_compat_config(
            &sender,
            compat,
            &module_storage,
            vec![module_bytes.into()],
        )
        .map_err(|err| {
            anyhow!(
                "Unable to publish module '{}'. Got VMError: {}",
                module.self_id(),
                err.format_test_output(move_test_debug() || verbose)
            )
        })?;
        for (module_id, bytes) in staging_module_storage
            .release_verified_module_bundle()
            .into_iter()
        {
            self.storage
                .add_module_bytes(module_id.address(), module.name(), bytes);
        }
        Ok((None, module))
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
        let code_storage = self.storage.clone().into_unsync_code_storage();

        let signers: Vec<_> = signers
            .into_iter()
            .map(|addr| self.compiled_state().resolve_address(&addr))
            .collect();

        let mut script_bytes = vec![];
        script.serialize_for_version(
            Some(self.storage.max_binary_format_version()),
            &mut script_bytes,
        )?;

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

        code_storage
            .load_script(&script_bytes, &type_args)
            .and_then(|func| self.execute_loaded_function(func, args, gas_budget, &code_storage))
            .map_err(|err| {
                anyhow!(
                    "Script execution failed with VMError: {}",
                    err.format_test_output(move_test_debug() || verbose)
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
        let module_storage = self.storage.clone().into_unsync_module_storage();

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

        let serialized_return_values = module_storage
            .load_function(module, function, &type_args)
            .and_then(|func| self.execute_loaded_function(func, args, gas_budget, &module_storage))
            .map_err(|err| {
                anyhow!(
                    "Function execution failed with VMError: {}",
                    err.format_test_output(move_test_debug() || verbose)
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
            .storage
            .get_resource_bytes_with_metadata_and_layout(&address, &tag, &[], None)
            .unwrap()
            .0
        {
            None => Ok("[No Resource Exists]".to_owned()),
            Some(data) => {
                let annotated =
                    MoveValueAnnotator::new(self.storage.clone()).view_resource(&tag, &data)?;
                Ok(format!("{}", annotated))
            },
        }
    }

    fn handle_subcommand(&mut self, _: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        unreachable!()
    }
}

impl SimpleVMTestAdapter<'_> {
    fn execute_loaded_function(
        &mut self,
        function: LoadedFunction,
        args: Vec<Vec<u8>>,
        gas_budget: Option<u64>,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        let mut gas_status = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            gas_budget,
        )
        .unwrap();

        let traversal_storage = TraversalStorage::new();
        let mut extensions = NativeContextExtensions::default();

        let mut data_cache = TransactionDataCache::empty();
        let return_values = MoveVM::execute_loaded_function(
            function,
            args,
            &mut data_cache,
            &mut gas_status,
            &mut TraversalContext::new(&traversal_storage),
            &mut extensions,
            module_storage,
            &self.storage,
        )?;

        let change_set = data_cache
            .into_effects(module_storage)
            .map_err(|err| err.finish(Location::Undefined))?;
        self.storage.apply(change_set).unwrap();
        Ok(return_values)
    }
}

fn create_runtime_environment(vm_config: VMConfig) -> RuntimeEnvironment {
    RuntimeEnvironment::new_with_config(
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

    pub fn get_pre_compiled_modules(&self) -> Vec<&AnnotatedCompiledModule> {
        self.units()
            .iter()
            .filter_map(|unit| {
                if let AnnotatedCompiledUnit::Module(m) = unit {
                    Some(m)
                } else {
                    None
                }
            })
            .collect()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestRunConfig {
    CompilerV2 {
        language_version: LanguageVersion,
        /// List of experiments and whether to enable them or not.
        experiments: Vec<(String, bool)>,
        /// Configuration for the VM that runs tests.
        vm_config: VMConfig,
    },
}

impl TestRunConfig {
    /// Returns compiler V2 config with default VM config.
    pub fn compiler_v2(
        language_version: LanguageVersion,
        experiments: Vec<(String, bool)>,
    ) -> Self {
        Self::CompilerV2 {
            language_version,
            experiments,
            vm_config: VMConfig {
                verifier_config: VerifierConfig::production(),
                paranoid_type_checks: true,
                ..VMConfig::default()
            },
        }
    }
}

pub fn run_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_test_with_config(
        TestRunConfig::compiler_v2(LanguageVersion::default(), vec![]),
        path,
    )
}

fn precompiled_v2_stdlib() -> &'static PrecompiledFilesModules {
    &PRECOMPILED_MOVE_STDLIB_V2
}

pub fn run_test_with_config(
    config: TestRunConfig,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (suffix, config) = (Some(EXP_EXT.to_owned()), config);
    let v2_lib = precompiled_v2_stdlib();
    run_test_impl::<SimpleVMTestAdapter>(config, path, v2_lib, &suffix)
}

pub fn run_test_with_config_and_exp_suffix(
    config: TestRunConfig,
    path: &Path,
    exp_suffix: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let v2_lib = precompiled_v2_stdlib();
    run_test_impl::<SimpleVMTestAdapter>(config, path, v2_lib, exp_suffix)
}
