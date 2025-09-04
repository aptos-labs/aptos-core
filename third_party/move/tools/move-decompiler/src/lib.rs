// Copyright (c) Velor Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use clap::Parser;
use codespan::Span;
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{Buffer, WriteColor},
};
use move_binary_format::{file_format::CompiledScript, module_script_conversion, CompiledModule};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;
use move_model::{
    metadata::LanguageVersion,
    model::{GlobalEnv, Loc, ModuleId},
    sourcifier::Sourcifier,
};
use move_stackless_bytecode::{
    astifier,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
};
use std::{collections::BTreeMap, fs, io::Write, mem, path::Path, rc::Rc, vec};

#[derive(Parser, Clone, Debug, Default)]
#[clap(author, version, about)]
pub struct Options {
    /// Language version of the produced output. The decompiler will not produce source
    /// which requires a newer version. If the decompiled bytecode requires
    /// language features beyond this version, decompilation will fail.
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,

    /// By default, the decompiler detects conditionals (`if` and `match`) in
    /// the bytecode. With this flag, the behavior can be turned off.
    #[clap(long, default_value = "false")]
    pub no_conditionals: bool,

    /// By default, the decompiler eliminates single assignments, and reconstructs
    /// expressions from the bytecode. With this flag, the behavior can be turned off.
    #[clap(long, default_value = "false")]
    pub no_expressions: bool,

    /// Directory where to place decompiled sources.
    #[clap(short, long, default_value = "")]
    pub output_dir: String,

    /// Ending to use for decompiled sources. Defaults to `mv.move`
    #[clap(short, long, default_value = "mv.move")]
    pub ending: String,

    /// Whether the input files should be interpreted as scripts. Default is
    /// to interpret them as modules.
    #[clap(long, short, default_value = "false")]
    pub script: bool,

    /// Directory where to search for source maps. Source maps are identified
    /// by the base name of the bytecode file with suffix `mvsm`.
    #[clap(long, default_value = "")]
    pub source_map_dir: String,

    /// Input files, interpreted as serialized versions of a module
    /// or script, depending on the `--script` flag.
    pub inputs: Vec<String>,
}

impl Options {
    /// Disable conditional transformation.
    pub fn disable_conditional_transformation(self, no_conditionals: bool) -> Self {
        Self {
            no_conditionals,
            ..self
        }
    }

    /// Disable assignment transformation.
    pub fn disable_assignment_transformation(self, no_expressions: bool) -> Self {
        Self {
            no_expressions,
            ..self
        }
    }
}

/// Represents an instance of a decompiler. The decompiler can be run end-to-end operating
/// on files, or incrementally without already deserialized modules and scripts.
pub struct Decompiler {
    options: Options,
    env: GlobalEnv,
}

impl Decompiler {
    /// Creates a new decompiler.
    pub fn new(options: Options) -> Self {
        Self {
            options,
            env: GlobalEnv::new(),
        }
    }

    /// Performs a run of the decompiler as specified by the options. This reads all
    /// bytecode input files and runs the stages of decompilation for each, producing
    /// output source files.
    pub fn run<W>(mut self, error_writer: &mut W) -> anyhow::Result<()>
    where
        W: Write + WriteColor,
    {
        for input_path in mem::take(&mut self.options.inputs) {
            let input_file = Path::new(Path::new(&input_path).file_name().unwrap_or_default())
                .to_string_lossy()
                .to_string();
            let bytes =
                fs::read(&input_path).with_context(|| format!("reading `{}`", input_path))?;
            let output = if self.options.script {
                let script = CompiledScript::deserialize(&bytes)
                    .with_context(|| format!("deserializing script `{}`", input_path))?;
                let source_map = self.get_source_map(&input_path, &input_file, &bytes);
                self.decompile_script(script, source_map)?
            } else {
                let module = CompiledModule::deserialize(&bytes)
                    .with_context(|| format!("deserializing module `{}`", input_path))?;
                let source_map = self.get_source_map(&input_path, &input_file, &bytes);
                self.decompile_module(module, source_map)?
            };
            self.env.check_diag(
                error_writer,
                Severity::Warning,
                &format!("decompiling `{}`", input_path),
            )?;
            let out_file = Path::new(&self.options.output_dir)
                .join(&input_file)
                .with_extension(&self.options.ending);
            fs::write(&out_file, output)
                .with_context(|| format!("writing `{}`", out_file.display()))?;
        }
        Ok(())
    }

    fn get_source_map(&mut self, input_path: &str, input_file: &str, bytes: &[u8]) -> SourceMap {
        let path = Path::new(&self.options.source_map_dir)
            .join(input_file)
            .with_extension("mvsm");
        match fs::read(path) {
            Ok(bytes) => bcs::from_bytes::<SourceMap>(&bytes)
                .unwrap_or_else(|_| self.empty_source_map(input_path, &bytes)),
            _ => self.empty_source_map(input_path, bytes),
        }
    }

    /// Decompile a package of modules together with their source maps.
    /// It loads all the modules into the environment first, and then decompile them one by one.
    pub fn decompile_package(
        &mut self,
        modules: Vec<CompiledModule>,
        source_maps: Vec<SourceMap>,
    ) -> Result<String> {
        let mut result = String::new();
        let mut module_ids = vec![];
        for (module, source_map) in modules.iter().zip(source_maps.iter()) {
            let module_id = self.load_module(module.clone(), source_map.clone())?;
            module_ids.push(module_id);
        }
        for &module_id in module_ids.iter() {
            result.push_str(&self.decompile_loaded_module(module_id)?);
        }
        Ok(result)
    }

    /// Decompiles the given binary module. A source map must be provided for error
    /// reporting; use `self.empty_source_map` to create one if none is available.
    pub fn decompile_module(
        &mut self,
        module: CompiledModule,
        source_map: SourceMap,
    ) -> Result<String> {
        let module_id = self.load_module(module, source_map)?;
        self.decompile_loaded_module(module_id)
    }

    pub fn load_module(
        &mut self,
        module: CompiledModule,
        source_map: SourceMap,
    ) -> Result<ModuleId> {
        self.validate_module(&module)?;
        let module_id = self
            .env
            .load_compiled_module(/*with_dep_closure*/ true, module, source_map);
        let mut error_writer = Buffer::no_color();
        self.env
            .check_diag(&mut error_writer, Severity::Warning, "load compiled module")?;
        Ok(module_id)
    }

    pub fn decompile_loaded_module(&mut self, module_id: ModuleId) -> Result<String> {
        // Lift file format bytecode to stackless bytecode.
        let mut targets = FunctionTargetsHolder::default();
        self.lift_to_stackless_bytecode(module_id, &mut targets);
        // Lift stackless bytecode to AST.
        self.lift_to_ast(&targets);
        // Sourcify the ast.
        let output = self.sourcify_ast(module_id);
        // Check for decompilation errors
        let mut error_writer = Buffer::no_color();
        if self
            .env()
            .check_diag(&mut error_writer, Severity::Warning, "decompilation")
            .is_err()
        {
            return Err(anyhow::Error::msg(format!(
                "decompilation errors:\n{}",
                String::from_utf8_lossy(&error_writer.into_inner())
            )));
        }
        Ok(output)
    }

    pub fn validate_module(&self, module: &CompiledModule) -> Result<()> {
        // Run bytecode verification on the module.
        move_bytecode_verifier::verify_module(module)
            .map_err(|e| anyhow::Error::msg(format!("Bytecode verification failed: {:#?}", e)))
    }

    pub fn lift_to_stackless_bytecode(
        &mut self,
        module_id: ModuleId,
        targets: &mut FunctionTargetsHolder,
    ) {
        // Create FunctionTargetsHolder with stackless bytecode for all functions in the module,
        let module_env = self.env.get_module(module_id);
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }
    }

    pub fn lift_to_ast(&mut self, targets: &FunctionTargetsHolder) {
        // Generate AST for all function targets.
        for fun_id in targets.get_funs() {
            let fun_env = self.env.get_function(fun_id);
            let target = targets.get_target(&fun_env, &FunctionVariant::Baseline);
            if target.get_bytecode().is_empty() {
                continue;
            }
            if let Some(def) = astifier::generate_ast_raw(&target) {
                let mut def = def.clone();
                // Run the AST transformation pipeline until a fixed point is reached.
                // Why doing this: one transformation can create new optimization opportunities for other transformations.
                //
                // Every transformation reduces the AST in a successful iteration, guaranteeing termination of the loop.
                // New transformations should follow the same principle.
                loop {
                    let mut new_def = def.clone();
                    if !self.options.no_expressions {
                        new_def = astifier::transform_assigns(&target, new_def);
                    }
                    if !self.options.no_conditionals {
                        new_def = astifier::transform_conditionals(&target, new_def);
                    }
                    if new_def == def {
                        break;
                    }
                    def = new_def;
                }
                // The next step must always happen to create valid Move
                let def = astifier::bind_free_vars(&target, def);
                self.env.set_function_def(fun_env.get_qualified_id(), def)
            }
        }
    }

    pub fn sourcify_ast(&self, module_id: ModuleId) -> String {
        // Render the module as source
        let sourcifier = Sourcifier::new(&self.env, true);
        sourcifier.print_module(module_id);
        sourcifier.result()
    }

    /// Decompiles the give binary script. Same as `decompile_module` but for scripts.
    pub fn decompile_script(
        &mut self,
        script: CompiledScript,
        source_map: SourceMap,
    ) -> Result<String> {
        let module = module_script_conversion::script_into_module(script, "main");
        self.decompile_module(module, source_map)
    }

    /// Return the environment the decompiler has built so far.
    pub fn env(&self) -> &GlobalEnv {
        &self.env
    }

    /// Creates an empty source map, with a location for the specified module. This virtual
    /// location is constructed from `name` (the file name of the module) and `contents`
    /// (the bytes). The internal logic of the environments needs currently both pieces
    /// information to create new locations (related to differences in Loc representation
    /// with legacy code).
    pub fn empty_source_map(&mut self, name: &str, unique_bytes: &[u8]) -> SourceMap {
        let file_id = self.env.add_source(
            FileHash::new_from_bytes(unique_bytes),
            Rc::new(BTreeMap::new()),
            name,
            "",
            true,
            true,
        );
        let loc = Loc::new(file_id, Span::new(0, 0));
        SourceMap::new(self.env.to_ir_loc(&loc), None)
    }
}
