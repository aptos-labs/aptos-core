// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    ast::{ModuleName, Spec},
    builder::model_builder::ModelBuilder,
    model::{FunId, FunctionData, GlobalEnv, Loc, ModuleData, ModuleId, StructId},
    options::ModelBuilderOptions,
    simplifier::{SpecRewriter, SpecRewriterPipeline},
};
use builder::module_builder::ModuleBuilder;
use codespan::ByteIndex;
use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};
use itertools::Itertools;
#[allow(unused_imports)]
use log::warn;
use move_binary_format::{
    access::ModuleAccess,
    check_bounds::BoundsChecker,
    file_format::{
        self_module_name, AddressIdentifierIndex, CompiledModule, CompiledScript,
        FunctionDefinition, FunctionDefinitionIndex, FunctionHandle, FunctionHandleIndex,
        IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex,
        StructDefinitionIndex, Visibility,
    },
};
use move_compiler::{
    self,
    compiled_unit::{self, AnnotatedCompiledScript, AnnotatedCompiledUnit},
    diagnostics::Diagnostics,
    expansion::ast::{self as E, Address, ModuleIdent, ModuleIdent_},
    naming::ast as N,
    parser::ast::{self as P, ModuleName as ParserModuleName},
    shared::{
        parse_named_address, unique_map::UniqueMap, Identifier as IdentifierTrait,
        NumericalAddress, PackagePaths,
    },
    typing::ast as T,
    Compiler, Flags, PASS_COMPILATION, PASS_EXPANSION, PASS_INLINING, PASS_PARSER,
};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_ir_types::location::sp;
use move_symbol_pool::Symbol as MoveSymbol;
use num::{BigUint, Num};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};

pub mod ast;
mod builder;
pub mod code_writer;
pub mod exp_generator;
pub mod exp_rewriter;
pub mod intrinsics;
pub mod model;
pub mod options;
pub mod pragmas;
pub mod simplifier;
pub mod spec_translator;
pub mod symbol;
pub mod ty;
pub mod well_known;

// =================================================================================================
// Entry Point

/// Build the move model with default compilation flags and default options and no named addresses.
/// This collects transitive dependencies for move sources from the provided directory list.
pub fn run_model_builder<
    Paths: Into<MoveSymbol> + Clone,
    NamedAddress: Into<MoveSymbol> + Clone,
>(
    move_sources: Vec<PackagePaths<Paths, NamedAddress>>,
    deps: Vec<PackagePaths<Paths, NamedAddress>>,
) -> anyhow::Result<GlobalEnv> {
    run_model_builder_with_options(move_sources, deps, ModelBuilderOptions::default())
}

/// Build the move model with default compilation flags and custom options and a set of provided
/// named addreses.
/// This collects transitive dependencies for move sources from the provided directory list.
pub fn run_model_builder_with_options<
    Paths: Into<MoveSymbol> + Clone,
    NamedAddress: Into<MoveSymbol> + Clone,
>(
    move_sources: Vec<PackagePaths<Paths, NamedAddress>>,
    deps: Vec<PackagePaths<Paths, NamedAddress>>,
    options: ModelBuilderOptions,
) -> anyhow::Result<GlobalEnv> {
    run_model_builder_with_options_and_compilation_flags(
        move_sources,
        deps,
        options,
        Flags::verification(),
    )
}

/// Build the move model with custom compilation flags and custom options
/// This collects transitive dependencies for move sources from the provided directory list.
pub fn run_model_builder_with_options_and_compilation_flags<
    Paths: Into<MoveSymbol> + Clone,
    NamedAddress: Into<MoveSymbol> + Clone,
>(
    move_sources: Vec<PackagePaths<Paths, NamedAddress>>,
    deps: Vec<PackagePaths<Paths, NamedAddress>>,
    options: ModelBuilderOptions,
    flags: Flags,
) -> anyhow::Result<GlobalEnv> {
    let mut env = GlobalEnv::new();
    env.set_extension(options);

    // Step 1: parse the program to get comments and a separation of targets and dependencies.
    let (files, comments_and_compiler_res) = Compiler::from_package_paths(move_sources, deps)
        .set_flags(flags)
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
                    /* is_dep */ false,
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
        let is_dep = dep_files.contains(&fhash);
        let aliases = parsed_prog
            .named_address_maps
            .get(member.named_address_map)
            .iter()
            .map(|(symbol, addr)| (env.symbol_pool().make(symbol.as_str()), *addr))
            .collect();
        env.add_source(fhash, Rc::new(aliases), fname.as_str(), fsrc, is_dep);
    }

    // If a move file does not contain any definition, it will not appear in `parsed_prog`. Add them explicitly.
    for fhash in files.keys().sorted() {
        if env.get_file_id(*fhash).is_none() {
            let (fname, fsrc) = files.get(fhash).unwrap();
            let is_dep = dep_files.contains(fhash);
            env.add_source(
                *fhash,
                Rc::new(BTreeMap::new()),
                fname.as_str(),
                fsrc,
                is_dep,
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
    let (compiler, expansion_ast) = match compiler.at_parser(parsed_prog).run::<PASS_EXPANSION>() {
        Err(diags) => {
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(compiler) => compiler.into_ast(),
    };

    // Extract the module/script closure
    let mut visited_modules = BTreeSet::new();
    for (_, mident, mdef) in &expansion_ast.modules {
        let src_file_hash = mdef.loc.file_hash();
        if !dep_files.contains(&src_file_hash) {
            collect_related_modules_recursive(mident, &expansion_ast.modules, &mut visited_modules);
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

    // Step 3: selective compilation.
    let mut expansion_ast = {
        let E::Program { modules, scripts } = expansion_ast;
        let modules = modules.filter_map(|mident, mut mdef| {
            visited_modules.contains(&mident.value).then(|| {
                mdef.is_source_module = true;
                mdef
            })
        });
        E::Program { modules, scripts }
    };

    // Step 4: retrospectively add lambda-lifted function to expansion AST
    let (compiler, inlining_ast) = match compiler
        .at_expansion(expansion_ast.clone())
        .run::<PASS_INLINING>()
    {
        Err(diags) => {
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(compiler) => compiler.into_ast(),
    };

    for (loc, module_id, expansion_module) in expansion_ast.modules.iter_mut() {
        match inlining_ast.modules.get_(module_id) {
            None => {
                env.error(
                    &env.to_loc(&loc),
                    "unable to find matching module in inlining AST",
                );
            },
            Some(inlining_module) => {
                retrospective_lambda_lifting(inlining_module, expansion_module);
            },
        }
    }

    // Step 5: Run the compiler from instrumented expansion AST fully to the compiled units
    let units = match compiler
        .at_expansion(expansion_ast.clone())
        .run::<PASS_COMPILATION>()
    {
        Err(diags) => {
            add_move_lang_diagnostics(&mut env, diags);
            return Ok(env);
        },
        Ok(compiler) => {
            let (units, warnings) = compiler.into_compiled_units();
            if !warnings.is_empty() {
                // NOTE: these diagnostics are just warnings. it should be feasible to continue the
                // model building here. But before that, register the warnings to the `GlobalEnv`
                // first so we get a chance to report these warnings as well.
                add_move_lang_diagnostics(&mut env, warnings);
            }
            units
        },
    };

    // Check for bytecode verifier errors (there should not be any)
    let diags = compiled_unit::verify_units(&units);
    if !diags.is_empty() {
        add_move_lang_diagnostics(&mut env, diags);
        return Ok(env);
    }

    // Now that it is known that the program has no errors, run the spec checker on verified units
    // plus expanded AST. This will populate the environment including any errors.
    run_spec_checker(&mut env, units, expansion_ast);
    Ok(env)
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

/// Build a `GlobalEnv` from a collection of `CompiledModule`'s. The `modules` list must be
/// topologically sorted by the dependency relation (i.e., a child node in the dependency graph
/// should appear earlier in the vector than its parents).
pub fn run_bytecode_model_builder<'a>(
    modules: impl IntoIterator<Item = &'a CompiledModule>,
) -> anyhow::Result<GlobalEnv> {
    let mut env = GlobalEnv::new();
    for (i, m) in modules.into_iter().enumerate() {
        let id = m.self_id();
        let addr = addr_to_big_uint(id.address());
        let module_name = ModuleName::new(addr, env.symbol_pool().make(id.name().as_str()));
        let module_id = ModuleId::new(i);
        let mut module_data = ModuleData::stub(module_name.clone(), module_id, m.clone());

        // add functions
        for (i, def) in m.function_defs().iter().enumerate() {
            let def_idx = FunctionDefinitionIndex(i as u16);
            let name = m.identifier_at(m.function_handle_at(def.function).name);
            let symbol = env.symbol_pool().make(name.as_str());
            let fun_id = FunId::new(symbol);
            let data = FunctionData::stub(symbol, def_idx, def.function);
            module_data.function_data.insert(fun_id, data);
            module_data.function_idx_to_id.insert(def_idx, fun_id);
        }

        // add structs
        for (i, def) in m.struct_defs().iter().enumerate() {
            let def_idx = StructDefinitionIndex(i as u16);
            let name = m.identifier_at(m.struct_handle_at(def.struct_handle).name);
            let symbol = env.symbol_pool().make(name.as_str());
            let struct_id = StructId::new(symbol);
            let data = env.create_move_struct_data(
                m,
                def_idx,
                symbol,
                Loc::default(),
                Vec::default(),
                Spec::default(),
            );
            module_data.struct_data.insert(struct_id, data);
            module_data.struct_idx_to_id.insert(def_idx, struct_id);
        }

        env.module_data.push(module_data);
    }
    Ok(env)
}

fn add_move_lang_diagnostics(env: &mut GlobalEnv, diags: Diagnostics) {
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

#[allow(deprecated)]
fn script_into_module(compiled_script: CompiledScript) -> CompiledModule {
    let mut script = compiled_script;

    // Add the "<SELF>" identifier if it isn't present.
    //
    // Note: When adding an element to the table, in theory it is possible for the index
    // to overflow. This will not be a problem if we get rid of the script/module conversion.
    let self_ident_idx = match script
        .identifiers
        .iter()
        .position(|ident| ident.as_ident_str() == self_module_name())
    {
        Some(idx) => IdentifierIndex::new(idx as u16),
        None => {
            let idx = IdentifierIndex::new(script.identifiers.len() as u16);
            script
                .identifiers
                .push(Identifier::new(self_module_name().to_string()).unwrap());
            idx
        },
    };

    // Add a dummy adress if none exists.
    let dummy_addr = AccountAddress::new([0xFF; AccountAddress::LENGTH]);
    let dummy_addr_idx = match script
        .address_identifiers
        .iter()
        .position(|addr| addr == &dummy_addr)
    {
        Some(idx) => AddressIdentifierIndex::new(idx as u16),
        None => {
            let idx = AddressIdentifierIndex::new(script.address_identifiers.len() as u16);
            script.address_identifiers.push(dummy_addr);
            idx
        },
    };

    // Add a self module handle.
    let self_module_handle_idx = match script
        .module_handles
        .iter()
        .position(|handle| handle.address == dummy_addr_idx && handle.name == self_ident_idx)
    {
        Some(idx) => ModuleHandleIndex::new(idx as u16),
        None => {
            let idx = ModuleHandleIndex::new(script.module_handles.len() as u16);
            script.module_handles.push(ModuleHandle {
                address: dummy_addr_idx,
                name: self_ident_idx,
            });
            idx
        },
    };

    // Find the index to the empty signature [].
    // Create one if it doesn't exist.
    let return_sig_idx = match script.signatures.iter().position(|sig| sig.0.is_empty()) {
        Some(idx) => SignatureIndex::new(idx as u16),
        None => {
            let idx = SignatureIndex::new(script.signatures.len() as u16);
            script.signatures.push(Signature(vec![]));
            idx
        },
    };

    // Create a function handle for the main function.
    let main_handle_idx = FunctionHandleIndex::new(script.function_handles.len() as u16);
    script.function_handles.push(FunctionHandle {
        module: self_module_handle_idx,
        name: self_ident_idx,
        parameters: script.parameters,
        return_: return_sig_idx,
        type_parameters: script.type_parameters,
    });

    // Create a function definition for the main function.
    let main_def = FunctionDefinition {
        function: main_handle_idx,
        visibility: Visibility::Public,
        is_entry: true,
        acquires_global_resources: vec![],
        code: Some(script.code),
    };

    let module = CompiledModule {
        version: script.version,
        module_handles: script.module_handles,
        self_module_handle_idx,
        struct_handles: script.struct_handles,
        function_handles: script.function_handles,
        field_handles: vec![],
        friend_decls: vec![],

        struct_def_instantiations: vec![],
        function_instantiations: script.function_instantiations,
        field_instantiations: vec![],

        signatures: script.signatures,

        identifiers: script.identifiers,
        address_identifiers: script.address_identifiers,
        constant_pool: script.constant_pool,
        metadata: script.metadata,

        struct_defs: vec![],
        function_defs: vec![main_def],
    };
    BoundsChecker::verify_module(&module).expect("invalid bounds in module");
    module
}

#[allow(deprecated)]
fn run_spec_checker(env: &mut GlobalEnv, units: Vec<AnnotatedCompiledUnit>, mut eprog: E::Program) {
    let mut builder = ModelBuilder::new(env);

    // Merge the compiled units with source ASTs, preserving the order of the compiled
    // units which is topological w.r.t. use relation.
    let mut modules = vec![];
    for unit in units {
        match unit {
            AnnotatedCompiledUnit::Module(annot_module) => {
                let module_ident = annot_module.module_ident();
                let expanded_module = match eprog.modules.remove(&module_ident) {
                    Some(m) => m,
                    None => {
                        env.error(
                            &env.unknown_loc(),
                                  &format!(
                                "[internal] cannot associate bytecode module `{}` with expansion AST",
                                module_ident
                            )
                        );
                        return;
                    },
                };
                modules.push((
                    module_ident,
                    expanded_module,
                    annot_module.named_module.module,
                    annot_module.named_module.source_map,
                    annot_module.function_infos,
                ));
            },
            AnnotatedCompiledUnit::Script(AnnotatedCompiledScript {
                loc: _loc,
                named_script: script,
                function_info,
            }) => {
                let expanded_script = match eprog.scripts.remove(&script.name) {
                    Some(s) => s,
                    None => {
                        env.error(
                            &env.unknown_loc(),
                            &format!(
                                "[internal] cannot associate bytecode script `{}` with expansion AST",
                                script.name
                            )
                        );
                        return;
                    },
                };

                // Convert the script into a module.
                let address = Address::Numerical(
                    None,
                    sp(expanded_script.loc, NumericalAddress::DEFAULT_ERROR_ADDRESS),
                );
                let ident = sp(
                    expanded_script.loc,
                    ModuleIdent_::new(address, ParserModuleName(expanded_script.function_name.0)),
                );
                let mut function_infos = UniqueMap::new();
                function_infos
                    .add(expanded_script.function_name, function_info)
                    .unwrap();

                let expanded_module = expansion_script_to_module(expanded_script);
                let module = script_into_module(script.script);
                modules.push((
                    ident,
                    expanded_module,
                    module,
                    script.source_map,
                    function_infos,
                ));
            },
        }
    }

    for (module_count, (module_id, expanded_module, compiled_module, source_map, function_infos)) in
        modules.into_iter().enumerate()
    {
        let loc = builder.to_loc(&expanded_module.loc);
        let addr_bytes = builder.resolve_address(&loc, &module_id.value.address);
        let module_name = ModuleName::from_address_bytes_and_name(
            addr_bytes,
            builder
                .env
                .symbol_pool()
                .make(&module_id.value.module.0.value),
        );
        let module_id = ModuleId::new(module_count);
        let mut module_translator = ModuleBuilder::new(&mut builder, module_id, module_name);
        module_translator.translate(
            loc,
            expanded_module,
            compiled_module,
            source_map,
            function_infos,
        );
    }

    // Populate GlobalEnv with model-level information
    builder.populate_env();

    // After all specs have been processed, warn about any unused schemas.
    builder.warn_unused_schemas();

    // Apply simplification passes
    run_spec_simplifier(env);
}

fn run_spec_simplifier(env: &mut GlobalEnv) {
    let options = env
        .get_extension::<ModelBuilderOptions>()
        .expect("options for model builder");
    let mut rewriter = SpecRewriterPipeline::new(&options.simplification_pipeline);
    rewriter
        .override_with_rewrite(env)
        .unwrap_or_else(|e| panic!("Failed to run spec simplification: {}", e))
}

fn retrospective_lambda_lifting(
    inlined_module: &T::ModuleDefinition,
    expansion_module: &mut E::ModuleDefinition,
) {
    // extract lifted lambda pack
    let mut lifted = vec![];
    for (_, _, fdef) in inlined_module.functions.iter() {
        match &fdef.body.value {
            T::FunctionBody_::Native => (),
            T::FunctionBody_::Defined(seq) => {
                collect_lambda_lifted_functions_in_sequence(&mut lifted, seq);
            },
        };
    }

    // downgrade the AST
    for lifted_fun in lifted {
        let T::SpecLambdaLiftedFunction {
            name: lifted_name,
            signature: lifted_signature,
            body: lifted_body,
            preset_args: _,
        } = lifted_fun;

        // convert function signature
        let N::FunctionSignature {
            type_parameters: lifted_ty_params,
            parameters: lifted_params,
            return_type: lifted_ret_ty,
        } = lifted_signature;

        let new_type_params = lifted_ty_params
            .into_iter()
            .map(|param| (param.user_specified_name, param.abilities))
            .collect();
        let new_params = lifted_params
            .into_iter()
            .map(|(v, t)| (v, downgrade_type_inlining_to_expansion(&t)))
            .collect();
        let new_ret_ty = downgrade_type_inlining_to_expansion(&lifted_ret_ty);

        // convert function body
        let new_body_exp = downgrade_exp_inlining_to_expansion(&lifted_body);
        let new_body_loc = new_body_exp.loc;
        let mut new_body_seq = E::Sequence::new();
        new_body_seq.push_back(sp(new_body_loc, E::SequenceItem_::Seq(new_body_exp)));
        let new_body = sp(new_body_loc, E::FunctionBody_::Defined(new_body_seq));

        // define the new function
        let new_fun = E::Function {
            attributes: E::Attributes::new(),
            loc: new_body_loc,
            inline: false,
            visibility: E::Visibility::Internal,
            signature: E::FunctionSignature {
                type_parameters: new_type_params,
                parameters: new_params,
                return_type: new_ret_ty,
            },
            entry: None,
            acquires: vec![], // TODO: might need inference here
            body: new_body,
            specs: BTreeMap::new(),
        };

        let new_name = P::FunctionName(sp(new_body_loc, lifted_name));
        expansion_module
            .functions
            .add(new_name, new_fun)
            .expect("duplicated entry of lambda function");
    }
}

// =================================================================================================
// Helpers

/// Converts an address identifier to a number representing the address.
pub fn addr_to_big_uint(addr: &AccountAddress) -> BigUint {
    BigUint::from_str_radix(&addr.to_string(), 16).unwrap()
}

/// Converts a biguint into an account address
pub fn big_uint_to_addr(i: &BigUint) -> AccountAddress {
    // TODO: do this in more efficient way (e.g., i.to_le_bytes() and pad out the resulting Vec<u8>
    // to ADDRESS_LENGTH
    AccountAddress::from_hex_literal(&format!("{:#x}", i)).unwrap()
}

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
    }
}

// =================================================================================================
// AST visitors

fn collect_lambda_lifted_functions_in_sequence(
    collection: &mut Vec<T::SpecLambdaLiftedFunction>,
    sequence: &T::Sequence,
) {
    for stmt in sequence {
        match &stmt.value {
            T::SequenceItem_::Seq(exp) | T::SequenceItem_::Bind(_, _, exp) => {
                collect_lambda_lifted_functions_in_exp(collection, exp.as_ref());
            },
            T::SequenceItem_::Declare(_) => (),
        }
    }
}

fn collect_lambda_lifted_functions_in_exp(
    collection: &mut Vec<T::SpecLambdaLiftedFunction>,
    exp: &T::Exp,
) {
    use T::UnannotatedExp_;

    match &exp.exp.value {
        UnannotatedExp_::Spec(anchor) => {
            collection.extend(anchor.used_lambda_funs.values().cloned());
        },
        UnannotatedExp_::Unit { .. }
        | UnannotatedExp_::Value(..)
        | UnannotatedExp_::Move { .. }
        | UnannotatedExp_::Copy { .. }
        | UnannotatedExp_::Use(..)
        | UnannotatedExp_::Constant(..)
        | UnannotatedExp_::BorrowLocal(..)
        | UnannotatedExp_::Break
        | UnannotatedExp_::Continue
        | UnannotatedExp_::UnresolvedError => (),

        UnannotatedExp_::ModuleCall(call) => {
            collect_lambda_lifted_functions_in_exp(collection, call.arguments.as_ref());
        },
        UnannotatedExp_::VarCall(_, args) | UnannotatedExp_::Builtin(_, args) => {
            collect_lambda_lifted_functions_in_exp(collection, args.as_ref());
        },
        UnannotatedExp_::Vector(_, _, _, args) => {
            collect_lambda_lifted_functions_in_exp(collection, args.as_ref());
        },

        UnannotatedExp_::IfElse(cond, lhs, rhs) => {
            collect_lambda_lifted_functions_in_exp(collection, cond.as_ref());
            collect_lambda_lifted_functions_in_exp(collection, lhs.as_ref());
            collect_lambda_lifted_functions_in_exp(collection, rhs.as_ref());
        },
        UnannotatedExp_::While(cond, body) => {
            collect_lambda_lifted_functions_in_exp(collection, cond.as_ref());
            collect_lambda_lifted_functions_in_exp(collection, body.as_ref());
        },
        UnannotatedExp_::Loop { body, .. } => {
            collect_lambda_lifted_functions_in_exp(collection, body.as_ref());
        },

        UnannotatedExp_::Block(seq) => {
            collect_lambda_lifted_functions_in_sequence(collection, seq);
        },
        UnannotatedExp_::Lambda(_, body) => {
            collect_lambda_lifted_functions_in_exp(collection, body);
        },

        UnannotatedExp_::Assign(_, _, val)
        | UnannotatedExp_::Return(val)
        | UnannotatedExp_::Abort(val)
        | UnannotatedExp_::Dereference(val)
        | UnannotatedExp_::UnaryExp(_, val)
        | UnannotatedExp_::Cast(val, _)
        | UnannotatedExp_::Annotate(val, _)
        | UnannotatedExp_::Borrow(_, val, _)
        | UnannotatedExp_::TempBorrow(_, val) => {
            collect_lambda_lifted_functions_in_exp(collection, val);
        },
        UnannotatedExp_::Mutate(lhs, rhs) | UnannotatedExp_::BinopExp(lhs, _, _, rhs) => {
            collect_lambda_lifted_functions_in_exp(collection, lhs);
            collect_lambda_lifted_functions_in_exp(collection, rhs);
        },

        UnannotatedExp_::Pack(_, _, _, fields) => {
            for (_, _, (_, (_, val))) in fields.iter() {
                collect_lambda_lifted_functions_in_exp(collection, val);
            }
        },
        UnannotatedExp_::ExpList(exprs) => {
            for e in exprs {
                match e {
                    T::ExpListItem::Single(val, _) | T::ExpListItem::Splat(_, val, _) => {
                        collect_lambda_lifted_functions_in_exp(collection, val);
                    },
                }
            }
        },
    }
}

fn downgrade_type_inlining_to_expansion(ty: &N::Type) -> E::Type {
    let rewritten = match &ty.value {
        N::Type_::Unit => E::Type_::Unit,
        N::Type_::Ref(is_mut, base) => {
            E::Type_::Ref(*is_mut, downgrade_type_inlining_to_expansion(base).into())
        },
        N::Type_::Param(param) => {
            let access = E::ModuleAccess_::Name(param.user_specified_name);
            E::Type_::Apply(sp(param.user_specified_name.loc, access), vec![])
        },
        N::Type_::Apply(_, name, args) => {
            let rewritten_args: Vec<_> = args
                .iter()
                .map(downgrade_type_inlining_to_expansion)
                .collect();
            match &name.value {
                N::TypeName_::Builtin(builtin) => {
                    let access = E::ModuleAccess_::Name(sp(
                        builtin.loc,
                        MoveSymbol::from(builtin.to_string()),
                    ));
                    E::Type_::Apply(sp(ty.loc, access), rewritten_args)
                },
                N::TypeName_::ModuleType(module_ident, struct_name) => {
                    let access = E::ModuleAccess_::ModuleAccess(*module_ident, struct_name.0);
                    E::Type_::Apply(sp(struct_name.loc(), access), rewritten_args)
                },
                N::TypeName_::Multiple(size) => {
                    assert_eq!(*size, rewritten_args.len());
                    E::Type_::Multiple(rewritten_args)
                },
            }
        },
        N::Type_::Anything | N::Type_::UnresolvedError | N::Type_::Var(_) => {
            panic!("ICE unresolved type")
        },
    };

    sp(ty.loc, rewritten)
}

fn downgrade_exp_inlining_to_expansion(exp: &T::Exp) -> E::Exp {
    use E::Exp_;
    use T::UnannotatedExp_;

    let rewritten = match &exp.exp.value {
        UnannotatedExp_::Unit { trailing } => Exp_::Unit {
            trailing: *trailing,
        },
        UnannotatedExp_::Value(val) => Exp_::Value(val.clone()),
        UnannotatedExp_::Move { from_user: _, var } => Exp_::Move(*var),
        UnannotatedExp_::Copy { from_user: _, var } => Exp_::Copy(*var),
        UnannotatedExp_::Use(_) => panic!("ICE unexpanded use"),
        UnannotatedExp_::Constant(module_ident_opt, name) => {
            let access = match module_ident_opt {
                None => E::ModuleAccess_::Name(name.0),
                Some(module_ident) => E::ModuleAccess_::ModuleAccess(*module_ident, name.0),
            };
            Exp_::Name(sp(name.loc(), access), None)
        },

        UnannotatedExp_::ModuleCall(call) => {
            let T::ModuleCall {
                module,
                name,
                is_macro,
                type_arguments,
                arguments,
                parameter_types: _,
                acquires: _,
            } = call.as_ref();
            let access = E::ModuleAccess_::ModuleAccess(*module, name.0);
            let rewritten_arguments = match downgrade_exp_inlining_to_expansion(arguments).value {
                Exp_::Unit { .. } => vec![],
                Exp_::ExpList(exps) => exps,
                e => vec![sp(arguments.exp.loc, e)],
            };
            let rewritten_ty_args: Vec<_> = type_arguments
                .iter()
                .map(downgrade_type_inlining_to_expansion)
                .collect();
            Exp_::Call(
                sp(name.loc(), access),
                *is_macro,
                if rewritten_ty_args.is_empty() {
                    None
                } else {
                    Some(rewritten_ty_args)
                },
                sp(exp.exp.loc, rewritten_arguments),
            )
        },
        UnannotatedExp_::VarCall(target, arguments) => {
            let access = E::ModuleAccess_::Name(target.0);
            let rewritten_arguments = match downgrade_exp_inlining_to_expansion(arguments).value {
                Exp_::Unit { .. } => vec![],
                Exp_::ExpList(exps) => exps,
                e => vec![sp(arguments.exp.loc, e)],
            };
            Exp_::Call(
                sp(target.loc(), access),
                false,
                None,
                sp(exp.exp.loc, rewritten_arguments),
            )
        },
        UnannotatedExp_::Builtin(builtin, arguments) => {
            let access = E::ModuleAccess_::Name(sp(
                builtin.loc,
                MoveSymbol::from(builtin.value.to_string()),
            ));
            let rewritten_arguments = match downgrade_exp_inlining_to_expansion(arguments).value {
                Exp_::Unit { .. } => vec![],
                Exp_::ExpList(exps) => exps,
                e => vec![sp(arguments.exp.loc, e)],
            };
            Exp_::Call(
                sp(builtin.loc, access),
                false,
                None,
                sp(exp.exp.loc, rewritten_arguments),
            )
        },
        UnannotatedExp_::Vector(l, _, ty, arguments) => {
            let rewritten_arguments = match downgrade_exp_inlining_to_expansion(arguments).value {
                Exp_::Unit { .. } => vec![],
                Exp_::ExpList(exps) => exps,
                e => vec![sp(arguments.exp.loc, e)],
            };
            let rewritten_ty = downgrade_type_inlining_to_expansion(ty);
            Exp_::Vector(
                *l,
                Some(vec![rewritten_ty]),
                sp(exp.exp.loc, rewritten_arguments),
            )
        },

        UnannotatedExp_::IfElse(cond, then_case, else_case) => Exp_::IfElse(
            downgrade_exp_inlining_to_expansion(cond).into(),
            downgrade_exp_inlining_to_expansion(then_case).into(),
            downgrade_exp_inlining_to_expansion(else_case).into(),
        ),
        UnannotatedExp_::While(cond, body) => Exp_::While(
            downgrade_exp_inlining_to_expansion(cond).into(),
            downgrade_exp_inlining_to_expansion(body).into(),
        ),
        UnannotatedExp_::Loop { has_break: _, body } => {
            Exp_::Loop(downgrade_exp_inlining_to_expansion(body).into())
        },
        UnannotatedExp_::Break => Exp_::Break,
        UnannotatedExp_::Continue => Exp_::Continue,

        UnannotatedExp_::Block(seq) => Exp_::Block(downgrade_sequence_inlining_to_expansion(seq)),
        UnannotatedExp_::Lambda(..) => {
            panic!("ICE unexpanded lambda after inlining");
        },

        UnannotatedExp_::Assign(vlist, _, val) => Exp_::Assign(
            downgrade_lvalue_list_inlining_to_expansion(vlist),
            downgrade_exp_inlining_to_expansion(val).into(),
        ),
        UnannotatedExp_::Mutate(lhs, rhs) => Exp_::Mutate(
            downgrade_exp_inlining_to_expansion(lhs).into(),
            downgrade_exp_inlining_to_expansion(rhs).into(),
        ),
        UnannotatedExp_::Return(val) => {
            Exp_::Return(downgrade_exp_inlining_to_expansion(val).into())
        },
        UnannotatedExp_::Abort(val) => Exp_::Abort(downgrade_exp_inlining_to_expansion(val).into()),

        UnannotatedExp_::Dereference(val) => {
            Exp_::Dereference(downgrade_exp_inlining_to_expansion(val).into())
        },
        UnannotatedExp_::UnaryExp(op, val) => {
            Exp_::UnaryExp(*op, downgrade_exp_inlining_to_expansion(val).into())
        },
        UnannotatedExp_::BinopExp(lhs, op, _, rhs) => Exp_::BinopExp(
            downgrade_exp_inlining_to_expansion(lhs).into(),
            *op,
            downgrade_exp_inlining_to_expansion(rhs).into(),
        ),

        UnannotatedExp_::Pack(module_ident, struct_name, ty_args, fields) => {
            let access = E::ModuleAccess_::ModuleAccess(*module_ident, struct_name.0);
            let rewritten_ty_args = ty_args
                .iter()
                .map(downgrade_type_inlining_to_expansion)
                .collect();
            let rewritten_fields =
                E::Fields::maybe_from_iter(fields.iter().map(|(loc, fid, (idx, (_, field)))| {
                    (
                        P::Field(sp(loc, *fid)),
                        (*idx, downgrade_exp_inlining_to_expansion(field)),
                    )
                }))
                .expect("field conversion");
            Exp_::Pack(
                sp(struct_name.loc(), access),
                Some(rewritten_ty_args),
                rewritten_fields,
            )
        },
        UnannotatedExp_::ExpList(items) => Exp_::ExpList(downgrade_exp_list_to_expansion(items)),

        UnannotatedExp_::BorrowLocal(is_mut, var) => {
            let access = E::ModuleAccess_::Name(var.0);
            Exp_::Borrow(
                *is_mut,
                sp(var.loc(), Exp_::Name(sp(var.loc(), access), None)).into(),
            )
        },
        UnannotatedExp_::TempBorrow(is_mut, target) => {
            Exp_::Borrow(*is_mut, downgrade_exp_inlining_to_expansion(target).into())
        },
        UnannotatedExp_::Borrow(is_mut, target, field) => {
            let base = E::ExpDotted_::Exp(downgrade_exp_inlining_to_expansion(target));
            let dotted = Exp_::ExpDotted(
                sp(
                    field.loc(),
                    E::ExpDotted_::Dot(sp(target.exp.loc, base).into(), field.0),
                )
                .into(),
            );
            Exp_::Borrow(*is_mut, sp(exp.exp.loc, dotted).into())
        },

        UnannotatedExp_::Cast(val, ty) => Exp_::Cast(
            downgrade_exp_inlining_to_expansion(val).into(),
            downgrade_type_inlining_to_expansion(ty),
        ),
        UnannotatedExp_::Annotate(val, ty) => Exp_::Annotate(
            downgrade_exp_inlining_to_expansion(val).into(),
            downgrade_type_inlining_to_expansion(ty),
        ),

        UnannotatedExp_::Spec(anchor) => {
            let used_locals = anchor.used_locals.values().map(|(_, v)| v.0).collect();
            let used_lambdas = anchor
                .used_lambda_funs
                .values()
                .map(|f| sp(exp.exp.loc, f.name))
                .collect();
            Exp_::Spec(anchor.id, used_locals, used_lambdas)
        },

        UnannotatedExp_::UnresolvedError => Exp_::UnresolvedError,
    };

    sp(exp.exp.loc, rewritten)
}

fn downgrade_exp_list_to_expansion(items: &[T::ExpListItem]) -> Vec<E::Exp> {
    items
        .iter()
        .map(|item| match item {
            T::ExpListItem::Single(e, _) | T::ExpListItem::Splat(_, e, _) => {
                downgrade_exp_inlining_to_expansion(e)
            },
        })
        .collect()
}

fn downgrade_lvalue_inlining_to_expansion(val: &T::LValue) -> E::LValue {
    let rewritten = match &val.value {
        T::LValue_::Var(var, _) => {
            let access = E::ModuleAccess_::Name(var.0);
            E::LValue_::Var(sp(var.loc(), access), None)
        },
        T::LValue_::Ignore => {
            let access = E::ModuleAccess_::Name(sp(val.loc, MoveSymbol::from("_")));
            E::LValue_::Var(sp(val.loc, access), None)
        },
        T::LValue_::Unpack(module_ident, struct_name, ty_args, fields)
        | T::LValue_::BorrowUnpack(_, module_ident, struct_name, ty_args, fields) => {
            let access = E::ModuleAccess_::ModuleAccess(*module_ident, struct_name.0);
            let rewritten_ty_args: Vec<_> = ty_args
                .iter()
                .map(downgrade_type_inlining_to_expansion)
                .collect();
            let rewritten_fields =
                E::Fields::maybe_from_iter(fields.iter().map(|(loc, fid, (idx, (_, field)))| {
                    (
                        P::Field(sp(loc, *fid)),
                        (*idx, downgrade_lvalue_inlining_to_expansion(field)),
                    )
                }))
                .expect("field conversion");

            E::LValue_::Unpack(
                sp(struct_name.loc(), access),
                if rewritten_ty_args.is_empty() {
                    None
                } else {
                    Some(rewritten_ty_args)
                },
                rewritten_fields,
            )
        },
    };

    sp(val.loc, rewritten)
}

fn downgrade_lvalue_list_inlining_to_expansion(val: &T::LValueList) -> E::LValueList {
    let rewritten = val
        .value
        .iter()
        .map(downgrade_lvalue_inlining_to_expansion)
        .collect();
    sp(val.loc, rewritten)
}

fn downgrade_sequence_inlining_to_expansion(seq: &T::Sequence) -> E::Sequence {
    let mut rewritten_seq = VecDeque::new();
    for stmt in seq {
        let rewritten_stmt = match &stmt.value {
            T::SequenceItem_::Seq(exp) => {
                E::SequenceItem_::Seq(downgrade_exp_inlining_to_expansion(exp))
            },
            T::SequenceItem_::Declare(vlist) => {
                E::SequenceItem_::Declare(downgrade_lvalue_list_inlining_to_expansion(vlist), None)
            },
            T::SequenceItem_::Bind(vlist, _, exp) => E::SequenceItem_::Bind(
                downgrade_lvalue_list_inlining_to_expansion(vlist),
                downgrade_exp_inlining_to_expansion(exp),
            ),
        };
        rewritten_seq.push_back(sp(stmt.loc, rewritten_stmt));
    }
    rewritten_seq
}

// =================================================================================================
// Crate Helpers

/// Helper to project the 1st element from a vector of pairs.
pub(crate) fn project_1st<T: Clone, R>(v: &[(T, R)]) -> Vec<T> {
    v.iter().map(|(x, _)| x.clone()).collect()
}

/// Helper to project the 2nd element from a vector of pairs.
pub(crate) fn project_2nd<T, R: Clone>(v: &[(T, R)]) -> Vec<R> {
    v.iter().map(|(_, x)| x.clone()).collect()
}
