// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Processes proof hint blocks from function specifications.
//!
//! This processor directly emits bytecode for all proof hints, keeping the
//! Boogie backend unaware of proof hints:
//! - `assert expr`: Emits `Prop(Assert, exp)` bytecode.
//! - `assume [trusted] expr`: Emits `Prop(Assume, exp)` bytecode.
//! - `use expr`: Emits `Prop(Assert, exp)` bytecode.
//! - `unfold f`: Emits `Prop(Assume, forall params :: f(params) == body)`.
//! - `trigger forall x: T with {exprs}`: Rewrites matching `Quant` nodes to
//!   append triggers.
//! - `split on expr`: Creates `Split` verification variants with case assumptions.
//! - `induct on var`: Creates `Induct` verification variants (base/step).

use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, ProofHint, QualifiedSymbol, QuantKind, SpecFunDecl},
    exp_generator::ExpGenerator,
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{FunctionEnv, GlobalEnv, ModuleId, NodeId, Parameter, SpecFunId},
    symbol::Symbol,
    ty::{PrimitiveType, Type, BOOL_TYPE},
};
use move_stackless_bytecode::{
    function_data_builder::FunctionDataBuilder,
    function_target::FunctionData,
    function_target_pipeline::{
        FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant, VerificationFlavor,
    },
    stackless_bytecode::{Bytecode, PropKind},
};
use num::BigInt;

pub struct ProofHintProcessor {}

impl ProofHintProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl FunctionTargetProcessor for ProofHintProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        _fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        // Default implementation: used for non-verification variants (Baseline).
        // The actual work is done in process_and_maybe_remove.
        data
    }

    fn process_and_maybe_remove(
        &self,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> Option<FunctionData> {
        if fun_env.is_native() || fun_env.is_intrinsic() {
            return Some(data);
        }

        // Only process verification variants.
        let flavor = match &data.variant {
            FunctionVariant::Verification(flavor) => flavor.clone(),
            _ => return Some(data),
        };

        let proof_hints = fun_env.get_spec().proof_hints.clone();
        if proof_hints.is_empty() {
            return Some(data);
        }

        let env = fun_env.module_env.env;

        // Classify hints.
        let mut assert_assume_hints: Vec<&ProofHint> = vec![];
        let mut unfold_funs: Vec<(QualifiedSymbol, Option<usize>)> = vec![];
        let mut trigger_hints: Vec<(Vec<(Symbol, Type)>, Vec<Vec<Exp>>)> = vec![];
        // Each element is a "split dimension": for a bool expression, it's a
        // single-element vec (true/false handled by combo bits); for an enum,
        // it's one element per variant (each is an `assume variant_test` expression).
        let mut split_dimensions: Vec<Vec<Exp>> = vec![];
        let mut induct_var: Option<(Symbol, usize)> = None;

        for hint in &proof_hints {
            match hint {
                ProofHint::Assert(..) | ProofHint::Assume(..) | ProofHint::Witness(..) => {
                    assert_assume_hints.push(hint);
                },
                ProofHint::Unfold(_loc, qsym, depth) => {
                    unfold_funs.push((qsym.clone(), *depth));
                },
                ProofHint::Trigger(_loc, binds, trigger_groups) => {
                    trigger_hints.push((binds.clone(), trigger_groups.clone()));
                },
                ProofHint::SplitOn(loc, exp) => {
                    let exp_ty = env.get_node_type(exp.node_id());
                    let exp_ty = exp_ty.skip_reference();
                    if let Type::Struct(mid, sid, _) = exp_ty {
                        // Enum type: one case per variant.
                        let struct_env = env.get_module(*mid).into_struct(*sid);
                        if struct_env.has_variants() {
                            let variants: Vec<Symbol> = struct_env.get_variants().collect();
                            let mut variant_tests = vec![];
                            for v in &variants {
                                let node_id = env.new_node(
                                    env.get_node_loc(exp.node_id()),
                                    Type::Primitive(PrimitiveType::Bool),
                                );
                                let test = ExpData::Call(
                                    node_id,
                                    Operation::TestVariants(*mid, *sid, vec![*v]),
                                    vec![exp.clone()],
                                )
                                .into_exp();
                                variant_tests.push(test);
                            }
                            split_dimensions.push(variant_tests);
                        } else {
                            let tctx = move_model::ty::TypeDisplayContext::new(env);
                            env.error(
                                loc,
                                &format!(
                                    "`split on` requires a bool or enum type, but `{}` \
                                     is a struct without variants",
                                    exp_ty.display(&tctx)
                                ),
                            );
                        }
                    } else if matches!(exp_ty, Type::Primitive(PrimitiveType::Bool)) {
                        // Boolean: true/false cases.
                        split_dimensions.push(vec![exp.clone()]);
                    } else {
                        let tctx = move_model::ty::TypeDisplayContext::new(env);
                        env.error(
                            loc,
                            &format!(
                                "`split on` requires a bool or enum type, but expression \
                                 has type `{}`",
                                exp_ty.display(&tctx)
                            ),
                        );
                    }
                },
                ProofHint::InductOn(_loc, sym) => {
                    let params = fun_env.get_parameters();
                    if let Some(idx) = params.iter().position(|p| p.0 == *sym) {
                        induct_var = Some((*sym, idx));
                    }
                },
            }
        }

        // --- Handle split/induct: create new variants, remove the original ---
        if !split_dimensions.is_empty() {
            // Compute the Cartesian product of all split dimensions.
            // For boolean dimensions (1 element), we have 2 cases: true/false.
            // For enum dimensions (N elements), we have N cases (one per variant).
            let dim_sizes: Vec<usize> = split_dimensions
                .iter()
                .map(|d| if d.len() == 1 { 2 } else { d.len() })
                .collect();
            let total_combos: usize = dim_sizes.iter().product();

            for combo in 0..total_combos {
                let new_flavor = VerificationFlavor::Split(Box::new(flavor.clone()), combo);
                let new_data = data.fork(FunctionVariant::Verification(new_flavor));
                let mut builder = FunctionDataBuilder::new(fun_env, new_data);
                let old_code = std::mem::take(&mut builder.data.code);

                // Emit assert/assume/unfold proof hints first.
                emit_proof_hints(&mut builder, env, &assert_assume_hints, &unfold_funs);

                // Emit case assumptions for this combination.
                let mut remaining = combo;
                for (dim_idx, dim) in split_dimensions.iter().enumerate() {
                    let dim_size = dim_sizes[dim_idx];
                    let choice = remaining % dim_size;
                    remaining /= dim_size;

                    let assume_exp = if dim.len() == 1 {
                        // Boolean dimension: choice 0 = negated, choice 1 = original.
                        if choice != 0 {
                            dim[0].clone()
                        } else {
                            builder.mk_not(dim[0].clone())
                        }
                    } else {
                        // Enum dimension: assume the variant test for this choice.
                        dim[choice].clone()
                    };
                    builder.set_loc(fun_env.get_spec_loc());
                    builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, assume_exp));
                }

                // Re-emit original code, applying trigger rewrites.
                let rewritten_code = rewrite_triggers_in_code(env, old_code, &trigger_hints);
                for bc in rewritten_code {
                    builder.emit(bc);
                }

                targets.insert_target_data(
                    &fun_env.get_qualified_id(),
                    builder.data.variant.clone(),
                    builder.data,
                );
            }
            // Remove the original variant.
            return None;
        }

        if let Some((_sym, param_idx)) = induct_var {
            // Base case: param == 0
            let base_flavor = VerificationFlavor::Induct(Box::new(flavor.clone()), false);
            let base_data = data.fork(FunctionVariant::Verification(base_flavor));
            {
                let mut builder = FunctionDataBuilder::new(fun_env, base_data);
                let old_code = std::mem::take(&mut builder.data.code);

                emit_proof_hints(&mut builder, env, &assert_assume_hints, &unfold_funs);

                // assume param == 0
                let param_exp = builder.mk_temporary(param_idx);
                let zero = builder.mk_num_const(BigInt::from(0));
                let base_cond = builder.mk_eq(param_exp, zero);
                builder.set_loc(fun_env.get_spec_loc());
                builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, base_cond));

                let rewritten_code = rewrite_triggers_in_code(env, old_code, &trigger_hints);
                for bc in rewritten_code {
                    builder.emit(bc);
                }

                targets.insert_target_data(
                    &fun_env.get_qualified_id(),
                    builder.data.variant.clone(),
                    builder.data,
                );
            }

            // Step case: param > 0
            let step_flavor = VerificationFlavor::Induct(Box::new(flavor.clone()), true);
            let step_data = data.fork(FunctionVariant::Verification(step_flavor));
            {
                let mut builder = FunctionDataBuilder::new(fun_env, step_data);
                let old_code = std::mem::take(&mut builder.data.code);

                emit_proof_hints(&mut builder, env, &assert_assume_hints, &unfold_funs);

                // assume param > 0
                let param_exp = builder.mk_temporary(param_idx);
                let zero = builder.mk_num_const(BigInt::from(0));
                let step_cond = builder.mk_bool_call(Operation::Gt, vec![param_exp, zero]);
                builder.set_loc(fun_env.get_spec_loc());
                builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, step_cond));

                let rewritten_code = rewrite_triggers_in_code(env, old_code, &trigger_hints);
                for bc in rewritten_code {
                    builder.emit(bc);
                }

                targets.insert_target_data(
                    &fun_env.get_qualified_id(),
                    builder.data.variant.clone(),
                    builder.data,
                );
            }

            // Remove the original variant.
            return None;
        }

        // --- No split/induct: just emit hints into the existing variant ---
        let mut builder = FunctionDataBuilder::new(fun_env, data);
        let old_code = std::mem::take(&mut builder.data.code);

        emit_proof_hints(&mut builder, env, &assert_assume_hints, &unfold_funs);

        // Re-emit original code, applying trigger rewrites.
        let rewritten_code = rewrite_triggers_in_code(env, old_code, &trigger_hints);
        for bc in rewritten_code {
            builder.emit(bc);
        }

        Some(builder.data)
    }

    fn name(&self) -> String {
        "proof_hint_processor".to_string()
    }
}

// =================================================================================================
// Proof hint emission

/// Emits assert/assume/use/unfold proof hints as Prop bytecode instructions.
fn emit_proof_hints(
    builder: &mut FunctionDataBuilder,
    env: &GlobalEnv,
    hints: &[&ProofHint],
    unfold_funs: &[(QualifiedSymbol, Option<usize>)],
) {
    // Emit unfold assumptions.
    for (qsym, depth) in unfold_funs {
        emit_unfold(builder, env, qsym, depth.unwrap_or(1));
    }

    // Emit assert/assume/use/witness hints.
    for hint in hints {
        match hint {
            ProofHint::Assert(loc, exp) => {
                builder.set_loc_and_vc_info(loc.clone(), "proof hint assertion");
                builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assert, exp.clone()));
            },
            ProofHint::Assume(loc, exp) => {
                builder.set_loc(loc.clone());
                builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, exp.clone()));
            },
            ProofHint::Witness(loc, exp) => {
                builder.set_loc_and_vc_info(loc.clone(), "witness does not satisfy existential");
                builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assert, exp.clone()));
            },
            _ => {},
        }
    }
}

/// Emits a `Prop(Assume, ...)` for unfolding a spec function definition.
/// For parameterized functions: `assume forall params :: f(params) == body`
/// For non-parameterized: `assume f() == body`
/// When `depth > 1`, recursively expands recursive calls in the body.
fn emit_unfold(
    builder: &mut FunctionDataBuilder,
    env: &GlobalEnv,
    qsym: &QualifiedSymbol,
    depth: usize,
) {
    let module_env = match env.find_module(&qsym.module_name) {
        Some(m) => m,
        None => return,
    };
    let (spec_fun_id, fun_decl) = match module_env.get_spec_funs_of_name(qsym.symbol).next() {
        Some((id, decl)) => (*id, decl),
        None => return,
    };
    let body = match &fun_decl.body {
        Some(b) => b.clone(),
        None => return,
    };

    let mid = module_env.get_id();

    // Expand recursive calls in the body if depth > 1.
    let expanded_body = if depth > 1 {
        expand_recursive_calls(env, mid, spec_fun_id, fun_decl, body, depth - 1)
    } else {
        body
    };

    if fun_decl.params.is_empty() {
        // Non-parameterized: assume f() == body
        let call_exp = builder.mk_call_with_inst(
            &fun_decl.result_type,
            vec![],
            Operation::SpecFunction(mid, spec_fun_id, None),
            vec![],
        );
        let eq_exp = builder.mk_eq(call_exp, expanded_body);
        builder.set_loc(fun_decl.loc.clone());
        builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, eq_exp));
    } else {
        // Parameterized: assume forall params :: f(params) == body
        let mut ranges = vec![];
        let mut args = vec![];
        for Parameter(name, ty, _) in &fun_decl.params {
            let var_node_id =
                env.new_node(fun_decl.loc.clone(), Type::TypeDomain(Box::new(ty.clone())));
            let range_exp = ExpData::Call(var_node_id, Operation::TypeDomain, vec![]).into_exp();
            let pat_node_id = env.new_node(fun_decl.loc.clone(), ty.clone());
            let pat = Pattern::Var(pat_node_id, *name);
            ranges.push((pat, range_exp));

            let local_node_id = env.new_node(fun_decl.loc.clone(), ty.clone());
            args.push(ExpData::LocalVar(local_node_id, *name).into_exp());
        }

        let call_exp = builder.mk_call_with_inst(
            &fun_decl.result_type,
            vec![],
            Operation::SpecFunction(mid, spec_fun_id, None),
            args,
        );
        let eq_exp = builder.mk_eq(call_exp, expanded_body);

        let quant_node_id = env.new_node(fun_decl.loc.clone(), BOOL_TYPE.clone());
        let quant_exp = ExpData::Quant(
            quant_node_id,
            QuantKind::Forall,
            ranges,
            vec![], // no triggers
            None,   // no condition
            eq_exp,
        )
        .into_exp();

        builder.set_loc(fun_decl.loc.clone());
        builder.emit_with(|id| Bytecode::Prop(id, PropKind::Assume, quant_exp));
    }
}

/// Recursively expands calls to `(mid, spec_fun_id)` in `body` by substituting
/// the function's body, up to `remaining_depth` levels.
fn expand_recursive_calls(
    env: &GlobalEnv,
    mid: ModuleId,
    spec_fun_id: SpecFunId,
    fun_decl: &SpecFunDecl,
    body: Exp,
    remaining_depth: usize,
) -> Exp {
    if remaining_depth == 0 {
        return body;
    }
    let fun_body = match &fun_decl.body {
        Some(b) => b.clone(),
        None => return body,
    };
    expand_spec_fun_calls(
        env,
        mid,
        spec_fun_id,
        fun_decl,
        &fun_body,
        body,
        remaining_depth,
    )
}

/// Walk `exp` and replace every call to `SpecFunction(mid, spec_fun_id, _)` with
/// the function body (with arguments substituted for parameters), then recurse.
fn expand_spec_fun_calls(
    env: &GlobalEnv,
    mid: ModuleId,
    spec_fun_id: SpecFunId,
    fun_decl: &SpecFunDecl,
    fun_body: &Exp,
    exp: Exp,
    remaining_depth: usize,
) -> Exp {
    struct SpecFunExpander<'a> {
        env: &'a GlobalEnv,
        mid: ModuleId,
        spec_fun_id: SpecFunId,
        fun_decl: &'a SpecFunDecl,
        fun_body: &'a Exp,
        remaining_depth: usize,
    }
    impl ExpRewriterFunctions for SpecFunExpander<'_> {
        fn rewrite_call(&mut self, _id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
            if let Operation::SpecFunction(call_mid, call_fid, _) = oper {
                if *call_mid == self.mid && *call_fid == self.spec_fun_id {
                    // Substitute parameters with arguments in the function body.
                    let mut substituted = self.fun_body.clone();
                    for (i, Parameter(name, _, _)) in self.fun_decl.params.iter().enumerate() {
                        if i < args.len() {
                            let sym = *name;
                            let arg = args[i].clone();
                            let mut replacer =
                                |_node_id: NodeId, target: RewriteTarget| -> Option<Exp> {
                                    if let RewriteTarget::LocalVar(s) = target {
                                        if s == sym {
                                            return Some(arg.clone());
                                        }
                                    }
                                    None
                                };
                            substituted =
                                ExpRewriter::new(self.env, &mut replacer).rewrite_exp(substituted);
                        }
                    }
                    // Recurse if depth allows.
                    if self.remaining_depth > 1 {
                        substituted = expand_spec_fun_calls(
                            self.env,
                            self.mid,
                            self.spec_fun_id,
                            self.fun_decl,
                            self.fun_body,
                            substituted,
                            self.remaining_depth - 1,
                        );
                    }
                    return Some(substituted);
                }
            }
            None
        }
    }
    let mut expander = SpecFunExpander {
        env,
        mid,
        spec_fun_id,
        fun_decl,
        fun_body,
        remaining_depth,
    };
    expander.rewrite_exp(exp)
}

// =================================================================================================
// Trigger rewriting

/// Rewrites triggers in all Prop bytecode instructions.
/// For each trigger hint, walks all Quant nodes in Prop instructions and appends
/// trigger groups to matching quantifiers.
fn rewrite_triggers_in_code(
    env: &GlobalEnv,
    code: Vec<Bytecode>,
    trigger_hints: &[(Vec<(Symbol, Type)>, Vec<Vec<Exp>>)],
) -> Vec<Bytecode> {
    if trigger_hints.is_empty() {
        return code;
    }

    let mut matched: Vec<bool> = vec![false; trigger_hints.len()];
    let new_code: Vec<Bytecode> = code
        .into_iter()
        .map(|bc| match bc {
            Bytecode::Prop(id, kind, exp) => {
                let mut any_rewrite = false;
                let mut new_exp = exp.clone();
                for (hint_idx, (bind_vars, trigger_groups)) in trigger_hints.iter().enumerate() {
                    let mut injector = TriggerInjector {
                        env,
                        bind_vars,
                        trigger_groups,
                        did_match: false,
                    };
                    new_exp = injector.rewrite_exp(new_exp);
                    if injector.did_match {
                        matched[hint_idx] = true;
                        any_rewrite = true;
                    }
                }
                if any_rewrite {
                    Bytecode::Prop(id, kind, new_exp)
                } else {
                    Bytecode::Prop(id, kind, exp)
                }
            },
            other => other,
        })
        .collect();

    // Report errors for unmatched trigger hints.
    for (idx, was_matched) in matched.iter().enumerate() {
        if !was_matched {
            let (bind_vars, _) = &trigger_hints[idx];
            let bindings_desc = bind_vars
                .iter()
                .map(|(s, t)| {
                    let tctx = move_model::ty::TypeDisplayContext::new(env);
                    format!("{}: {}", s.display(env.symbol_pool()), t.display(&tctx))
                })
                .collect::<Vec<_>>()
                .join(", ");
            // Use the function's loc — we don't have the trigger hint loc here,
            // but the error message is clear enough.
            env.error(
                &env.unknown_loc(),
                &format!(
                    "trigger hint `forall {}` did not match any quantifier \
                     in the verification condition",
                    bindings_desc,
                ),
            );
        }
    }

    new_code
}

/// Expression rewriter that injects trigger groups into matching quantifiers.
///
/// Matching is done modulo variable renaming and ordering: the trigger hint's
/// bound variable types are matched against the quantifier's range element types
/// regardless of variable names or declaration order. When matched, trigger
/// expressions are renamed from the hint's variable names to the quantifier's
/// actual variable names.
struct TriggerInjector<'a> {
    env: &'a GlobalEnv,
    bind_vars: &'a [(Symbol, Type)],
    trigger_groups: &'a [Vec<Exp>],
    did_match: bool,
}

impl TriggerInjector<'_> {
    /// Try to match bind_vars against ranges by type, returning a renaming map
    /// from hint variable names to quantifier variable names if successful.
    /// Matching is type-based and order-independent: we sort both sides by type
    /// and pair them positionally within each type group.
    fn try_match_ranges(&self, ranges: &[(Pattern, Exp)]) -> Option<Vec<(Symbol, Symbol)>> {
        if self.bind_vars.len() != ranges.len() {
            return None;
        }

        // Collect (type, symbol) for hint bind vars, sorted by type.
        let mut hint_vars: Vec<(Type, Symbol)> = self
            .bind_vars
            .iter()
            .map(|(sym, ty)| (ty.clone(), *sym))
            .collect();
        hint_vars.sort_by(|(t1, _), (t2, _)| t1.cmp(t2));

        // Collect (type, symbol) for quantifier ranges, sorted by type.
        // Use the pattern's node type (= bound variable type), which is the
        // element type regardless of whether the range is TypeDomain, Vector,
        // ResourceDomain, etc.
        let mut quant_vars: Vec<(Type, Symbol)> = ranges
            .iter()
            .filter_map(|(pat, _range)| {
                if let Pattern::Var(_, sym) = pat {
                    let var_ty = self.env.get_node_type(pat.node_id());
                    Some((var_ty, *sym))
                } else {
                    None
                }
            })
            .collect();
        if quant_vars.len() != ranges.len() {
            return None; // non-Var patterns present
        }
        quant_vars.sort_by(|(t1, _), (t2, _)| t1.cmp(t2));

        // Check types match and build renaming.
        let mut renaming = vec![];
        for ((hint_ty, hint_sym), (quant_ty, quant_sym)) in hint_vars.iter().zip(quant_vars.iter())
        {
            if hint_ty != quant_ty {
                return None;
            }
            renaming.push((*hint_sym, *quant_sym));
        }
        Some(renaming)
    }

    /// Rename variables in a trigger expression according to the renaming map.
    fn rename_exp(&self, exp: &Exp, renaming: &[(Symbol, Symbol)]) -> Exp {
        struct VarRenamer<'a> {
            renaming: &'a [(Symbol, Symbol)],
        }
        impl ExpRewriterFunctions for VarRenamer<'_> {
            fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
                for (from, to) in self.renaming {
                    if sym == *from {
                        return Some(ExpData::LocalVar(id, *to).into_exp());
                    }
                }
                None
            }
        }
        let mut renamer = VarRenamer { renaming };
        renamer.rewrite_exp(exp.clone())
    }
}

impl ExpRewriterFunctions for TriggerInjector<'_> {
    fn rewrite_quant(
        &mut self,
        id: NodeId,
        ranges: &[(Pattern, Exp)],
        triggers: &[Vec<Exp>],
        cond: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        let renaming = self.try_match_ranges(ranges)?;
        self.did_match = true;

        // Rename trigger expressions from hint variable names to quantifier
        // variable names, then append to existing triggers.
        let mut new_triggers: Vec<Vec<Exp>> = triggers.to_vec();
        for group in self.trigger_groups {
            let renamed_group: Vec<Exp> = group
                .iter()
                .map(|e| self.rename_exp(e, &renaming))
                .collect();
            new_triggers.push(renamed_group);
        }

        Some(
            ExpData::Quant(
                id,
                QuantKind::Forall,
                ranges.to_vec(),
                new_triggers,
                cond.clone(),
                body.clone(),
            )
            .into_exp(),
        )
    }
}
