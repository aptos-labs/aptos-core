// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Build GlobalEnv from already-parsed AST (no filesystem access required)
//!
//! This module provides an alternative to run_model_builder_in_compiler_mode()
//! that accepts an already-parsed AST instead of file paths. This enables
//! compilation in environments without filesystem access (WASM, testing, etc.).

use crate::{add_move_lang_diagnostics, model::GlobalEnv, options::ModelBuilderOptions};
use itertools::Itertools;
use legacy_move_compiler::{
    command_line::compiler::{SteppedCompiler, EMPTY_COMPILER, PASS_EXPANSION},
    diagnostics::FilesSourceText,
    parser::ast::Program as ParsedProgram,
    shared::{CompilationEnv, Flags, NamedAddressMaps},
};
use std::{collections::{BTreeMap, BTreeSet}, rc::Rc};

/// Build Move model from already-parsed AST
///
/// This is the filesystem-free equivalent of `run_model_builder_in_compiler_mode()`.
/// Instead of accepting file paths and parsing them, it accepts an already-parsed
/// program AST and file contents.
///
/// # Arguments
/// * `parsed_program` - Already-parsed Move program (from `parse_program_from_sources`)
/// * `files` - Map of file hashes to (name, content) for source files
/// * `named_address_maps` - Named address mappings
/// * `target_file_names` - Set of file names that are compilation targets (vs deps)
/// * `options` - Model builder options
/// * `flags` - Compilation flags
/// * `known_attributes` - Set of recognized attributes
///
/// # Returns
/// GlobalEnv with the compiled model, or errors if compilation fails
///
/// # Example
/// ```ignore
/// use move_compiler_v2::sources::SourceMap;
/// use legacy_move_compiler::parser::parse_program_from_sources;
///
/// // Parse from sources
/// let mut sources = SourceMap::new();
/// sources.add_file("test.move", "module 0x1::Test {}");
///
/// let targets = sources.to_files_source_text();
/// let (files, pprog_res) = parse_program_from_sources(...)?;
/// let (pprog, comments) = pprog_res?;
///
/// // Build model from AST
/// let env = run_model_builder_from_ast(
///     pprog,
///     files,
///     named_address_maps,
///     target_files,
///     options,
///     flags,
///     &known_attrs,
/// )?;
/// ```
pub fn run_model_builder_from_ast(
    parsed_program: ParsedProgram,
    files: FilesSourceText,
    _named_address_maps: NamedAddressMaps,
    target_file_names: BTreeSet<String>,
    options: ModelBuilderOptions,
    flags: Flags,
    known_attributes: &BTreeSet<String>,
) -> anyhow::Result<GlobalEnv> {
    let mut env = GlobalEnv::new();
    env.set_language_version(options.language_version);
    env.set_extension(options);

    // Identify which files are dependencies vs targets
    let dep_files: BTreeSet<_> = parsed_program
        .lib_definitions
        .iter()
        .map(|p| p.def.file_hash())
        .collect();

    // Add source files to the environment
    for member in parsed_program
        .source_definitions
        .iter()
        .chain(parsed_program.lib_definitions.iter())
    {
        let fhash = member.def.file_hash();
        let (fname, fsrc) = files.get(&fhash).unwrap();
        let is_target = !dep_files.contains(&fhash);

        let aliases = parsed_program
            .named_address_maps
            .get(member.named_address_map)
            .iter()
            .map(|(symbol, addr)| (env.symbol_pool().make(symbol.as_str()), *addr))
            .collect();

        env.add_source(
            fhash,
            Rc::new(aliases),
            fname.as_str(),
            fsrc,
            is_target,
            target_file_names.contains(fname.as_str()),
        );
    }

    // Add any files that don't contain definitions
    for fhash in files.keys().sorted() {
        if env.get_file_id(*fhash).is_none() {
            let (fname, fsrc) = files.get(fhash).unwrap();
            let is_target = !dep_files.contains(fhash);
            env.add_source(
                *fhash,
                Rc::new(BTreeMap::new()),
                fname.as_str(),
                fsrc,
                is_target,
                target_file_names.contains(fname.as_str()),
            );
        }
    }

    // Step 1: Extract module dependency closure information before moving parsed_program
    // Identify which files are targets vs dependencies
    let dep_files: BTreeSet<_> = parsed_program
        .lib_definitions
        .iter()
        .map(|p| p.def.file_hash())
        .collect();

    // Create a compiler from the parsed AST (bypasses file reading/parsing)
    // This is the key: we use at_parser() instead of from_files()
    let compilation_env = CompilationEnv::new(flags.clone(), known_attributes.clone());
    let empty_compiler: SteppedCompiler<EMPTY_COMPILER> = SteppedCompiler::new(compilation_env, None);
    let compiler = empty_compiler.at_parser(parsed_program);

    // Step 2: Run expansion
    let expansion_ast = match compiler.run::<PASS_EXPANSION>() {
        Err(diags) => {
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(mut compiler) => {
            // Check for warnings/errors after expansion
            use legacy_move_compiler::diagnostics::codes::Severity;
            if let Err(diags) = compiler.compilation_env().check_diags_at_or_above_severity(Severity::Warning) {
                add_move_lang_diagnostics(&mut env, diags);
                if env.has_errors() {
                    return Ok(env);
                }
            }
            let (_, expansion_ast) = compiler.into_ast();
            expansion_ast
        },
    };

    // Step 3: Collect module dependencies
    use crate::{collect_related_modules_recursive, ImplicitModuleDeps, run_move_checker, well_known};
    use legacy_move_compiler::expansion::ast as E;

    let mut visited_modules = BTreeSet::new();
    let mut implicit_modules = [
        ImplicitModuleDeps::new(well_known::VECTOR_MODULE),
        ImplicitModuleDeps::new(well_known::CMP_MODULE),
        ImplicitModuleDeps::new(well_known::STRING_MODULE),
        ImplicitModuleDeps::new(well_known::STRING_UTILS_MODULE),
        ImplicitModuleDeps::new(well_known::SIGNER_MODULE),
    ];

    // Collect all related modules from targets
    for (_, mident, mdef) in &expansion_ast.modules {
        let src_file_hash = mdef.loc.file_hash();
        if !dep_files.contains(&src_file_hash) {
            collect_related_modules_recursive(mident, &expansion_ast.modules, &mut visited_modules);
        }
        for implicit_module in &mut implicit_modules {
            implicit_module.process(mident, &expansion_ast.modules);
        }
    }

    for sdef in expansion_ast.scripts.values() {
        let src_file_hash = sdef.loc.file_hash();
        if !dep_files.contains(&src_file_hash) {
            for (_, mident, _neighbor) in &sdef.immediate_neighbors {
                collect_related_modules_recursive(
                    mident,
                    &expansion_ast.modules,
                    &mut visited_modules,
                );
            }
        }
    }

    // Step 4: Prepare expansion AST for compiler v2, and build model
    let expansion_ast = {
        let E::Program { modules, scripts } = expansion_ast;
        let modules = modules.filter_map(|mident, mut mdef| {
            // Include implicit modules and their dependencies
            (implicit_modules
                .iter()
                .any(|implicit_module| implicit_module.contains(&mident.value))
                || visited_modules.contains(&mident.value))
            .then(|| {
                mdef.is_source_module = true;
                mdef
            })
        });
        E::Program { modules, scripts }
    };

    // Step 5: Run the move checker (type checking and model building)
    run_move_checker(&mut env, expansion_ast);

    Ok(env)
}
