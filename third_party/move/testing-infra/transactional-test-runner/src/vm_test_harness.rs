// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    framework::{
        merge_output, run_test_impl, run_test_impl_with_baseline, BaselineTarget, CompiledState,
        MoveTestAdapter,
    },
    tasks::{EmptyCommand, InitCommand, SyntaxChoice, TaskInput},
};
use anyhow::{anyhow, bail, Result};
use bytes::Bytes;
use clap::Parser;
use legacy_move_compiler::{
    compiled_unit::{AnnotatedCompiledModule, AnnotatedCompiledUnit},
    shared::known_attributes::KnownAttribute,
};
use move_binary_format::{
    compatibility::Compatibility,
    errors,
    errors::{PartialVMResult, VMError, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_bytecode_utils::layout::TypeLayoutBuilder;
use move_bytecode_verifier::VerifierConfig;
use move_command_line_common::{
    address::{NumericalAddress, ParsedAddress},
    env::read_bool_env_var,
    files::verify_and_create_named_address_mapping,
    testing::EXP_EXT,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveTypeLayout, MoveValue},
    vm_status::StatusType,
};
use move_model::metadata::LanguageVersion;
use move_resource_viewer::MoveValueAnnotator;
use move_stdlib::move_stdlib_named_addresses;
use move_vm_runtime::{
    config::VMConfig,
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    execution_tracing::{FullTraceRecorder, Trace, TraceRecorder},
    module_traversal::*,
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    native_functions::{make_table_from_iter, NativeContext, NativeFunction},
    AsFunctionValueExtension, AsUnsyncCodeStorage, AsUnsyncModuleStorage, CodeStorage,
    InstantiatedFunctionLoader, LegacyLoaderConfig, RuntimeEnvironment, ScriptLoader,
    StagingModuleStorage, TypeChecker, VerifiedModuleBundle,
};
use move_vm_test_utils::{
    gas_schedule::{CostTable, Gas, GasStatus},
    InMemoryStorage,
};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    resolver::ResourceResolver,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{Struct, Value},
};
use once_cell::sync::Lazy;
use smallvec::smallvec;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    iter::Iterator,
    path::Path,
    sync::Arc,
};

const STD_ADDR: AccountAddress = AccountAddress::ONE;

const ORDERING_LESS_VARIANT: u16 = 0;
const ORDERING_EQUAL_VARIANT: u16 = 1;
const ORDERING_GREATER_VARIANT: u16 = 2;

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

/// Specifies entrypoint to dispatch execution of a script or a Move function.
enum EntryPoint<'a> {
    Script {
        script_bytes: &'a [u8],
    },
    Function {
        module: &'a ModuleId,
        function: &'a IdentStr,
    },
}

#[derive(Debug, Parser)]
pub struct AdapterExecuteArgs {
    /// Print more complete information for VM errors during the run.
    #[clap(long)]
    pub verbose: bool,
    #[clap(long)]
    /// Displays the trace collected during execution.
    pub display_trace: bool,
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
        // Set stable test display of VM Errors so we can use the --verbose flag in baseline tests
        errors::set_stable_test_display();

        let runtime_environment = create_runtime_environment(run_config.vm_config.clone());
        let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);
        for (module_id, bytes) in stage_precompiled_stdlib(&storage, pre_compiled_deps_v2) {
            storage.add_module_bytes(module_id.address(), module_id.name(), bytes);
        }

        let adapter = Self {
            compiled_state: compiled_state_with_stdlib(
                pre_compiled_deps_v2,
                task_opt.map(|task| task.command.0),
            ),
            default_syntax,
            run_config,
            storage,
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
        let module_storage = self.storage.clone().into_unsync_module_storage();
        let module_bytes = serialize_module(&self.storage, &module)?;
        let id = module.self_id();
        let sender = *id.address();

        let staging_module_storage = StagingModuleStorage::create_with_compat_config(
            &sender,
            self.run_config.publish_compatibility(&extra_args),
            &module_storage,
            vec![module_bytes],
        )
        .map_err(|err| publish_error(&id, &err, extra_args.verbose))?;
        for (module_id, bytes) in staging_module_storage
            .release_verified_module_bundle()
            .into_iter()
        {
            self.storage
                .add_module_bytes(module_id.address(), module_id.name(), bytes);
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
    ) -> Option<String> {
        let code_storage = self.storage.clone().into_unsync_code_storage();

        let signers: Vec<_> = signers
            .into_iter()
            .map(|addr| self.compiled_state().resolve_address(&addr))
            .collect();

        let mut script_bytes = vec![];
        if let Err(err) = script.serialize_for_version(
            Some(self.storage.max_binary_format_version()),
            &mut script_bytes,
        ) {
            return Some(format!("Error: {}", err));
        }

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

        let (result, trace) = self.execute_entrypoint(
            EntryPoint::Script {
                script_bytes: &script_bytes,
            },
            &type_args,
            args,
            gas_budget,
            &code_storage,
        );

        let trace_str =
            trace.and_then(|t| extra_args.display_trace.then_some(t.to_string_for_tests()));
        match result {
            Ok(_) => trace_str,
            Err(err) => {
                let err = anyhow!(
                    "Script execution failed with VMError: {}",
                    err.format_test_output(move_test_debug() || verbose)
                );
                let err_str = Some(format!("Error: {}", err));
                merge_output(trace_str, err_str)
            },
        }
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
    ) -> Option<String> {
        let code_storage = self.storage.clone().into_unsync_code_storage();

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

        let (result, trace) = self.execute_entrypoint(
            EntryPoint::Function { module, function },
            &type_args,
            args,
            gas_budget,
            &code_storage,
        );

        let trace_str =
            trace.and_then(|t| extra_args.display_trace.then_some(t.to_string_for_tests()));
        match result {
            Ok(return_values) => {
                let rendered_return_value = self.display_return_values(return_values);
                merge_output(trace_str, rendered_return_value)
            },
            Err(err) => {
                let err_str = Some(format!(
                    "Error: {}",
                    function_execution_error(&err, verbose)
                ));
                merge_output(trace_str, err_str)
            },
        }
    }

    fn view_data(
        &mut self,
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>,
    ) -> Result<String> {
        view_resource(&self.storage, address, module, resource, type_args)
    }

    fn handle_subcommand(&mut self, _: TaskInput<Self::Subcommand>) -> Result<Option<String>> {
        unreachable!()
    }

    fn deserialize(&self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value> {
        deserialize_value(&self.storage, bytes, layout)
    }
}

/// Formats a publishing failure for transactional test baselines.
pub fn publish_error(module_id: &ModuleId, err: &VMError, verbose: bool) -> anyhow::Error {
    anyhow!(
        "Unable to publish module '{}'. Got VMError: {}",
        module_id,
        err.format_test_output(move_test_debug() || verbose)
    )
}

/// Formats a function execution failure for transactional test baselines.
pub fn function_execution_error(err: &VMError, verbose: bool) -> anyhow::Error {
    anyhow!(
        "Function execution failed with VMError: {}",
        err.format_test_output(move_test_debug() || verbose)
    )
}

/// Renders the resource of type `module::resource<type_args>` at `address`
/// from `storage`.
pub fn view_resource(
    storage: &InMemoryStorage,
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
    match storage
        .get_resource_bytes_with_metadata_and_layout(&address, &tag, &[], None)
        .unwrap()
        .0
    {
        None => Ok("[No Resource Exists]".to_owned()),
        Some(data) => {
            let annotated = MoveValueAnnotator::new(storage.clone()).view_resource(&tag, &data)?;
            Ok(format!("{}", annotated))
        },
    }
}

/// The runtime layout of `tag`, resolving struct definitions through
/// `storage`. The builder refuses `signer`, which a test function may still
/// return, so signers and the vectors around them are built here.
// TODO(completeness): the builder also rejects enums and signer- or
// function-typed fields, which V1's loader-derived layouts render.
pub fn type_layout(storage: &InMemoryStorage, tag: &TypeTag) -> Result<MoveTypeLayout> {
    match tag {
        TypeTag::Signer => Ok(MoveTypeLayout::Signer),
        TypeTag::Vector(elem) => Ok(MoveTypeLayout::Vector(Box::new(type_layout(
            storage, elem,
        )?))),
        TypeTag::Bool
        | TypeTag::U8
        | TypeTag::U16
        | TypeTag::U32
        | TypeTag::U64
        | TypeTag::U128
        | TypeTag::U256
        | TypeTag::I8
        | TypeTag::I16
        | TypeTag::I32
        | TypeTag::I64
        | TypeTag::I128
        | TypeTag::I256
        | TypeTag::Address
        | TypeTag::Struct(_)
        | TypeTag::Function(_) => TypeLayoutBuilder::build_runtime(tag, storage),
    }
}

/// Deserializes a value using `storage` to resolve function argument types
/// for any function values it contains.
pub fn deserialize_value(
    storage: &InMemoryStorage,
    bytes: &[u8],
    layout: &MoveTypeLayout,
) -> Option<Value> {
    let module_storage = storage.as_unsync_module_storage();
    let function_extension = module_storage.as_function_value_extension();
    ValueSerDeContext::new(function_extension.max_value_nest_depth())
        .with_func_args_deserialization(&function_extension)
        .deserialize(bytes, layout)
}

/// Serializes `module` at the maximum binary format version supported by `storage`.
pub fn serialize_module(storage: &InMemoryStorage, module: &CompiledModule) -> Result<Bytes> {
    let mut module_bytes = vec![];
    module.serialize_for_version(Some(storage.max_binary_format_version()), &mut module_bytes)?;
    Ok(module_bytes.into())
}

impl SimpleVMTestAdapter<'_> {
    fn execute_entrypoint(
        &mut self,
        entry_point: EntryPoint,
        ty_args: &[TypeTag],
        args: Vec<Vec<u8>>,
        gas_budget: Option<u64>,
        code_storage: &impl CodeStorage,
    ) -> (VMResult<SerializedReturnValues>, Option<Trace>) {
        let mut gas_meter = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            gas_budget,
        )
        .unwrap();

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);
        let mut extensions = NativeContextExtensions::default();
        let mut data_cache = TransactionDataCache::empty();

        let (return_values, trace) = dispatch_loader!(code_storage, loader, {
            let legacy_loader_config = LegacyLoaderConfig::unmetered();
            let result = match entry_point {
                EntryPoint::Script { script_bytes } => loader.load_script(
                    &legacy_loader_config,
                    &mut gas_meter,
                    &mut traversal_context,
                    script_bytes,
                    ty_args,
                ),
                EntryPoint::Function { module, function } => loader.load_instantiated_function(
                    &legacy_loader_config,
                    &mut gas_meter,
                    &mut traversal_context,
                    module,
                    function,
                    ty_args,
                ),
            };
            let function = match result {
                Ok(function) => function,
                Err(err) => return (Err(err), None),
            };

            let mut data_cache =
                MoveVmDataCacheAdapter::new(&mut data_cache, &self.storage, &loader);
            if self.run_config.tracing {
                let mut logger = FullTraceRecorder::new();
                let result = MoveVM::execute_loaded_function_with_tracing(
                    function,
                    args,
                    &mut data_cache,
                    &mut gas_meter,
                    &mut traversal_context,
                    &mut extensions,
                    &loader,
                    &mut logger,
                );
                let trace = logger.finish();
                let replay_result = TypeChecker::new(code_storage).replay(&trace);
                if let Err(err) = &replay_result {
                    // Replay validates that the execution trace is well-typed. If replay fails,
                    // it should only be due to invariant violations (e.g., type mismatches in the trace).
                    // Any other error type indicates an unexpected failure in the replay mechanism itself,
                    // which we want to catch immediately during testing.
                    if err.status_type() != StatusType::InvariantViolation {
                        panic!(
                            "Replay should never fail with non-invariant violation: {:?}",
                            err
                        );
                    }
                }
                match replay_result.and(result) {
                    Ok(return_values) => (return_values, Some(trace)),
                    Err(err) => return (Err(err), Some(trace)),
                }
            } else {
                let result = MoveVM::execute_loaded_function(
                    function,
                    args,
                    &mut data_cache,
                    &mut gas_meter,
                    &mut traversal_context,
                    &mut extensions,
                    &loader,
                );
                match result {
                    Ok(return_values) => (return_values, None),
                    Err(err) => return (Err(err), None),
                }
            }
        });

        let change_set = data_cache
            .into_effects(code_storage)
            .expect("Producing a change set always succeeds");
        self.storage.apply(change_set).unwrap();
        (Ok(return_values), trace)
    }
}

fn native_compare(
    _context: &mut NativeContext,
    ty_args: &[Type],
    arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(arguments.len(), 2);

    let variant = match arguments[0].compare(&arguments[1])? {
        std::cmp::Ordering::Less => ORDERING_LESS_VARIANT,
        std::cmp::Ordering::Equal => ORDERING_EQUAL_VARIANT,
        std::cmp::Ordering::Greater => ORDERING_GREATER_VARIANT,
    };
    let result = Value::struct_(Struct::pack(vec![Value::u16(variant)]));
    Ok(NativeResult::ok(0.into(), smallvec![result]))
}

/// Creates the test runtime environment with stdlib natives and `cmp::compare`.
pub fn create_runtime_environment(vm_config: VMConfig) -> RuntimeEnvironment {
    let mut natives = move_stdlib::natives::all_natives(
        STD_ADDR,
        // TODO: come up with a suitable gas schedule
        move_stdlib::natives::GasParameters::zeros(),
    );
    natives.extend(make_table_from_iter(STD_ADDR, [(
        "cmp",
        "compare",
        Arc::new(native_compare) as NativeFunction,
    )]));
    RuntimeEnvironment::new_with_config(natives, vm_config)
}

/// Combines stdlib named addresses with those declared by `//# init`.
/// Panics if the declarations are invalid or reuse a stdlib name.
fn test_named_address_mapping(init: Option<InitCommand>) -> BTreeMap<String, NumericalAddress> {
    let additional_mapping = match init {
        Some(InitCommand { named_addresses }) => {
            verify_and_create_named_address_mapping(named_addresses)
                .expect("the `//# init` task declares well-formed named addresses")
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
    named_address_mapping
}

/// Initializes compiled state with the precompiled stdlib and the test's named addresses.
pub fn compiled_state_with_stdlib<'a>(
    pre_compiled_deps_v2: &'a PrecompiledFilesModules,
    init: Option<InitCommand>,
) -> CompiledState<'a> {
    CompiledState::new(test_named_address_mapping(init), pre_compiled_deps_v2, None)
}

/// Stages the precompiled stdlib with V1's publishing checks against empty
/// module storage. Returns the verified bundle for the caller to store.
pub fn stage_precompiled_stdlib(
    storage: &InMemoryStorage,
    pre_compiled_deps_v2: &PrecompiledFilesModules,
) -> VerifiedModuleBundle<ModuleId, Bytes> {
    let modules = pre_compiled_deps_v2.get_pre_compiled_modules();
    let addresses = modules
        .iter()
        .map(|tmod| *tmod.named_module.module.self_addr())
        .collect::<BTreeSet<_>>();
    assert_eq!(addresses.len(), 1);
    let sender = *addresses
        .first()
        .expect("the precompiled stdlib lives at one address");

    let module_bundle = modules
        .into_iter()
        .map(|tmod| {
            serialize_module(storage, &tmod.named_module.module)
                .expect("the precompiled stdlib serializes")
        })
        .collect();
    StagingModuleStorage::create(&sender, &storage.as_unsync_module_storage(), module_bundle)
        .expect("All modules should publish")
        .release_verified_module_bundle()
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
    let mut sources = move_stdlib::move_stdlib_files();
    sources.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/cmp.move")
            .to_string_lossy()
            .into_owned(),
    );
    let options = move_compiler_v2::Options {
        sources: sources.clone(),
        sources_deps: vec![],
        dependencies: vec![],
        named_address_mapping: move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: None,
        ..move_compiler_v2::Options::default()
    };

    let (_global_env, modules) = move_compiler_v2::run_move_compiler_to_stderr(options)
        .expect("stdlib compilation succeeds");
    PrecompiledFilesModules::new(sources, modules)
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunConfig {
    pub language_version: LanguageVersion,
    /// List of experiments and whether to enable them or not.
    pub experiments: Vec<(String, bool)>,
    /// Configuration for the VM that runs tests.
    pub vm_config: VMConfig,
    /// Whether to print each command executed to test output.
    pub echo: bool,
    /// Set of targets into which to cross-compile.
    pub cross_compilation_targets: BTreeSet<CrossCompileTarget>,
    /// If enabled, records execution trace (disabling runtime type checks).
    pub tracing: bool,
}

/// A cross-compile target. A new transactional test source file
/// is generated for the target, with all embedded source code
/// replaced by the result of decompiling or disassembling it.
/// The file is placed in `<path>.decompiled` and `<path>.disassembled`,
/// respectively.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrossCompileTarget {
    /// The syntax into which to cross-compile.
    pub syntax: SyntaxChoice,
    /// Whether the cross-compiled result should be run as a test
    /// after cross-compilation.
    pub run_after: bool,
    /// Optional suffix to append to the file name of the code for cross compilation.
    pub suffix: Option<String>,
}

impl Default for TestRunConfig {
    fn default() -> Self {
        TestRunConfig::new(LanguageVersion::latest(), vec![])
    }
}

impl TestRunConfig {
    /// Returns compiler V2 config with default VM config.
    pub fn new(language_version: LanguageVersion, experiments: Vec<(String, bool)>) -> Self {
        Self {
            language_version,
            experiments,
            // `optimize_trusted_code` stays at its `false` default: tests trust
            // no code, so framework frames are paranoid-checked like any other.
            // The move-vm suite's matrix entries set it to `true` because that
            // corpus is where trusted-code skipping itself is tested.
            vm_config: VMConfig {
                verifier_config: VerifierConfig::production_testing(),
                paranoid_type_checks: true,
                enable_enum_option: false,
                ..VMConfig::default_for_test()
            },
            echo: true,
            cross_compilation_targets: BTreeSet::new(),
            tracing: false,
        }
    }

    pub fn cross_compile_into(
        self,
        syntax: SyntaxChoice,
        run_after: bool,
        suffix: Option<String>,
    ) -> Self {
        assert!(matches!(syntax, SyntaxChoice::ASM | SyntaxChoice::Source));
        let mut cross_compilation_targets = self.cross_compilation_targets.clone();
        cross_compilation_targets.insert(CrossCompileTarget {
            syntax,
            run_after,
            suffix,
        });
        Self {
            cross_compilation_targets,
            ..self
        }
    }

    pub fn with_echo(self) -> Self {
        Self { echo: true, ..self }
    }

    pub(crate) fn verifier_disabled(&self) -> bool {
        self.vm_config.verifier_config.verify_nothing()
    }

    /// Selects compatibility checks for a `//# publish` task from its flags and VM config.
    pub fn publish_compatibility(&self, args: &AdapterPublishArgs) -> Compatibility {
        if args.skip_check_struct_and_pub_function_linking || self.verifier_disabled() {
            Compatibility::no_check()
        } else {
            Compatibility::new(
                !args.skip_check_struct_layout,
                !args.skip_check_friend_linking,
                true,
                false,
                true,
            )
        }
    }

    pub fn with_runtime_ref_checks(self) -> Self {
        Self {
            vm_config: self.vm_config.set_paranoid_ref_checks(true),
            ..self
        }
    }
}

pub fn run_test(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    run_test_with_config(TestRunConfig::new(LanguageVersion::default(), vec![]), path)
}

pub fn precompiled_v2_stdlib() -> &'static PrecompiledFilesModules {
    &PRECOMPILED_MOVE_STDLIB_V2
}

#[cfg(feature = "fuzzing")]
pub fn precompiled_v2_stdlib_fuzzer() -> &'static PrecompiledFilesModules {
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

/// Runs a test against an explicit [`BaselineTarget`], including its update
/// policy.
pub fn run_test_with_config_and_baseline(
    config: TestRunConfig,
    path: &Path,
    baseline: &BaselineTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    let v2_lib = precompiled_v2_stdlib();
    run_test_impl_with_baseline::<SimpleVMTestAdapter>(config, path, v2_lib, baseline)
}
