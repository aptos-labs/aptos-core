// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    tasks::{
        taskify, InitCommand, PrintBytecodeCommand, PrintBytecodeInputChoice, PublishCommand,
        RunCommand, SyntaxChoice, TaskCommand, TaskInput, ViewCommand,
    },
    vm_test_harness::{PrecompiledFilesModules, TestRunConfig},
};
use anyhow::Result;
use clap::Parser;
use legacy_move_compiler::{compiled_unit::AnnotatedCompiledUnit, shared::NumericalAddress};
use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{CompiledModule, CompiledScript},
};
use move_bytecode_source_map::mapping::SourceMapping;
use move_command_line_common::{
    address::ParsedAddress,
    files::{MOVE_EXTENSION, MOVE_IR_EXTENSION},
    testing::{add_update_baseline_fix, format_diff, read_env_update_baseline, EXP_EXT},
    types::ParsedType,
    values::{ParsableValue, ParsedValue},
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::MoveTypeLayout,
};
use move_disassembler::disassembler::{Disassembler, DisassemblerOptions};
use move_ir_types::location::Spanned;
use move_model::{metadata::LanguageVersion, model::GlobalEnv};
use move_symbol_pool::Symbol;
use move_vm_runtime::move_vm::SerializedReturnValues;
use move_vm_types::values::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::{Debug, Write as FmtWrite},
    io::Write,
    path::Path,
};
use tempfile::NamedTempFile;

pub struct ProcessedModule {
    module: CompiledModule,
    source_file: Option<(String, NamedTempFile)>,
}

pub struct CompiledState<'a> {
    pre_compiled_deps_v2: &'a PrecompiledFilesModules,
    pre_compiled_ids: BTreeSet<(AccountAddress, String)>,
    compiled_module_named_address_mapping: BTreeMap<ModuleId, Symbol>,
    pub named_address_mapping: BTreeMap<String, NumericalAddress>,
    default_named_address_mapping: Option<NumericalAddress>,
    modules: BTreeMap<ModuleId, ProcessedModule>,
    temp_file_mapping: BTreeMap<String, String>,
}

impl CompiledState<'_> {
    pub fn resolve_named_address(&self, s: &str) -> AccountAddress {
        if let Some(addr) = self
            .named_address_mapping
            .get(s)
            .or(self.default_named_address_mapping.as_ref())
        {
            return AccountAddress::new(addr.into_bytes());
        }
        panic!("Failed to resolve named address '{}'", s)
    }

    pub fn resolve_address(&self, addr: &ParsedAddress) -> AccountAddress {
        match addr {
            ParsedAddress::Named(named_addr) => self.resolve_named_address(named_addr.as_str()),
            ParsedAddress::Numerical(addr) => addr.into_inner(),
        }
    }

    pub fn resolve_args<Extra: ParsableValue>(
        &self,
        args: Vec<ParsedValue<Extra>>,
    ) -> Result<Vec<Extra::ConcreteValue>> {
        args.into_iter()
            .map(|arg| arg.into_concrete_value(&|s| Some(self.resolve_named_address(s))))
            .collect()
    }

    pub fn resolve_type_args(&self, type_args: Vec<ParsedType>) -> Result<Vec<TypeTag>> {
        type_args
            .into_iter()
            .map(|arg| arg.into_type_tag(&|s| Some(self.resolve_named_address(s))))
            .collect()
    }
}

fn merge_output(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (None, right) => right,
        (left, None) => left,
        (Some(mut left), Some(right)) => {
            left.push_str(&right);
            Some(left)
        },
    }
}

pub trait MoveTestAdapter<'a>: Sized {
    type ExtraPublishArgs: Parser;
    type ExtraValueArgs: ParsableValue;
    type ExtraRunArgs: Parser;
    type Subcommand: Parser;
    type ExtraInitArgs: Parser;

    fn deserialize(&self, bytes: &[u8], layout: &MoveTypeLayout) -> Option<Value>;
    fn compiled_state(&mut self) -> &mut CompiledState<'a>;
    fn default_syntax(&self) -> SyntaxChoice;
    fn known_attributes(&self) -> &BTreeSet<String>;
    fn run_config(&self) -> TestRunConfig {
        TestRunConfig::compiler_v2(LanguageVersion::default(), vec![])
    }
    fn init(
        default_syntax: SyntaxChoice,
        run_config: TestRunConfig,
        pre_compiled_deps_v2: &'a PrecompiledFilesModules,
        init_data: Option<TaskInput<(InitCommand, Self::ExtraInitArgs)>>,
    ) -> (Self, Option<String>);
    fn publish_module(
        &mut self,
        module: CompiledModule,
        named_addr_opt: Option<Identifier>,
        gas_budget: Option<u64>,
        extra: Self::ExtraPublishArgs,
    ) -> Result<(Option<String>, CompiledModule)>;
    fn execute_script(
        &mut self,
        script: CompiledScript,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        args: Vec<<<Self as MoveTestAdapter<'a>>::ExtraValueArgs as ParsableValue>::ConcreteValue>,
        gas_budget: Option<u64>,
        extra: Self::ExtraRunArgs,
    ) -> Result<Option<String>>;
    fn call_function(
        &mut self,
        module: &ModuleId,
        function: &IdentStr,
        type_args: Vec<TypeTag>,
        signers: Vec<ParsedAddress>,
        args: Vec<<<Self as MoveTestAdapter<'a>>::ExtraValueArgs as ParsableValue>::ConcreteValue>,
        gas_budget: Option<u64>,
        extra: Self::ExtraRunArgs,
    ) -> Result<(Option<String>, SerializedReturnValues)>;
    fn view_data(
        &mut self,
        address: AccountAddress,
        module: &ModuleId,
        resource: &IdentStr,
        type_args: Vec<TypeTag>,
    ) -> Result<String>;

    fn handle_subcommand(
        &mut self,
        subcommand: TaskInput<Self::Subcommand>,
    ) -> Result<Option<String>>;

    fn compile_module(
        &mut self,
        syntax: SyntaxChoice,
        data: Option<NamedTempFile>,
        start_line: usize,
        command_lines_stop: usize,
    ) -> Result<(
        NamedTempFile,
        Option<Symbol>,
        CompiledModule,
        Option<String>,
    )> {
        let (data, named_addr_opt, module, _opt_model, warnings_opt) =
            self.compile_module_default(syntax, data, start_line, command_lines_stop)?;
        Ok((data, named_addr_opt, module, warnings_opt))
    }

    fn compile_module_default(
        &mut self,
        syntax: SyntaxChoice,
        data: Option<NamedTempFile>,
        start_line: usize,
        command_lines_stop: usize,
    ) -> Result<(
        NamedTempFile,
        Option<Symbol>,
        CompiledModule,
        Option<GlobalEnv>,
        Option<String>,
    )> {
        let data = match data {
            Some(f) => f,
            None => panic!(
                "Expected a module text block following 'publish' starting on lines {}-{}",
                start_line, command_lines_stop
            ),
        };
        let data_path = data.path().to_str().unwrap();
        let run_config = self.run_config();
        let state = self.compiled_state();
        let (named_addr_opt, module, opt_model, warnings_opt) = match syntax {
            SyntaxChoice::Source => {
                let (unit, opt_model, warnings_opt) = match run_config {
                    // Run the V2 compiler if requested
                    TestRunConfig::CompilerV2 {
                        language_version,
                        experiments,
                        vm_config: _,
                    } => compile_source_unit_v2(
                        state.pre_compiled_deps_v2,
                        state.named_address_mapping.clone(),
                        &state.source_files().cloned().collect::<Vec<_>>(),
                        data_path.to_owned(),
                        self.known_attributes(),
                        language_version,
                        experiments,
                    )?,
                };
                let (named_addr_opt, module) = match unit {
                    AnnotatedCompiledUnit::Module(annot_module) => {
                        let (named_addr_opt, _id) = annot_module.module_id();
                        (
                            named_addr_opt.map(|n| n.value),
                            annot_module.named_module.module,
                        )
                    },
                    AnnotatedCompiledUnit::Script(_) => panic!(
                        "Expected a module text block, not a script, following 'publish' \
                         starting on lines {}-{}",
                        start_line, command_lines_stop
                    ),
                };
                (named_addr_opt, module, opt_model, warnings_opt)
            },
            SyntaxChoice::IR => {
                let module = compile_ir_module(state.dep_modules(), data_path)?;
                (None, module, None, None)
            },
        };
        self.register_temp_filename(&data);
        Ok((data, named_addr_opt, module, opt_model, warnings_opt))
    }

    fn compile_script(
        &mut self,
        syntax: SyntaxChoice,
        data: Option<NamedTempFile>,
        start_line: usize,
        command_lines_stop: usize,
    ) -> Result<(CompiledScript, Option<String>)> {
        let (compiled_script, _opt_model, warnings_opt) =
            self.compile_script_default(syntax, data, start_line, command_lines_stop)?;
        Ok((compiled_script, warnings_opt))
    }

    fn compile_script_default(
        &mut self,
        syntax: SyntaxChoice,
        data: Option<NamedTempFile>,
        start_line: usize,
        command_lines_stop: usize,
    ) -> Result<(CompiledScript, Option<GlobalEnv>, Option<String>)> {
        let data = match data {
            Some(f) => f,
            None => panic!(
                "Expected a script text block following 'run' starting on lines {}-{}",
                start_line, command_lines_stop
            ),
        };
        let data_path = data.path().to_str().unwrap();
        let run_config = self.run_config();
        let state = self.compiled_state();
        let (script, opt_model, warning_opt) = match syntax {
            SyntaxChoice::Source => {
                let (unit, opt_model, warning_opt) = match run_config {
                    // Run the V2 compiler.
                    TestRunConfig::CompilerV2 {
                        language_version,
                        experiments: v2_experiments,
                        vm_config: _,
                    } => compile_source_unit_v2(
                        state.pre_compiled_deps_v2,
                        state.named_address_mapping.clone(),
                        &state.source_files().cloned().collect::<Vec<_>>(),
                        data_path.to_owned(),
                        self.known_attributes(),
                        language_version,
                        v2_experiments,
                    )?,
                };
                match unit {
                    AnnotatedCompiledUnit::Script(annot_script) => (annot_script.named_script.script, opt_model, warning_opt),
                    AnnotatedCompiledUnit::Module(_) => panic!(
                        "Expected a script text block, not a module, following 'run' starting on lines {}-{}",
                        start_line, command_lines_stop
                    ),
                }
            },
            SyntaxChoice::IR => {
                let script = compile_ir_script(state.dep_modules(), data_path)?;
                (script, None, None)
            },
        };
        Ok((script, opt_model, warning_opt))
    }

    fn handle_command(
        &mut self,
        task: TaskInput<
            TaskCommand<
                Self::ExtraInitArgs,
                Self::ExtraPublishArgs,
                Self::ExtraValueArgs,
                Self::ExtraRunArgs,
                Self::Subcommand,
            >,
        >,
    ) -> Result<Option<String>> {
        let TaskInput {
            command,
            name,
            number,
            start_line,
            command_lines_stop,
            stop_line,
            data,
        } = task;
        if let Some(data) = &data {
            self.register_temp_filename(data);
        }
        match command {
            TaskCommand::Init { .. } => {
                panic!("The 'init' command is optional. But if used, it must be the first command")
            },
            TaskCommand::PrintBytecode(PrintBytecodeCommand { input, syntax }) => {
                let syntax = syntax.unwrap_or_else(|| self.default_syntax());
                let result = match input {
                    PrintBytecodeInputChoice::Script => {
                        let (script, _warning_opt) =
                            self.compile_script(syntax, data, start_line, command_lines_stop)?;
                        disassembler_for_view(BinaryIndexedView::Script(&script)).disassemble()?
                    },
                    PrintBytecodeInputChoice::Module => {
                        let (_data, _named_addr_opt, module, _warnings_opt) =
                            self.compile_module(syntax, data, start_line, command_lines_stop)?;
                        disassembler_for_view(BinaryIndexedView::Module(&module)).disassemble()?
                    },
                };
                Ok(Some(result))
            },
            TaskCommand::Publish(
                PublishCommand {
                    gas_budget,
                    syntax,
                    print_bytecode,
                },
                extra_args,
            ) => {
                let syntax = syntax.unwrap_or_else(|| self.default_syntax());
                let (data, named_addr_opt, module, warnings_opt) =
                    self.compile_module(syntax, data, start_line, command_lines_stop)?;
                self.register_temp_filename(&data);
                let printed = if print_bytecode {
                    let disassembler = disassembler_for_view(BinaryIndexedView::Module(&module));
                    Some(format!(
                        "\n== BEGIN Bytecode ==\n{}\n== END Bytecode ==",
                        disassembler.disassemble()?
                    ))
                } else {
                    None
                };
                let (mut output, module) = self.publish_module(
                    module,
                    named_addr_opt.map(|s| Identifier::new(s.as_str()).unwrap()),
                    gas_budget,
                    extra_args,
                )?;
                if print_bytecode {
                    output = merge_output(output, printed);
                }
                let data_path = data.path().to_str().unwrap();
                match syntax {
                    SyntaxChoice::Source => self.compiled_state().add_with_source_file(
                        named_addr_opt,
                        module,
                        (data_path.to_owned(), data),
                    ),
                    SyntaxChoice::IR => {
                        self.compiled_state()
                            .add_and_generate_interface_file(module);
                    },
                };
                Ok(merge_output(warnings_opt, output))
            },
            TaskCommand::Run(
                RunCommand {
                    signers,
                    args,
                    type_args,
                    gas_budget,
                    syntax,
                    name: None,
                    print_bytecode,
                },
                extra_args,
            ) => {
                let syntax = syntax.unwrap_or_else(|| self.default_syntax());
                let (script, warning_opt) =
                    self.compile_script(syntax, data, start_line, command_lines_stop)?;
                let printed = if print_bytecode {
                    let disassembler = disassembler_for_view(BinaryIndexedView::Script(&script));
                    Some(format!(
                        "\n== BEGIN Bytecode ==\n{}\n== END Bytecode ==",
                        disassembler.disassemble()?
                    ))
                } else {
                    None
                };
                let args = self.compiled_state().resolve_args(args)?;
                let type_args = self.compiled_state().resolve_type_args(type_args)?;
                let mut output =
                    self.execute_script(script, type_args, signers, args, gas_budget, extra_args)?;
                if print_bytecode {
                    output = merge_output(output, printed);
                }
                Ok(merge_output(warning_opt, output))
            },
            TaskCommand::Run(
                RunCommand {
                    signers,
                    args,
                    type_args,
                    gas_budget,
                    syntax,
                    name: Some((raw_addr, module_name, name)),
                    print_bytecode: _,
                },
                extra_args,
            ) => {
                assert!(
                    syntax.is_none(),
                    "syntax flag meaningless with function execution"
                );
                let addr = self.compiled_state().resolve_address(&raw_addr);
                let module_id = ModuleId::new(addr, module_name);
                let type_args = self.compiled_state().resolve_type_args(type_args)?;
                let args = self.compiled_state().resolve_args(args)?;
                let (output, return_values) = self.call_function(
                    &module_id,
                    name.as_ident_str(),
                    type_args,
                    signers,
                    args,
                    gas_budget,
                    extra_args,
                )?;
                let rendered_return_value = self.display_return_values(return_values);
                Ok(merge_output(output, rendered_return_value))
            },
            TaskCommand::View(ViewCommand { address, resource }) => {
                let state: &CompiledState<'a> = self.compiled_state();
                let StructTag {
                    address: module_addr,
                    module,
                    name,
                    type_args: type_arguments,
                } = resource
                    .into_struct_tag(&|s| Some(state.resolve_named_address(s)))
                    .unwrap();
                let module_id = ModuleId::new(module_addr, module);
                let address = self.compiled_state().resolve_address(&address);
                Ok(Some(self.view_data(
                    address,
                    &module_id,
                    name.as_ident_str(),
                    type_arguments,
                )?))
            },
            TaskCommand::Subcommand(c) => self.handle_subcommand(TaskInput {
                command: c,
                name,
                number,
                start_line,
                command_lines_stop,
                stop_line,
                data,
            }),
        }
    }

    fn register_temp_filename(&mut self, data: &NamedTempFile) {
        let data_path = data.path().to_str().unwrap();
        if !data_path.is_empty() {
            let compiled_state = self.compiled_state();
            let mapping = &mut compiled_state.temp_file_mapping;
            if !mapping.contains_key(data_path) {
                let generic_name = match mapping.len() {
                    0 => "TEMPFILE".to_string(),
                    idx => format!("TEMPFILE{}", idx),
                };
                mapping.insert(data_path.to_owned(), generic_name);
            }
        }
    }

    fn rewrite_temp_filenames(&mut self, output: String) -> String {
        let compiled_state = self.compiled_state();
        let mapping = &mut compiled_state.temp_file_mapping;
        if !mapping.is_empty() {
            let mut result_string = output;
            for (source, target) in mapping.iter() {
                result_string = result_string.replace(source, target);
            }
            result_string
        } else {
            output
        }
    }

    fn display_return_values(&self, return_values: SerializedReturnValues) -> Option<String> {
        let SerializedReturnValues {
            mutable_reference_outputs,
            return_values,
        } = return_values;
        let mut output = vec![];
        if !mutable_reference_outputs.is_empty() {
            let values = mutable_reference_outputs
                .iter()
                .map(|(idx, bytes, layout)| {
                    let value = self.deserialize(bytes, layout).unwrap();
                    (idx, value)
                })
                .collect::<Vec<_>>();
            let printed = values
                .iter()
                .map(|(idx, v)| {
                    let mut buf = String::new();
                    move_vm_types::values::debug::print_value(&mut buf, v).unwrap();
                    format!("local#{}: {}", idx, buf)
                })
                .collect::<Vec<_>>()
                .join(", ");
            output.push(format!("mutable inputs after call: {}", printed))
        };
        if !return_values.is_empty() {
            let values = return_values
                .iter()
                .map(|(bytes, layout)| self.deserialize(bytes, layout).unwrap())
                .collect::<Vec<_>>();
            let printed = values
                .iter()
                .map(|v| {
                    let mut buf = String::new();
                    move_vm_types::values::debug::print_value(&mut buf, v).unwrap();
                    buf
                })
                .collect::<Vec<_>>()
                .join(", ");
            output.push(format!("return values: {}", printed))
        };
        if output.is_empty() {
            None
        } else {
            Some(output.join("\n"))
        }
    }
}

fn disassembler_for_view(view: BinaryIndexedView) -> Disassembler {
    let source_mapping =
        SourceMapping::new_from_view(view, Spanned::unsafe_no_loc(()).loc).expect("source mapping");
    Disassembler::new(source_mapping, DisassemblerOptions::new())
}

impl<'a> CompiledState<'a> {
    pub fn new(
        named_address_mapping: BTreeMap<String, NumericalAddress>,
        pre_compiled_deps_v2: &'a PrecompiledFilesModules,
        default_named_address_mapping: Option<NumericalAddress>,
    ) -> Self {
        let pre_compiled_ids = pre_compiled_deps_v2
            .get_pre_compiled_modules()
            .into_iter()
            .map(|annot_module| {
                let ident = annot_module.module_ident();
                (
                    ident.value.address.into_addr_bytes().into_inner(),
                    ident.value.module.to_string(),
                )
            })
            .collect();
        let mut state = Self {
            pre_compiled_deps_v2,
            pre_compiled_ids,
            modules: BTreeMap::new(),
            compiled_module_named_address_mapping: BTreeMap::new(),
            named_address_mapping,
            default_named_address_mapping,
            temp_file_mapping: BTreeMap::new(),
        };
        for annot_module in pre_compiled_deps_v2.get_pre_compiled_modules() {
            let (named_addr_opt, _id) = annot_module.module_id();
            state.add_precompiled(
                named_addr_opt.map(|n| n.value),
                annot_module.named_module.module.clone(),
            );
        }
        state
    }

    pub fn dep_modules(&self) -> impl Iterator<Item = &CompiledModule> {
        self.modules.values().map(|pmod| &pmod.module)
    }

    pub fn source_files(&self) -> impl Iterator<Item = &String> {
        self.modules
            .iter()
            .filter_map(|(_, pmod)| Some(&pmod.source_file.as_ref()?.0))
    }

    pub fn add_with_source_file(
        &mut self,
        named_addr_opt: Option<Symbol>,
        module: CompiledModule,
        source_file: (String, NamedTempFile),
    ) {
        let id = module.self_id();
        self.check_not_precompiled(&id);
        if let Some(named_addr) = named_addr_opt {
            self.compiled_module_named_address_mapping
                .insert(id.clone(), named_addr);
        }

        let processed = ProcessedModule {
            module,
            source_file: Some(source_file),
        };
        self.modules.insert(id, processed);
    }

    pub fn add_and_generate_interface_file(&mut self, module: CompiledModule) {
        let id = module.self_id();
        self.check_not_precompiled(&id);
        let interface_file = NamedTempFile::new().unwrap();
        let path = interface_file.path().to_str().unwrap().to_owned();
        let (_id, interface_text) =
            legacy_move_compiler::interface_generator::write_module_to_string(
                &self.compiled_module_named_address_mapping,
                &module,
            )
            .unwrap();
        interface_file
            .reopen()
            .unwrap()
            .write_all(interface_text.as_bytes())
            .unwrap();
        let source_file = Some((path, interface_file));
        let processed = ProcessedModule {
            module,
            source_file,
        };
        self.modules.insert(id, processed);
    }

    fn add_precompiled(&mut self, named_addr_opt: Option<Symbol>, module: CompiledModule) {
        let id = module.self_id();
        if let Some(named_addr) = named_addr_opt {
            self.compiled_module_named_address_mapping
                .insert(id.clone(), named_addr);
        }
        let processed = ProcessedModule {
            module,
            source_file: None,
        };
        self.modules.insert(id, processed);
    }

    pub fn is_precompiled_dep(&self, id: &ModuleId) -> bool {
        let addr = *id.address();
        let name = id.name().to_string();
        self.pre_compiled_ids.contains(&(addr, name))
    }

    fn check_not_precompiled(&self, id: &ModuleId) {
        assert!(
            !self.is_precompiled_dep(id),
            "Error publishing module: '{}'. \
             Re-publishing modules in pre-compiled lib is not yet supported",
            id
        );
    }
}

fn compile_source_unit_v2(
    pre_compiled_deps: &PrecompiledFilesModules,
    named_address_mapping: BTreeMap<String, NumericalAddress>,
    deps: &[String],
    path: String,
    known_attributes: &BTreeSet<String>,
    language_version: LanguageVersion,
    experiments: Vec<(String, bool)>,
) -> Result<(AnnotatedCompiledUnit, Option<GlobalEnv>, Option<String>)> {
    let all_deps = {
        // The v2 compiler does not really support precompiled programs, so we must include all the
        // dependent sources with their directories here.
        let mut dirs: BTreeSet<_> = pre_compiled_deps
            .filenames()
            .iter()
            .filter_map(|file_name| {
                Path::new(file_name.as_str())
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .collect();
        remove_sub_dirs(&mut dirs);
        dirs.extend(deps.iter().cloned());
        dirs.into_iter().collect::<Vec<_>>()
    };

    let mut options = move_compiler_v2::Options {
        sources: vec![path],
        dependencies: all_deps,
        named_address_mapping: named_address_mapping
            .into_iter()
            .map(|(alias, addr)| format!("{}={}", alias, addr))
            .collect(),
        known_attributes: known_attributes.clone(),
        language_version: Some(language_version),
        ..move_compiler_v2::Options::default()
    };
    for (exp, value) in experiments {
        options = options.set_experiment(exp, value)
    }
    let mut error_writer = termcolor::Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut error_writer);
        move_compiler_v2::run_move_compiler(emitter.as_mut(), options)
    };
    let error_str = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    let (model, mut units) =
        result.map_err(|_| anyhow::anyhow!("compilation errors:\n {}", error_str))?;
    let unit = if units.len() != 1 {
        anyhow::bail!("expected either one script or one module")
    } else {
        units.pop().unwrap()
    };
    if error_str.is_empty() {
        Ok((unit, Some(model), None))
    } else {
        Ok((unit, None, Some(error_str)))
    }
}

fn remove_sub_dirs(dirs: &mut BTreeSet<String>) {
    for dir in dirs.clone() {
        for other_dir in dirs.clone() {
            if dir != other_dir && dir.starts_with(&other_dir) {
                dirs.remove(&dir);
                break;
            }
        }
    }
}

fn compile_ir_module<'a>(
    deps: impl Iterator<Item = &'a CompiledModule>,
    file_name: &str,
) -> Result<CompiledModule> {
    use move_ir_compiler::Compiler as IRCompiler;
    let code = std::fs::read_to_string(file_name).unwrap();
    IRCompiler::new(deps.collect()).into_compiled_module(&code)
}

fn compile_ir_script<'a>(
    deps: impl Iterator<Item = &'a CompiledModule>,
    file_name: &str,
) -> Result<CompiledScript> {
    use move_ir_compiler::Compiler as IRCompiler;
    let code = std::fs::read_to_string(file_name).unwrap();
    let (script, _) = IRCompiler::new(deps.collect()).into_compiled_script_and_source_map(&code)?;
    Ok(script)
}

pub fn run_test_impl<'a, Adapter>(
    config: TestRunConfig,
    path: &Path,
    pre_compiled_deps_v2: &'a PrecompiledFilesModules,
    exp_suffix: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>>
where
    Adapter: MoveTestAdapter<'a>,
    Adapter::ExtraInitArgs: Debug,
    Adapter::ExtraPublishArgs: Debug,
    Adapter::ExtraValueArgs: Debug,
    Adapter::ExtraRunArgs: Debug,
    Adapter::Subcommand: Debug,
{
    let extension = path.extension().unwrap().to_str().unwrap();
    let default_syntax = if extension == MOVE_IR_EXTENSION {
        SyntaxChoice::IR
    } else {
        assert!(extension == MOVE_EXTENSION);
        SyntaxChoice::Source
    };

    let mut output = String::new();
    let mut tasks = taskify::<
        TaskCommand<
            Adapter::ExtraInitArgs,
            Adapter::ExtraPublishArgs,
            Adapter::ExtraValueArgs,
            Adapter::ExtraRunArgs,
            Adapter::Subcommand,
        >,
    >(path)?
    .into_iter()
    .collect::<VecDeque<_>>();
    assert!(!tasks.is_empty());
    let num_tasks = tasks.len();
    writeln!(
        &mut output,
        "processed {} task{}",
        num_tasks,
        if num_tasks > 1 { "s" } else { "" }
    )
    .unwrap();
    let first_task = tasks.pop_front().unwrap();
    let init_opt = match &first_task.command {
        TaskCommand::Init(_, _) => Some(first_task.map(|known| match known {
            TaskCommand::Init(command, extra_args) => (command, extra_args),
            _ => unreachable!(),
        })),
        _ => {
            tasks.push_front(first_task);
            None
        },
    };
    let (mut adapter, result_opt) = Adapter::init(
        default_syntax,
        config.clone(),
        pre_compiled_deps_v2,
        init_opt,
    );
    if let Some(result) = result_opt {
        writeln!(output, "\ninit:\n{}", result)?;
    }
    for task in tasks {
        handle_known_task(&mut output, &mut adapter, task);
    }

    handle_expected_output(path, output, exp_suffix)?;
    Ok(())
}

fn handle_known_task<'a, Adapter: MoveTestAdapter<'a>>(
    output: &mut String,
    adapter: &mut Adapter,
    task: TaskInput<
        TaskCommand<
            Adapter::ExtraInitArgs,
            Adapter::ExtraPublishArgs,
            Adapter::ExtraValueArgs,
            Adapter::ExtraRunArgs,
            Adapter::Subcommand,
        >,
    >,
) {
    let task_number = task.number;
    let task_name = task.name.to_owned();
    let start_line = task.start_line;
    let stop_line = task.stop_line;
    if let Some(data) = &task.data {
        adapter.register_temp_filename(data);
    }
    let result = adapter.handle_command(task);
    let result_string = match result {
        Ok(None) => return,
        Ok(Some(s)) => s,
        Err(e) => format!("Error: {}", e),
    };
    let result_string = adapter.rewrite_temp_filenames(result_string);
    assert!(!result_string.is_empty());
    writeln!(
        output,
        "\ntask {} '{}'. lines {}-{}:\n{}",
        task_number, task_name, start_line, stop_line, result_string
    )
    .unwrap();
}

fn handle_expected_output(
    test_path: &Path,
    output: impl AsRef<str>,
    exp_suffix: &Option<String>,
) -> Result<()> {
    let output = output.as_ref();
    assert!(!output.is_empty());
    let exp_path = if let Some(suffix) = exp_suffix {
        test_path.with_extension(suffix)
    } else {
        test_path.with_extension(EXP_EXT)
    };

    if read_env_update_baseline() {
        std::fs::write(exp_path, output).unwrap();
        return Ok(());
    }

    if !exp_path.exists() {
        std::fs::write(&exp_path, "").unwrap();
    }
    let expected_output = std::fs::read_to_string(&exp_path)
        .unwrap()
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    if output != expected_output {
        let msg = format!(
            "Expected errors differ from actual errors:\n{}",
            format_diff(expected_output, output),
        );
        anyhow::bail!(add_update_baseline_fix(msg))
    } else {
        Ok(())
    }
}
