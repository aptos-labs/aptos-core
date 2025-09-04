// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    ast::ModuleName,
    builder::model_builder::ModelBuilder,
    metadata::LanguageVersion,
    model::{FunId, GlobalEnv, Loc, ModuleId, StructId},
    options::ModelBuilderOptions,
};
use builder::module_builder::ModuleBuilder;
use codespan::ByteIndex;
use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};
use itertools::Itertools;
use legacy_move_compiler::{
    self,
    diagnostics::{codes::Severity, Diagnostics},
    expansion::ast::{self as E, ModuleIdent, ModuleIdent_},
    parser::ast::{self as P},
    shared::{
        parse_named_address, unique_map::UniqueMap, CompilationEnv, NumericalAddress, PackagePaths,
    },
    Compiler, Flags, PASS_EXPANSION, PASS_PARSER,
};
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol as MoveSymbol;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    rc::Rc,
};

pub mod ast;
mod builder;
pub mod code_writer;
pub mod constant_folder;
pub mod exp_builder;
pub mod exp_generator;
pub mod exp_rewriter;
pub mod intrinsics;
pub mod metadata;
pub mod model;
pub mod options;
pub mod pragmas;
pub mod pureness_checker;
pub mod sourcifier;
pub mod spec_translator;
pub mod symbol;
pub mod ty;
pub mod ty_invariant_analysis;
pub mod well_known;

pub use builder::binary_module_loader;

/// Represents information about a package: the sources it contains and the package private
/// address mapping.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub sources: Vec<String>,
    pub address_map: BTreeMap<String, NumericalAddress>,
}

/// Builds the Move model for the v2 compiler. This builds the model, compiling both code and specs
/// from sources into typed-checked AST. No bytecode is attached to the model.  This currently uses
/// the v1 compiler as the parser (up to expansion AST), after that a new type checker.
///
/// Note that `source` and  `source_deps` are either Move files or package subdirectories which
/// contain Move files, all of which should be compiled (not the root of a package, but the
/// `sources`, `scripts`, and/or `tests`, depending on compilation mode.
pub fn run_model_builder_in_compiler_mode(
    source: PackageInfo,
    source_deps: PackageInfo,
    deps: Vec<PackageInfo>,
    skip_attribute_checks: bool,
    known_attributes: &BTreeSet<String>,
    language_version: LanguageVersion,
    warn_of_deprecation_use: bool,
    warn_of_deprecation_use_in_velor_libs: bool,
    compile_test_code: bool,
    compile_verify_code: bool,
) -> anyhow::Result<GlobalEnv> {
    let to_package_paths = |PackageInfo {
                                sources,
                                address_map,
                            }| PackagePaths {
        name: None,
        paths: sources,
        named_address_map: address_map,
    };
    run_model_builder_with_options_and_compilation_flags(
        vec![to_package_paths(source)],
        vec![to_package_paths(source_deps)],
        deps.into_iter().map(to_package_paths).collect(),
        ModelBuilderOptions {
            language_version,
            ..ModelBuilderOptions::default()
        },
        Flags::model_compilation()
            .set_warn_of_deprecation_use(warn_of_deprecation_use)
            .set_warn_of_deprecation_use_in_velor_libs(warn_of_deprecation_use_in_velor_libs)
            .set_skip_attribute_checks(skip_attribute_checks)
            .set_verify(compile_verify_code)
            .set_keep_testing_functions(compile_test_code)
            .set_language_version(language_version.into()),
        known_attributes,
    )
}

/// Build the move model with custom compilation flags and custom options
pub fn run_model_builder_with_options_and_compilation_flags<
    Paths: Into<MoveSymbol> + Clone + Debug,
    NamedAddress: Into<MoveSymbol> + Clone + Debug,
>(
    move_sources_targets: Vec<PackagePaths<Paths, NamedAddress>>,
    move_sources_deps: Vec<PackagePaths<Paths, NamedAddress>>,
    deps: Vec<PackagePaths<Paths, NamedAddress>>,
    options: ModelBuilderOptions,
    flags: Flags,
    known_attributes: &BTreeSet<String>,
) -> anyhow::Result<GlobalEnv> {
    let mut env = GlobalEnv::new();
    env.set_language_version(options.language_version);
    env.set_extension(options);

    let move_sources = move_sources_targets
        .iter()
        .chain(move_sources_deps.iter())
        .cloned()
        .collect();
    let target_sources_names: BTreeSet<String> = move_sources_targets
        .iter()
        .flat_map(|pack| pack.paths.iter())
        .map(|sym| {
            <Paths as Into<MoveSymbol>>::into(sym.clone())
                .as_str()
                .to_owned()
        })
        .collect();

    // Step 1: parse the program to get comments and a separation of targets and dependencies.
    let (files, comments_and_compiler_res) =
        Compiler::from_package_paths(move_sources, deps, flags, known_attributes)
            .run::<PASS_PARSER>()?;
    let (comment_map, compiler) = match comments_and_compiler_res {
        Err(diags) => {
            // Add source files so that the env knows how to translate locations of parse errors
            let empty_alias = Rc::new(BTreeMap::new());
            for (fhash, (fname, fsrc)) in &files {
                env.add_source(
                    *fhash,
                    empty_alias.clone(),
                    fname.as_str(),
                    fsrc,
                    /* is_target */ true,
                    target_sources_names.contains(fname.as_str()),
                );
            }
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(res) => res,
    };
    let (compiler, parsed_prog) = compiler.into_ast();

    // Add source files for targets and dependencies
    let dep_files: BTreeSet<_> = parsed_prog
        .lib_definitions
        .iter()
        .map(|p| p.def.file_hash())
        .collect();

    for member in parsed_prog
        .source_definitions
        .iter()
        .chain(parsed_prog.lib_definitions.iter())
    {
        let fhash = member.def.file_hash();
        let (fname, fsrc) = files.get(&fhash).unwrap();
        let is_target = !dep_files.contains(&fhash);
        let aliases = parsed_prog
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
            target_sources_names.contains(fname.as_str()),
        );
    }

    // If a move file does not contain any definition, it will not appear in `parsed_prog`. Add them explicitly.
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
                target_sources_names.contains(fname.as_str()),
            );
        }
    }

    // Add any documentation comments found by the Move compiler to the env.
    for (fhash, documentation) in comment_map {
        let file_id = env.get_file_id(fhash).expect("file name defined");
        env.add_documentation(
            file_id,
            documentation
                .into_iter()
                .map(|(idx, s)| (ByteIndex(idx), s))
                .collect(),
        )
    }

    // Step 2: run the compiler up to expansion
    let parsed_prog = {
        let P::Program {
            named_address_maps,
            mut source_definitions,
            lib_definitions,
        } = parsed_prog;
        source_definitions.extend(lib_definitions);
        P::Program {
            named_address_maps,
            source_definitions,
            lib_definitions: vec![],
        }
    };
    let (_, expansion_ast) = match compiler.at_parser(parsed_prog).run::<PASS_EXPANSION>() {
        Err(diags) => {
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(mut compiler) => {
            // There may have been errors but nevertheless a stepped compiler is returned.
            let compiler_env: &mut CompilationEnv = compiler.compilation_env();
            if let Err(diags) = compiler_env.check_diags_at_or_above_severity(Severity::Warning) {
                add_move_lang_diagnostics(&mut env, diags);
                if env.has_errors() {
                    return Ok(env);
                }
            }
            compiler.into_ast()
        },
    };

    // Extract the module/script dependency closure
    let mut visited_modules = BTreeSet::new();
    // Extract the module dependency closure for the vector module
    let mut vector_and_its_dependencies = BTreeSet::new();
    // Extract the module dependency closure for the std::cmp module
    let mut cmp_and_its_dependencies = BTreeSet::new();
    let mut seen_vector = false;
    let mut seen_cmp = false;
    for (_, mident, mdef) in &expansion_ast.modules {
        let src_file_hash = mdef.loc.file_hash();
        if !dep_files.contains(&src_file_hash) {
            collect_related_modules_recursive(mident, &expansion_ast.modules, &mut visited_modules);
        }
        if !seen_vector && is_vector(*mident) {
            seen_vector = true;
            // Collect the vector module and its dependencies.
            collect_related_modules_recursive(
                mident,
                &expansion_ast.modules,
                &mut vector_and_its_dependencies,
            );
        }
        if !seen_cmp && is_cmp(*mident) {
            seen_cmp = true;
            // Collect the cmp module and its dependencies.
            collect_related_modules_recursive(
                mident,
                &expansion_ast.modules,
                &mut cmp_and_its_dependencies,
            );
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

    // Step 3: Prepare expansion AST for compiler v2, and build model.
    let expansion_ast = {
        let E::Program { modules, scripts } = expansion_ast;
        let modules = modules.filter_map(|mident, mut mdef| {
            // For compiler v2, we need to always include the `vector` module and any of its dependencies,
            // to handle cases of implicit usage.
            // E.g., index operation on a vector results in a call to `vector::borrow`.
            // TODO(#15483): consider refactoring code to avoid this special case.
            (vector_and_its_dependencies.contains(&mident.value)
                // We also need to always include the `cmp` module and its dependencies,
                // so that we can use interfaces offered by `cmp` to support comparison, Lt/Le/Gt/Ge, over non-integer types.
                || cmp_and_its_dependencies.contains(&mident.value)
                || visited_modules.contains(&mident.value))
            .then(|| {
                mdef.is_source_module = true;
                mdef
            })
        });
        E::Program { modules, scripts }
    };

    // The expansion AST will be type checked. No bytecode is attached.
    run_move_checker(&mut env, expansion_ast);
    Ok(env)
}

/// Is `module_ident` the `vector` module?
fn is_vector(module_ident: ModuleIdent_) -> bool {
    module_ident.address.into_addr_bytes().into_inner() == AccountAddress::ONE
        && module_ident.module.0.value.as_str() == "vector"
}

/// Is `module_ident` the `0x1::cmp` module?
fn is_cmp(module_ident: ModuleIdent_) -> bool {
    module_ident.address.into_addr_bytes().into_inner() == AccountAddress::ONE
        && module_ident.module.0.value.as_str() == well_known::CMP_MODULE
}

fn run_move_checker(env: &mut GlobalEnv, program: E::Program) {
    let mut builder = ModelBuilder::new(env);
    for (module_count, (module_id, module_def)) in program
        .modules
        .into_iter()
        .sorted_by_key(|(_, def)| def.dependency_order)
        .enumerate()
    {
        let loc = builder.to_loc(&module_def.loc);
        let addr_bytes = builder.resolve_address(&loc, &module_id.value.address);
        let module_name = ModuleName::from_address_bytes_and_name(
            addr_bytes,
            builder
                .env
                .symbol_pool()
                .make(&module_id.value.module.0.value),
        );
        // Assign new module id in the model.
        let module_id = ModuleId::new(module_count);
        // Associate the module name with the module id for lookups.
        builder.module_table.insert(module_name.clone(), module_id);
        let mut module_translator = ModuleBuilder::new(&mut builder, module_id, module_name);
        module_translator.translate(loc, module_def);
    }
    for (i, (_, script_def)) in program.scripts.into_iter().enumerate() {
        let loc = builder.to_loc(&script_def.loc);
        let module_name = ModuleName::pseudo_script_name(builder.env.symbol_pool(), i);
        let module_id = ModuleId::new(builder.env.module_data.len());
        let mut module_translator = ModuleBuilder::new(&mut builder, module_id, module_name);
        let module_def = expansion_script_to_module(script_def);
        module_translator.translate(loc, module_def);
    }

    // Populate GlobalEnv with model-level information
    builder.populate_env();

    // After all specs have been processed, warn about any unused schemas.
    builder.warn_unused_schemas();

    builder.add_friend_decl_for_package_visibility();

    // Perform any remaining friend-declaration checks and update friend module id information.
    check_and_update_friend_info(builder);
}

/// Checks if any friend declarations are invalid because:
///     - they are self-friend declarations
///     - they are out-of-address friend declarations
///     - they refer to unbound modules
/// If so, report errors.
/// Also, update friend module id information: this function assumes all modules have been assigned ids.
///
/// Note: we assume (a) friend declarations creating cyclic dependencies (cycle size > 1),
///                 (b) duplicate friend declarations
/// have been reported already. Currently, these checks happen in the expansion phase.
fn check_and_update_friend_info(mut builder: ModelBuilder) {
    let module_table = std::mem::take(&mut builder.module_table);
    let env = builder.env;
    // To save (loc, name) info about self friend decls.
    let mut self_friends = vec![];
    // To save (loc, friend_module_name, current_module_name) info about out-of-address friend decls.
    let mut out_of_address_friends = vec![];
    // To save (loc, friend_module_name) info about unbound modules in friend decls.
    let mut unbound_friend_modules = vec![];
    // Patch up information about friend module ids, as all the modules have ids by now.
    for module in env.module_data.iter_mut() {
        let mut friend_modules = BTreeSet::new();
        for friend_decl in module.friend_decls.iter_mut() {
            // Save information of out-of-address friend decls to report error later.
            if friend_decl.module_name.addr() != module.name.addr() {
                out_of_address_friends.push((
                    friend_decl.loc.clone(),
                    friend_decl.module_name.clone(),
                    module.name.clone(),
                ));
            }
            if let Some(friend_mod_id) = module_table.get(&friend_decl.module_name) {
                friend_decl.module_id = Some(*friend_mod_id);
                friend_modules.insert(*friend_mod_id);
                // Save information of self-friend decls to report error later.
                if module.id == *friend_mod_id {
                    self_friends.push((friend_decl.loc.clone(), friend_decl.module_name.clone()));
                }
            } else {
                // Save information of unbound modules in friend decls to report error later.
                unbound_friend_modules
                    .push((friend_decl.loc.clone(), friend_decl.module_name.clone()));
            }
        }
        module.friend_modules = friend_modules;
    }
    // Report self-friend errors.
    for (loc, module_name) in self_friends {
        env.error(
            &loc,
            &format!(
                "cannot declare module `{}` as a friend of itself",
                module_name.display_full(env)
            ),
        );
    }
    // Report out-of-address friend errors.
    for (loc, friend_mod_name, cur_mod_name) in out_of_address_friends {
        env.error(
            &loc,
            &format!(
                "friend modules of `{}` must have the same address, \
                    but the declared friend module `{}` has a different address",
                cur_mod_name.display_full(env),
                friend_mod_name.display_full(env),
            ),
        );
    }
    // Report unbound friend errors.
    for (loc, friend_mod_name) in unbound_friend_modules {
        env.error(
            &loc,
            &format!(
                "unbound module `{}` in friend declaration",
                friend_mod_name.display_full(env)
            ),
        );
    }
}

fn collect_related_modules_recursive<'a>(
    mident: &'a ModuleIdent_,
    modules: &'a UniqueMap<ModuleIdent, E::ModuleDefinition>,
    visited_modules: &mut BTreeSet<ModuleIdent_>,
) {
    if visited_modules.contains(mident) {
        return;
    }
    let mdef = modules.get_(mident).unwrap();
    visited_modules.insert(*mident);
    for (_, next_mident, _) in &mdef.immediate_neighbors {
        collect_related_modules_recursive(next_mident, modules, visited_modules);
    }
}

pub fn add_move_lang_diagnostics(env: &mut GlobalEnv, diags: Diagnostics) {
    let mk_label = |is_primary: bool, (loc, msg): (move_ir_types::location::Loc, String)| {
        let style = if is_primary {
            LabelStyle::Primary
        } else {
            LabelStyle::Secondary
        };
        let loc = env.to_loc(&loc);
        Label::new(style, loc.file_id(), loc.span()).with_message(msg)
    };
    for (severity, msg, primary_label, secondary_labels, notes) in diags.into_codespan_format() {
        let diag = Diagnostic::new(severity)
            .with_labels(vec![mk_label(true, primary_label)])
            .with_message(msg)
            .with_labels(
                secondary_labels
                    .into_iter()
                    .map(|e| mk_label(false, e))
                    .collect(),
            )
            .with_notes(notes);
        env.add_diag(diag);
    }
}

// =================================================================================================
// Helpers

pub fn parse_addresses_from_options(
    named_addr_strings: Vec<String>,
) -> anyhow::Result<BTreeMap<String, NumericalAddress>> {
    named_addr_strings
        .iter()
        .map(|x| parse_named_address(x))
        .collect()
}

fn expansion_script_to_module(script: E::Script) -> E::ModuleDefinition {
    let E::Script {
        package_name,
        attributes,
        loc,
        immediate_neighbors,
        used_addresses,
        function_name,
        constants,
        function,
        specs,
        use_decls,
    } = script;

    // Construct a pseudo module definition.
    let mut functions = UniqueMap::new();
    functions.add(function_name, function).unwrap();

    E::ModuleDefinition {
        package_name,
        attributes,
        loc,
        dependency_order: usize::MAX,
        immediate_neighbors,
        used_addresses,
        is_source_module: true,
        friends: UniqueMap::new(),
        structs: UniqueMap::new(),
        constants,
        functions,
        specs,
        use_decls,
    }
}
