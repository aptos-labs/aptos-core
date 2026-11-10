// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs unit tests on the V1 VM (`move-vm-runtime`).

use crate::{
    backend::{
        BackendConfig, TestEffects, TestExecution, TestRun, UnitTestBackend, UnitTestBackendFactory,
    },
    extensions,
    test_reporter::{TestRunInfo, UnitTestFactory},
};
use anyhow::Result;
use legacy_move_compiler::unit_test::{NamedOrBytecodeModule, TestCase, TestPlan};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, StructFieldInformation},
};
use move_bytecode_utils::Modules;
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet, Op},
    identifier::IdentStr,
    language_storage::{ModuleId, OPTION_MODULE_ID, OPTION_STRUCT_NAME_STR},
    value::serialize_values,
};
use move_resource_viewer::MoveValueAnnotator;
use move_table_extension::TableChangeSet;
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_functions::NativeFunctionTable,
    AsFunctionValueExtension, AsUnsyncModuleStorage, InstantiatedFunctionLoader,
    LegacyLoaderConfig, RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use std::{sync::Mutex, time::Instant};

/// Builds [`V1Backend`]s. `F` supplies each test's gas meter.
pub struct V1BackendFactory<F> {
    /// `None` for the Move stdlib natives.
    natives: Option<NativeFunctionTable>,
    enable_enum_option: bool,
    gas_factory: Mutex<F>,
}

impl<F> V1BackendFactory<F> {
    pub fn new(
        natives: Option<NativeFunctionTable>,
        enable_enum_option: bool,
        gas_factory: F,
    ) -> Self {
        Self {
            natives,
            enable_enum_option,
            gas_factory: Mutex::new(gas_factory),
        }
    }
}

impl<F: UnitTestFactory + Send> UnitTestBackendFactory for V1BackendFactory<F> {
    fn build<'a>(
        &'a self,
        plan: &TestPlan,
        genesis: &ChangeSet,
        config: &BackendConfig,
    ) -> Result<Box<dyn UnitTestBackend + 'a>> {
        let natives = self.natives.clone().unwrap_or_else(|| {
            move_stdlib::natives::all_natives(
                AccountAddress::ONE,
                move_stdlib::natives::GasParameters::zeros(),
            )
        });
        let modules: Vec<&CompiledModule> = plan
            .module_info
            .values()
            .map(|info| match info {
                NamedOrBytecodeModule::Named(named_compiled_module) => {
                    &named_compiled_module.module
                },
                NamedOrBytecodeModule::Bytecode(compiled_module) => compiled_module,
            })
            .collect();
        let runtime_environment = RuntimeEnvironment::new_for_move_third_party_tests(
            natives,
            self.enable_enum_option,
            enable_framework_for_option(&modules),
        );
        let mut storage = setup_test_storage(modules.into_iter(), runtime_environment)?;
        storage.apply(genesis.clone())?;
        Ok(Box::new(V1Backend {
            storage,
            gas_factory: &self.gas_factory,
            config: config.clone(),
        }))
    }
}

/// Whether the `Option` in `modules` is the enum-based framework one. True if
/// `modules` defines no `Option`.
fn enable_framework_for_option(modules: &[&CompiledModule]) -> bool {
    for module in modules {
        if module.self_name() != OPTION_MODULE_ID.name()
            || module.self_addr() != OPTION_MODULE_ID.address()
        {
            continue;
        }
        for struct_def in &module.struct_defs {
            let handle = module.struct_handle_at(struct_def.struct_handle);
            if module.identifier_at(handle.name).as_str() == OPTION_STRUCT_NAME_STR {
                return matches!(
                    struct_def.field_information,
                    StructFieldInformation::DeclaredVariants(_)
                );
            }
        }
    }
    true
}

/// Populates test storage with modules in dependency order.
fn setup_test_storage<'a>(
    modules: impl Iterator<Item = &'a CompiledModule>,
    runtime_environment: RuntimeEnvironment,
) -> Result<InMemoryStorage> {
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);
    let modules = Modules::new(modules);
    for module in modules
        .compute_dependency_graph()
        .compute_topological_order()?
    {
        let mut module_bytes = Vec::new();
        module.serialize_for_version(Some(module.version), &mut module_bytes)?;
        storage.add_module_bytes(module.self_addr(), module.self_name(), module_bytes.into());
    }

    Ok(storage)
}

pub struct V1Backend<'a, F> {
    storage: InMemoryStorage,
    gas_factory: &'a Mutex<F>,
    config: BackendConfig,
}

impl<F: UnitTestFactory + Send> UnitTestBackend for V1Backend<'_, F> {
    fn name(&self) -> &str {
        "V1"
    }

    fn execute(&self, module_id: &ModuleId, test: &TestCase) -> TestExecution {
        let module_storage = self.storage.as_unsync_module_storage();

        let mut extensions = extensions::new_extensions();
        let mut gas_meter = self
            .gas_factory
            .lock()
            .expect("no thread panics while holding the gas factory")
            .new_gas_meter();
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);
        let mut data_cache = TransactionDataCache::empty();

        let now = Instant::now();
        let result = dispatch_loader!(&module_storage, loader, {
            loader
                .load_instantiated_function(
                    &LegacyLoaderConfig::unmetered(),
                    &mut gas_meter,
                    &mut traversal_context,
                    module_id,
                    IdentStr::new(&test.test_name).unwrap(),
                    // No type args for now.
                    &[],
                )
                .and_then(|function| {
                    let args = serialize_values(test.arguments.iter());
                    MoveVM::execute_loaded_function(
                        function,
                        args,
                        &mut MoveVmDataCacheAdapter::new(&mut data_cache, &self.storage, &loader),
                        &mut gas_meter,
                        &mut traversal_context,
                        &mut extensions,
                        &loader,
                    )
                })
        });
        let elapsed = now.elapsed();

        let mut result = result.map(|res| {
            res.return_values
                .into_iter()
                .map(|(bytes, _layout)| bytes)
                .collect()
        });
        if !self.config.report_stacktrace_on_abort {
            if let Err(err) = &mut result {
                err.remove_exec_state();
            }
        }

        // A resource serialization error leaves the execution result unchanged
        // and omits effects and the storage dump.
        let Ok(change_set) = data_cache.into_effects(&module_storage) else {
            return TestExecution::Ran(TestRun {
                result,
                v1_exact_error: true,
                effects: None,
                storage_dump: None,
                gas_used: 0,
                elapsed,
            });
        };
        let gas_used = self
            .gas_factory
            .lock()
            .expect("no thread panics while holding the gas factory")
            .finalize_test_run_info(
                &change_set,
                &mut extensions,
                gas_meter,
                TestRunInfo::new(test.test_name.clone(), elapsed),
            )
            .gas_used;

        // Prepare a dump for executions that may fail the test's expectation.
        let render_storage_dump =
            self.config.render_storage_dump && (result.is_err() || test.expected_failure.is_some());
        let function_value_extension = module_storage.as_function_value_extension();
        // A table serialization error omits tables from the dump and prevents
        // effect collection.
        let table_changes = if self.config.capture_effects || render_storage_dump {
            extensions::take_table_changes(&mut extensions, &function_value_extension)
        } else {
            Ok(None)
        };
        let storage_dump = render_storage_dump
            .then(|| {
                let tables = table_changes.as_ref().ok().and_then(Option::as_ref);
                print_resources_and_tables(&change_set, tables, &self.storage).ok()
            })
            .flatten();

        let effects = match table_changes {
            Ok(table_changes) if self.config.capture_effects => {
                let mut effects = TestEffects::new(change_set);
                for (handle, change) in table_changes
                    .into_iter()
                    .flat_map(|changes| changes.changes)
                {
                    effects.table_items.insert(handle.0, change.entries);
                }
                extensions::collect_hooked_effects(
                    &mut extensions,
                    &function_value_extension,
                    &mut effects,
                )
                .ok()
                .map(|()| effects)
            },
            Ok(_) | Err(_) => None,
        };

        TestExecution::Ran(TestRun {
            result,
            v1_exact_error: true,
            effects,
            storage_dump,
            gas_used,
            elapsed,
        })
    }
}

/// Prints the updates to storage represented by `cs` and `table_changes` in
/// the context of the starting storage state `storage`.
fn print_resources_and_tables(
    cs: &ChangeSet,
    table_changes: Option<&TableChangeSet>,
    storage: &InMemoryStorage,
) -> Result<String> {
    use std::fmt::Write;
    let mut buf = String::new();
    let annotator = MoveValueAnnotator::new(storage.clone());
    for (account_addr, account_state) in cs.accounts() {
        writeln!(&mut buf, "0x{}:", account_addr.short_str_lossless())?;

        for (tag, resource_op) in account_state.resources() {
            if let Op::New(resource) | Op::Modify(resource) = resource_op {
                writeln!(
                    &mut buf,
                    "\t{}",
                    format!("=> {}", annotator.view_resource(tag, resource)?).replace('\n', "\n\t")
                )?;
            }
        }
    }
    if let Some(table_changes) = table_changes {
        extensions::print_table_changes(&mut buf, table_changes)?;
    }
    Ok(buf)
}
