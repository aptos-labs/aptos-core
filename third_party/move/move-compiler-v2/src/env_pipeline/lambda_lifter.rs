// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements lambda lifting, rewriting function definitions
//! in the global environment.
//!
//! Lambda lifting is currently restricted to only lift lambdas which do
//! not modify free variables.
//!
//! Lambda lifting rewrites lambda expressions into construction
//! of *closures* using the `Closure` operation. A closure refers to a function and contains a list
//! of captured arguments for that function, essentially currying it. Example
//!
//!
//! ```ignore
//! let c = 1;
//! vec.any(|x| x > c)
//! ==>
//! let c = 1;
//! vec.any(Closure(lifted, [c]))
//! where
//!   fun lifted(c: u64, x: u64): bool { x > c }
//! ```
//!
//! The lambda lifting in this module also takes care of patterns as lambda arguments.
//! Example:
//!
//! ```ignore
//! let c = 1;
//! vec.any(|S{x}| x > c)
//! ==>
//! let c = 1;
//! vec.any(Closure(lifted, c))
//! where
//!   fun lifted(c: u64, arg$2: S): bool { let S{x} = arg$2; x > y }
//! ```
//!
//! If possible, the code in this module attempts to curry existing functions instead of
//! introducing new ones via lambda lifting, utilizing `ClosureMask`. For example, a lambda
//! like `|x| f(c, x)` can be represented as `Closure(f, mask(0b01), c)`, whereas
//! `|x| f(x, c)` can be represented as `Closure(f, mask(0b10), c)`.

use itertools::Itertools;
use move_binary_format::file_format::Visibility;
use move_core_types::function::ClosureMask;
use move_model::{
    ast::{Exp, ExpData, LambdaCaptureKind, Operation, Pattern, Spec, TempIndex},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{FunId, FunctionData, FunctionEnv, GlobalEnv, Loc, NodeId, Parameter, TypeParameter},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

/// Marker used in name of a function resulting from lambda lifting.
/// Currently, the identifier core infra does not allow special characters in VM and
/// elsewhere, so this can create name clashes,  On the other hand, they appear very
/// unlikely, and cannot do harm besides a bytecode verification error.
const LIFTED_FUN_MARKER: &str = "__lambda__";

/// Returns true if this is a function resulting from lambda lifting.
pub fn is_lambda_lifted_fun(fun_env: &FunctionEnv) -> bool {
    fun_env
        .symbol_pool()
        .string(fun_env.get_name())
        .contains(LIFTED_FUN_MARKER)
}

#[derive(Debug, Clone, Default)]
pub struct LambdaLiftingOptions {
    /// Whether to include inline functions, both definitions and arguments of calls.
    pub include_inline_functions: bool,
}

/// Performs lambda lifting for all target modules in the environment.
pub fn lift_lambdas(options: LambdaLiftingOptions, env: &mut GlobalEnv) {
    // Go over target modules and transitive closures one by one.
    // Since in each iteration we need to mutate the module, iterate over a vector of plain ids.
    for module_id in env
        .get_target_modules_transitive_closure()
        .into_iter()
        .map(|me| me.get_id())
        .collect_vec()
    {
        let module = env.get_module(module_id);
        let mut updated_funs = BTreeMap::new();
        let mut new_funs = vec![];
        for fun in module.get_functions() {
            if fun.is_inline() && !options.include_inline_functions || fun.is_native_or_intrinsic()
            {
                continue;
            }
            let def = fun.get_def().expect("has definition");
            let mut lifter = LambdaLifter::new(&options, &fun, None);
            let new_def = lifter.rewrite_exp(def.clone());
            if def != &new_def {
                new_funs.append(&mut lifter.lifted);
                updated_funs.insert(fun.get_id(), new_def);
            }
        }
        // Now that we have processed all functions and released
        // all references to the env, mutate it.
        for (fun_id, new_def) in updated_funs {
            env.set_function_def(module_id.qualified(fun_id), new_def)
        }
        for ClosureFunction {
            loc,
            fun_id,
            type_params,
            params,
            result_type,
            def,
            spec,
        } in new_funs
        {
            env.add_function_def(
                module_id,
                fun_id.symbol(),
                loc,
                Visibility::Private,
                false,
                type_params,
                params,
                result_type,
                def,
                spec,
            )
        }
    }
}

/// Structure which is used to rewrite one function definition,
/// using the `ExpRewriterFunctions` trait. During
/// traversal of an expression, on ascent all the used but
/// so far unbound parameters and locals are found here.
/// These are the ones which need to be captured in a closure.
pub struct LambdaLifter<'a> {
    /// The options for lambda lifting.
    options: &'a LambdaLiftingOptions,
    /// Function being processed.
    fun_env: &'a FunctionEnv<'a>,
    /// The generated closure functions.
    lifted: Vec<ClosureFunction>,
    /// Local name scopes.
    scopes: Vec<BTreeSet<Symbol>>,
    /// Any unbound parameters names used so far.
    free_params: BTreeMap<TempIndex, VarInfo>,
    /// Any unbound locals used so far.
    free_locals: BTreeMap<Symbol, VarInfo>,
    /// NodeId's of lambdas which are parameters to inline functions
    /// and should be exempted from lifting. Pushed down during descend.
    exempted_lambdas: BTreeSet<NodeId>,
    /// Optional suffix to attach in the function name
    name_suffix: Option<String>,
}

struct VarInfo {
    /// The node were this variable was found.
    node_id: NodeId,
    /// Whether the variable is modified
    modified: bool,
}

/// A new function to be created in the global env.
pub struct ClosureFunction {
    loc: Loc,
    fun_id: FunId,
    type_params: Vec<TypeParameter>,
    params: Vec<Parameter>,
    result_type: Type,
    def: Exp,
    spec: Option<Spec>,
}

impl ClosureFunction {
    pub fn generate_function_data(&self, env: &GlobalEnv) -> FunctionData {
        env.construct_function_data(
            self.fun_id.symbol(),
            self.loc.clone(),
            Visibility::Private,
            false,
            self.type_params.clone(),
            self.params.clone(),
            self.result_type.clone(),
            self.def.clone(),
            self.spec.clone(),
        )
    }
}

impl<'a> LambdaLifter<'a> {
    pub fn new(
        options: &'a LambdaLiftingOptions,
        fun_env: &'a FunctionEnv,
        name_suffix: Option<String>,
    ) -> Self {
        LambdaLifter {
            options,
            fun_env,
            lifted: vec![],
            scopes: vec![],
            free_params: BTreeMap::default(),
            free_locals: BTreeMap::default(),
            exempted_lambdas: BTreeSet::default(),
            name_suffix,
        }
    }

    pub fn lifted_len(&self) -> usize {
        self.lifted.len()
    }

    pub fn get_lifted_at(&self, i: usize) -> Option<&ClosureFunction> {
        if self.lifted_len() > i {
            Some(&self.lifted[i])
        } else {
            None
        }
    }

    fn gen_parameter_name(&self, parameter_pos: usize) -> Symbol {
        self.fun_env
            .module_env
            .env
            .symbol_pool()
            .make(&format!("param${}", parameter_pos))
    }

    fn gen_closure_function_name(&mut self) -> Symbol {
        let env = self.fun_env.module_env.env;
        env.symbol_pool().make(&format!(
            "{}{}{}__{}",
            LIFTED_FUN_MARKER,
            self.lifted.len() + 1,
            self.name_suffix.clone().unwrap_or("".to_string()),
            self.fun_env.get_name().display(env.symbol_pool()),
        ))
    }

    fn bind(&self, mut bindings: Vec<(Pattern, Exp)>, exp: Exp) -> Exp {
        if let Some((pat, binding)) = bindings.pop() {
            let env = self.fun_env.module_env.env;
            let body = self.bind(bindings, exp);
            let loc = env.get_node_loc(pat.node_id());
            let body_id = body.node_id();
            let ty = env.get_node_type(body_id);
            let new_id = env.new_node(loc, ty);
            if let Some(inst) = env.get_node_instantiation_opt(body_id) {
                env.set_node_instantiation(new_id, inst);
            }
            ExpData::Block(new_id, pat, Some(binding), body).into_exp()
        } else {
            exp
        }
    }

    /// For the current state, calculate: (params, closure_args, param_index_mapping), where
    /// - `params` = new Parameter for each free var to represent it in the lifted function
    /// - `closure_args` = corresponding expressions to provide as actual arg for each param
    /// - `param_index_mapping` = for each free var which is a Parameter from the enclosing function,
    ///    a mapping from index there to index in the params list
    fn get_params_for_freevars(
        &mut self,
    ) -> Option<(Vec<Parameter>, Vec<Exp>, BTreeMap<usize, usize>)> {
        let env = self.fun_env.module_env.env;
        let mut closure_args = vec![];

        // Add captured parameters. We also need to record a mapping of
        // parameter indices in the lambda context to indices in the lifted
        // functions (courtesy of #12317)
        let mut param_index_mapping = BTreeMap::new();
        let mut params = vec![];
        let mut saw_error = false;

        for (used_param_count, (param, var_info)) in self.free_params.iter().enumerate() {
            let name = self.fun_env.get_local_name(*param);
            let var_node_id = var_info.node_id;
            let ty = env.get_node_type(var_node_id);
            let loc = env.get_node_loc(var_node_id);
            if var_info.modified {
                env.error(
                    &loc,
                    &format!(
                        "captured variable `{}` cannot be modified inside of a lambda",
                        name.display(env.symbol_pool())
                    ),
                );
                saw_error = true;
            }
            params.push(Parameter(name, ty.clone(), loc.clone()));
            let new_id = env.new_node(loc, ty);
            if let Some(inst) = env.get_node_instantiation_opt(var_node_id) {
                env.set_node_instantiation(new_id, inst);
            }
            closure_args.push(ExpData::Temporary(new_id, *param).into_exp());
            param_index_mapping.insert(*param, used_param_count);
        }

        // Add captured LocalVar parameters
        for (name, var_info) in self.free_locals.iter() {
            let var_info_id = var_info.node_id;
            let ty = env.get_node_type(var_info_id);
            let loc = env.get_node_loc(var_info_id);
            if var_info.modified {
                env.error(
                    &loc,
                    &format!(
                        "captured variable `{}` cannot be modified inside of a lambda", // TODO(LAMBDA)
                        name.display(env.symbol_pool())
                    ),
                );
                saw_error = true;
            }
            params.push(Parameter(*name, ty.clone(), loc.clone()));
            let new_id = env.new_node(loc, ty);
            if let Some(inst) = env.get_node_instantiation_opt(var_info_id) {
                env.set_node_instantiation(new_id, inst);
            }
            closure_args.push(ExpData::LocalVar(new_id, *name).into_exp())
        }

        if !saw_error {
            Some((params, closure_args, param_index_mapping))
        } else {
            None
        }
    }

    /// Attempts to rewrite a lambda by currying existing function instead of performing
    /// lambda lifting. See module documentation for background.
    fn try_to_reduce_lambda_to_curry(
        &self,
        id: NodeId,
        lambda_params: &[Parameter],
        body: &Exp,
    ) -> Option<Exp> {
        use ExpData::*;
        match body.as_ref() {
            Call(call_id, oper, args) => {
                match oper {
                    Operation::Closure(..) => {
                        // We might be able to do something with this,
                        // but skip for now because it will be complicated.
                        None
                    },
                    Operation::MoveFunction(mid, fid) => {
                        let lambda_bound = lambda_params
                            .iter()
                            .map(|Parameter(name, ..)| *name)
                            .collect::<BTreeSet<_>>();
                        let mut lambda_param_pos = 0;
                        let mut captured = vec![];
                        let mut mask = ClosureMask::new(0);
                        for (pos, arg) in args.iter().enumerate() {
                            if Self::exp_is_capturable(arg)
                                && arg.free_vars().is_disjoint(&lambda_bound)
                            {
                                // We can capture an argument if it can be eagerly evaluated and if
                                // it does not depend on lambda arguments.
                                if pos >= ClosureMask::MAX_ARGS {
                                    // Exceeded maximal number of arguments which can be captured
                                    return None;
                                }
                                captured.push(arg.clone());
                                mask.set_captured(pos);
                            } else if lambda_param_pos < lambda_params.len()
                                && matches!(arg.as_ref(), LocalVar(_, name) if name == &lambda_params[lambda_param_pos].0)
                            {
                                // We can curry an argument if it directly refers to the next lambda
                                // parameter.
                                lambda_param_pos += 1
                            } else {
                                return None;
                            }
                        }
                        // We must have curried all arguments to the lambda
                        if lambda_param_pos < lambda_params.len() {
                            return None;
                        }
                        // Create a new node id. We inherit location and type from the lambda,
                        // but instantiation is taken from the call of the curried function.
                        let env = self.fun_env.module_env.env;
                        let curry_id = env.new_node(env.get_node_loc(id), env.get_node_type(id));
                        if let Some(inst) = env.get_node_instantiation_opt(*call_id) {
                            env.set_node_instantiation(curry_id, inst)
                        }
                        Some(
                            Call(curry_id, Operation::Closure(*mid, *fid, mask), captured)
                                .into_exp(),
                        )
                    },
                    _ => None,
                }
            },
            Sequence(_id, exp_vec) => {
                if let [exp] = &exp_vec[..] {
                    self.try_to_reduce_lambda_to_curry(id, lambda_params, exp)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    // Only allow expressions to be captured which cannot abort, since we are pulling
    // them out of the lambda expression.
    fn exp_is_capturable(exp: &Exp) -> bool {
        use ExpData::*;
        match exp.as_ref() {
            Call(_, op, args) => {
                op.is_ok_to_remove_from_code() && args.iter().all(Self::exp_is_capturable)
            },
            Sequence(_, exp_vec) => {
                if let [exp] = &exp_vec[..] {
                    Self::exp_is_capturable(exp)
                } else {
                    false
                }
            },
            IfElse(_, e1, e2, e3) => {
                Self::exp_is_capturable(e1)
                    && Self::exp_is_capturable(e2)
                    && Self::exp_is_capturable(e3)
            },
            Lambda(_, _pat, _body, _capture_kind, _spec_opt) => {
                // Maybe could test lambda_is_direct_curry(pat, body)
                // and do something with it, but it is nontrivial.
                false
            },
            LocalVar(..) | Temporary(..) | Value(..) => true,
            Invalid(..) | Invoke(..) | Quant(..) | Block(..) | Match(..) | Return(..)
            | Loop(..) | LoopCont(..) | Assign(..) | Mutate(..) | SpecBlock(..) => false,
        }
    }
}

impl ExpRewriterFunctions for LambdaLifter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        // Intercept descent and compute lambdas being exempted from lifting, currently
        // those passed as parameters to inline functions.
        if !self.options.include_inline_functions {
            if let ExpData::Call(_, Operation::MoveFunction(mid, fid), args) = exp.as_ref() {
                let env = self.fun_env.module_env.env;
                if env.get_function(mid.qualified(*fid)).is_inline() {
                    for arg in args {
                        if matches!(arg.as_ref(), ExpData::Lambda(..)) {
                            self.exempted_lambdas.insert(arg.node_id());
                        }
                    }
                }
            }
        }
        // Also if this is a lambda, before descent, clear any usages from siblings in the
        // context, so we get the isolated usage information for the lambda's body.
        if matches!(exp.as_ref(), ExpData::Lambda(..)) {
            let mut curr_free_params = mem::take(&mut self.free_params);
            let mut curr_free_locals = mem::take(&mut self.free_locals);
            let result = self.rewrite_exp_descent(exp);
            self.free_params.append(&mut curr_free_params);
            self.free_locals.append(&mut curr_free_locals);
            result
        } else {
            self.rewrite_exp_descent(exp)
        }
    }

    fn rewrite_enter_scope<'b>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
    ) {
        self.scopes
            .push(vars.map(|(_, sym)| sym).cloned().collect())
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        let exiting = self.scopes.pop().expect("stack balanced");
        // Remove all locals which are bound in the scope we are exiting.
        self.free_locals.retain(|name, _| !exiting.contains(name));
    }

    fn rewrite_local_var(&mut self, node_id: NodeId, sym: Symbol) -> Option<Exp> {
        // duplicates are OK -- they are all the same local at different locations
        self.free_locals.entry(sym).or_insert(VarInfo {
            node_id,
            modified: false,
        });
        None
    }

    fn rewrite_temporary(&mut self, node_id: NodeId, idx: TempIndex) -> Option<Exp> {
        // duplicates are OK -- they are all the same parameter at different locations
        self.free_params.entry(idx).or_insert(VarInfo {
            node_id,
            modified: false,
        });
        None
    }

    fn rewrite_assign(&mut self, _node_id: NodeId, lhs: &Pattern, _rhs: &Exp) -> Option<Exp> {
        for (node_id, name) in lhs.vars() {
            self.free_locals.insert(name, VarInfo {
                node_id,
                modified: true,
            });
        }
        None
    }

    fn rewrite_call(&mut self, _node_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if matches!(oper, Operation::Borrow(ReferenceKind::Mutable)) {
            match args[0].as_ref() {
                ExpData::LocalVar(node_id, name) => {
                    self.free_locals.insert(*name, VarInfo {
                        node_id: *node_id,
                        modified: true,
                    });
                },
                ExpData::Temporary(node_id, param) => {
                    self.free_params.insert(*param, VarInfo {
                        node_id: *node_id,
                        modified: true,
                    });
                },
                _ => {},
            }
        }
        None
    }

    fn rewrite_lambda(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        body: &Exp,
        capture_kind: LambdaCaptureKind,
        spec_opt: &Option<Exp>,
    ) -> Option<Exp> {
        if self.exempted_lambdas.contains(&id) {
            return None;
        }
        let env = self.fun_env.module_env.env;
        let module_id = self.fun_env.module_env.get_id();

        match capture_kind {
            LambdaCaptureKind::Default => {
                // OK.
            },
            LambdaCaptureKind::Move | LambdaCaptureKind::Copy => {
                let loc = env.get_node_loc(id);
                env.error(
                    &loc,
                    &format!(
                        "capture kind for lambdas not supported in language version {}",
                        env.language_version()
                    ),
                );
                return None;
            },
        };

        // params = new Parameter for each free var to represent it in the lifted function
        // closure_args = corresponding expressions to provide as actual arg for each param
        // param_index_mapping = for each free var which is a Parameter from the enclosing function,
        //      a mapping from index there to index in the params list; other free vars are
        //      substituted automatically by using the same symbol for the param
        let (mut params, closure_args, param_index_mapping) = self.get_params_for_freevars()?;

        if closure_args.len() > ClosureMask::MAX_ARGS {
            env.error(
                &env.get_node_loc(id),
                &format!(
                    "too many arguments captured in lambda (can only capture up to a maximum of `{}`, but captured `{}`)",
                    ClosureMask::MAX_ARGS, closure_args.len()
                ),
            );
            return None;
        }

        // Add lambda args. For dealing with patterns in lambdas (`|S{..}|e`) we need
        // to collect a list of bindings.
        let mut bindings = vec![];
        let mut lambda_params = vec![];
        for (i, arg) in pat.clone().flatten().into_iter().enumerate() {
            let id = arg.node_id();
            let ty = env.get_node_type(id);
            let loc = env.get_node_loc(id);
            if let Pattern::Var(_, name) = arg {
                lambda_params.push(Parameter(name, ty, loc));
            } else {
                let name = self.gen_parameter_name(i);
                lambda_params.push(Parameter(name, ty.clone(), loc.clone()));
                let new_id = env.new_node(loc, ty);
                if let Some(inst) = env.get_node_instantiation_opt(id) {
                    env.set_node_instantiation(new_id, inst);
                }
                bindings.push((arg.clone(), ExpData::LocalVar(new_id, name).into_exp()))
            }
        }

        // Try to rewrite a lambda directly into a curry expression
        if bindings.is_empty() {
            let possible_curry_exp = self.try_to_reduce_lambda_to_curry(id, &lambda_params, body);
            if possible_curry_exp.is_some() {
                return possible_curry_exp;
            }
        }

        // Following code assumes params include lambda_params
        params.append(&mut lambda_params);

        // Add new closure function
        let fun_name = self.gen_closure_function_name();
        let lambda_loc = env.get_node_loc(id).clone();
        let lambda_type = env.get_node_type(id);
        let result_type = match &lambda_type {
            Type::Fun(_, result_type, _) => result_type.as_ref().clone(),
            Type::Struct(mid, sid, inst) => {
                let senv = env.get_struct(mid.qualified(*sid));
                if let Some(Type::Fun(_, result_type, _)) = senv.get_function_wrapper_type(inst) {
                    *result_type
                } else {
                    Type::Error // type error reported
                }
            },
            _ => Type::Error, // type error reported
        };

        // Rewrite references to Temporary in the new functions body (#12317)
        let mut replacer = |id: NodeId, target: RewriteTarget| {
            if let RewriteTarget::Temporary(temp) = target {
                let new_temp = param_index_mapping.get(&temp).cloned().unwrap_or(temp);
                return Some(ExpData::Temporary(id, new_temp).into_exp());
            }
            None
        };
        let body = ExpRewriter::new(env, &mut replacer).rewrite_exp(body.clone());
        let fun_id = FunId::new(fun_name);
        // Spec rewriter needs to map parameters to temporary indices in the spec
        let mut spec_replacer = |id: NodeId, target: RewriteTarget| {
            if let RewriteTarget::Temporary(temp) = target {
                let new_temp = param_index_mapping.get(&temp).cloned().unwrap_or(temp);
                return Some(ExpData::Temporary(id, new_temp).into_exp());
            } else if let RewriteTarget::LocalVar(sym) = target {
                for (i, par) in params.iter().enumerate() {
                    if sym == par.0 {
                        return Some(ExpData::Temporary(id, i).into_exp());
                    }
                }
            }
            None
        };
        let spec = if let Some(spec_exp) = spec_opt {
            if let ExpData::SpecBlock(_, _) = spec_exp.as_ref() {
                let new_spec_exp =
                    ExpRewriter::new(env, &mut spec_replacer).rewrite_exp(spec_exp.clone());
                if let ExpData::SpecBlock(_, new_spec) = new_spec_exp.as_ref() {
                    Some(new_spec.clone())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        self.lifted.push(ClosureFunction {
            loc: lambda_loc.clone(),
            fun_id,
            type_params: self.fun_env.get_type_parameters(),
            params,
            result_type: result_type.clone(),
            def: self.bind(bindings, body),
            spec,
        });

        // Create and return closure expression. The type instantiation
        // of the closure expression is by the type parameters of the enclosing
        // function.
        let id = env.new_node(lambda_loc, lambda_type);
        let type_param_count = self.fun_env.get_type_parameter_count();
        if type_param_count > 0 {
            env.set_node_instantiation(
                id,
                (0..type_param_count)
                    .map(|i| Type::TypeParameter(i as u16))
                    .collect(),
            )
        }
        Some(
            ExpData::Call(
                id,
                Operation::Closure(
                    module_id,
                    fun_id,
                    ClosureMask::new_for_leading(closure_args.len()),
                ),
                closure_args,
            )
            .into_exp(),
        )
    }
}
