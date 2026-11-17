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
    errors::{PartialVMError, PartialVMResult, VMError, VMResult},
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
    effects::{ChangeSet, Op},
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{serialize_values, MoveFieldLayout, MoveStructLayout, MoveTypeLayout, MoveValue},
    vm_status::{StatusCode, StatusType},
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
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    resolver::ResourceResolver,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::{Reference, Struct, Value},
};
use once_cell::sync::Lazy;
use smallvec::smallvec;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Write,
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
    /// Sets MonoMove's heap capacity in bytes to test garbage collection under allocation
    /// pressure. V1 ignores this option.
    #[clap(long)]
    pub heap_size: Option<usize>,
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

        let signers = self.compiled_state().resolve_signers(signers);
        let script_bytes = match serialize_script(&self.storage, &script) {
            Ok(script_bytes) => script_bytes,
            Err(err) => return Some(format!("Error: {}", err)),
        };

        let args = serialize_values(&txn_args);
        // TODO rethink testing signer args
        let args = signers
            .iter()
            .map(|a| MoveValue::Signer(*a).simple_serialize().unwrap())
            .chain(args)
            .collect();
        let verbose = extra_args.verbose;

        let (result, trace, effects) = self.execute_entrypoint(
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
        let output = match result {
            Ok(_) => trace_str,
            Err(err) => {
                let err_str = Some(format!("Error: {}", script_execution_error(&err, verbose)));
                merge_output(trace_str, err_str)
            },
        };
        append_line(output, effects)
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

        let signers = self.compiled_state().resolve_signers(signers);

        let args = serialize_values(&txn_args);
        // TODO rethink testing signer args
        let args = signers
            .iter()
            .map(|a| MoveValue::Signer(*a).simple_serialize().unwrap())
            .chain(args)
            .collect();
        let verbose = extra_args.verbose;

        let (result, trace, effects) = self.execute_entrypoint(
            EntryPoint::Function { module, function },
            &type_args,
            args,
            gas_budget,
            &code_storage,
        );
        if let (true, Ok(values)) = (self.run_config.check_serde_round_trip, &result) {
            let function_extension = code_storage.as_function_value_extension();
            for (bytes, layout) in &values.return_values {
                check_serde_round_trip(&function_extension, bytes, layout, "a return value");
            }
            for (_, bytes, layout) in &values.mutable_reference_outputs {
                check_serde_round_trip(
                    &function_extension,
                    bytes,
                    layout,
                    "a mutable reference output",
                );
            }
        }

        let trace_str =
            trace.and_then(|t| extra_args.display_trace.then_some(t.to_string_for_tests()));
        let output = match result {
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
        };
        append_line(output, effects)
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

/// Formats a script execution failure for transactional test baselines.
pub fn script_execution_error(err: &VMError, verbose: bool) -> anyhow::Error {
    anyhow!(
        "Script execution failed with VMError: {}",
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

/// Serializes `script` at the storage's binary format version.
pub fn serialize_script(storage: &InMemoryStorage, script: &CompiledScript) -> Result<Vec<u8>> {
    let mut script_bytes = vec![];
    script.serialize_for_version(Some(storage.max_binary_format_version()), &mut script_bytes)?;
    Ok(script_bytes)
}

impl SimpleVMTestAdapter<'_> {
    /// Executes `entry_point` and commits its writes on success. Also returns the effects report
    /// when [`TestRunConfig::report_effects`] is set.
    fn execute_entrypoint(
        &mut self,
        entry_point: EntryPoint,
        ty_args: &[TypeTag],
        args: Vec<Vec<u8>>,
        gas_budget: Option<u64>,
        code_storage: &impl CodeStorage,
    ) -> (
        VMResult<SerializedReturnValues>,
        Option<Trace>,
        Option<String>,
    ) {
        let mut gas_meter = get_gas_status(
            &move_vm_test_utils::gas_schedule::INITIAL_COST_SCHEDULE,
            gas_budget,
        )
        .unwrap();
        let initial_balance = gas_meter.balance_internal();

        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);
        let mut extensions = NativeContextExtensions::default();
        let mut data_cache = TransactionDataCache::empty();

        let (result, trace) = dispatch_loader!(code_storage, loader, {
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
                Err(err) => return (Err(err), None, None),
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
                (replay_result.and(result), Some(trace))
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
                (result, None)
            }
        });

        let mut report = self.run_config.report_effects.then(|| {
            let gas_used = initial_balance
                .checked_sub(gas_meter.balance_internal())
                .expect("gas balance never grows");
            vec![format!("gas used: {}", u64::from(gas_used))]
        });
        if result.is_ok() {
            let change_set = if self.run_config.check_serde_round_trip {
                let function_extension = code_storage.as_function_value_extension();
                // Use the same serialization settings as `into_effects` before checking the bytes.
                data_cache.into_custom_effects(&|value, layout, _| {
                    let bytes = serde_context(&function_extension)
                        .serialize(&value, &layout)?
                        .ok_or_else(|| {
                            PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                                .with_message("Error when serializing resource.")
                        })?;
                    check_serde_round_trip(
                        &function_extension,
                        &bytes,
                        &layout,
                        "a resource write",
                    );
                    Ok(Bytes::from(bytes))
                })
            } else {
                data_cache.into_effects(code_storage)
            }
            .expect("Producing a change set always succeeds");
            if let Some(lines) = report.as_mut() {
                lines.extend(render_writes(&change_set));
            }
            self.storage.apply(change_set).unwrap();
        }
        (result, trace, report.map(|lines| lines.join("\n")))
    }
}

/// Checks that deserializing and reserializing `bytes` with `layout` preserves the bytes.
/// Checks both the VM and move-core-types, skipping the latter for layouts containing signers.
/// Panics on failure, using `what` to identify the value.
fn check_serde_round_trip(
    function_extension: &dyn FunctionValueExtension,
    bytes: &[u8],
    layout: &MoveTypeLayout,
    what: &str,
) {
    let Some(value) = serde_context(function_extension).deserialize(bytes, layout) else {
        serde_round_trip_failure(what, "the VM cannot deserialize them", layout, bytes);
    };
    if !matches!(serde_context(function_extension).serialize(&value, layout), Ok(Some(reserialized)) if reserialized == bytes)
    {
        serde_round_trip_failure(what, "the VM reserializes them differently", layout, bytes);
    }
    if mentions_signer(layout) {
        return;
    }
    let move_value = MoveValue::simple_deserialize(bytes, layout).unwrap_or_else(|err| {
        serde_round_trip_failure(
            what,
            &format!("move-core-types cannot deserialize them: {err}"),
            layout,
            bytes,
        )
    });
    if move_value.simple_serialize().as_deref() != Some(bytes) {
        serde_round_trip_failure(
            what,
            "move-core-types reserializes them differently",
            layout,
            bytes,
        );
    }
}

/// Whether a value of `layout` can hold a signer.
fn mentions_signer(layout: &MoveTypeLayout) -> bool {
    let any_field =
        |fields: &[MoveFieldLayout]| fields.iter().any(|field| mentions_signer(&field.layout));
    match layout {
        MoveTypeLayout::Signer => true,
        MoveTypeLayout::Vector(inner) | MoveTypeLayout::Native(_, inner) => mentions_signer(inner),
        MoveTypeLayout::Struct(struct_layout) => match struct_layout.as_ref() {
            MoveStructLayout::Runtime(fields) => fields.iter().any(mentions_signer),
            MoveStructLayout::RuntimeVariants(variants) => {
                variants.iter().flatten().any(mentions_signer)
            },
            MoveStructLayout::WithFields(fields) | MoveStructLayout::WithTypes { fields, .. } => {
                any_field(fields)
            },
            MoveStructLayout::WithVariants { variants, .. } => {
                variants.iter().any(|variant| any_field(&variant.fields))
            },
        },
        MoveTypeLayout::Bool
        | MoveTypeLayout::U8
        | MoveTypeLayout::U16
        | MoveTypeLayout::U32
        | MoveTypeLayout::U64
        | MoveTypeLayout::U128
        | MoveTypeLayout::U256
        | MoveTypeLayout::I8
        | MoveTypeLayout::I16
        | MoveTypeLayout::I32
        | MoveTypeLayout::I64
        | MoveTypeLayout::I128
        | MoveTypeLayout::I256
        | MoveTypeLayout::Address
        | MoveTypeLayout::Function => false,
    }
}

/// The VM's value serializer and deserializer, configured as `into_effects` configures it.
fn serde_context(function_extension: &dyn FunctionValueExtension) -> ValueSerDeContext<'_> {
    ValueSerDeContext::new(function_extension.max_value_nest_depth())
        .with_func_args_deserialization(function_extension)
}

fn serde_round_trip_failure(what: &str, problem: &str, layout: &MoveTypeLayout, bytes: &[u8]) -> ! {
    let hex = hex(bytes);
    panic!("serde round trip of {what} failed: {problem}\nlayout: {layout:?}\nbytes: {hex}");
}

fn hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(hex, "{byte:02x}").expect("writing to a String succeeds");
    }
    hex
}

/// Renders each resource write in `change_set` as a line of the effects report.
pub fn render_writes(change_set: &ChangeSet) -> impl Iterator<Item = String> + '_ {
    change_set.resources().map(|(addr, tag, op)| {
        let (kind, value) = match op {
            Op::New(bytes) => ("new", hex(bytes)),
            Op::Modify(bytes) => ("modify", hex(bytes)),
            Op::Delete => ("delete", String::new()),
        };
        format!(
            "write {kind} {} {} {value}",
            addr.to_hex_literal(),
            tag.to_canonical_string(),
        )
    })
}

/// Appends `line` to `output` on a new line.
pub fn append_line(output: Option<String>, line: Option<String>) -> Option<String> {
    match (output, line) {
        (Some(output), Some(line)) => Some(format!("{output}\n{line}")),
        (output, line) => output.or(line),
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

fn native_swap(
    _context: &mut NativeContext,
    ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(arguments.len(), 2);

    let right = pop_arg!(arguments, Reference);
    let left = pop_arg!(arguments, Reference);
    left.swap_values(right)?;
    Ok(NativeResult::ok(0.into(), smallvec![]))
}

/// Creates the test runtime environment with stdlib natives, `cmp::compare` and `mem::swap`.
pub fn create_runtime_environment(vm_config: VMConfig) -> RuntimeEnvironment {
    let mut natives = move_stdlib::natives::all_natives(
        STD_ADDR,
        // TODO: come up with a suitable gas schedule
        move_stdlib::natives::GasParameters::zeros(),
    );
    natives.extend(make_table_from_iter(STD_ADDR, [
        ("cmp", "compare", Arc::new(native_compare) as NativeFunction),
        ("mem", "swap", Arc::new(native_swap) as NativeFunction),
    ]));
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
    sources.extend(["src/cmp.move", "src/mem.move"].map(|file| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(file)
            .to_string_lossy()
            .into_owned()
    }));
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
    /// Reports committed resource writes with gas used for V1 execution or GC count for MonoMove.
    pub report_effects: bool,
    /// Checks V1 resource writes, return values, and mutable reference outputs for byte-preserving
    /// serialization round trips. Checks both the VM and move-core-types, skipping move-core-types
    /// for layouts containing signers. Panics on failure; MonoMove ignores this option.
    pub check_serde_round_trip: bool,
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
            report_effects: false,
            check_serde_round_trip: false,
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

    pub fn with_effects_report(self) -> Self {
        Self {
            report_effects: true,
            ..self
        }
    }

    pub fn with_serde_round_trip(self) -> Self {
        Self {
            check_serde_round_trip: true,
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
