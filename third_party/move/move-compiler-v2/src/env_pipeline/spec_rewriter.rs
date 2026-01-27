// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! The spec rewriter runs on the whole model after inlining and after check for pureness
//! and does the following:
//!
//! - For every transitively used Move function in specs, it derives
//!   a spec function version of it.
//! - It rewrites all specification expressions to call the derived spec
//!   function instead of the Move function.
//! - It also rewrites expression to replace Move constructs with spec
//!   constructs where possible. This includes replacing references
//!   with values. This transformation assumes that expressions
//!   are already checked for pureness.
//! - For all spec functions (including the derived ones) it computes
//!   transitive memory usage and callee functions.
//! - It checks that data invariants do not depend on memory, and flags
//!   errors if not. This can only be done after transitive memory
//!   usage is known.
//! - It collects all global invariants and attaches them, together
//!   with their memory usage, to the model.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::debug;
use move_model::{
    ast::{
        BehaviorKind, BehaviorTarget, ConditionKind, Exp, ExpData, GlobalInvariant, Operation,
        RewriteResult, SpecBlockTarget, SpecFunDecl, TempIndex,
    },
    exp_builder::ExpBuilder,
    exp_generator::ExpGenerator,
    exp_rewriter::ExpRewriterFunctions,
    metadata::LanguageVersion,
    model::{
        FunId, FunctionData, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, Parameter, QualifiedId,
        QualifiedInstId, SpecFunId, StructEnv,
    },
    spec_translator::SpecTranslator,
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use petgraph::prelude::DiGraphMap;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
};

pub fn run_spec_rewriter(env: &mut GlobalEnv) {
    debug!("rewriting specifications");

    // Collect all spec blocks and spec functions in the whole program, plus
    // functions in compilation scope. For the later we need to process
    // inline spec blocks.
    // TODO: we may want to optimize this to only rewrite specs involved in
    //   a verification problem, but we need to have a precise definition
    //   what this entails. For example, pre/post conditions need to be present
    //   only if the function spec is marked as opaque.
    let mut targets = RewriteTargets::create(env, RewritingScope::Everything);
    targets.filter(|target, _| match target {
        RewriteTarget::MoveFun(fid) => {
            let fun = env.get_function(*fid);
            fun.module_env.is_target() && !fun.is_inline() && !fun.is_native()
        },
        RewriteTarget::SpecFun(fid) => {
            let fun = env.get_spec_fun(*fid);
            !fun.is_native
        },
        RewriteTarget::SpecBlock(_) => true,
    });

    // Identify the Move functions transitively called by those targets. They need to be
    // converted to spec functions.
    let mut called_funs = BTreeSet::new();
    for target in targets.keys() {
        let callees: BTreeSet<_> = match target {
            RewriteTarget::MoveFun(_) => {
                if let RewriteState::Def(def) = target.get_env_state(env) {
                    let mut spec_callees = BTreeSet::new();
                    def.visit_inline_specs(&mut |spec| {
                        spec_callees.extend(spec.used_funs_with_uses().into_keys());
                        true // keep going
                    });
                    spec_callees
                } else {
                    BTreeSet::new()
                }
            },
            RewriteTarget::SpecFun(_) | RewriteTarget::SpecBlock(_) => {
                target.used_funs_with_uses(env).into_keys().collect()
            },
        };
        for callee in callees {
            called_funs.insert(callee);
            let mut transitive = env
                .get_function(callee)
                .get_transitive_closure_of_called_functions();
            called_funs.append(&mut transitive);
        }
    }

    // For compatibility reasons with the v1 way how to compile spec
    // blocks of inline functions, we also need to add all 'lambda'
    // lifted functions.

    // Derive spec functions for all called Move functions,
    // building a mapping between function ids. Also add
    // those new spec functions to `targets` for subsequent
    // processing.
    let mut function_mapping = BTreeMap::new();
    for fun_id in called_funs {
        let spec_fun_id = derive_spec_fun(env, fun_id, false);
        function_mapping.insert(fun_id, spec_fun_id);
        // Add new spec fun to targets for later processing
        targets.entry(RewriteTarget::SpecFun(spec_fun_id));
        // Mark spec fun to be used in environment
        env.add_used_spec_fun(spec_fun_id)
    }

    // Based on the mapping above, now visit all targets and convert them.
    for target in targets.keys().collect_vec() {
        use RewriteState::*;
        use RewriteTarget::*;
        let get_param_names =
            |params: &[Parameter]| params.iter().map(|Parameter(name, ..)| *name).collect_vec();
        match (&target, target.get_env_state(env)) {
            (MoveFun(_), Def(exp)) => {
                let mut converter = SpecConverter::new(env, &function_mapping, false);
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecFun(id), Def(exp)) => {
                let paras = get_param_names(&env.get_spec_fun(*id).params);
                let mut converter =
                    SpecConverter::new(env, &function_mapping, true).symbolized_parameters(paras);
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecBlock(sb_target), Spec(spec)) => {
                let mut converter = match sb_target {
                    SpecBlockTarget::SpecFunction(mid, spec_fun_id) => {
                        // When the spec block is associated with a spec function, need to replace temps with parameter names
                        let paras =
                            get_param_names(&env.get_spec_fun(mid.qualified(*spec_fun_id)).params);
                        SpecConverter::new(env, &function_mapping, true)
                            .symbolized_parameters(paras)
                    },
                    SpecBlockTarget::Function(mid, fid)
                    | SpecBlockTarget::FunctionCode(mid, fid, _) => {
                        // Set enclosing function context for behavioral predicate parameter handling
                        SpecConverter::new(env, &function_mapping, true)
                            .with_enclosing_fun(mid.qualified(*fid))
                    },
                    _ => SpecConverter::new(env, &function_mapping, true),
                };

                let (changed, new_spec) = converter.rewrite_spec_descent(sb_target, &spec);
                if changed {
                    *targets.state_mut(&target) = Spec(new_spec)
                }
            },
            _ => {},
        }
    }
    targets.write_to_env(env);

    // Now that all functions are defined, compute transitive callee and used memory.
    // Since specification functions can be recursive we compute the strongly-connected
    // components first and then process each in bottom-up order.
    let mut graph: DiGraphMap<QualifiedId<SpecFunId>, ()> = DiGraphMap::new();
    let spec_funs = env
        .get_modules()
        .flat_map(|m| {
            m.get_spec_funs()
                .map(|(id, _)| m.get_id().qualified(*id))
                .collect_vec()
        })
        .collect_vec();
    for qid in spec_funs {
        graph.add_node(qid);
        let (initial_callees, initial_usage) = if let Some(def) = &env.get_spec_fun(qid).body {
            let callees = def.called_spec_funs(env);
            for callee in &callees {
                graph.add_edge(qid, callee.to_qualified_id(), ());
            }
            (callees, def.directly_used_memory(env))
        } else {
            Default::default()
        };
        let decl_mut = env.get_spec_fun_mut(qid);
        (decl_mut.callees, decl_mut.used_memory) = (initial_callees, initial_usage);
    }
    for scc in petgraph::algo::kosaraju_scc(&graph) {
        // Within each cycle, the transitive usage is the union of the transitive
        // usage of each member.
        let mut transitive_callees = BTreeSet::new();
        let mut transitive_usage = BTreeSet::new();
        for qid in &scc {
            let decl = env.get_spec_fun(*qid);
            // Add direct usage.
            transitive_callees.extend(decl.callees.iter().cloned());
            transitive_usage.extend(decl.used_memory.iter().cloned());
            // Add indirect usage
            for callee in &decl.callees {
                let decl = env.get_spec_fun(callee.to_qualified_id());
                transitive_callees.extend(
                    decl.callees
                        .iter()
                        .map(|id| id.clone().instantiate(&callee.inst)),
                );
                transitive_usage.extend(
                    decl.used_memory
                        .iter()
                        .map(|mem| mem.clone().instantiate(&callee.inst)),
                );
            }
        }
        // Store result back
        for qid in scc {
            let decl_mut = env.get_spec_fun_mut(qid);
            decl_mut.callees.clone_from(&transitive_callees);
            decl_mut.used_memory.clone_from(&transitive_usage);
        }
    }

    // Last, process invariants
    for module in env.get_modules() {
        if module.is_target() {
            for str in module.get_structs() {
                check_data_invariants(&str)
            }
        }
    }
    collect_global_invariants_to_env(env)
}

/// Entry point to generate spec functions from lambda expressions to be expanded during inlining
pub fn run_spec_rewriter_inline(
    env: &mut GlobalEnv,
    module_id: ModuleId,
    lambda_fun_map: BTreeMap<usize, FunctionData>,
) -> BTreeMap<usize, (QualifiedId<SpecFunId>, QualifiedId<FunId>)> {
    debug!("rewriting specifications in inline functions");

    let mut targets = RewriteTargets::create_fun_targets(env, vec![]);

    // Traverse lifted lambda functions,
    // generate corresponding spec fun and ad them as rewrite target
    let mut function_data_mapping = BTreeMap::new();
    for (i, data) in lambda_fun_map {
        // Add lifted lambda expressions into env
        let fun_id = env.add_function_def_from_data(module_id, data);
        let qualified_fun_id = module_id.qualified(fun_id);
        let spec_fun_id = derive_spec_fun(env, qualified_fun_id, true);
        function_data_mapping.insert(i, (spec_fun_id, qualified_fun_id));
        // Add new spec fun to targets for later processing
        targets.entry(RewriteTarget::SpecFun(spec_fun_id));
        // For simplicity, we assume that all lambda parameters will be called
        env.add_used_spec_fun(spec_fun_id);
    }
    let function_mapping = BTreeMap::new();

    for target in targets.keys().collect_vec() {
        use RewriteState::*;
        use RewriteTarget::*;
        let get_param_names =
            |params: &[Parameter]| params.iter().map(|Parameter(name, ..)| *name).collect_vec();
        if let (SpecFun(id), Def(exp)) = (&target, target.get_env_state(env)) {
            let paras: Vec<Symbol> = get_param_names(&env.get_spec_fun(*id).params);
            let mut converter = SpecConverter::new_for_inline(env, &function_mapping, true)
                .symbolized_parameters(paras);
            let new_exp = converter.rewrite_exp(exp.clone());
            // If the spec function contains imperative expressions, set it to uninterpreted
            if converter.contains_imperative_expression {
                let spec_fun = converter.env.get_spec_fun_mut(*id);
                spec_fun.uninterpreted = true;
                spec_fun.body = None;
            } else if !ExpData::ptr_eq(&new_exp, &exp) {
                *targets.state_mut(&target) = Def(new_exp)
            }
        }
    }
    targets.write_to_env(env);
    function_data_mapping
}

// -------------------------------------------------------------------------------------------
// Deriving Specification Functions

// Derive a specification function from a Move function. Initially the body is the
// original one, not yet converted to the specification representation.
fn derive_spec_fun(
    env: &mut GlobalEnv,
    fun_id: QualifiedId<FunId>,
    for_inline: bool,
) -> QualifiedId<SpecFunId> {
    let fun = env.get_function(fun_id);
    let (is_native, body) = if fun.is_native() {
        (true, None)
    } else {
        let exp = fun.get_def().expect("function body").clone();
        (false, Some(exp))
    };

    // For historical reasons, those names are prefixed with `$` even though there
    // is no name clash allowed.
    let inline_prefix = if for_inline { "inline_" } else { "" };
    let name = env.symbol_pool().make(&format!(
        "${}{}",
        inline_prefix,
        fun.get_name().display(env.symbol_pool())
    ));
    // Eliminate references in parameters and result type
    let params = fun
        .get_parameters()
        .into_iter()
        .map(|Parameter(sym, ty, loc)| Parameter(sym, ty.skip_reference().clone(), loc))
        .collect();
    let result_type = fun.get_result_type().skip_reference().clone();

    // Currently we only attach the spec block when it is generated during inlining
    let spec = if for_inline {
        fun.get_spec().clone()
    } else {
        Default::default()
    };
    let decl = SpecFunDecl {
        loc: fun.get_loc(),
        name,
        type_params: fun.get_type_parameters(),
        params,
        context_params: None,
        result_type,
        used_memory: BTreeSet::new(),
        uninterpreted: false,
        is_move_fun: true,
        is_native,
        body,
        callees: BTreeSet::new(),
        is_recursive: RefCell::new(None),
        insts_using_generic_type_reflection: Default::default(),
        spec: RefCell::new(spec),
    };
    env.add_spec_function_def(fun_id.module_id, decl)
}

// -------------------------------------------------------------------------------------------
// Behavioral Predicate Reduction

/// Minimal ExpGenerator implementation for behavioral predicate reduction.
/// Provides enough context for SpecTranslator without full bytecode infrastructure.
///
/// Notice that `TempIndex` locals only exist artificially during translation as
/// placeholders for actual expressions in the context of spec expressions. For historical
/// reasons spec expressions use TempIndex for parameters exclusively, and Symbol for locals.
struct BehaviorExpGenerator<'env> {
    fun_env: FunctionEnv<'env>,
    loc: Loc,
    next_temp: TempIndex,
    temp_types: BTreeMap<TempIndex, Type>,
}

impl<'env> BehaviorExpGenerator<'env> {
    fn new(fun_env: FunctionEnv<'env>, loc: Loc, param_count: usize) -> Self {
        // Start allocating temps after the function's parameters.
        // In spec expressions, only function parameters can be TempIndex (0..param_count-1).
        Self {
            fun_env,
            loc,
            next_temp: param_count,
            temp_types: BTreeMap::new(),
        }
    }
}

impl<'env> ExpGenerator<'env> for BehaviorExpGenerator<'env> {
    fn function_env(&self) -> &FunctionEnv<'env> {
        &self.fun_env
    }

    fn get_current_loc(&self) -> Loc {
        self.loc.clone()
    }

    fn set_loc(&mut self, loc: Loc) {
        self.loc = loc;
    }

    fn add_local(&mut self, ty: Type) -> TempIndex {
        let idx = self.next_temp;
        self.temp_types.insert(idx, ty);
        self.next_temp += 1;
        idx
    }

    fn get_local_type(&self, temp: TempIndex) -> Type {
        self.temp_types.get(&temp).cloned().unwrap_or(Type::Error)
    }
}

// -------------------------------------------------------------------------------------------
// Expressions Conversion

/// The expression converter takes a Move expression and converts it to a
/// specification expression. It expects the expression to be checked to be pure.
struct SpecConverter<'a> {
    env: &'a mut GlobalEnv,
    /// Whether we are in a specification expression. Conversion applies only if.
    in_spec: bool,
    /// The mapping from Move function to spec function ids.
    function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
    /// If non-empty, Temporary expressions should be mapped to symbolic LocalVar
    /// expressions. This is the convention for specification function parameters.
    symbolized_parameters: Vec<Symbol>,
    /// NodeIds which are exempted from stripping references. For compatibility
    /// reasons nodes which fetch temporaries should not be stripped as the reference
    /// info is needed for correct treatment of the `old(..)` expression.
    reference_strip_exempted: BTreeSet<NodeId>,
    /// Set true if the expression contains imperative expressions that currently cannot be translated into a spec function
    contains_imperative_expression: bool,
    /// Set to true when rewriting spec during inlining phase
    for_inline: bool,
    /// The enclosing function for spec blocks (needed for parameter target handling)
    enclosing_fun: Option<QualifiedId<FunId>>,
    /// Cache for generated behavioral predicate spec functions:
    /// (enclosing_fun, kind, param_sym) -> generated spec function id
    behavior_spec_funs: HashMap<(QualifiedId<FunId>, BehaviorKind, Symbol), QualifiedId<SpecFunId>>,
}

impl<'a> SpecConverter<'a> {
    fn new(
        env: &'a mut GlobalEnv,
        function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
        in_spec: bool,
    ) -> Self {
        Self {
            env,
            in_spec,
            function_mapping,
            symbolized_parameters: vec![],
            reference_strip_exempted: Default::default(),
            contains_imperative_expression: false,
            for_inline: false,
            enclosing_fun: None,
            behavior_spec_funs: HashMap::new(),
        }
    }

    fn new_for_inline(
        env: &'a mut GlobalEnv,
        function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
        in_spec: bool,
    ) -> Self {
        Self {
            env,
            in_spec,
            function_mapping,
            symbolized_parameters: vec![],
            reference_strip_exempted: Default::default(),
            contains_imperative_expression: false,
            for_inline: true,
            enclosing_fun: None,
            behavior_spec_funs: HashMap::new(),
        }
    }

    fn symbolized_parameters(self, symbolized_parameters: Vec<Symbol>) -> Self {
        Self {
            symbolized_parameters,
            ..self
        }
    }

    fn with_enclosing_fun(self, fun_id: QualifiedId<FunId>) -> Self {
        Self {
            enclosing_fun: Some(fun_id),
            ..self
        }
    }

    /// Generates an uninterpreted spec function for a behavioral predicate with a
    /// function-typed parameter target. Returns a cached spec function if one was
    /// already generated for this (enclosing_fun, kind, param) combination.
    fn generate_behavior_spec_fun(
        &mut self,
        enclosing_fun: QualifiedId<FunId>,
        kind: BehaviorKind,
        param_sym: Symbol,
        param_type: &Type,
        loc: &Loc,
    ) -> QualifiedId<SpecFunId> {
        // Check cache first
        let cache_key = (enclosing_fun, kind, param_sym);
        if let Some(spec_fun_id) = self.behavior_spec_funs.get(&cache_key) {
            return *spec_fun_id;
        }

        let fun_env = self.env.get_function(enclosing_fun);

        // Build spec function name: $<kind>$<enclosing_fun_name>$<param_name>
        let name = self.env.symbol_pool().make(&format!(
            "${}${}${}",
            kind,
            fun_env.get_name().display(self.env.symbol_pool()),
            param_sym.display(self.env.symbol_pool())
        ));

        // Extract argument types and result types from the function type
        let (fn_arg_types, fn_result_types) = match param_type {
            Type::Fun(arg, result, _) => (
                arg.as_ref().clone().flatten(),
                result.as_ref().clone().flatten(),
            ),
            _ => {
                self.env.diag(
                    Severity::Bug,
                    loc,
                    &format!(
                        "behavioral predicate parameter `{}` has non-function type",
                        param_sym.display(self.env.symbol_pool())
                    ),
                );
                (vec![], vec![])
            },
        };

        // Build parameters for the spec function (function's input arguments only)
        let mut params: Vec<Parameter> = fn_arg_types
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                let arg_name = self.env.symbol_pool().make(&format!("arg{}", i));
                Parameter(arg_name, ty.clone(), loc.clone())
            })
            .collect();

        // For EnsuresOf, add parameters for result values
        if kind == BehaviorKind::EnsuresOf {
            for (i, ty) in fn_result_types.iter().enumerate() {
                let result_name = self.env.symbol_pool().make(&format!("result{}", i));
                params.push(Parameter(result_name, ty.clone(), loc.clone()));
            }
        }

        // Create the uninterpreted spec function declaration
        let decl = SpecFunDecl {
            loc: loc.clone(),
            name,
            type_params: fun_env.get_type_parameters(),
            params,
            context_params: None,
            result_type: Type::Primitive(PrimitiveType::Bool),
            used_memory: BTreeSet::new(),
            uninterpreted: true,
            is_move_fun: false,
            is_native: false,
            body: None,
            callees: BTreeSet::new(),
            is_recursive: RefCell::new(None),
            insts_using_generic_type_reflection: Default::default(),
            spec: RefCell::new(Default::default()),
        };

        // Add to environment
        let spec_fun_id = self
            .env
            .add_spec_function_def(enclosing_fun.module_id, decl);

        // Cache and return
        self.behavior_spec_funs.insert(cache_key, spec_fun_id);
        spec_fun_id
    }

    /// Reduces a behavioral predicate (requires_of, aborts_of, ensures_of, modifies_of)
    /// for a known function target into plain predicates by using SpecTranslator
    /// to properly extract and substitute the function's specification conditions.
    fn reduce_behavior_predicate(
        &mut self,
        id: NodeId,
        kind: BehaviorKind,
        target_fun: &QualifiedInstId<FunId>,
        args: &[Exp],
    ) -> Exp {
        let loc = self.env.get_node_loc(id);

        // Scope the immutable borrow for SpecTranslator
        let (translated, param_temps, ret_temps) = {
            let env = &self.env;
            let target_fun_env = env.get_function(target_fun.to_qualified_id());
            let param_count = target_fun_env.get_parameter_count();
            let type_args = &target_fun.inst;

            // Create minimal ExpGenerator, starting temps after the function's parameters
            let mut generator =
                BehaviorExpGenerator::new(target_fun_env.clone(), loc.clone(), param_count);

            // Allocate temps for parameters (used by param_substitution)
            let param_temps: Vec<TempIndex> = (0..param_count)
                .map(|i| {
                    let ty = if i < args.len() {
                        env.get_node_type(args[i].node_id())
                    } else {
                        Type::Error
                    };
                    generator.add_local(ty)
                })
                .collect();

            // Allocate temps for results (used by ret_locals).
            // We must allocate based on the function's return type, not the args provided,
            // because SpecTranslator translates ALL conditions including ensures which may
            // reference `result` even when we only need requires.
            let ret_temps: Vec<TempIndex> = target_fun_env
                .get_result_type()
                .flatten()
                .iter()
                .map(|ty| generator.add_local(ty.clone()))
                .collect();

            // Translate the spec using SpecTranslator
            let translated = SpecTranslator::translate_fun_spec(
                false, // auto_trace
                true,  // for_call
                &mut generator,
                &target_fun_env,
                type_args,
                Some(&param_temps),
                &ret_temps,
            );

            (translated, param_temps, ret_temps)
        };

        // Build a mapping from saved param temps back to original param temps.
        // SpecTranslator creates saved_params when translating post conditions to
        // preserve pre-state values: saved_params maps param_temp -> saved_temp.
        // We need the reverse: saved_temp -> param_temp, so we can trace back to args.
        let saved_to_param: BTreeMap<TempIndex, TempIndex> = translated
            .saved_params
            .iter()
            .map(|(param, saved)| (*saved, *param))
            .collect();

        // Extract and combine conditions based on behavior kind
        let builder = ExpBuilder::new(self.env);
        match kind {
            BehaviorKind::RequiresOf => {
                let conditions: Vec<Exp> = translated
                    .pre
                    .into_iter()
                    .map(|(_, e)| {
                        Self::substitute_temps_with_args(
                            &e,
                            &param_temps,
                            &ret_temps,
                            &saved_to_param,
                            args,
                        )
                    })
                    .collect();
                builder.and_n(&loc, conditions)
            },
            BehaviorKind::AbortsOf => {
                // Combine all abort conditions from TranslatedSpec
                let abort_conds: Vec<Exp> = translated
                    .aborts
                    .into_iter()
                    .map(|(_, e, _)| {
                        Self::substitute_temps_with_args(
                            &e,
                            &param_temps,
                            &ret_temps,
                            &saved_to_param,
                            args,
                        )
                    })
                    .collect();
                builder.or_n(&loc, abort_conds)
            },
            BehaviorKind::EnsuresOf => {
                let conditions: Vec<Exp> = translated
                    .post
                    .into_iter()
                    .map(|(_, e)| {
                        Self::substitute_temps_with_args(
                            &e,
                            &param_temps,
                            &ret_temps,
                            &saved_to_param,
                            args,
                        )
                    })
                    .collect();
                builder.and_n(&loc, conditions)
            },
            BehaviorKind::ModifiesOf => {
                // Return true for now; proper semantics TBD
                builder.bool_const(&loc, true)
            },
        }
    }

    /// Substitutes temporary references back to the original argument expressions.
    /// SpecTranslator rewrites params to Temporary(param_temps[i]) and results to Temporary(ret_temps[j]).
    /// For post conditions, params may be saved via saved_params (param_temp -> saved_temp).
    /// We need to substitute these back to the original arg expressions.
    fn substitute_temps_with_args(
        exp: &Exp,
        param_temps: &[TempIndex],
        ret_temps: &[TempIndex],
        saved_to_param: &BTreeMap<TempIndex, TempIndex>,
        args: &[Exp],
    ) -> Exp {
        ExpData::rewrite(exp.clone(), &mut |e| {
            if let ExpData::Temporary(_, idx) = e.as_ref() {
                // Check if it's a param temp
                if let Some(pos) = param_temps.iter().position(|t| *t == *idx) {
                    if pos < args.len() {
                        return RewriteResult::Rewritten(args[pos].clone());
                    }
                }
                // Check if it's a result temp
                if let Some(pos) = ret_temps.iter().position(|t| *t == *idx) {
                    let arg_idx = param_temps.len() + pos;
                    if arg_idx < args.len() {
                        return RewriteResult::Rewritten(args[arg_idx].clone());
                    }
                }
                // Check if it's a saved param temp (from post condition translation)
                if let Some(original_param_temp) = saved_to_param.get(idx) {
                    // Find the position of the original param temp
                    if let Some(pos) = param_temps.iter().position(|t| *t == *original_param_temp) {
                        if pos < args.len() {
                            return RewriteResult::Rewritten(args[pos].clone());
                        }
                    }
                }
            }
            RewriteResult::Unchanged(e)
        })
    }
}

impl ExpRewriterFunctions for SpecConverter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        use ExpData::*;
        use Operation::*;
        if !self.in_spec {
            // If not in spec mode, check whether we need to switch to it,
            // and descent
            if matches!(exp.as_ref(), ExpData::SpecBlock(..)) {
                self.in_spec = true;
                let result = self.rewrite_exp_descent(exp);
                self.in_spec = false;
                result
            } else {
                let exp = self.rewrite_exp_descent(exp);
                if self
                    .env
                    .language_version()
                    .is_at_least(LanguageVersion::V2_2)
                    && !self.for_inline
                {
                    if let ExpData::Invoke(id, call, args) = exp.as_ref() {
                        if let ExpData::Call(_, Closure(mid, fid, mask), captured) = call.as_ref() {
                            let mut new_args = vec![];
                            let mut captured_num = 0;
                            let mut free_num = 0;
                            let fun = self.env.get_function(mid.qualified(*fid));
                            for i in 0..fun.get_parameter_count() {
                                if mask.is_captured(i) {
                                    new_args.push(captured[captured_num].clone());
                                    captured_num += 1;
                                } else {
                                    new_args.push(args[free_num].clone());
                                    free_num += 1;
                                }
                            }
                            return Call(*id, MoveFunction(*mid, *fid), new_args.clone())
                                .into_exp();
                        }
                    }
                }
                exp
            }
        } else {
            // Simplification which need to be done before descent
            let exp = match exp.as_ref() {
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort(_), _)) =>
                {
                    // The code pattern produced by an `assert!`: `if (c) () else abort`.
                    // Reduce to unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                Temporary(id, _) | LocalVar(id, _) => {
                    self.reference_strip_exempted.insert(*id);
                    exp
                },
                _ => exp,
            };

            let exp = self.rewrite_exp_descent(exp);

            // Simplification after descent
            match exp.as_ref() {
                Temporary(id, idx) => {
                    // For specification functions, parameters are represented as LocalVar,
                    // so apply mapping if present.
                    if let Some(sym) = self.symbolized_parameters.get(*idx) {
                        LocalVar(*id, *sym).into_exp()
                    } else {
                        exp.clone()
                    }
                },
                Call(id, BorrowGlobal(ReferenceKind::Immutable), args) => {
                    // Map borrow_global to specification global
                    Call(*id, Global(None), args.clone()).into_exp()
                },
                Call(_, Borrow(_), args) | Call(_, Deref, args) => {
                    // Skip local borrow
                    args[0].clone()
                },
                Call(id, MoveFunction(mid, fid), args) => {
                    // During inlining process, we skip replacing move fun with spec fun
                    // Later phase will do it for spec blocks
                    if !self.for_inline {
                        // Rewrite to associated spec function
                        let spec_fun_id = self
                            .function_mapping
                            .get(&mid.qualified(*fid))
                            .unwrap_or_else(|| {
                                panic!(
                                    "associated spec fun for {}",
                                    self.env.get_function(mid.qualified(*fid)).get_name_str()
                                )
                            });

                        Call(
                            *id,
                            SpecFunction(spec_fun_id.module_id, spec_fun_id.id, None),
                            args.clone(),
                        )
                        .into_exp()
                    } else {
                        exp.clone()
                    }
                },
                // Deal with removing various occurrences of Abort and spec blocks
                SpecBlock(id, ..) => {
                    // Replace direct call by unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort(_), _)) =>
                {
                    // The code pattern produced by an `assert!`: `if (c) () else abort`.
                    // Reduce to unit as well
                    Call(*id, Tuple, vec![]).into_exp()
                },
                Sequence(id, exps) => {
                    // Remove aborts, units, and spec blocks
                    let mut reduced_exps = exps
                        .iter()
                        .take(exps.len() - 1)
                        .flat_map(|e| {
                            if matches!(
                                e.as_ref(),
                                SpecBlock(..) | Call(_, Abort(_), _) | Call(_, Tuple, _)
                            ) {
                                None
                            } else {
                                Some(e.clone())
                            }
                        })
                        .collect_vec();
                    reduced_exps.push(exps.last().unwrap().clone());
                    if reduced_exps.len() != exps.len() {
                        if reduced_exps.len() == 1 {
                            reduced_exps.pop().unwrap()
                        } else {
                            self.contains_imperative_expression = true;
                            Sequence(*id, reduced_exps).into_exp()
                        }
                    } else {
                        if reduced_exps.len() != 1 {
                            self.contains_imperative_expression = true;
                        }
                        exp.clone()
                    }
                },
                Invoke(id, call, args) => {
                    // Rewrite invoke into a spec function call
                    // this is for general function value
                    if let ExpData::Call(_, Closure(mid, fid, mask), captured) = call.as_ref() {
                        let spec_fun_id = self
                            .function_mapping
                            .get(&mid.qualified(*fid))
                            .unwrap_or_else(|| {
                                panic!(
                                    "associated spec fun for {}",
                                    self.env.get_function(mid.qualified(*fid)).get_name_str()
                                )
                            });
                        let spec_fun_decl: &SpecFunDecl = self.env.get_spec_fun(*spec_fun_id);
                        let mut new_args = vec![];
                        let mut captured_num = 0;
                        let mut free_num = 0;
                        for i in 0..spec_fun_decl.params.len() {
                            if mask.is_captured(i) {
                                new_args.push(captured[captured_num].clone());
                                captured_num += 1;
                            } else {
                                new_args.push(args[free_num].clone());
                                free_num += 1;
                            }
                        }

                        return Call(
                            *id,
                            SpecFunction(spec_fun_id.module_id, spec_fun_id.id, None),
                            new_args.clone(),
                        )
                        .into_exp();
                    }
                    exp.clone()
                },
                ExpData::Return(..)
                | ExpData::Loop(..)
                | ExpData::Assign(..)
                | ExpData::Mutate(..)
                | ExpData::LoopCont(..) => {
                    self.contains_imperative_expression = true;
                    exp.clone()
                },
                _ => exp.clone(),
            }
        }
    }

    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        if !self.in_spec || self.reference_strip_exempted.contains(&id) {
            // Skip the processing below
            return None;
        }
        if let Some(new_ty) = self
            .env
            .get_node_type_opt(id)
            .map(|ty| ty.skip_reference().clone())
        {
            let new_id = self.env.new_node(self.env.get_node_loc(id), new_ty);
            if let Some(new_inst) = self.env.get_node_instantiation_opt(id).map(|inst| {
                inst.into_iter()
                    .map(|ty| ty.skip_reference().clone())
                    .collect_vec()
            }) {
                self.env.set_node_instantiation(new_id, new_inst);
            }
            Some(new_id)
        } else {
            None
        }
    }

    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if let Operation::Behavior(kind, _pre_label, target, _post_label) = oper {
            match target {
                BehaviorTarget::Function(qid) => {
                    Some(self.reduce_behavior_predicate(id, *kind, qid, args))
                },
                BehaviorTarget::Parameter(sym) => {
                    let loc = self.env.get_node_loc(id);

                    // modifies_of is not supported for parameter targets
                    if *kind == BehaviorKind::ModifiesOf {
                        self.env.error(
                            &loc,
                            "`modifies_of` is not supported for function-typed parameters",
                        );
                        return None;
                    }

                    // Get enclosing function context
                    let enclosing_fun = self.enclosing_fun.or_else(|| {
                        self.env.diag(
                            Severity::Bug,
                            &loc,
                            "behavioral predicate with parameter target requires \
                             enclosing function context",
                        );
                        None
                    })?;
                    let fun_env = self.env.get_function(enclosing_fun);
                    let param_type = fun_env
                        .get_parameters()
                        .iter()
                        .find(|p| p.0 == *sym)
                        .map(|p| p.1.clone())
                        .expect("parameter not found");

                    // Generate or retrieve the uninterpreted spec function
                    let spec_fun_id = self.generate_behavior_spec_fun(
                        enclosing_fun,
                        *kind,
                        *sym,
                        &param_type,
                        &loc,
                    );

                    // Build call: $kind$fun$param(arg1, arg2, ...)
                    // The function parameter is encoded in the spec function name, not passed as arg
                    let call_args: Vec<Exp> = args.to_vec();

                    // Create node for the spec function call
                    let result_type = Type::Primitive(PrimitiveType::Bool);
                    let call_node_id = self.env.new_node(loc, result_type);
                    // Copy type instantiation from original node for generic functions
                    self.env
                        .set_node_instantiation(call_node_id, self.env.get_node_instantiation(id));

                    Some(
                        ExpData::Call(
                            call_node_id,
                            Operation::SpecFunction(spec_fun_id.module_id, spec_fun_id.id, None),
                            call_args,
                        )
                        .into_exp(),
                    )
                },
            }
        } else {
            None
        }
    }
}

// -------------------------------------------------------------------------------------------
// Processing Invariants

fn collect_global_invariants_to_env(env: &mut GlobalEnv) {
    let mut invariants = vec![];
    for module_env in env.get_modules() {
        for cond in &module_env.get_spec().conditions {
            if matches!(
                cond.kind,
                ConditionKind::GlobalInvariant(..) | ConditionKind::GlobalInvariantUpdate(..)
            ) {
                let id = env.new_global_id();
                invariants.push(GlobalInvariant {
                    id,
                    loc: cond.loc.clone(),
                    kind: cond.kind.clone(),
                    mem_usage: cond
                        .exp
                        .used_memory(env)
                        .into_iter()
                        .map(|(mem, _)| mem.clone())
                        .collect(),
                    declaring_module: module_env.get_id(),
                    cond: cond.exp.clone(),
                    properties: cond.properties.clone(),
                });
            }
        }
    }
    for invariant in invariants {
        env.add_global_invariant(invariant)
    }
}

fn check_data_invariants(struct_env: &StructEnv) {
    let env = struct_env.module_env.env;
    for cond in &struct_env.get_spec().conditions {
        if matches!(cond.kind, ConditionKind::StructInvariant) {
            let usage = cond.exp.used_memory(env);
            if !usage.is_empty() {
                env.error(
                    &cond.loc,
                    &format!(
                        "data invariants cannot depend on global state \
                    but found dependency to `{}`",
                        usage
                            .into_iter()
                            .map(|(sid, _)| env.display(&sid).to_string())
                            .join(",")
                    ),
                )
            }
        }
    }
}
