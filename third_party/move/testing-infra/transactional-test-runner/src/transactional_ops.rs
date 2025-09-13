// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Transactional operations for building and executing transactional tests.
//! This module provides helper functions and data types for converting
//! transactional test files into executable operations.
use crate::{
    framework::{CompiledState, MoveTestAdapter},
    tasks::{
        EmptyCommand, InitCommand, PublishCommand, RunCommand, SyntaxChoice, TaskCommand, TaskInput,
    },
    vm_test_harness::{
        AdapterExecuteArgs, AdapterPublishArgs, PrecompiledFilesModules, TestRunConfig,
    },
};
use anyhow::Result;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        CompiledModule, CompiledScript, FunctionDefinition, FunctionDefinitionIndex,
        FunctionHandle, SignatureToken, Visibility,
    },
};
use move_command_line_common::{
    files::verify_and_create_named_address_mapping, values::ParsableValue,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    value::{MoveTypeLayout, MoveValue},
};
use move_vm_runtime::move_vm::SerializedReturnValues;
use move_vm_types::values::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Write,
};

/// Minimal adapter implementation for transactional test processing
/// This adapter provides the minimum functionality needed to compile and process
/// transactional test files without executing them.
pub struct MinimalAdapter<'a> {
    compiled_state: CompiledState<'a>,
    default_syntax: SyntaxChoice,
    run_config: TestRunConfig,
}

impl<'a> MoveTestAdapter<'a> for MinimalAdapter<'a> {
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
        static EMPTY_SET: std::sync::LazyLock<BTreeSet<String>> =
            std::sync::LazyLock::new(BTreeSet::new);
        &EMPTY_SET
    }

    fn init(
        default_syntax: SyntaxChoice,
        run_config: TestRunConfig,
        pre_compiled_deps_v2: &'a PrecompiledFilesModules,
        init_data: Option<TaskInput<(InitCommand, Self::ExtraInitArgs)>>,
    ) -> (Self, Option<String>) {
        // Named address mapping
        let additional_named_address_mapping = match init_data.as_ref().map(|t| &t.command) {
            Some((InitCommand { named_addresses }, _)) => {
                verify_and_create_named_address_mapping(named_addresses.clone()).unwrap()
            },
            None => BTreeMap::new(),
        };

        let mut named_address_mapping = aptos_framework::named_addresses().clone();

        for (name, addr) in additional_named_address_mapping.clone() {
            if named_address_mapping.contains_key(&name) {
                panic!("Invalid init. The named address '{}' already exists.", name)
            }
            named_address_mapping.insert(name, addr);
        }

        let compiled_state = CompiledState::new(named_address_mapping, pre_compiled_deps_v2, None);
        (
            MinimalAdapter {
                compiled_state,
                default_syntax,
                run_config,
            },
            None,
        )
    }

    // All of these are required minimal functions to be implemented
    fn publish_module(
        &mut self,
        _module: CompiledModule,
        _named_addr_opt: Option<Identifier>,
        _gas_budget: Option<u64>,
        _extra: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    fn execute_script(
        &mut self,
        _script: CompiledScript,
        _type_args: Vec<TypeTag>,
        _signers: Vec<move_command_line_common::address::ParsedAddress>,
        _args: Vec<MoveValue>,
        _gas_budget: Option<u64>,
        _extra: Self::ExtraRunArgs,
    ) -> Result<Option<String>> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    fn call_function(
        &mut self,
        _module: &ModuleId,
        _function: &IdentStr,
        _type_args: Vec<TypeTag>,
        _signers: Vec<move_command_line_common::address::ParsedAddress>,
        _args: Vec<MoveValue>,
        _gas_budget: Option<u64>,
        _extra: Self::ExtraRunArgs,
    ) -> Result<(Option<String>, SerializedReturnValues)> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    fn view_data(
        &mut self,
        _address: AccountAddress,
        _module: &ModuleId,
        _resource: &IdentStr,
        _type_args: Vec<TypeTag>,
    ) -> Result<String> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    fn handle_subcommand(
        &mut self,
        _subcommand: TaskInput<Self::Subcommand>,
    ) -> Result<Option<String>> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    fn deserialize(&self, _bytes: &[u8], _layout: &MoveTypeLayout) -> Option<Value> {
        unimplemented!("MinimalAdapter is only used for compilation, not execution")
    }

    // We need this to set the compiler version to the latest version
    fn run_config(&self) -> TestRunConfig {
        self.run_config.clone()
    }
}

/// Represents a single operation in a transactional test sequence
#[derive(Debug, Eq, PartialEq, Clone, arbitrary::Arbitrary, dearbitrary::Dearbitrary)]
pub enum TransactionalOperation {
    /// Publish one or more modules
    PublishModule { _module: CompiledModule },
    /// Run a script
    RunScript {
        _script: CompiledScript,
        _type_args: Vec<TypeTag>,
        _args: Vec<MoveValue>,
    },
    /// Call a function in a published module
    CallFunction {
        _module: ModuleId,
        _function: FunctionDefinitionIndex,
        _type_args: Vec<TypeTag>,
        _args: Vec<Vec<u8>>,
    },
}

/// Helper function to convert a sequence of TaskInputs into TransactionalOperations
/// using the framework's internal compilation functions
pub fn tasks_to_transactional_operations<'a, Adapter>(
    adapter: &mut Adapter,
    tasks: VecDeque<
        TaskInput<
            TaskCommand<
                Adapter::ExtraInitArgs,
                Adapter::ExtraPublishArgs,
                Adapter::ExtraValueArgs,
                Adapter::ExtraRunArgs,
                Adapter::Subcommand,
            >,
        >,
    >,
) -> Result<Vec<TransactionalOperation>>
where
    Adapter: MoveTestAdapter<'a>,
    Adapter::ExtraValueArgs: ParsableValue,
    <Adapter::ExtraValueArgs as ParsableValue>::ConcreteValue: Into<MoveValue> + Clone,
{
    let mut operations = Vec::new();
    let mut published_modules = Vec::new();

    for task in tasks {
        match task.command {
            TaskCommand::Publish(
                PublishCommand {
                    gas_budget: _,
                    syntax,
                    print_bytecode: _,
                },
                _extra_args,
            ) => {
                let syntax = syntax.unwrap_or_else(|| adapter.default_syntax());
                let (data, named_addr_opt, module, _warnings_opt) = adapter.compile_module(
                    syntax,
                    task.data,
                    task.start_line,
                    task.command_lines_stop,
                )?;

                // Track the module for function resolution
                published_modules.push(module.clone());
                let data_path = data.path().to_str().unwrap();
                adapter.compiled_state().add_with_source_file(
                    named_addr_opt,
                    module.clone(),
                    (data_path.to_owned(), data),
                );

                operations.push(TransactionalOperation::PublishModule { _module: module });
            },
            TaskCommand::Run(
                RunCommand {
                    signers: _,
                    args,
                    type_args,
                    gas_budget: _,
                    syntax,
                    name: None, // Because script call main function
                    print_bytecode: _,
                },
                _extra_args,
            ) => {
                // Script execution
                let syntax = syntax.unwrap_or_else(|| adapter.default_syntax());
                let (script, _warning_opt) = adapter.compile_script(
                    syntax,
                    task.data,
                    task.start_line,
                    task.command_lines_stop,
                )?;

                let resolved_args = adapter.compiled_state().resolve_args(args)?;
                let resolved_type_args = adapter.compiled_state().resolve_type_args(type_args)?;

                // Convert resolved args to MoveValue - they are already ConcreteValue
                let move_args: Vec<MoveValue> =
                    resolved_args.into_iter().map(|arg| arg.into()).collect();

                operations.push(TransactionalOperation::RunScript {
                    _script: script,
                    _type_args: resolved_type_args,
                    _args: move_args,
                });
            },
            TaskCommand::Run(
                RunCommand {
                    signers: _,
                    args,
                    type_args,
                    gas_budget: _,
                    syntax: _,
                    name: Some((raw_addr, module_name, function_name)),
                    print_bytecode: _,
                },
                _extra_args,
            ) => {
                // Function call
                let addr = adapter.compiled_state().resolve_address(&raw_addr);
                let module_id = ModuleId::new(addr, module_name.clone());
                let resolved_type_args = adapter.compiled_state().resolve_type_args(type_args)?;
                let resolved_args_concrete_values = adapter.compiled_state().resolve_args(args)?;

                let target_compiled_module = published_modules
                    .iter()
                    .find(|m| m.self_id() == module_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Module {} not found for function call", module_id)
                    })?;

                let (target_func_def, target_func_handle, func_idx) = target_compiled_module
                    .function_defs
                    .iter()
                    .enumerate()
                    .find_map(|(idx, f_def)| {
                        let f_handle =
                            &target_compiled_module.function_handles[f_def.function.0 as usize];
                        if target_compiled_module.identifier_at(f_handle.name).as_str()
                            == function_name.as_str()
                        {
                            Some((f_def, f_handle, FunctionDefinitionIndex::new(idx as u16)))
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Function {} not found in module {}",
                            function_name,
                            module_id
                        )
                    })?;

                let return_sig = target_compiled_module.signature_at(target_func_handle.return_);
                // Transactional test runner does works mainly with public functions, so we need to wrap them in a script
                // to make it runnable in the VM flow used by the fuzzer.
                let needs_wrapper = !return_sig.0.is_empty() || !target_func_def.is_entry;

                if needs_wrapper {
                    if std::env::var("DEBUG").is_ok() {
                        println!(
                            "[transactional_ops] Wrapper needed for {}::{}",
                            module_id, function_name
                        );
                    }
                    let wrapper_script_source = generate_script_wrapper_for_non_entry_function(
                        &module_id,
                        target_compiled_module,
                        target_func_def,
                        target_func_handle,
                        &resolved_type_args,
                    )?;
                    // store the wrapper script source in a temp file
                    let mut temp_file = tempfile::NamedTempFile::new()?;
                    temp_file.write_all(wrapper_script_source.as_bytes())?;

                    let (compiled_wrapper_script, _warnings_opt) =
                        adapter.compile_script(SyntaxChoice::Source, Some(temp_file), 0, 0)?;

                    let move_args: Vec<MoveValue> = resolved_args_concrete_values
                        .into_iter()
                        .map(|arg| arg.into())
                        .collect();

                    operations.push(TransactionalOperation::RunScript {
                        _script: compiled_wrapper_script,
                        _type_args: resolved_type_args,
                        _args: move_args,
                    });
                } else {
                    let serialized_args: Vec<Vec<u8>> = resolved_args_concrete_values
                        .into_iter()
                        .map(|arg| {
                            let move_value: MoveValue = arg.into();
                            bcs::to_bytes(&move_value)
                                .map_err(|e| anyhow::anyhow!("Failed to serialize argument: {}", e))
                        })
                        .collect::<Result<Vec<_>>>()?;

                    operations.push(TransactionalOperation::CallFunction {
                        _module: module_id,
                        _function: func_idx,
                        _type_args: resolved_type_args,
                        _args: serialized_args,
                    });
                }
            },
            _ => {
                // Skip other task types (Init, PrintBytecode, View, Subcommand)
                continue;
            },
        }
    }

    Ok(operations)
}

// Helper to convert SignatureToken to a Move type string for the wrapper script
fn signature_token_to_move_type_string_for_wrapper(
    token: &SignatureToken,
    module: &CompiledModule,
) -> anyhow::Result<String> {
    match token {
        SignatureToken::Bool => Ok("bool".to_string()),
        SignatureToken::U8 => Ok("u8".to_string()),
        SignatureToken::U16 => Ok("u16".to_string()),
        SignatureToken::U32 => Ok("u32".to_string()),
        SignatureToken::U64 => Ok("u64".to_string()),
        SignatureToken::U128 => Ok("u128".to_string()),
        SignatureToken::U256 => Ok("u256".to_string()),
        SignatureToken::Address => Ok("address".to_string()),
        SignatureToken::Signer => Ok("signer".to_string()),
        SignatureToken::Vector(inner_token) => Ok(format!(
            "vector<{}>",
            signature_token_to_move_type_string_for_wrapper(inner_token, module)?
        )),
        SignatureToken::Function(args, returns, abilities) => {
            let args_str = args
                .iter()
                .map(|t| signature_token_to_move_type_string_for_wrapper(t, module))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let returns_str = returns
                .iter()
                .map(|t| signature_token_to_move_type_string_for_wrapper(t, module))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(format!(
                "|{}|{}{}",
                args_str.join(", "),
                returns_str.join(", "),
                abilities.display_postfix()
            ))
        },
        SignatureToken::Struct(sh_idx) => {
            let struct_handle = module.struct_handle_at(*sh_idx);
            let mh = module.module_handle_at(struct_handle.module);
            let struct_name = module.identifier_at(struct_handle.name);
            let module_name = module.identifier_at(mh.name);
            let module_addr = module.address_identifier_at(mh.address);
            Ok(format!(
                "{}::{}::{}",
                module_addr.to_hex_literal(),
                module_name,
                struct_name
            ))
        },
        SignatureToken::StructInstantiation(sh_idx, type_args) => {
            let struct_handle = module.struct_handle_at(*sh_idx);
            let mh = module.module_handle_at(struct_handle.module);
            let struct_name = module.identifier_at(struct_handle.name);
            let module_name = module.identifier_at(mh.name);
            let module_addr = module.address_identifier_at(mh.address);
            let type_arg_strings: Vec<String> = type_args
                .iter()
                .map(|t| signature_token_to_move_type_string_for_wrapper(t, module))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(format!(
                "{}::{}::{}<{}>",
                module_addr.to_hex_literal(),
                module_name,
                struct_name,
                type_arg_strings.join(", ")
            ))
        },
        SignatureToken::TypeParameter(idx) => Ok(format!("T{}", idx)),
        SignatureToken::Reference(inner_token) => Ok(format!(
            "&{}",
            signature_token_to_move_type_string_for_wrapper(inner_token, module)?
        )),
        SignatureToken::MutableReference(inner_token) => Ok(format!(
            "&mut {}",
            signature_token_to_move_type_string_for_wrapper(inner_token, module)?
        )),
    }
}

// Generates the Move source code for a wrapper script.
fn generate_script_wrapper_for_non_entry_function(
    _target_module_id: &ModuleId,
    target_module: &CompiledModule,
    _target_func_def: &FunctionDefinition,
    target_func_handle: &FunctionHandle,
    _target_type_args: &[TypeTag],
) -> anyhow::Result<String> {
    let func_name_ident = target_module.identifier_at(target_func_handle.name);

    // skip if the function is not public
    if _target_func_def.visibility != Visibility::Public {
        return Err(anyhow::anyhow!(
            "Function {} is not public",
            func_name_ident.to_string()
        ));
    }

    let module_handle = target_module.module_handle_at(target_func_handle.module);
    let module_name_ident = target_module.identifier_at(module_handle.name);
    let module_addr_literal = target_module
        .address_identifier_at(module_handle.address)
        .to_hex_literal();

    let parameters_sig = target_module.signature_at(target_func_handle.parameters);
    let return_sig = target_module.signature_at(target_func_handle.return_);

    let has_signer_by_value = parameters_sig.0.contains(&SignatureToken::Signer);
    let script_signer_param = if has_signer_by_value {
        "s: signer"
    } else {
        "s: &signer"
    };
    let mut script_params_str_parts = vec![script_signer_param.to_string()];
    let mut call_args_str_parts = vec![];

    for (i, param_token) in parameters_sig.0.iter().enumerate() {
        match param_token {
            SignatureToken::Signer => {
                call_args_str_parts.push("s".to_string());
            },
            SignatureToken::Reference(inner_token)
                if matches!(**inner_token, SignatureToken::Signer) =>
            {
                if has_signer_by_value {
                    call_args_str_parts.push("&s".to_string());
                } else {
                    call_args_str_parts.push("s".to_string());
                }
            },
            _ => {
                let type_str =
                    signature_token_to_move_type_string_for_wrapper(param_token, target_module)?;
                script_params_str_parts.push(format!("arg{}: {}", i, type_str));
                call_args_str_parts.push(format!("arg{}", i));
            },
        }
    }

    let type_parameters_decl_str = if target_func_handle.type_parameters.is_empty() {
        String::new()
    } else {
        let params: Vec<String> = (0..target_func_handle.type_parameters.len())
            .map(|i| format!("T{}", i))
            .collect();
        format!("<{}>", params.join(", "))
    };

    // Check if the function returns a unit type (empty tuple)
    let is_unit_return = return_sig.0.is_empty();

    // Generate the function call line based on return type
    let function_call_line = if is_unit_return {
        // For unit return types, don't assign to a variable
        format!(
            "{}::{}{}({});",
            module_name_ident,
            func_name_ident,
            type_parameters_decl_str,
            call_args_str_parts.join(", ")
        )
    } else {
        // For non-unit return types, assign to a variable.
        // If multiple values are returned, destructure the tuple.
        let num_return_values = return_sig.0.len();
        let bindings = if num_return_values > 1 {
            format!("({})", vec!["_"; num_return_values].join(", "))
        } else {
            "_".to_string()
        };
        format!(
            "let {} = {}::{}{}({});",
            bindings,
            module_name_ident,
            func_name_ident,
            type_parameters_decl_str,
            call_args_str_parts.join(", ")
        )
    };

    let script_source = format!(
        r#"
script {{
    use {}::{};

    fun main{}({}) {{
        {}
    }}
}}
        "#,
        module_addr_literal,
        module_name_ident,
        type_parameters_decl_str,
        script_params_str_parts.join(", "),
        function_call_line
    );

    if std::env::var("DEBUG").is_ok() {
        println!(
            "[transactional_ops] Generated wrapper script source:\n{}",
            script_source
        );
    }
    Ok(script_source)
}
