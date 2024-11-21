// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements lambda lifting, rewriting function definitions
//! in the global environment.
//!
//! Currently, lambda lifting is performed only in selected cases:
//!
//! - Inside of specification expressions;
//! - For a lambda argument of a regular (not inline) function
//!
//! Also, lambda lifting is currently restricted to only lift lambdas which do
//! not modify free variables.
//!
//! Lambda lifting rewrites lambda expressions into construction
//! of *closures*. A closure refers to a function and contains a partial list
//! of arguments for that function, essentially currying it.  We use the
//! `Bind` operation to construct a closure from a function and set of arguemnts,
//! qualified by a mask which allows early-bound arguments to be anywhere in
//! the function argument list.
//!
//! ```ignore
//! let c = 1;
//! vec.any(|x| x > c)
//! ==>
//! let c = 1;
//! vec.any(Bind(0x10u64)(lifted, c))
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
//! vec.any(Bind(0x10)(lifted, c))
//! where
//!   fun lifted(c: u64, arg$2: S): bool { let S{x} = arg$2; x > y }
//! ```

use itertools::Itertools;
use move_binary_format::file_format::{AbilitySet, Visibility};
use move_model::{
    ast::{self, Exp, ExpData, LambdaCaptureKind, Operation, Pattern, TempIndex},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, Parameter, TypeParameter},
    symbol::Symbol,
    ty::{AbilityInference, AbilityInferer, ReferenceKind, Type},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

#[derive(Debug, Clone, Default)]
pub struct LambdaLiftingOptions {
    /// Whether to include inline functions, both definitions and arguments of calls.
    pub include_inline_functions: bool,
}

/// Performs lambda lifting for all target modules in the environment.
pub fn lift_lambdas(options: LambdaLiftingOptions, env: &mut GlobalEnv) {
    // Go over target modules one by one. Since in each iteration
    // we need to mutate the module, iterate over a vector of plain ids.
    for module_id in env
        .get_target_modules()
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
            let mut lifter = LambdaLifter {
                options: &options,
                fun_env: &fun,
                lifted: vec![],
                scopes: vec![],
                free_params: BTreeMap::default(),
                free_locals: BTreeMap::default(),
                exempted_lambdas: BTreeSet::default(),
            };
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
            )
        }
    }
}

/// Structure which is used to rewrite one function definition,
/// using the `ExpRewriterFunctions` trait. During
/// traversal of an expression, on ascent all the used but
/// so far unbound parameters and locals are found here.
/// These are the ones which need to be captured in a closure.
struct LambdaLifter<'a> {
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
}

struct VarInfo {
    /// The node were this variable was found.
    node_id: NodeId,
    /// Whether the variable is modified
    modified: bool,
}

/// A new function to be created in the global env.
struct ClosureFunction {
    loc: Loc,
    fun_id: FunId,
    type_params: Vec<TypeParameter>,
    params: Vec<Parameter>,
    result_type: Type,
    def: Exp,
}

impl<'a> LambdaLifter<'a> {
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
            "{}$lambda${}",
            self.fun_env.get_name().display(env.symbol_pool()),
            self.lifted.len() + 1
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
    fn get_params_for_freevars(&mut self) -> (Vec<Parameter>, Vec<Exp>, BTreeMap<usize, usize>) {
        let env = self.fun_env.module_env.env;
        let mut closure_args = vec![];

        // Add captured parameters. We also need to record a mapping of
        // parameter indices in the lambda context to indices in the lifted
        // functions (courtesy of #12317)
        let mut param_index_mapping = BTreeMap::new();
        let mut params = vec![];

        for (used_param_count, (param, var_info)) in
            mem::take(&mut self.free_params).into_iter().enumerate()
        {
            let name = self.fun_env.get_local_name(param);
            let var_node_id = var_info.node_id;
            let ty = env.get_node_type(var_node_id);
            let loc = env.get_node_loc(var_node_id);
            if var_info.modified {
                env.error(
                    &loc,
                    &format!(
                        "captured variable `{}` cannot be modified inside of a lambda", // TODO(LAMBDA)
                        name.display(env.symbol_pool())
                    ),
                );
            }
            params.push(Parameter(name, ty.clone(), loc.clone()));
            let new_id = env.new_node(loc, ty);
            if let Some(inst) = env.get_node_instantiation_opt(var_node_id) {
                env.set_node_instantiation(new_id, inst);
            }
            closure_args.push(ExpData::Temporary(new_id, param).into_exp());
            param_index_mapping.insert(param, used_param_count);
        }

        // Add captured LocalVar parameters
        for (name, var_info) in mem::take(&mut self.free_locals) {
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
            }
            params.push(Parameter(name, ty.clone(), loc.clone()));
            let new_id = env.new_node(loc, ty);
            if let Some(inst) = env.get_node_instantiation_opt(var_info_id) {
                env.set_node_instantiation(new_id, inst);
            }
            closure_args.push(ExpData::LocalVar(new_id, name).into_exp())
        }

        (params, closure_args, param_index_mapping)
    }

    fn get_arg_if_simple(arg: &Exp) -> Option<&Exp> {
        use ExpData::*;
        match &arg.as_ref() {
            Value(..) | LocalVar(..) | Temporary(..) => Some(arg),
            Sequence(_, exp_vec) => {
                if let [exp] = &exp_vec[..] {
                    Self::get_arg_if_simple(exp)
                } else {
                    None
                }
            },
            Invalid(..) | Call(..) | Invoke(..) | Lambda(..) | Quant(..) | Block(..)
            | IfElse(..) | Match(..) | Return(..) | Loop(..) | LoopCont(..) | Assign(..)
            | Mutate(..) | SpecBlock(..) => None,
        }
    }

    fn get_args_if_simple(args: &[Exp]) -> Option<Vec<&Exp>> {
        let result: Vec<&Exp> = args
            .iter()
            .filter_map(|exp| Self::get_arg_if_simple(exp))
            .collect();
        if result.len() == args.len() && result.len() <= 64 {
            // Curry/Bind bitmask is only a u64, so we limit to 64 args here.
            Some(result)
        } else {
            None
        }
    }

    fn get_fun_if_simple(&self, lambda_params: &Vec<Parameter>, fn_exp: &Exp) -> Option<Exp> {
        use ExpData::*;
        match fn_exp.as_ref() {
            Value(_, ast::Value::Function(..)) => Some(fn_exp.clone()),
            Call(id, Operation::Bind(mask), args) => {
                let mut args_iter = args.iter();
                args_iter.next().and_then(|fn_exp| {
                    let fn_args: Vec<_> = args_iter.cloned().collect();
                    Self::get_args_if_simple(&fn_args).map(|simp_args| {
                        let mut bind_args: Vec<_> =
                            simp_args.iter().map(|expref| (*expref).clone()).collect();
                        bind_args.insert(0, fn_exp.clone());
                        ExpData::Call(*id, Operation::Bind(*mask), bind_args).into_exp()
                    })
                })
            },
            Sequence(_, exp_vec) => {
                if let [exp] = &exp_vec[..] {
                    self.get_fun_if_simple(lambda_params, exp)
                } else {
                    None
                }
            },
            Lambda(_, _pat, _body, _capture_kind, _abilities) => {
                // maybe could test lambda_is_direct_curry(pat, body)
                // and do something with it, but it is nontrivial.
                None
            },
            LocalVar(_, name) => {
                // Make sure `name` is a free var, not a lambda param.
                if lambda_params.iter().any(|param| param.get_name() == *name) {
                    None
                } else {
                    Some(fn_exp.clone())
                }
            },
            Temporary(..) => {
                // Lambda doesn't bind temps, must be a free var
                Some(fn_exp.clone())
            },
            Value(..) | Invalid(..) | Call(..) | Invoke(..) | Quant(..) | Block(..)
            | IfElse(..) | Match(..) | Return(..) | Loop(..) | LoopCont(..) | Assign(..)
            | Mutate(..) | SpecBlock(..) => None,
        }
    }

    fn make_move_fn_exp(
        &mut self,
        loc: Loc,
        fn_type: Type,
        module_id: ModuleId,
        fun_id: FunId,
        instantiation: Option<Vec<Type>>,
    ) -> Exp {
        let env = self.fun_env.module_env.env;
        let id = env.new_node(loc, fn_type);
        if let Some(inst) = instantiation {
            env.set_node_instantiation(id, inst);
        }
        let fn_exp = ExpData::Value(id, ast::Value::Function(module_id, fun_id));
        fn_exp.into_exp()
    }

    fn get_move_fn_type(&mut self, expr_id: NodeId, module_id: ModuleId, fun_id: FunId) -> Type {
        let env = self.fun_env.module_env.env;
        let fn_env = env.get_function(module_id.qualified(fun_id));
        let fun_abilities = if fn_env.visibility().is_public() {
            AbilitySet::PUBLIC_FUNCTIONS
        } else {
            AbilitySet::PRIVATE_FUNCTIONS
        };
        let params = fn_env.get_parameters_ref();
        let param_types = params.iter().map(|param| param.get_type()).collect();
        let node_instantiation = env.get_node_instantiation(expr_id);
        let result_type = fn_env.get_result_type();
        Type::Fun(
            Box::new(Type::Tuple(param_types)),
            Box::new(result_type),
            fun_abilities,
        )
        .instantiate(&node_instantiation)
    }

    // If body is a function call expression with the function value and each parameter a
    // simple expression (constant, var, or Move function name) then returns expressions for
    // the function and the arguments.  Otherwise, returns `None`.
    fn lambda_reduces_to_curry<'b>(
        &mut self,
        lambda_params: &Vec<Parameter>,
        body: &'b Exp,
    ) -> Option<(Exp, Vec<&'b Exp>)> {
        use ExpData::*;
        let env = self.fun_env.module_env.env;
        match body.as_ref() {
            Call(id, oper, args) => {
                match oper {
                    Operation::Bind(_mask) => {
                        // TODO(LAMBDA): We may be able to do something with this,
                        // but skip for now because it will be complicated.
                        None
                    },
                    Operation::MoveFunction(mid, fid) => {
                        Self::get_args_if_simple(args).map(|args| {
                            let fn_type = self.get_move_fn_type(*id, *mid, *fid);
                            let loc = env.get_node_loc(*id);
                            let fn_exp = self.make_move_fn_exp(
                                loc,
                                fn_type,
                                *mid,
                                *fid,
                                env.get_node_instantiation_opt(*id),
                            );
                            (fn_exp, args)
                        })
                    },
                    _ => None,
                }
            },
            Invoke(_id, fn_exp, args) => Self::get_args_if_simple(args).and_then(|args| {
                self.get_fun_if_simple(lambda_params, fn_exp)
                    .map(|exp| (exp, args))
            }),
            ExpData::Sequence(_id, exp_vec) => {
                if let [exp] = &exp_vec[..] {
                    self.lambda_reduces_to_curry(lambda_params, exp)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    // We can rewrite a lambda directly into a curry expression if:
    // - lambda parameters are a simple variable tuple (v1, v2, ...) === (bindings.is_empty())
    // Caller should already check that, and place the tuple of variables in parameter list lambda_params.
    //
    // Then, we can reduce to curry if:
    // - lambda body is a function call with
    //   - each lambda parameter used exactly once as a call argument, in order (possibly with gaps)
    //   - every other argument is a simple expression containing only constants and free variables
    // Arguments here are
    //   - id: original lambda expr NodeId
    //   - body: body of lambda
    //   - lambda_params: a Parameter corresponding to each lambda param
    fn try_to_reduce_lambda_to_curry(
        &mut self,
        id: NodeId,
        body: &Exp,
        lambda_params: Vec<Parameter>,
        abilities: &AbilitySet,
    ) -> Option<Exp> {
        if let Some((fn_exp, args)) = self.lambda_reduces_to_curry(&lambda_params, body) {
            // lambda has form |lambda_params| fn_exp(args)
            // where each arg is a constant or simple variable
            let mut new_args = vec![];
            let mut mask: u64 = 0; // has a 1 bit for the position of each lambda parameter in call
            let mut param_cursor = 0; // tracks which lambda param we are expecting
            let mut has_mismatch = false;
            let param_syms: BTreeSet<Symbol> =
                lambda_params.iter().map(|param| param.get_name()).collect();
            let mut mask_cursor = 1u64; // = 1u64<<(loop index)
            for arg in args.iter() {
                let this_arg_is_lambda_param = if let ExpData::LocalVar(_id, name) = arg.as_ref() {
                    if param_syms.contains(name) {
                        if let Some(param) = lambda_params.get(param_cursor) {
                            if param.get_name() == *name {
                                param_cursor += 1;
                                true
                            } else {
                                has_mismatch = true;
                                break;
                            }
                        } else {
                            has_mismatch = true;
                            break;
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !this_arg_is_lambda_param {
                    // this arg is a free var and is provided as an arg to curry
                    new_args.push((*arg).clone());
                    mask |= mask_cursor;
                } else {
                    // this arg is a lambda parameter
                }
                mask_cursor <<= 1;
            }
            if !has_mismatch {
                let env = self.fun_env.module_env.env;
                let fn_id = fn_exp.node_id();
                let fn_type = env.get_node_type(fn_id);
                if let Type::Fun(_fn_param_type, _fn_result_type, fun_abilities) = &fn_type {
                    new_args.insert(0, fn_exp);
                    let ty_params = self.fun_env.get_type_parameters_ref();
                    // Check bound free var abilities
                    let ability_inferer = AbilityInferer::new(env, ty_params);
                    let bound_value_abilities: Vec<_> = new_args
                        .iter()
                        .map(|exp| {
                            let node = exp.as_ref().node_id();
                            let ty = env.get_node_type(node);
                            let node_abilities = ability_inferer.infer_abilities(&ty).1;
                            (env.get_node_loc(node), node_abilities)
                        })
                        .collect();
                    let mut bound_value_missing_abilities: Vec<_> = bound_value_abilities
                        .iter()
                        .filter_map(|(loc, node_abilities)| {
                            if !abilities.is_subset(*node_abilities) {
                                let missing = abilities.setminus(*node_abilities);
                                Some((
                                    loc.clone(),
                                    format!(
                                        "Captured free value is missing abilities: {}",
                                        missing
                                    ),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !abilities.is_subset(*fun_abilities) {
                        let missing = abilities.setminus(*fun_abilities);
                        let tdc = env.get_type_display_ctx();
                        bound_value_missing_abilities.push((
                            env.get_node_loc(fn_id),
                            format!(
                                "Base function of closure has type {}, is missing abilities: {}",
                                fn_type.display(&tdc),
                                missing
                            ),
                        ));
                    }
                    let closure_abilities = bound_value_abilities
                        .iter()
                        .map(|(_loc, node_abilities)| *node_abilities)
                        .reduce(|abs1, abs2| abs1.intersect(abs2))
                        .unwrap_or(*fun_abilities);
                    if !bound_value_missing_abilities.is_empty() {
                        let missing_abilities = abilities.setminus(closure_abilities);
                        let loc = env.get_node_loc(id);
                        env.error_with_labels(
                            &loc,
                            &format!("Lambda captures free variables with types that do not have some declared abilities: {}",
                                     missing_abilities),
                            bound_value_missing_abilities);
                        return None;
                    };
                    Some(ExpData::Call(id, Operation::Bind(mask), new_args).into_exp())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'a> ExpRewriterFunctions for LambdaLifter<'a> {
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
        abilities: AbilitySet, // TODO(LAMBDA): do something with this
    ) -> Option<Exp> {
        if self.exempted_lambdas.contains(&id) {
            return None;
        }
        let env = self.fun_env.module_env.env;
        let module_id = self.fun_env.module_env.get_id();

        match capture_kind {
            LambdaCaptureKind::Move => {
                // OK.
            },
            LambdaCaptureKind::Default | LambdaCaptureKind::Copy => {
                let loc = env.get_node_loc(id);
                env.error(
                    &loc,
                    // TODO(LAMBDA)
                    "Currently, lambda expressions must explicitly declare `move` capture of free variables, except when appearing as an argument to an inline functioncall."
                );
                return None;
            },
        };

        // params = new Parameter for each free var to represent it in the lifted function
        // closure_args = corresponding expressions to provide as actual arg for each param
        // param_index_mapping = for each free var which is a Parameter from the enclosing function,
        //      a mapping from index there to index in the params list; other free vars are
        //      substituted automatically by using the same symbol for the param
        let (mut params, mut closure_args, param_index_mapping) = self.get_params_for_freevars();

        // Some(ExpData::Invalid(env.clone_node(id)).into_exp());
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

        // We can rewrite a lambda directly into a curry expression if:
        // - lambda parameters are a simple variable tuple (v1, v2, ...) === (bindings.is_empty())
        //
        // - lambda body is a function call with
        //   - each lambda parameter used exactly once as a call argument, in order (possibly with gaps)
        //   - every other argument is a simple expression containing only constants and free variables
        if bindings.is_empty() {
            let possible_curry_exp =
                self.try_to_reduce_lambda_to_curry(id, body, lambda_params.clone(), &abilities);
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
        let lambda_inst_opt = env.get_node_instantiation_opt(id);
        let result_type = if let Type::Fun(_, result_type, _) = &lambda_type {
            *result_type.clone()
        } else {
            Type::Error // type error reported
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
        let params_types = params.iter().map(|param| param.get_type()).collect();
        self.lifted.push(ClosureFunction {
            loc: lambda_loc.clone(),
            fun_id,
            type_params: self.fun_env.get_type_parameters(),
            params,
            result_type: result_type.clone(),
            def: self.bind(bindings, body),
        });

        // Create an expression for the function reference
        let fn_type = Type::Fun(
            Box::new(Type::Tuple(params_types)),
            Box::new(result_type),
            abilities,
        );
        let fn_exp = self.make_move_fn_exp(
            lambda_loc.clone(),
            fn_type,
            module_id,
            fun_id,
            lambda_inst_opt.clone(),
        );

        let bound_param_count = closure_args.len();
        if bound_param_count == 0 {
            // No free variables, just return the function reference
            Some(fn_exp)
        } else {
            // Create a bitmask for the early-bound parameters.
            let bitmask: u64 = (1u64 << bound_param_count) - 1;

            // Create and return closure expression
            let id = env.new_node(lambda_loc, lambda_type);
            if let Some(inst) = lambda_inst_opt {
                env.set_node_instantiation(id, inst);
            }
            closure_args.insert(0, fn_exp);
            Some(ExpData::Call(id, Operation::Bind(bitmask), closure_args).into_exp())
        }
    }
}
