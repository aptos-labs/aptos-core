// Copyright (c) Aptos Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use clap::Parser;
use codespan::Span;
use codespan_reporting::{diagnostic::Severity, term::termcolor::WriteColor};
use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;
use move_model::{
    metadata::LanguageVersion,
    model::{GlobalEnv, Loc},
    sourcifier::Sourcifier,
};
use move_stackless_bytecode::{
    astifier,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
};
use std::{collections::BTreeMap, fs, io::Write, mem, path::Path, rc::Rc};

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

/// Represent an instance of a decompiler. The decompiler can be run end-to-end operating
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
                self.decompile_script(script, source_map)
            } else {
                let module = CompiledModule::deserialize(&bytes)
                    .with_context(|| format!("deserializing module `{}`", input_path))?;
                let source_map = self.get_source_map(&input_path, &input_file, &bytes);
                self.decompile_module(module, source_map)
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

    /// Decompiles the given binary module. A source map must be provided for error
    /// reporting; use `self.empty_source_map` to create one if none is available.
    pub fn decompile_module(&mut self, module: CompiledModule, source_map: SourceMap) -> String {
        // Verify the module
        if let Err(e) = move_bytecode_verifier::verify_module(&module) {
            self.env.error(
                &self.env.to_loc(&source_map.definition_location),
                &format!("bytecode verification failed: {:#?}", e),
            );
            return String::default();
        }
        // Load module into environment, creating stubs for any missing dependencies
        let module_id = self
            .env
            .load_compiled_module(/*with_dep_closure*/ true, module, source_map);
        if self.env.has_errors() {
            return String::default();
        }

        // Create FunctionTargetsHolder with stackless bytecode for all functions in the module,
        let module_env = self.env.get_module(module_id);
        let mut targets = FunctionTargetsHolder::default();
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }

        // Generate AST for all function targets.
        for fun_id in targets.get_funs() {
            let fun_env = self.env.get_function(fun_id);
            let target = targets.get_target(&fun_env, &FunctionVariant::Baseline);
            if target.get_bytecode().is_empty() {
                continue;
            }
            if let Some(def) = astifier::generate_ast_raw(&target) {
                let def = if self.options.no_expressions {
                    def
                } else {
                    astifier::transform_assigns(&target, def)
                };
                let def = if self.options.no_conditionals {
                    def
                } else {
                    astifier::transform_conditionals(&target, def)
                };
                // The next step must always happen to create valid Move
                let def = astifier::bind_free_vars(&target, def);
                self.env.set_function_def(fun_env.get_qualified_id(), def)
            }
        }

        // Render the module as source
        let sourcifier = Sourcifier::new(&self.env);
        sourcifier.print_module(module_id);
        sourcifier.result()
    }

    /// Decompiles the give binary script. Same as `decompile_module` but for scripts.
    pub fn decompile_script(&mut self, script: CompiledScript, source_map: SourceMap) -> String {
        self.decompile_module(
            move_model::convert_script_to_module(script, self.env.get_module_count()),
            source_map,
        )
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
