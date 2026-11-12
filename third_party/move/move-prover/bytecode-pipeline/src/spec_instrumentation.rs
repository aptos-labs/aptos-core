// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Transformation which injects specifications (Move function spec blocks) into the bytecode.

use crate::{
    number_operation::BvInternalBoundaryProps, options::ProverOptions, verification_analysis,
};
use itertools::Itertools;
use move_core_types::function::ClosureMask;
use move_model::{
    ast,
    ast::{Exp, ExpData, MemoryLabel, QuantKind, RewriteResult, TempIndex, Value},
    exp_generator::ExpGenerator,
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    memory_labels::all_labels_in_exp,
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, QualifiedId, QualifiedInstId,
        SpecVarId, StructId,
    },
    pragmas::{
        ABORTS_IF_IS_PARTIAL_PRAGMA, BV_INTERNAL_PRAGMA, CONDITION_ABSTRACT_PROP,
        CONDITION_CONCRETE_PROP, EMITS_IS_PARTIAL_PRAGMA, EMITS_IS_STRICT_PRAGMA,
    },
    spec_derivation,
    spec_translator::{ProofAction, SpecTranslator, TranslatedSpec},
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type, TypeDisplayContext, BOOL_TYPE, NUM_TYPE},
};
use move_stackless_bytecode::{
    function_data_builder::FunctionDataBuilder,
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{
        FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant, VerificationFlavor,
    },
    livevar_analysis::LiveVarAnalysisProcessor,
    reaching_def_analysis::ReachingDefProcessor,
    stackless_bytecode::{
        AbortAction, AssignKind, AttrId, BorrowEdge, BorrowNode, Bytecode, HavocKind, Label,
        Operation, PropKind,
    },
    usage_analysis,
    usage_analysis::{ReachedEffects, UsageState},
    COMPILED_MODULE_AVAILABLE,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Mutex,
};

const REQUIRES_FAILS_MESSAGE: &str = "precondition does not hold at this call";
const ENSURES_FAILS_MESSAGE: &str = "post-condition does not hold";
pub(crate) const ABORTS_IF_FAILS_MESSAGE: &str = "function does not abort under this condition";
pub(crate) const ABORT_NOT_COVERED: &str = "abort not covered by any of the `aborts_if` clauses";
pub(crate) const ABORTS_CODE_NOT_COVERED: &str =
    "abort code not covered by any of the `aborts_if` or `aborts_with` clauses";
const EMITS_FAILS_MESSAGE: &str = "function does not emit the expected event";
const EMITS_NOT_COVERED: &str = "emitted event not covered by any of the `emits` clauses";
const CHOICE_WITNESS_FAILS_MESSAGE: &str = "choice expression requires a witness to exist";

// ================================================================================================
// # Spec Instrumenter

pub struct SpecInstrumentationProcessor {
    /// Memory usage of every function, taken before instrumentation changes the code.
    usage: Mutex<BTreeMap<QualifiedId<FunId>, UsageState>>,
    /// Closure footprint walks, taken before instrumentation clears or rewrites any body.
    reached: Mutex<ReachedCache>,
}

impl SpecInstrumentationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            usage: Mutex::new(BTreeMap::new()),
            reached: Mutex::new(ReachedCache::new()),
        })
    }
}

impl FunctionTargetProcessor for SpecInstrumentationProcessor {
    fn initialize(&self, env: &GlobalEnv, targets: &mut FunctionTargetsHolder) {
        let usage: BTreeMap<_, _> = targets
            .get_funs()
            .map(|qid| {
                let fun_env = env.get_function(qid);
                let target = targets.get_target(&fun_env, &FunctionVariant::Baseline);
                (qid, usage_analysis::get_memory_usage(&target).clone())
            })
            .collect();
        // In inference mode the target's specification is intentionally absent
        // or incomplete until SpecInferenceProcessor runs later in the pipeline.
        // Checking caller/callee modifies relations here would therefore reject
        // valid inference targets merely because an existing caller already has
        // a frame declaration for memory the target modifies.
        if !ProverOptions::get(env).inference {
            check_modifies(env, targets);
        }
        // Runs in every mode: the usage analysis counts an undeclared function parameter as
        // bounded only because this check enforces its frame.
        let reached = reached_cache(env, targets, &usage);
        check_fun_arg_frames(env, targets, &usage, &reached);
        *self.usage.lock().unwrap() = usage;
        *self.reached.lock().unwrap() = reached;
    }

    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        assert_eq!(data.variant, FunctionVariant::Baseline);
        if fun_env.is_native() || fun_env.is_intrinsic() {
            return data;
        }

        let options = ProverOptions::get(fun_env.module_env.env);
        let verification_info =
            verification_analysis::get_info(&FunctionTarget::new(fun_env, &data));
        let is_verified = verification_info.verified;
        let is_inlined = verification_info.inlined;
        let usage = self.usage.lock().unwrap();
        let reached = self.reached.lock().unwrap();

        if is_verified {
            // Create a clone of the function data, moving annotations
            // out of this data and into the clone.
            let mut verification_data =
                data.fork(FunctionVariant::Verification(VerificationFlavor::Regular));
            let (instrumented, split_points) = Instrumenter::run(
                &options,
                targets,
                fun_env,
                verification_data,
                scc_opt,
                &usage,
                &reached,
            );
            verification_data = instrumented;

            if split_points.is_empty() || options.inference {
                // No splits — insert the single verification variant.
                // Proof-block split-fork is verification-only; during inference
                // (`options.inference`) we never fork, so each function has at
                // most one verified variant for inference to walk. By
                // construction `Instrumenter::run` should not produce any
                // `split_points` under inference, but check explicitly so the
                // gate is local to this site too.
                targets.insert_target_data(
                    &fun_env.get_qualified_id(),
                    verification_data.variant.clone(),
                    verification_data,
                );
            } else {
                // Expand splits into multiple verification variants.
                Self::expand_splits(
                    fun_env.module_env.env,
                    targets,
                    fun_env,
                    verification_data,
                    &split_points,
                );
            }
        }

        // Instrument baseline variant only if it is inlined.
        if is_inlined {
            let (data, _) =
                Instrumenter::run(&options, targets, fun_env, data, scc_opt, &usage, &reached);
            data
        } else {
            // Clear code but keep function data stub.
            // TODO(refactoring): the stub is currently still needed because boogie_wrapper
            //   seems to access information about it, which it should not in fact.
            // NOTE(mengxu): do not clear the code if we are instrumenting for the interpreter.
            // For functions whose verification is turned off with `pragma verify=false` or due to
            // other reasons, we still want to keep a copy of the code in baseline.
            if !options.for_interpretation {
                data.code = vec![];
            }
            data
        }
    }

    fn name(&self) -> String {
        "spec_instrumenter".to_string()
    }

    fn dump_result(
        &self,
        f: &mut fmt::Formatter,
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        writeln!(f, "\n\n==== spec-instrumenter input specs ====\n")?;
        for ref module in env.get_modules() {
            if !module.is_target() {
                continue;
            }
            for ref fun in module.get_functions() {
                if fun.is_not_prover_target() || !fun.is_compiled() {
                    continue;
                }
                for (variant, target) in targets.get_targets(fun) {
                    let spec = &*target.get_spec();
                    let has_frame = spec.frame_spec.as_ref().is_some_and(|f| {
                        !f.modifies_targets.is_empty() || !f.reads_targets.is_empty()
                    });
                    if !spec.conditions.is_empty() || !spec.properties.is_empty() || has_frame {
                        writeln!(
                            f,
                            "fun {}[{}]\n{}",
                            fun.get_full_name_str(),
                            variant,
                            fun.module_env.env.display(spec)
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn opaque_framed_memory(
    env: &GlobalEnv,
    callee: &FunctionEnv,
) -> BTreeSet<QualifiedInstId<StructId>> {
    callee
        .get_frame_spec()
        .map(|frame| {
            frame
                .modifies_targets
                .iter()
                .filter_map(
                    |target| match env.get_node_type(target.node_id()).skip_reference() {
                        Type::Struct(mid, sid, inst) => {
                            Some(mid.qualified_inst(*sid, inst.clone()))
                        },
                        _ => None,
                    },
                )
                .collect()
        })
        .unwrap_or_default()
}

/// Maximum number of split combinations before we emit an error.
const MAX_SPLIT_COMBINATIONS: usize = 1024;

impl SpecInstrumentationProcessor {
    /// Expand split points into multiple verification variants.
    /// Each split point on a bool expression creates 2 cases; on an enum, N cases
    /// (one per variant). If a split has a guard (path condition from an enclosing `if`
    /// in the proof block), an extra case is added for `!guard`, and the base cases
    /// are conjuncted with `guard`. The total combinations is the product of all case counts.
    fn expand_splits(
        env: &GlobalEnv,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        verification_data: FunctionData,
        split_points: &[(AttrId, Exp, Option<Exp>)],
    ) {
        // Determine the number of base cases for each split point, validating types.
        let mut has_error = false;
        let base_case_counts: Vec<usize> = split_points
            .iter()
            .map(|(_, exp, _)| {
                let ty = env.get_node_type(exp.node_id());
                if ty == BOOL_TYPE {
                    2
                } else if let Some((struct_env, _)) = ty.get_struct(env) {
                    if struct_env.has_variants() {
                        struct_env.get_variants().count()
                    } else {
                        env.error(
                            &env.get_node_loc(exp.node_id()),
                            "`split` expression must be of type `bool` or an enum type",
                        );
                        has_error = true;
                        2
                    }
                } else {
                    env.error(
                        &env.get_node_loc(exp.node_id()),
                        "`split` expression must be of type `bool` or an enum type",
                    );
                    has_error = true;
                    2
                }
            })
            .collect();

        if has_error {
            // Insert the original variant to avoid cascading errors.
            targets.insert_target_data(
                &fun_env.get_qualified_id(),
                verification_data.variant.clone(),
                verification_data,
            );
            return;
        }

        // If there's a guard, add one extra case for `!guard`.
        let case_counts: Vec<usize> = split_points
            .iter()
            .zip(base_case_counts.iter())
            .map(|((_, _, guard), &base)| if guard.is_some() { base + 1 } else { base })
            .collect();

        let total: usize = case_counts.iter().product();
        if total > MAX_SPLIT_COMBINATIONS {
            env.error(
                &fun_env.get_loc(),
                &format!(
                    "split produces {} verification variants, exceeding the limit of {}",
                    total, MAX_SPLIT_COMBINATIONS
                ),
            );
            // Fall back: insert the original (with Nop placeholders) as single variant.
            targets.insert_target_data(
                &fun_env.get_qualified_id(),
                verification_data.variant.clone(),
                verification_data,
            );
            return;
        }

        let original_flavor = match &verification_data.variant {
            FunctionVariant::Verification(f) => f.clone(),
            _ => VerificationFlavor::Regular,
        };

        for combo in 0..total {
            let new_flavor = VerificationFlavor::Split(Box::new(original_flavor.clone()), combo);
            let new_variant = FunctionVariant::Verification(new_flavor);
            let mut forked = verification_data.fork(new_variant.clone());

            // For each split point, replace the Nop at that offset with the
            // appropriate Assume.
            let mut remainder = combo;
            for (i, (split_attr, exp, guard)) in split_points.iter().enumerate() {
                let case_idx = remainder % case_counts[i];
                remainder /= case_counts[i];

                let base_cases = base_case_counts[i];

                // Extra guard-false case: assume !guard.
                if guard.is_some() && case_idx == base_cases {
                    let guard_exp = guard.as_ref().unwrap();
                    let not_node =
                        env.new_node(env.get_node_loc(guard_exp.node_id()), BOOL_TYPE.clone());
                    let assume_exp =
                        ExpData::Call(not_node, ast::Operation::Not, vec![guard_exp.clone()])
                            .into_exp();
                    if let Some(offset) = forked
                        .code
                        .iter()
                        .position(|bc| matches!(bc, Bytecode::Nop(aid) if *aid == *split_attr))
                    {
                        forked.code[offset] =
                            Bytecode::Prop(*split_attr, PropKind::Assume, assume_exp);
                    }
                    continue;
                }

                let ty = env.get_node_type(exp.node_id());
                let base_assume = if ty == BOOL_TYPE {
                    if case_idx == 0 {
                        // true case: assume the expression
                        exp.clone()
                    } else {
                        // false case: assume !expression
                        let not_node =
                            env.new_node(env.get_node_loc(exp.node_id()), BOOL_TYPE.clone());
                        ExpData::Call(not_node, ast::Operation::Not, vec![exp.clone()]).into_exp()
                    }
                } else if let Some((struct_env, inst)) = ty.get_struct(env) {
                    // Enum case: assume TestVariants for variant at case_idx
                    let variant_sym: Symbol = struct_env
                        .get_variants()
                        .nth(case_idx)
                        .expect("variant exists");
                    let test_node =
                        env.new_node(env.get_node_loc(exp.node_id()), BOOL_TYPE.clone());
                    env.set_node_instantiation(test_node, inst.to_vec());
                    ExpData::Call(
                        test_node,
                        ast::Operation::TestVariants(
                            struct_env.module_env.get_id(),
                            struct_env.get_id(),
                            vec![variant_sym],
                        ),
                        vec![exp.clone()],
                    )
                    .into_exp()
                } else {
                    // Fallback (shouldn't happen): just assume the expression
                    exp.clone()
                };

                // If guarded, conjunct with the guard: assume guard && base_assume.
                let assume_exp = if let Some(guard_exp) = guard {
                    let and_node = env.new_node(env.get_node_loc(exp.node_id()), BOOL_TYPE.clone());
                    ExpData::Call(and_node, ast::Operation::And, vec![
                        guard_exp.clone(),
                        base_assume,
                    ])
                    .into_exp()
                } else {
                    base_assume
                };

                // Find and replace the Nop with matching AttrId (stable across optimization).
                if let Some(offset) = forked
                    .code
                    .iter()
                    .position(|bc| matches!(bc, Bytecode::Nop(aid) if *aid == *split_attr))
                {
                    forked.code[offset] = Bytecode::Prop(*split_attr, PropKind::Assume, assume_exp);
                }
            }

            targets.insert_target_data(&fun_env.get_qualified_id(), new_variant, forked);
        }
    }
}

/// Returns the set of labels DEFINED by a condition (all range.post labels).
pub fn defined_state_labels(exp: &Exp) -> BTreeSet<MemoryLabel> {
    let mut labels = BTreeSet::new();
    exp.visit_pre_order(&mut |node| {
        // A bound state is an existential/universal proof obligation, not
        // a free intermediate state whose definition can be assumed.
        if matches!(node, ExpData::Quant(..) | ExpData::Lambda(..)) {
            return false;
        }
        if let ExpData::Call(_, op, _) = node {
            match op {
                ast::Operation::Behavior(_, range)
                | ast::Operation::SpecFunction(_, _, range)
                | ast::Operation::SpecPublish(range)
                | ast::Operation::SpecRemove(range)
                | ast::Operation::SpecUpdate(range) => labels.extend(range.post),
                _ => {},
            }
        }
        true
    });
    labels
}

/// Extract intermediate-state definitions, preserving guards and binding scope.
/// Merely containing a definition does not make an entire conjunct assumable:
/// it may also constrain the actual return state. A result projection defines
/// its intermediate state through the corresponding `ensures_of`, not through
/// a property of the projected value (which still has to be proved).
pub fn state_label_defining_fragment<'env>(generator: &impl ExpGenerator<'env>, exp: &Exp) -> Exp {
    state_label_defining_fragment_in_context(generator, exp, true)
}

/// Extracts only facts which are entailed by the expression. `positive` is false
/// in contexts where a Boolean child is merely inspected, rather than asserted.
/// Result projections still contribute their callee relation in those contexts:
/// using a projected result denotes the call independently of how that value is
/// subsequently tested.
fn state_label_defining_fragment_in_context<'env>(
    generator: &impl ExpGenerator<'env>,
    exp: &Exp,
    positive: bool,
) -> Exp {
    use ast::{BehaviorKind, Operation as Op};
    let env = generator.global_env();
    let bool_node = || env.new_node(env.get_node_loc(exp.node_id()), BOOL_TYPE.clone());
    let truth = || ExpData::Value(bool_node(), Value::Bool(true)).into_exp();
    let and = |a, b| ExpData::Call(bool_node(), Op::And, vec![a, b]).into_exp();
    let or = |a, b| ExpData::Call(bool_node(), Op::Or, vec![a, b]).into_exp();
    if defined_state_labels(exp).is_empty() {
        return truth();
    }
    match exp.as_ref() {
        ExpData::Call(id, op, args) => {
            let term_children = || {
                args.iter()
                    .map(|arg| state_label_defining_fragment_in_context(generator, arg, false))
                    .fold(truth(), and)
            };
            match op {
                Op::And if positive => and(
                    state_label_defining_fragment_in_context(generator, &args[0], true),
                    state_label_defining_fragment_in_context(generator, &args[1], true),
                ),
                Op::Or if positive => or(
                    state_label_defining_fragment_in_context(generator, &args[0], true),
                    state_label_defining_fragment_in_context(generator, &args[1], true),
                ),
                Op::Implies if positive => and(
                    state_label_defining_fragment_in_context(generator, &args[0], false),
                    ExpData::Call(bool_node(), Op::Implies, vec![
                        args[0].clone(),
                        state_label_defining_fragment_in_context(generator, &args[1], true),
                    ])
                    .into_exp(),
                ),
                Op::SpecPublish(range) | Op::SpecRemove(range) | Op::SpecUpdate(range)
                    if positive && range.post.is_some() =>
                {
                    let exists_node = bool_node();
                    env.set_node_instantiation(exists_node, env.get_node_instantiation(*id));
                    let exists =
                        ExpData::Call(exists_node, Op::Exists(range.pre), vec![args[0].clone()])
                            .into_exp();
                    // Procedure instrumentation has explicit pre-state saves.
                    // In a behavioral spec body the implicit pre-state is old_.
                    let exists = if range.pre.is_none() {
                        generator.mk_old(exists)
                    } else {
                        exists
                    };
                    let enabled = if matches!(op, Op::SpecPublish(_)) {
                        generator.mk_not(exists)
                    } else {
                        exists
                    };
                    and(term_children(), generator.mk_implies(enabled, exp.clone()))
                },
                Op::Behavior(kind, range) if range.post.is_some() => {
                    let definition = match kind {
                        BehaviorKind::ResultOf => {
                            let node = bool_node();
                            env.set_node_instantiation(node, env.get_node_instantiation(*id));
                            let Type::Fun(inputs, results, _) =
                                env.get_node_type(args[0].node_id())
                            else {
                                return truth();
                            };
                            let num_inputs = inputs.flatten().len();
                            let num_results = results.clone().flatten().len();
                            let mut canonical_args = args[..1 + num_inputs].to_vec();
                            for index in 0..num_results {
                                canonical_args.push(generator.mk_result_of_at_with_state(
                                    args[0].clone(),
                                    args[1..].to_vec(),
                                    &results,
                                    index,
                                    num_results,
                                    range.pre,
                                    range.post,
                                ));
                            }
                            canonical_args.extend_from_slice(&args[1 + num_inputs..]);
                            ExpData::Call(
                                node,
                                Op::Behavior(BehaviorKind::EnsuresOf, range.clone()),
                                canonical_args,
                            )
                            .into_exp()
                        },
                        BehaviorKind::EnsuresOf if positive => exp.clone(),
                        _ => truth(),
                    };
                    // A call's postcondition is available only in its valid,
                    // non-aborting domain. In particular, an always-aborting
                    // callee may legitimately have `ensures false`; importing
                    // that as a definition would prove every abort obligation.
                    let definition = if matches!(kind, BehaviorKind::ResultOf)
                        || positive && matches!(kind, BehaviorKind::EnsuresOf)
                    {
                        let Type::Fun(inputs, _, _) = env.get_node_type(args[0].node_id()) else {
                            return truth();
                        };
                        let inputs = args[..1 + inputs.flatten().len()].to_vec();
                        let pre_range = ast::MemoryRange {
                            pre: range.pre,
                            post: None,
                        };
                        let requires = generator.mk_bool_call(
                            Op::Behavior(BehaviorKind::RequiresOf, pre_range.clone()),
                            inputs.clone(),
                        );
                        let aborts = generator
                            .mk_bool_call(Op::Behavior(BehaviorKind::AbortsOf, pre_range), inputs);
                        generator.mk_implies(and(requires, generator.mk_not(aborts)), definition)
                    } else {
                        definition
                    };
                    and(term_children(), definition)
                },
                Op::SpecFunction(_, _, range)
                    if positive && range.post.is_some() && env.get_node_type(*id) == BOOL_TYPE =>
                {
                    and(term_children(), exp.clone())
                },
                // Negation, equivalence, equality on Booleans, and other
                // non-monotone contexts do not entail their Boolean children.
                // Descend in term context only to retain call-result relations.
                _ => term_children(),
            }
        },
        ExpData::Block(_, pattern, binding, body) => {
            let body = ExpData::Block(
                bool_node(),
                pattern.clone(),
                binding.clone(),
                state_label_defining_fragment_in_context(generator, body, positive),
            )
            .into_exp();
            if let Some(binding) = binding {
                and(
                    state_label_defining_fragment_in_context(generator, binding, false),
                    body,
                )
            } else {
                body
            }
        },
        ExpData::IfElse(_, cond, then, otherwise) => and(
            state_label_defining_fragment_in_context(generator, cond, false),
            ExpData::IfElse(
                bool_node(),
                cond.clone(),
                state_label_defining_fragment_in_context(generator, then, positive),
                state_label_defining_fragment_in_context(generator, otherwise, positive),
            )
            .into_exp(),
        ),
        _ => truth(),
    }
}

struct Instrumenter<'a> {
    options: &'a ProverOptions,
    builder: FunctionDataBuilder<'a>,
    ret_locals: Vec<TempIndex>,
    abort_local: TempIndex,
    abort_label: Label,
    can_abort: bool,
    mem_info: &'a BTreeSet<QualifiedInstId<StructId>>,
    /// Transitive memory effects of opaque callees in the original bytecode.
    opaque_callee_modifies: BTreeMap<QualifiedId<FunId>, BTreeSet<QualifiedInstId<StructId>>>,
    /// Opaque callees that may run a function value nothing bounds (`unbounded_invoke`,
    /// `invoke_frame_wildcard`, or no usage summary): a call may touch any memory.
    world_havoc_callees: BTreeSet<QualifiedId<FunId>>,
    /// The memory this function accesses, re-assumed well-formed after a world havoc.
    accessed_memory: BTreeSet<QualifiedInstId<StructId>>,
    /// Whether this function may run a function value nothing bounds.
    runs_unknown_code: bool,
    /// Resource types explicitly framed by opaque callees before call-site type
    /// substitution.
    opaque_callee_framed_memory: BTreeMap<QualifiedId<FunId>, BTreeSet<QualifiedInstId<StructId>>>,
    memory_free_callees: BTreeSet<QualifiedId<FunId>>,
    /// Unframed memory possibly modified by opaque closures invoked in this function.
    opaque_closure_modifies: BTreeSet<QualifiedInstId<StructId>>,
    /// Footprints of the closures bound to temporaries of this function.
    closure_footprints: BTreeMap<TempIndex, ClosureFootprint>,
    /// Map from Nop AttrId to split expression and optional guard (for `split` proof items).
    /// AttrIds are stable across optimization passes, unlike bytecode offsets.
    /// The optional guard is a path condition from enclosing `if` in the proof block.
    split_points: Vec<(AttrId, Exp, Option<Exp>)>,
    /// Whether the function being instrumented declares `pragma bv_internal`.
    self_is_bv_internal: bool,
    /// Locations of this function's `[concrete]` conditions: verified against the body,
    /// never assumed by a caller, so not boundary language.
    concrete_cond_locs: BTreeSet<Loc>,
}

// =================================================================================================
// # Core Instrumentation

impl<'a> Instrumenter<'a> {
    fn run(
        options: &'a ProverOptions,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv<'a>,
        data: FunctionData,
        scc_opt: Option<&[FunctionEnv]>,
        usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
        reached: &ReachedCache,
    ) -> (FunctionData, Vec<(AttrId, Exp, Option<Exp>)>) {
        // Pre-collect properties and opaque-callee effects from the original code.
        let memory_free_callees = data
            .code
            .iter()
            .filter_map(|bc| {
                let Bytecode::Call(_, _, Operation::Function(mid, fid, _), _, _) = bc else {
                    return None;
                };
                let qid = mid.qualified(*fid);
                if qid == fun_env.get_qualified_id() {
                    return None;
                }
                let callee = fun_env.module_env.env.get_function(qid);
                let target = targets.get_target(&callee, &FunctionVariant::Baseline);
                let callee_usage = usage_analysis::get_memory_usage(&target);
                // A function value nothing bounds may read any memory.
                (callee_usage.accessed.all.is_empty()
                    && !callee_usage.unbounded_invoke
                    && !callee_usage.invoke_frame_wildcard
                    && callee.get_spec_used_memory().is_empty()
                    && callee.get_spec_generic_used_memory().is_empty()
                    && callee.get_parameters().iter().all(|p| {
                        !p.1.is_mutable_reference()
                            && !matches!(p.1.skip_reference(), Type::Fun(..))
                    }))
                .then_some(qid)
            })
            .collect();
        let props: Vec<_> = data
            .code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Prop(id, PropKind::Assume, exp)
                | Bytecode::Prop(id, PropKind::Assert, exp) => Some((*id, exp.clone())),
                _ => None,
            })
            .collect();
        let opaque_callee_modifies = data
            .code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Call(_, _, Operation::Function(mid, fid, _), _, _) => {
                    let callee = fun_env.module_env.env.get_module(*mid).into_function(*fid);
                    callee.is_opaque().then(|| {
                        let callee_usage = &usage[&mid.qualified(*fid)];
                        (
                            mid.qualified(*fid),
                            callee_usage
                                .modified
                                .all
                                .iter()
                                .chain(callee_usage.invoke_frame_other.iter())
                                .cloned()
                                .collect(),
                        )
                    })
                },
                _ => None,
            })
            .collect();
        let world_havoc_callees = data
            .code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Call(_, _, Operation::Function(mid, fid, _), _, _) => {
                    let qid = mid.qualified(*fid);
                    let callee = fun_env.module_env.env.get_function(qid);
                    (callee.is_opaque()
                        && usage.get(&qid).is_none_or(|callee_usage| {
                            callee_usage.unbounded_invoke || callee_usage.invoke_frame_wildcard
                        }))
                    .then_some(qid)
                },
                _ => None,
            })
            .collect();
        let runs_unknown_code = usage
            .get(&fun_env.get_qualified_id())
            .is_none_or(|own| own.unbounded_invoke || own.invoke_frame_wildcard);
        let accessed_memory = usage
            .get(&fun_env.get_qualified_id())
            .map(|own| own.accessed.all.iter().cloned().collect())
            .unwrap_or_default();
        let opaque_callee_framed_memory = data
            .code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Call(_, _, Operation::Function(mid, fid, _), _, _) => {
                    let callee = fun_env.module_env.env.get_module(*mid).into_function(*fid);
                    callee.is_opaque().then(|| {
                        (
                            mid.qualified(*fid),
                            opaque_framed_memory(fun_env.module_env.env, &callee),
                        )
                    })
                },
                _ => None,
            })
            .collect();
        let mut opaque_closure_modifies: BTreeSet<QualifiedInstId<StructId>> = BTreeSet::new();
        for bc in &data.code {
            let Bytecode::Call(_, _, Operation::Closure(mid, fid, targs, _), _, _) = bc else {
                continue;
            };
            let callee = fun_env.module_env.env.get_module(*mid).into_function(*fid);
            if !callee.is_opaque() {
                continue;
            }
            let callee_usage = &usage[&mid.qualified(*fid)];
            let framed_memory = opaque_framed_memory(fun_env.module_env.env, &callee);
            opaque_closure_modifies.extend(
                callee_usage
                    .modified
                    .all
                    .iter()
                    .chain(callee_usage.invoke_frame.all.iter())
                    .filter(|mem| !framed_memory.contains(mem))
                    .map(|mem| mem.instantiate_ref(targs)),
            );
        }
        let closure_footprints = closure_footprints(
            fun_env.module_env.env,
            fun_env.get_qualified_id(),
            usage,
            reached,
            &data.code,
        );

        let mut builder = FunctionDataBuilder::new(fun_env, data);

        // After `NormalizeExitsProcessor`, the input bytecode has at most one
        // `Ret` instruction. Capture its source temps as `ret_locals` so the
        // spec translation rewrites `result_*` references to them directly,
        // and the assertions emitted by `emit_return_assertions` reference the
        // same temps that the trailing `Ret` returns. For divergent functions
        // (no `Ret`), allocate placeholder locals — the translated spec is
        // never asserted in that case.
        let ret_locals = builder
            .data
            .code
            .iter()
            .find_map(|bc| match bc {
                Bytecode::Ret(_, temps) => Some(temps.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                builder
                    .data
                    .result_type
                    .clone()
                    .flatten()
                    .into_iter()
                    .map(|ty| builder.new_temp(ty))
                    .collect_vec()
            });

        // Similarly create label and local for unified abort exit point. We translate `Abort(c)`
        // into `Assign(r, c); Jump(AbortLabel)`, as well as `Call(..)` into `Call(..);
        // OnAbort(AbortLabel, r)`. The `OnAbort` is a new instruction: if the last
        // call aborted, it stores the abort code in `r` and jumps to the label.
        let abort_local = builder.new_temp(NUM_TYPE.clone());
        let abort_label = builder.new_label();

        // Translate the specification. This deals with elimination of `old(..)` expressions,
        // as well as replaces `result_n` references with `ret_locals`.
        let auto_trace = options.auto_trace_level.verified_functions()
            && builder.data.variant.is_verified()
            || options.auto_trace_level.functions();
        let spec = SpecTranslator::translate_fun_spec(
            auto_trace,
            false,
            &mut builder,
            fun_env,
            &[],
            None,
            &ret_locals,
        );

        // Translate inlined properties. This deals with elimination of `old(..)` expressions in
        // ilined spec blocks
        let inlined_props: BTreeMap<_, _> = props
            .into_iter()
            .filter(|(_, prop)| {
                // Anchor markers are consumed positionally below.
                !matches!(
                    prop.as_ref(),
                    ExpData::Call(
                        _,
                        move_model::ast::Operation::SaveStateAnchor(..)
                            | move_model::ast::Operation::FoldsCaptureAnchor(..)
                            | move_model::ast::Operation::InlineCallSummary,
                        _,
                    )
                )
            })
            .map(|(id, prop)| {
                let loc = builder.get_loc(id);
                (
                    id,
                    SpecTranslator::translate_inline_property(
                        &loc,
                        auto_trace,
                        &mut builder,
                        &prop,
                    ),
                )
            })
            .collect();

        let mut mem_info = BTreeSet::new();

        if auto_trace {
            // Retrieve the memory used by the function
            mem_info =
                usage_analysis::get_memory_usage(&FunctionTarget::new(fun_env, &builder.data))
                    .accessed
                    .get_all_inst(&builder.data.type_args);
        }

        // Create and run the instrumenter.
        let mut instrumenter = Instrumenter {
            options,
            builder,
            ret_locals,
            abort_local,
            abort_label,
            can_abort: false,
            mem_info: &mem_info,
            opaque_callee_modifies,
            world_havoc_callees,
            accessed_memory,
            runs_unknown_code,
            opaque_callee_framed_memory,
            memory_free_callees,
            opaque_closure_modifies,
            closure_footprints,
            split_points: vec![],
            self_is_bv_internal: fun_env.is_pragma_true(BV_INTERNAL_PRAGMA, || false),
            concrete_cond_locs: fun_env
                .get_spec()
                .conditions
                .iter()
                .filter(|cond| {
                    let env = &fun_env.module_env.env;
                    env.is_property_true(&cond.properties, CONDITION_CONCRETE_PROP)
                        .unwrap_or(false)
                        && !env
                            .is_property_true(&cond.properties, CONDITION_ABSTRACT_PROP)
                            .unwrap_or(false)
                })
                .map(|cond| cond.loc.clone())
                .collect(),
        };
        // Always inline spec lets in proof actions, independent of the
        // `inline_spec_lets` option. Proof-action exps are recorded in
        // `split_points` outside the bytecode, so they are not rewritten by
        // the subsequent live-var analysis — inlining here is the only way
        // to ensure they reference only stable Move-level temps.
        //
        // Skip during spec inference: proof blocks are a verification-only
        // construct and must not influence the bytecode the inference walker
        // sees. `instrument` likewise skips emitting proof actions under
        // `options.inference`, so this work would be wasted anyway.
        let mut spec = spec;
        if !options.inference {
            instrumenter.inline_lets_in_proof_actions(&mut spec);
        }
        if ProverOptions::get(instrumenter.builder.global_env()).inline_spec_lets {
            instrumenter.inline_lets(&mut spec, false);
        }
        instrumenter.instrument(&spec, &inlined_props);

        // Extract split points before consuming the instrumenter.
        let split_points = std::mem::take(&mut instrumenter.split_points);

        // Run copy propagation (reaching definitions) and then assignment
        // elimination (live vars). This cleans up some redundancy created by
        // the instrumentation scheme.
        let mut data = instrumenter.builder.data;
        let reach_def = ReachingDefProcessor::new();
        let live_vars = LiveVarAnalysisProcessor::new_no_annotate();
        data = reach_def.process(targets, fun_env, data, scc_opt);
        (
            live_vars.process(targets, fun_env, data, scc_opt),
            split_points,
        )
    }

    fn is_verified(&self) -> bool {
        self.builder.data.variant.is_verified()
    }

    fn instrument(
        &mut self,
        spec: &TranslatedSpec,
        inlined_props: &BTreeMap<AttrId, (TranslatedSpec, Exp)>,
    ) {
        use Bytecode::*;
        use PropKind::*;

        // Extract and clear current code
        let old_code = std::mem::take(&mut self.builder.data.code);

        // Inject the entry state saves: for the verification variant those its post-conditions
        // need, otherwise those of inlined specs that use "old" values. For inline properties,
        // only the function-entry scoped saves are emitted here; saves registered under an
        // anchor label are emitted at the matching `SaveStateAnchor` marker. They precede the
        // `let` bindings, which may read memory saved under an entry label.
        self.builder
            .set_loc(self.builder.fun_env.get_loc().at_start());
        let mut saved_mems = BTreeSet::new();
        let mut saved_vars = BTreeSet::new();
        if self.is_verified() {
            for translated_spec in
                std::iter::once(spec).chain(inlined_props.values().map(|(s, _)| s))
            {
                self.emit_state_saves(translated_spec, None, &mut saved_mems, &mut saved_vars);
            }
            // Every function type has a variant of unknown origin, so a predicate over any
            // function value takes the world memory: its saves are needed wherever memory is saved.
            if self.runs_unknown_code || self.builder.global_env().world_memory().is_some() {
                self.emit_world_saves(spec, &mut saved_mems);
            }
        } else {
            assert!(
                verification_analysis::get_info(&FunctionTarget::new(
                    self.builder.fun_env,
                    &self.builder.data
                ))
                .inlined
            );
            for translated_spec in inlined_props.values().map(|(s, _)| s) {
                self.emit_state_saves(translated_spec, None, &mut saved_mems, &mut saved_vars);
            }
        }

        // Emit `let` bindings.
        self.emit_lets(spec, false, self.self_is_bv_internal);

        // Assume preconditions in the verification variant only. A Baseline body is
        // inlined into its callers, which assert the callee's caller-visible preconditions
        // at the call; it assumes nothing they do not check.
        self.builder
            .set_loc(self.builder.fun_env.get_loc().at_start()); // reset to function level
        if self.is_verified() {
            for (loc, exp) in spec.pre_conditions(&self.builder) {
                let is_concrete = self.is_concrete_cond(&loc);
                self.builder.set_loc(loc);
                self.builder
                    .emit_with(move |attr_id| Prop(attr_id, Assume, exp));
                self.tag_contract_prop(self.self_is_bv_internal && !is_concrete);
            }
        }

        // Emit well-formedness checks for choice expressions in let bindings.
        // This must happen AFTER preconditions are assumed, so that preconditions
        // like `requires exists y: T: pred(y)` can establish witnesses for choices.
        self.emit_choice_wellformedness(spec, false);

        if self.is_verified() {
            // Inject 'CanModify' assumptions for this function.
            for (loc, exp) in &spec.modifies {
                let struct_ty = self
                    .builder
                    .global_env()
                    .get_node_type(exp.node_id())
                    .skip_reference()
                    .clone();
                self.generate_modifies_check(
                    PropKind::Assume,
                    spec,
                    loc,
                    &struct_ty,
                    exp,
                    exp.call_args()[0].to_owned(),
                );
            }
        }

        // Instrument and generate new code.
        // Peel leading TraceLocal instructions and emit them before pre_proof
        // so that pre-proof assertion errors can display model values in counter-examples.
        //
        // Skip proof-action emission entirely during spec inference: proof
        // blocks are a verification-only construct, and emitting their props
        // (or accumulating their split points) would cause the inference
        // walker to fork per case and produce per-variant inferred specs.
        let emit_proof = !self.options.inference;
        let mut old_code = old_code.into_iter();
        let mut pre_proof_emitted = false;
        for bc in old_code.by_ref() {
            if matches!(&bc, Call(_, _, Operation::TraceLocal(_), _, _)) {
                self.builder.emit(bc);
            } else {
                // First non-trace instruction: emit pre_proof, then this instruction.
                if emit_proof {
                    self.emit_proof_actions(&spec.pre_proof, spec);
                }
                pre_proof_emitted = true;
                self.instrument_bytecode(spec, inlined_props, bc);
                break;
            }
        }
        if !pre_proof_emitted && emit_proof {
            self.emit_proof_actions(&spec.pre_proof, spec);
        }
        // Continue with remaining old_code.
        for bc in old_code {
            self.instrument_bytecode(spec, inlined_props, bc);
        }

        // The return assertion block is emitted in-place at the trailing `Ret`
        // (see `instrument_bytecode`). Only the abort exit needs a separate
        // out-of-line block at this point.
        if self.can_abort {
            self.generate_abort_block(spec);
        }
    }

    fn instrument_bytecode(
        &mut self,
        spec: &TranslatedSpec,
        inlined_props: &BTreeMap<AttrId, (TranslatedSpec, Exp)>,
        bc: Bytecode,
    ) {
        use Bytecode::*;
        use Operation::*;

        // Prefix with modifies checks for builtin memory modifiers. Notice that we assume
        // the BorrowGlobal at this point represents a mutation and immutable references have
        // been removed.
        match &bc {
            Call(id, _, BorrowGlobal(mid, sid, targs), srcs, _)
            | Call(id, _, MoveFrom(mid, sid, targs), srcs, _) => {
                let addr_exp = self.builder.mk_temporary(srcs[0]);
                self.generate_modifies_check(
                    PropKind::Assert,
                    spec,
                    &self.builder.get_loc(*id),
                    &Type::Struct(*mid, *sid, targs.to_owned()),
                    &addr_exp,
                    addr_exp.clone(),
                );
            },
            Call(id, _, MoveTo(mid, sid, targs), srcs, _) => {
                let addr_exp = self.builder.mk_temporary(srcs[0]);
                self.generate_modifies_check(
                    PropKind::Assert,
                    spec,
                    &self.builder.get_loc(*id),
                    &Type::Struct(*mid, *sid, targs.to_owned()),
                    &addr_exp,
                    addr_exp.clone(),
                );
            },
            _ => {},
        }

        // Instrument bytecode.
        match bc {
            Ret(id, results) => {
                self.builder.set_loc_from_attr(id);
                // Emit the spec-decoration block (lets, updates, ensures
                // assertions, emits checks, ...) in-place before the actual
                // return. After `NormalizeExitsProcessor` this is the unique
                // exit point of the function.
                debug_assert_eq!(self.ret_locals, results);
                self.emit_return_assertions(spec);
                self.builder.emit(Ret(id, results));
            },
            Abort(id, code, _) => {
                self.builder.set_loc_from_attr(id);
                let abort_local = self.abort_local;
                let abort_label = self.abort_label;
                self.builder
                    .emit_with(|id| Assign(id, abort_local, code, AssignKind::Move));
                self.builder.emit_with(|id| Jump(id, abort_label));
                self.can_abort = true;
            },
            Call(id, dests, Function(mid, fid, targs), srcs, aa) => {
                self.instrument_call(spec, id, dests, mid, fid, targs, srcs, aa);
            },
            Call(id, dests, Invoke, srcs, _) => {
                // Havoc before `$apply` so the closure's postconditions remain
                // available after the invocation.
                self.emit_global_havocs(self.opaque_closure_modifies.clone());
                self.builder.emit(Call(
                    id,
                    dests,
                    Invoke,
                    srcs,
                    Some(AbortAction(self.abort_label, self.abort_local)),
                ));
                self.can_abort = true;
            },
            Call(id, dests, oper, srcs, _) if oper.can_abort() => {
                self.builder.emit(Call(
                    id,
                    dests,
                    oper,
                    srcs,
                    Some(AbortAction(self.abort_label, self.abort_local)),
                ));
                self.can_abort = true;
            },
            Prop(id, kind @ PropKind::Assume, prop) | Prop(id, kind @ PropKind::Assert, prop) => {
                if let ExpData::Call(
                    _,
                    move_model::ast::Operation::SaveStateAnchor(label)
                    | move_model::ast::Operation::FoldsCaptureAnchor(label),
                    _,
                ) = prop.as_ref()
                {
                    // Emit the state saves registered under this anchor label
                    // by any property anchored at this program point, and
                    // drop the marker. A property's saves under other labels
                    // are emitted elsewhere: at the matching marker for other
                    // anchor labels, at function entry for the rest.
                    let mut saved_mems = BTreeSet::new();
                    let mut saved_vars = BTreeSet::new();
                    for translated_spec in inlined_props
                        .values()
                        .map(|(s, _)| s)
                        .filter(|s| s.anchors.contains(label))
                    {
                        self.emit_state_saves(
                            translated_spec,
                            Some(*label),
                            &mut saved_mems,
                            &mut saved_vars,
                        );
                    }
                    return;
                }
                if matches!(
                    prop.as_ref(),
                    ExpData::Call(_, move_model::ast::Operation::InlineCallSummary, _,)
                ) {
                    return;
                }
                match inlined_props.get(&id) {
                    None => {
                        self.builder.emit(Prop(id, kind, prop));
                    },
                    Some((translated_spec, exp)) => {
                        let binding = self.builder.fun_env.get_spec();
                        let cond_opt = binding.update_map.get(&prop.node_id());
                        if cond_opt.is_some() {
                            self.emit_updates(translated_spec, Some(prop));
                        } else {
                            self.emit_traces(translated_spec, exp);
                            self.builder.emit(Prop(id, kind, exp.clone()));
                        }
                    },
                }
            },
            _ => self.builder.emit(bc),
        }
    }

    fn instrument_call(
        &mut self,
        spec: &TranslatedSpec,
        id: AttrId,
        dests: Vec<TempIndex>,
        mid: ModuleId,
        fid: FunId,
        targs: Vec<Type>,
        srcs: Vec<TempIndex>,
        aa: Option<AbortAction>,
    ) {
        use Bytecode::*;
        use PropKind::*;
        let targs = &targs; // linter does not allow `ref targs` parameter

        let env = self.builder.global_env();

        let callee_env = env.get_module(mid).into_function(fid);
        let callee_opaque = callee_env.is_opaque();
        let callee_is_bv_internal = callee_env.is_pragma_true(BV_INTERNAL_PRAGMA, || false);
        let mut callee_spec = SpecTranslator::translate_fun_spec(
            self.options.auto_trace_level.functions(),
            true,
            &mut self.builder,
            &callee_env,
            targs,
            Some(&srcs),
            &dests,
        );

        // Function specs cannot contain `WithStateAnchor` wrappers (those only
        // arise in inline spec-block properties), so all of the callee spec's
        // saves are entry-scoped and can be emitted at the call site below.
        debug_assert!(
            callee_spec.anchors.is_empty() && callee_spec.anchored_saved_params.is_empty(),
            "unexpected state anchors in callee function spec"
        );

        // Freshen state labels to avoid collisions between different inlining
        // sites. Labels come from the environment's global id allocator, like
        // all other memory labels (e.g. inline-expansion anchor labels): an
        // independent counter could collide with those, letting one label's
        // memory snapshot overwrite another's.
        callee_spec.freshen_labels(env);

        self.builder.set_loc_from_attr(id);

        if ProverOptions::get(self.builder.global_env()).inline_spec_lets {
            self.inline_lets(&mut callee_spec, true);
        }

        // Emit `let` assignments.
        self.emit_lets(&callee_spec, false, callee_is_bv_internal);
        self.builder.set_loc_from_attr(id);

        // Callee preconditions are the caller's obligation in every variant. A Baseline
        // body is inlined into the verified roots that call it, so its assertions are
        // discharged there.
        for (loc, cond) in callee_spec.pre_conditions(&self.builder) {
            self.emit_traces(&callee_spec, &cond);
            self.builder
                .set_loc_and_vc_info(loc, REQUIRES_FAILS_MESSAGE);
            self.builder.emit_with(|id| Prop(id, Assert, cond));
            self.tag_contract_prop(callee_is_bv_internal);
        }

        // Emit well-formedness checks for choice expressions in callee's pre-state let bindings.
        // This must happen AFTER preconditions are asserted, so that preconditions
        // like `requires exists y: T: pred(y)` can establish witnesses for choices.
        if callee_opaque {
            self.emit_choice_wellformedness(&callee_spec, false);
        }

        // Emit modify permissions as assertions if this is the verification variant. For
        // non-verification variants, we don't need to do this because they are independently
        // verified.
        if self.is_verified() {
            let loc = self.builder.get_loc(id);
            for (_, exp) in &callee_spec.modifies {
                let rty = env.get_node_type(exp.node_id()).skip_reference().clone();
                let new_addr = exp.call_args()[0].clone();
                self.generate_modifies_check(
                    PropKind::Assert,
                    &callee_spec,
                    &loc,
                    &rty,
                    exp,
                    new_addr,
                );
            }
        }

        // From here on code differs depending on whether the callee is opaque or not.
        if !callee_env.is_opaque() || self.options.for_interpretation || self.options.inference {
            self.builder.emit(Call(
                id,
                dests,
                Operation::Function(mid, fid, targs.clone()),
                srcs,
                Some(AbortAction(self.abort_label, self.abort_local)),
            ));
            self.can_abort = true;
        } else {
            // Generates OpaqueCallBegin.
            self.generate_opaque_call(
                dests.clone(),
                mid,
                fid,
                targs,
                srcs.clone(),
                aa.clone(),
                true,
            );

            // The aggregated behavioral predicates assumed on the success
            // path (see below) take the callee's pre-state arguments, which
            // for `&mut` arguments are gone once the call havocs them: save
            // those too.
            let callee_is_higher_order = callee_env
                .get_parameters()
                .iter()
                .any(|p| matches!(p.1.skip_reference(), Type::Fun(..)));
            let emits_behavior_aggregate = find_behavior_pre_label_for_callee(spec, mid, fid)
                .is_some()
                && !callee_is_higher_order;
            // The aggregates describe THIS call, so they are stated over the
            // memory at this call: a fresh label, saved here, for every
            // resource the callee's evaluator takes. The enclosing
            // specification's own occurrence of the predicate keeps its entry
            // label; the two name the same state exactly when nothing the
            // callee reads was written between entry and the call, which is
            // what makes a caller's `aborts_if aborts_of<callee>(..)` true or
            // false. (Binding the aggregates to the entry label instead let a
            // write before the call be ignored: the success of a call on the
            // written state was assumed over the entry state, and a later
            // abort on the same state was pruned as contradictory.)
            let entry_label = if emits_behavior_aggregate {
                for src in &srcs {
                    if self.builder.data.local_types[*src].is_mutable_reference() {
                        let ty = self.builder.get_local_type(*src).skip_reference().clone();
                        callee_spec
                            .saved_params
                            .entry(*src)
                            .or_insert_with(|| self.builder.new_temp(ty));
                    }
                }
                let env = self.builder.global_env();
                let label = MemoryLabel::new(env.new_global_id().as_usize());
                let callee_qid = mid.qualified(fid);
                for mem in spec_derivation::behavioral_target_memory(env, callee_qid, targs) {
                    callee_spec.saved_memory.insert((mem, label));
                }
                Some(label)
            } else {
                None
            };

            // Emit saves for parameters used in old(..) context. Those can be referred
            // to in aborts conditions, and must be initialized before evaluating those.
            self.emit_save_for_old(&callee_spec.saved_params);

            // Emit memory and spec var saves BEFORE label defines. The label-defining
            // ensures conditions reference these saved labels (e.g., global[@0] maps to
            // a SaveMem'd memory). They must be initialized before the label defines
            // and abort condition evaluation — on BOTH the abort and success paths.
            if self.world_havoc_callees.contains(&mid.qualified(fid))
                || self.builder.global_env().world_memory().is_some()
            {
                add_world_saves(self.builder.global_env(), &mut callee_spec);
            }
            for (mem, label) in std::mem::take(&mut callee_spec.saved_memory) {
                self.builder.emit_with(|id| SaveMem(id, label, mem));
            }
            for (var, label) in std::mem::take(&mut callee_spec.saved_spec_vars) {
                self.builder.emit_with(|id| SaveSpecVar(id, label, var));
            }

            // Emit state label defining conditions BEFORE the abort condition.
            // This constrains abstract memory labels that may be referenced by
            // aborts_if conditions. Uses abort_path=true to emit only the
            // defining fragment (label-defining conjuncts), not full postcondition
            // properties that reference post-havoc state.
            self.emit_state_label_assumes(&callee_spec, true, callee_is_bv_internal);
            // Keep the full postconditions for the successful-call path: the
            // definitions above deliberately omit their final-state properties.

            let callee_aborts_if_is_partial =
                callee_env.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false);

            // Translate the abort condition. If the abort_cond_temp_opt is None, it indicates
            // that the abort condition is known to be false, so we can skip the abort handling.
            let (abort_cond_temp_opt, code_cond) = self.generate_abort_opaque_cond(
                callee_aborts_if_is_partial,
                &callee_spec,
                callee_is_bv_internal,
            );
            if let Some(abort_cond_temp) = abort_cond_temp_opt {
                let abort_local = self.abort_local;
                let abort_label = self.abort_label;
                let no_abort_label = self.builder.new_label();
                let abort_here_label = self.builder.new_label();
                self.builder
                    .emit_with(|id| Branch(id, abort_here_label, no_abort_label, abort_cond_temp));
                self.builder.emit_with(|id| Label(id, abort_here_label));
                if let Some(cond) = code_cond {
                    self.emit_traces(&callee_spec, &cond);
                    self.builder.emit_with(move |id| Prop(id, Assume, cond));
                    self.tag_contract_prop(callee_is_bv_internal);
                }
                // Aggregate `aborts_of<callee>` on the abort path so a
                // caller's `aborts_of<callee>` assertion discharges directly,
                // over the memory saved at this call. See the success-path
                // comment below `OpaqueCall` end for soundness and the
                // gating rationale.
                if emits_behavior_aggregate {
                    let abort_arg_exps: Vec<Exp> =
                        srcs.iter().map(|s| self.builder.mk_temporary(*s)).collect();
                    let (abort_closure_exp, _) =
                        self.builder
                            .mk_closure(mid, fid, targs, ClosureMask::empty(), vec![]);
                    let abort_aborts_exp = self.builder.mk_aborts_of_with_state(
                        abort_closure_exp,
                        abort_arg_exps,
                        entry_label,
                        None,
                    );
                    self.builder
                        .emit_with(|id| Prop(id, Assume, abort_aborts_exp));
                }
                self.builder.emit_with(move |id| {
                    Call(id, vec![], Operation::TraceAbort, vec![abort_local], None)
                });
                self.builder.emit_with(|id| Jump(id, abort_label));
                self.builder.emit_with(|id| Label(id, no_abort_label));
                self.can_abort = true;
            }

            // Note: saved_memory and saved_spec_vars were already emitted above,
            // before the label defines and abort condition evaluation.

            // Emit precise, address-level memory havocs from the callee frame.
            for (_, exp) in std::mem::take(&mut callee_spec.modifies) {
                self.emit_traces(&callee_spec, &exp);
                self.builder.emit_with(|id| Prop(id, Modifies, exp));
            }

            // An omitted address frame on an opaque callee is conservatively
            // represented by havocing the complete memory for that resource.
            // This preserves soundness while allowing coarse modular summaries.
            let callee_qid = mid.qualified(fid);
            let framed_memory = self
                .opaque_callee_framed_memory
                .get(&callee_qid)
                .cloned()
                .unwrap_or_default();
            let unframed_memory = self
                .opaque_callee_modifies
                .get(&callee_qid)
                .into_iter()
                .flatten()
                .filter(|mem| !framed_memory.contains(mem))
                .map(|mem| mem.instantiate_ref(targs))
                .collect_vec();
            self.emit_global_havocs(unframed_memory);

            // What a function-typed argument may modify: at least the frame the callee
            // declares for the parameter. A known closure footprint may replace it: it covers
            // everything the closure writes, in any module; where the frame check runs, it also
            // lies within that frame.
            let mut arg_writes = BTreeSet::new();
            for (param, src) in callee_env.get_parameters().iter().zip(&srcs) {
                if !param.1.skip_reference().is_function() {
                    continue;
                }
                let footprint = self.closure_footprints.get(src);
                if let Some(footprint) = footprint {
                    arg_writes.extend(footprint.writes.iter().cloned());
                }
                if !footprint.is_some_and(|f| f.known) {
                    let frame = DeclaredFrame::of_fun_param(&callee_env, param.0, targs);
                    arg_writes.extend(frame.writes);
                }
            }
            self.emit_global_havocs(arg_writes);

            // A callee running a function value nothing bounds may touch any memory, whatever
            // its `modifies` frame says: callers never rely on that frame.
            if self.world_havoc_callees.contains(&callee_qid) {
                self.emit_world_havoc();
            }

            // Havoc all &mut parameters, their post-value are to be determined by the post
            // conditions.
            //
            // There is some special case here about EventHandle types. Even though
            // they are `&mut`, they are never modified, and this is not expressed in the
            // specifications. We treat this by skipping the Havoc for them. TODO: find a better
            // solution
            let mut_srcs = srcs
                .iter()
                .cloned()
                .filter(|src| {
                    let ty = &self.builder.data.local_types[*src];
                    ty.is_mutable_reference()
                        && !self
                            .builder
                            .global_env()
                            .is_wellknown_event_handle_type(ty.skip_reference())
                })
                .collect_vec();
            for src in &mut_srcs {
                self.builder.emit_with(|id| {
                    Call(
                        id,
                        vec![*src],
                        Operation::Havoc(HavocKind::MutationValue),
                        vec![],
                        None,
                    )
                });
            }

            // Emit placeholders for assuming well-formedness of return values and mutable ref
            // parameters.
            for idx in mut_srcs.into_iter().chain(dests.iter().cloned()) {
                let exp = self
                    .builder
                    .mk_call(&BOOL_TYPE, ast::Operation::WellFormed, vec![self
                        .builder
                        .mk_temporary(idx)]);
                self.builder.emit_with(move |id| Prop(id, Assume, exp));
            }

            // Emit `let post` assignments.
            self.emit_lets(&callee_spec, true, callee_is_bv_internal);

            // Emit well-formedness checks for choice expressions in callee's post-state let bindings.
            self.emit_choice_wellformedness(&callee_spec, true);

            // Emit spec var updates.
            self.emit_updates(&callee_spec, None);

            // Emit post conditions as assumptions.
            for (_, cond) in std::mem::take(&mut callee_spec.post) {
                self.emit_traces(&callee_spec, &cond);
                self.builder.emit_with(|id| Prop(id, Assume, cond));
                self.tag_contract_prop(callee_is_bv_internal);
            }

            // Emit the events in the `emits` specs of the callee.
            for (_, msg, handle, cond) in std::mem::take(&mut callee_spec.emits) {
                self.emit_traces(&callee_spec, &msg);
                self.emit_traces(&callee_spec, &handle);
                if let Some(c) = &cond {
                    self.emit_traces(&callee_spec, c);
                }

                let temp_msg = self.builder.emit_let(msg).0;
                // the event handle needs to be let-bound without the reference as we do not want
                // to create mutable references via Assume(Identical) as it complicates both
                // the live var analysis and the borrow analysis.
                let temp_handle = self.builder.emit_let_skip_reference(handle).0;

                let mut temp_list = vec![temp_msg, temp_handle];
                if let Some(cond) = cond {
                    temp_list.push(self.builder.emit_let(cond).0);
                }
                self.builder
                    .emit(Call(id, vec![], Operation::EmitEvent, temp_list, None));
            }
            // TODO: We treat the emits spec of a opaque function "strictly" for convenience,
            //   ignoring its own EMITS_IS_STRICT_PRAGMA flag.
            if callee_env.is_pragma_true(EMITS_IS_PARTIAL_PRAGMA, || false) {
                self.builder
                    .emit(Call(id, vec![], Operation::EventStoreDiverge, vec![], None));
            }

            // Aggregate the per-clause post-state assumes into the callee's
            // behavioral predicates `ensures_of<callee>` and `!aborts_of<callee>`
            // on the success path. The per-clause assumes already establish
            // the body of these predicates with the call-site memory labels
            // as concrete witnesses; emitting these aggregated assumes lets a
            // caller's `ensures_of<callee>` / `aborts_of<callee>` assertion
            // discharge directly (rather than forcing the SMT solver to
            // re-Skolemize the existential over intermediate state labels at
            // each caller's exit — which without this triggers runaway
            // quantifier instantiations and timeouts). Sound because the
            // predicates are defined as the existential closure of the same
            // conjunction we just assumed.
            //
            // Skip when the callee has a function-typed parameter: in that
            // case the callee's `bp_*_of` body delegates to a generic
            // dispatcher whose memory-arg handling is invoked via the
            // per-clause assumes above; emitting a parallel `bp_*_of<callee>`
            // assume would use the dispatcher with weaker memory args
            // (current memory for both pre and post) and create an
            // inconsistent context.
            //
            // Only emit when the enclosing spec already references this
            // callee via `ensures_of<callee>` or `aborts_of<callee>` — that
            // guarantees `spec.saved_memory` has the right pre-state label
            // for the callee's used memory (so the emitted memory args match
            // the assertion's at the caller's exit). For callees the
            // enclosing spec doesn't mention, our assume would have nothing
            // to discharge and risks using a mismatched label.
            if emits_behavior_aggregate {
                // Pre-state arguments: a `&mut` argument holds its post-state
                // here, its pre-state was saved before the call.
                let arg_exps: Vec<Exp> = srcs
                    .iter()
                    .map(|s| {
                        let pre = if self.builder.data.local_types[*s].is_mutable_reference() {
                            callee_spec.saved_params[s]
                        } else {
                            *s
                        };
                        self.builder.mk_temporary(pre)
                    })
                    .collect();
                let (closure_exp, _) =
                    self.builder
                        .mk_closure(mid, fid, targs, ClosureMask::empty(), vec![]);
                // ensures_of takes args followed by results: explicit dests
                // first, then post-values of &mut srcs (matches the order
                // produced by `behavioral_output_types` in the Boogie backend).
                let mut ensures_args = arg_exps.clone();
                for &dest in &dests {
                    ensures_args.push(self.builder.mk_temporary(dest));
                }
                for &src in &srcs {
                    if self.builder.data.local_types[src].is_mutable_reference() {
                        ensures_args.push(self.builder.mk_temporary(src));
                    }
                }
                let ensures_exp = self.builder.mk_ensures_of_with_state(
                    closure_exp.clone(),
                    ensures_args,
                    entry_label,
                    None,
                );
                self.builder.emit_with(|id| Prop(id, Assume, ensures_exp));
                let aborts_exp =
                    self.builder
                        .mk_aborts_of_with_state(closure_exp, arg_exps, entry_label, None);
                let not_aborts_exp = self.builder.mk_not(aborts_exp);
                self.builder
                    .emit_with(|id| Prop(id, Assume, not_aborts_exp));
            }

            // Generate OpaqueCallEnd instruction if invariant_v2.
            // A memory-free Move call is a deterministic function of its inputs.
            // Its actual return is the result_of carrier even when its stated
            // postconditions do not uniquely determine that return. The ordinary
            // contract still constrains the carrier only on successful calls.
            let uses_result_carrier = spec
                .pre
                .iter()
                .chain(&spec.post)
                .map(|(_, e)| e)
                .chain(spec.aborts.iter().map(|(_, e, _)| e))
                .any(|exp| {
                    exp.any(&mut |node| {
                        let ExpData::Call(
                            _,
                            ast::Operation::Behavior(ast::BehaviorKind::ResultOf, _),
                            args,
                        ) = node
                        else {
                            return false;
                        };
                        matches!(args.first().map(|a| a.as_ref()),
                        Some(ExpData::Call(_, ast::Operation::Closure(cmid, cfid, _), _))
                        if *cmid == mid && *cfid == fid)
                    })
                });
            if uses_result_carrier
                && self
                    .memory_free_callees
                    .contains(&callee_env.get_qualified_id())
            {
                let args = srcs
                    .iter()
                    .map(|s| self.builder.mk_temporary(*s))
                    .collect::<Vec<_>>();
                let (closure, _) =
                    self.builder
                        .mk_closure(mid, fid, targs, ClosureMask::empty(), vec![]);
                let result_ty = callee_env.get_result_type().instantiate(targs);
                for (i, dest) in dests.iter().enumerate() {
                    let result = self.builder.mk_result_of_at_with_state(
                        closure.clone(),
                        args.clone(),
                        &result_ty,
                        i,
                        dests.len(),
                        None,
                        None,
                    );
                    let actual = self.builder.mk_temporary(*dest);
                    let equality = self.builder.mk_eq(actual, result);
                    self.builder.emit_with(|id| Prop(id, Assume, equality));
                    self.tag_contract_prop(callee_is_bv_internal);
                }
            }
            self.generate_opaque_call(dests, mid, fid, targs, srcs, aa, false);
        }
    }

    /// Mark the `Prop` just emitted as a `bv_internal` contract condition (`pre`, `post`,
    /// `aborts`, `lets`) — the only ones proven in one context and assumed in another.
    /// `owner_is_bv_internal` is the callee at a call site, not the enclosing function.
    /// Whether `loc` is a `[concrete]` condition of this function.
    fn is_concrete_cond(&self, loc: &Loc) -> bool {
        self.concrete_cond_locs.contains(loc)
    }

    fn tag_contract_prop(&mut self, owner_is_bv_internal: bool) {
        if !owner_is_bv_internal {
            return;
        }
        let Some(Bytecode::Prop(attr_id, ..)) = self.builder.data.code.last() else {
            return;
        };
        let attr_id = *attr_id;
        let env = self.builder.global_env();
        let mut boundary = env
            .get_extension::<BvInternalBoundaryProps>()
            .map(|rc| rc.as_ref().clone())
            .unwrap_or_default();
        boundary.insert(
            self.builder.fun_env.module_env.get_id(),
            self.builder.fun_env.get_id(),
            self.builder.data.variant == FunctionVariant::Baseline,
            attr_id,
        );
        env.set_extension(boundary);
    }
}

// =================================================================================================
// # Spec Condition Emission

impl<'a> Instrumenter<'a> {
    /// Emits the state save instructions (`SaveMem`, `SaveSpecVar`, and
    /// parameter saves) of the given translated spec at the current program
    /// point. With `at_anchor == None` (function entry), only the entries
    /// whose label is not one of the spec's anchor labels are emitted; with
    /// `at_anchor == Some(label)` (a `SaveStateAnchor` marker), only the
    /// entries registered under that anchor label. The sets dedup saves
    /// across multiple specs emitted at the same program point.
    fn emit_state_saves(
        &mut self,
        translated_spec: &TranslatedSpec,
        at_anchor: Option<MemoryLabel>,
        saved_mems: &mut BTreeSet<(QualifiedInstId<StructId>, MemoryLabel)>,
        saved_vars: &mut BTreeSet<(QualifiedInstId<SpecVarId>, MemoryLabel)>,
    ) {
        use Bytecode::*;
        let label_selected = |label: &MemoryLabel| match &at_anchor {
            Some(anchor) => label == anchor,
            None => !translated_spec.anchors.contains(label),
        };
        for (mem, label) in &translated_spec.saved_memory {
            if label_selected(label) && saved_mems.insert((mem.clone(), *label)) {
                let mem = mem.clone();
                let label = *label;
                self.builder
                    .emit_with(|attr_id| SaveMem(attr_id, label, mem));
            }
        }
        for (spec_var, label) in &translated_spec.saved_spec_vars {
            if label_selected(label) && saved_vars.insert((spec_var.clone(), *label)) {
                let spec_var = spec_var.clone();
                let label = *label;
                self.builder
                    .emit_with(|attr_id| SaveSpecVar(attr_id, label, spec_var));
            }
        }
        let saved_params = match at_anchor {
            None => translated_spec.saved_params.clone(),
            Some(anchor) => translated_spec
                .anchored_saved_params
                .get(&anchor)
                .cloned()
                .unwrap_or_default(),
        };
        self.emit_save_for_old(&saved_params);
    }

    /// Saves the world memory under every function-entry label `translated_spec` saves memory
    /// under (see `add_world_saves`).
    fn emit_world_saves(
        &mut self,
        translated_spec: &TranslatedSpec,
        saved_mems: &mut BTreeSet<(QualifiedInstId<StructId>, MemoryLabel)>,
    ) {
        let Some(world) = self.builder.global_env().world_memory() else {
            return;
        };
        let labels: BTreeSet<MemoryLabel> = translated_spec
            .saved_memory
            .iter()
            .map(|(_, label)| *label)
            .filter(|label| !translated_spec.anchors.contains(label))
            .collect();
        for label in labels {
            if saved_mems.insert((world.clone(), label)) {
                let mem = world.clone();
                self.builder
                    .emit_with(|attr_id| Bytecode::SaveMem(attr_id, label, mem));
            }
        }
    }

    fn emit_save_for_old(&mut self, vars: &BTreeMap<TempIndex, TempIndex>) {
        use Bytecode::*;
        for (idx, saved_idx) in vars {
            if self.builder.data.local_types[*idx].is_reference() {
                self.builder.emit_with(|attr_id| {
                    Call(
                        attr_id,
                        vec![*saved_idx],
                        Operation::ReadRef,
                        vec![*idx],
                        None,
                    )
                })
            } else {
                self.builder
                    .emit_with(|attr_id| Assign(attr_id, *saved_idx, *idx, AssignKind::Copy))
            }
        }
    }

    /// Emits an update of a ghost field (`update base.g = rhs`), where `base`
    /// is a local or parameter, possibly a mutable reference: the value is
    /// rebuilt via `UpdateField` and stored back — a plain assignment for an
    /// owned local, a `WriteRef` through a reference.
    fn emit_ghost_field_update(&mut self, lhs: &Exp, rhs: &Exp) {
        let (sel_node, mid, sid, fid, base) =
            if let ExpData::Call(id, ast::Operation::Select(mid, sid, fid), args) = lhs.as_ref() {
                (*id, *mid, *sid, *fid, args[0].clone())
            } else {
                unreachable!("ghost field update lhs must be a field selection")
            };
        let base_idx = if let ExpData::Temporary(_, idx) = base.as_ref() {
            *idx
        } else {
            unreachable!("ghost field update base must be a local or parameter")
        };
        let base_ty = self.builder.get_local_type(base_idx);
        let struct_ty = base_ty.skip_reference().clone();
        let inst = self.builder.global_env().get_node_instantiation(sel_node);
        // Bitwise-classified RHS expressions were rejected at model build
        // time (`module_builder`): a `bv<N>`-typed argument to `$Update...`
        // against the ghost's Boogie `int` field would be ill-typed.
        // References are implicitly dereferenced in specification expressions,
        // so the temporary can be used directly regardless of its kind.
        let base_val = self.builder.mk_temporary(base_idx);
        let update_loc = self.builder.get_current_loc();
        let env = self.builder.global_env();
        // The RHS expression carries a spec-arithmetic type (`num`); the
        // range to check is the FIELD's declared type, so type the WellFormed
        // argument with it.
        let field_ty = env
            .get_module(mid)
            .into_struct(sid)
            .get_field(fid)
            .get_type()
            .instantiate(&self.builder.global_env().get_node_instantiation(sel_node));
        // Assert the new value stays within the declared type's value domain:
        // $IsValid at boundaries includes ghost fields, which is only sound
        // if every ghost write is checked in range. The check is skipped
        // when it holds by construction (`num` targets, and pure copies of
        // already-well-formed values); computed values keep it. Skipping
        // also avoids perturbing the emitted Boogie of enclosing VCs.
        let rhs_val = if range_check_needed(env, &field_ty, rhs, &self.builder.data.local_types) {
            // The RHS may be a reference-typed local (implicitly dereferenced
            // in spec expressions), so bind its dereferenced value.
            let (rhs_temp, rhs_temp_exp) = self.builder.emit_let_skip_reference(rhs.clone());
            let wf_arg_node = env.new_node(update_loc.clone(), field_ty);
            let wf_node = env.new_node(update_loc.clone(), BOOL_TYPE.clone());
            let wf_exp = ExpData::Call(wf_node, ast::Operation::WellFormed, vec![
                ExpData::Temporary(wf_arg_node, rhs_temp).into_exp(),
            ])
            .into_exp();
            self.builder.set_loc_and_vc_info(
                update_loc,
                "ghost field update value is out of the field type's value domain",
            );
            self.builder
                .emit_with(|id| Bytecode::Prop(id, PropKind::Assert, wf_exp));
            rhs_temp_exp
        } else {
            rhs.clone()
        };
        let new_val = self.builder.mk_call_with_inst(
            &struct_ty,
            inst,
            ast::Operation::UpdateField(mid, sid, fid),
            vec![base_val, rhs_val],
        );
        let (new_temp, _) = self.builder.emit_let(new_val);
        if base_ty.is_reference() {
            self.builder.emit_with(|id| {
                Bytecode::Call(
                    id,
                    vec![],
                    Operation::WriteRef,
                    vec![base_idx, new_temp],
                    None,
                )
            });
        } else {
            self.builder
                .emit_with(|id| Bytecode::Assign(id, base_idx, new_temp, AssignKind::Store));
        }
        // Trigger the data-invariant instrumentation pass to assert struct
        // invariants over the updated value. Without this, an invariant
        // constraining the ghost field (e.g. `invariant self.g == self.x`)
        // would be silently violable by a ghost update — and downstream
        // `WellFormed(x)` at opaque calls would then propagate the (violated)
        // invariant, letting callers certify false postconditions.
        self.builder
            .emit_with(|id| Bytecode::Call(id, vec![], Operation::PackRef, vec![base_idx], None));
    }

    fn emit_updates(&mut self, spec: &TranslatedSpec, prop_rhs_opt: Option<Exp>) {
        for (loc, lhs, rhs) in &spec.updates {
            // Emit update of lhs, which is guaranteed to represent a ghost memory access.
            // We generate the actual byte code operations which would appear on a regular
            // memory update, such that subsequent phases (like invariant instrumentation)
            // interpret this like any other memory access.
            self.builder.set_loc(loc.clone());
            self.emit_traces(spec, lhs);
            let new_rhs = if let Some(ref prop_rhs) = prop_rhs_opt {
                prop_rhs
            } else {
                rhs
            };
            self.emit_traces(spec, new_rhs);

            if lhs
                .extract_ghost_mem_access(self.builder.global_env())
                .is_none()
            {
                // Not a ghost variable access: the builder guarantees the
                // only other update target is a ghost field of a local. When
                // the update comes from an inline spec block, the maintained
                // Prop expression carries the current temporaries as
                // `Eq(fresh_lhs, unlowered_rhs)` (the fun-spec copy has
                // stale temporaries; the pipeline renumbers Props but not
                // spec entries). But the fun-spec `rhs` has the properly
                // lowered form of `old(...)` etc. Combine: use `fresh_lhs`
                // and substitute the fun-spec's `rhs` to swap stale temps
                // for the fresh ones observable in `fresh_lhs`.
                if let Some(ExpData::Call(_, ast::Operation::Eq, prop_args)) =
                    prop_rhs_opt.as_ref().map(|e| e.as_ref())
                {
                    let fresh_lhs = &prop_args[0];
                    let fresh_rhs = &prop_args[1];
                    let subst_rhs = align_temps(
                        self.builder.global_env(),
                        lhs.as_ref(),
                        fresh_lhs.as_ref(),
                        fresh_rhs.as_ref(),
                        rhs,
                    );
                    self.emit_ghost_field_update(fresh_lhs, &subst_rhs);
                } else {
                    self.emit_ghost_field_update(lhs, new_rhs);
                }
                continue;
            }

            // Extract the ghost mem from lhs
            let (ghost_mem, _field_id, addr) = lhs
                .extract_ghost_mem_access(self.builder.global_env())
                .expect("lhs of update valid");
            let ghost_mem_ty = ghost_mem.to_type();

            // Assert the new value stays in the spec variable's declared
            // type domain (the ghost memory struct's single field). $IsValid
            // over ghost memory is assumed at every function entry, which is
            // only sound if every update is checked in range. Use `num` for
            // an unbounded spec variable. The RHS may be a reference-typed
            // local (implicitly dereferenced in spec expressions), so bind
            // its dereferenced value.
            let env = self.builder.global_env();
            let var_ty = env
                .get_module(ghost_mem.module_id)
                .into_struct(ghost_mem.id)
                .get_field_by_offset(0)
                .get_type()
                .instantiate(&ghost_mem.inst);
            let update_loc = self.builder.get_current_loc();
            // The range check is skipped when it holds by construction
            // (`num` targets, and pure copies of already-well-formed
            // values); computed values keep it. Skipping also leaves the
            // emitted Boogie of enclosing VCs unperturbed.
            let rhs_val =
                if range_check_needed(env, &var_ty, new_rhs, &self.builder.data.local_types) {
                    // The RHS may be a reference-typed local (implicitly
                    // dereferenced in spec expressions), so bind its
                    // dereferenced value.
                    let (rhs_val_temp, rhs_val_exp) =
                        self.builder.emit_let_skip_reference(new_rhs.clone());
                    let wf_arg_node = env.new_node(update_loc.clone(), var_ty);
                    let wf_node = env.new_node(update_loc.clone(), BOOL_TYPE.clone());
                    let wf_exp = ExpData::Call(wf_node, ast::Operation::WellFormed, vec![
                        ExpData::Temporary(wf_arg_node, rhs_val_temp).into_exp(),
                    ])
                    .into_exp();
                    self.builder.set_loc_and_vc_info(
                        update_loc,
                        "spec variable update value is out of the variable type's value domain",
                    );
                    self.builder
                        .emit_with(|id| Bytecode::Prop(id, PropKind::Assert, wf_exp));
                    rhs_val_exp
                } else {
                    new_rhs.clone()
                };

            // Construct new ghost mem struct value from rhs. We assign the struct value
            // directly, ignoring the `_field_id`. This is currently possible because
            // each ghost memory struct contains exactly one field. Should this change,
            // this code here needs to be generalized.
            let (rhs_temp, _) = self.builder.emit_let(self.builder.mk_call_with_inst(
                &ghost_mem_ty,
                ghost_mem.inst.clone(),
                ast::Operation::Pack(ghost_mem.module_id, ghost_mem.id, None),
                vec![rhs_val],
            ));

            // Update memory. We create a mut ref for the location then write the value back to it.
            let (addr_temp, _) = self.builder.emit_let(addr);
            let mem_ref = self.builder.new_temp(Type::Reference(
                ReferenceKind::Mutable,
                Box::new(ghost_mem_ty),
            ));
            // mem_ref = borrow_global_mut<ghost_mem>(addr)
            self.builder.emit_with(|id| {
                Bytecode::Call(
                    id,
                    vec![mem_ref],
                    Operation::BorrowGlobal(
                        ghost_mem.module_id,
                        ghost_mem.id,
                        ghost_mem.inst.clone(),
                    ),
                    vec![addr_temp],
                    None,
                )
            });
            // *mem_ref = rhs_temp
            self.builder.emit_with(|id| {
                Bytecode::Call(
                    id,
                    vec![],
                    Operation::WriteRef,
                    vec![mem_ref, rhs_temp],
                    None,
                )
            });
            // write_back[GhostMem](mem_ref)
            self.builder.emit_with(|id| {
                Bytecode::Call(
                    id,
                    vec![],
                    Operation::WriteBack(
                        BorrowNode::GlobalRoot(ghost_mem.clone()),
                        BorrowEdge::Direct,
                    ),
                    vec![mem_ref],
                    None,
                )
            });
        }
    }

    fn emit_lets(&mut self, spec: &TranslatedSpec, post_state: bool, owner_is_bv_internal: bool) {
        use Bytecode::*;
        let lets = spec
            .lets
            .iter()
            .filter(|(_, is_post, ..)| *is_post == post_state);
        for (loc, _, temp, exp) in lets {
            self.emit_traces(spec, exp);
            self.builder.set_loc(loc.to_owned());
            let assign = self
                .builder
                .mk_identical(self.builder.mk_temporary(*temp), exp.clone());
            self.builder
                .emit_with(|id| Prop(id, PropKind::Assume, assign));
            self.tag_contract_prop(owner_is_bv_internal);
        }
    }

    /// Inline spec let bindings into proof-block actions only, leaving
    /// `spec.lets` and all other conditions untouched. Proof actions are
    /// recorded out of band (notably `Split` exps in `split_points`), so
    /// the bytecode-level temp remap performed by later live-var analysis
    /// does not reach them. Substituting here replaces references to
    /// spec-let temps with their defining expressions, which use only
    /// Move-level temps that survive the remap with stable indices.
    ///
    /// `LetPre` bindings substituted into post-state proof actions
    /// (`post_proof`) are annotated with pre-state memory labels and have
    /// their parameter references remapped to saved versions, mirroring the
    /// treatment in `inline_lets`. Without this, a `let pre = R[a].f;`
    /// referenced from a `post { ... }` action would evaluate `R[a].f` at
    /// post-state instead of the saved entry-state snapshot.
    fn inline_lets_in_proof_actions(&mut self, spec: &mut TranslatedSpec) {
        if spec.pre_proof.is_empty() && spec.post_proof.is_empty() {
            return;
        }
        let env = self.builder.global_env();
        let substitute_exp =
            |env: &GlobalEnv, cond: &mut Exp, temp: TempIndex, replacement: &Exp| {
                let replacement = replacement.clone();
                let mut replacer =
                    |_id: move_model::model::NodeId, target: RewriteTarget| -> Option<Exp> {
                        if let RewriteTarget::Temporary(idx) = target {
                            if idx == temp {
                                return Some(replacement.clone());
                            }
                        }
                        None
                    };
                *cond = ExpRewriter::new(env, &mut replacer).rewrite_exp(cond.clone());
            };
        let substitute_proof_action =
            |env: &GlobalEnv, action: &mut ProofAction, temp, replacement: &Exp| match action {
                ProofAction::Assert(exp, _) | ProofAction::Assume(exp) => {
                    substitute_exp(env, exp, temp, replacement);
                },
                ProofAction::Split(exp, guard) => {
                    substitute_exp(env, exp, temp, replacement);
                    if let Some(g) = guard {
                        substitute_exp(env, g, temp, replacement);
                    }
                },
            };
        // Compute the post-state replacement for a `LetPre` lazily — only when
        // a let temp is actually referenced from a `post_proof` action. This
        // avoids allocating fresh `saved_memory` labels and `saved_params`
        // temps for every spec let, which would pollute the pre-state save
        // sets and bloat downstream Boogie translation even when no proof
        // action consumes them.
        let mut resolved_lets: Vec<(TempIndex, bool, Exp)> = Vec::new();
        for (_, is_post, temp, exp) in spec.lets.clone() {
            let mut pre_rhs = exp.clone();
            for (prev_temp, _, prev_pre) in &resolved_lets {
                substitute_exp(env, &mut pre_rhs, *prev_temp, prev_pre);
            }
            resolved_lets.push((temp, is_post, pre_rhs));
        }
        for (temp, is_post, pre_replacement) in &resolved_lets {
            for (_, action) in &mut spec.pre_proof {
                substitute_proof_action(env, action, *temp, pre_replacement);
            }
            if spec.post_proof.is_empty() {
                continue;
            }
            // Only compute the (potentially state-allocating) post replacement
            // if the temp is actually referenced from a post_proof action.
            let exp_uses_temp = |exp: &Exp, temp: TempIndex| {
                let mut found = false;
                exp.visit_pre_order(&mut |e| {
                    if let ExpData::Temporary(_, idx) = e {
                        if *idx == temp {
                            found = true;
                        }
                    }
                    !found
                });
                found
            };
            let referenced = spec.post_proof.iter().any(|(_, action)| match action {
                ProofAction::Assert(exp, _) | ProofAction::Assume(exp) => exp_uses_temp(exp, *temp),
                ProofAction::Split(exp, guard) => {
                    exp_uses_temp(exp, *temp)
                        || guard.as_ref().is_some_and(|g| exp_uses_temp(g, *temp))
                },
            });
            if !referenced {
                continue;
            }
            let post_replacement = if !*is_post {
                let annotated = Self::annotate_pre_state_memory(
                    env,
                    pre_replacement.clone(),
                    &mut spec.saved_memory,
                    &spec.anchors,
                );
                self.remap_params_to_saved(annotated, &mut spec.saved_params)
            } else {
                pre_replacement.clone()
            };
            for (_, action) in &mut spec.post_proof {
                substitute_proof_action(env, action, *temp, &post_replacement);
            }
        }
    }

    /// Inline let bindings by substituting the expression directly into conditions.
    /// For `LetPre` expressions in post-state conditions, annotates memory references
    /// with pre-state labels to preserve pre-state evaluation semantics.
    /// Clears `spec.lets` so subsequent `emit_lets` calls are no-ops.
    fn inline_lets(&mut self, spec: &mut TranslatedSpec, for_call: bool) {
        let env = self.builder.global_env();
        let mut lets = std::mem::take(&mut spec.lets);

        let substitute_exp =
            |env: &GlobalEnv, cond: &mut Exp, temp: TempIndex, replacement: &Exp| {
                let replacement = replacement.clone();
                let mut replacer =
                    |_id: move_model::model::NodeId, target: RewriteTarget| -> Option<Exp> {
                        if let RewriteTarget::Temporary(idx) = target {
                            if idx == temp {
                                return Some(replacement.clone());
                            }
                        }
                        None
                    };
                *cond = ExpRewriter::new(env, &mut replacer).rewrite_exp(cond.clone());
            };

        for i in 0..lets.len() {
            let (_, is_post, temp, ref exp) = lets[i];
            let exp = exp.clone();

            // Skip choice expressions — they need the Identical temp for
            // the witness wellformedness assertion.
            if matches!(exp.as_ref(), ExpData::Quant(_, kind, ..) if kind.is_choice()) {
                spec.lets.push(lets[i].clone());
                continue;
            }

            // For LetPre substituted into post-state conditions:
            // 1. Annotate memory references with pre-state labels
            // 2. Remap param temporaries to saved (pre-state) versions
            let post_state_replacement = if !is_post {
                let annotated = Self::annotate_pre_state_memory(
                    env,
                    exp.clone(),
                    &mut spec.saved_memory,
                    &spec.anchors,
                );
                self.remap_params_to_saved(annotated, &mut spec.saved_params)
            } else {
                exp.clone()
            };
            let pre_state_replacement = exp;

            // Substitute into subsequent let expressions (for chained lets like
            // `let x = a + 1; let y = x + b;`). When a LetPre is referenced by
            // a subsequent LetPost, use post_state_replacement (with saved params)
            // since the LetPost will be used in post-state context.
            for subsequent_let in lets.iter_mut().skip(i + 1) {
                let subsequent_is_post = subsequent_let.1;
                let replacement = if subsequent_is_post && !is_post {
                    &post_state_replacement
                } else {
                    &pre_state_replacement
                };
                substitute_exp(env, &mut subsequent_let.3, temp, replacement);
            }

            // Pre-state conditions: requires (always pre-state)
            for (_, cond) in &mut spec.pre {
                substitute_exp(env, cond, temp, &pre_state_replacement);
            }
            // Aborts_if: pre-state when translated for a call, post-state for a definition
            let aborts_replacement = if for_call {
                &pre_state_replacement
            } else {
                &post_state_replacement
            };
            for (_, cond, code_opt) in &mut spec.aborts {
                substitute_exp(env, cond, temp, aborts_replacement);
                if let Some(code) = code_opt {
                    substitute_exp(env, code, temp, aborts_replacement);
                }
            }

            // Post-state conditions: ensures
            for (_, cond) in &mut spec.post {
                substitute_exp(env, cond, temp, &post_state_replacement);
            }

            // Other conditions
            for (_, cond) in &mut spec.modifies {
                substitute_exp(env, cond, temp, &pre_state_replacement);
            }
            for (_, lhs, rhs) in &mut spec.updates {
                substitute_exp(env, lhs, temp, &post_state_replacement);
                substitute_exp(env, rhs, temp, &post_state_replacement);
            }
            for (_, cond, msg, flag_opt) in &mut spec.emits {
                substitute_exp(env, cond, temp, &post_state_replacement);
                substitute_exp(env, msg, temp, &post_state_replacement);
                if let Some(flag) = flag_opt {
                    substitute_exp(env, flag, temp, &post_state_replacement);
                }
            }

            // Proof block actions
            let substitute_proof_action =
                |env: &GlobalEnv, action: &mut ProofAction, temp, replacement: &Exp| match action {
                    ProofAction::Assert(exp, _) | ProofAction::Assume(exp) => {
                        substitute_exp(env, exp, temp, replacement);
                    },
                    ProofAction::Split(exp, guard) => {
                        substitute_exp(env, exp, temp, replacement);
                        if let Some(g) = guard {
                            substitute_exp(env, g, temp, replacement);
                        }
                    },
                };
            for (_, action) in &mut spec.pre_proof {
                substitute_proof_action(env, action, temp, &pre_state_replacement);
            }
            for (_, action) in &mut spec.post_proof {
                substitute_proof_action(env, action, temp, &post_state_replacement);
            }
        }
    }

    /// Annotate an expression's unlabeled memory references (`Global(None)`, `Exists(None)`)
    /// with a pre-state memory label. This is used when a LetPre expression (translated in
    /// pre-state context) is substituted into a post-state condition (ensures), so that
    /// memory accesses still evaluate against entry-state. For pure expressions without
    /// memory references, this is a no-op. An existing save of the resource is only
    /// reused if its label is saved at function entry, i.e. is not one of the spec's
    /// anchor labels (those are saved at their `SaveStateAnchor` markers instead).
    fn annotate_pre_state_memory(
        env: &GlobalEnv,
        exp: Exp,
        saved_memory: &mut BTreeSet<(QualifiedInstId<StructId>, MemoryLabel)>,
        anchors: &BTreeSet<MemoryLabel>,
    ) -> Exp {
        use ast::Operation as AstOp;
        let mut label_for = |mem: QualifiedInstId<StructId>| -> MemoryLabel {
            if let Some((_, l)) = saved_memory
                .iter()
                .find(|(m, l)| *m == mem && !anchors.contains(l))
            {
                *l
            } else {
                let l = MemoryLabel::new(env.new_global_id().as_usize());
                saved_memory.insert((mem, l));
                l
            }
        };
        ExpData::rewrite(exp, &mut |e| match e.as_ref() {
            ExpData::Call(id, AstOp::Global(None), args) => {
                let mem = env.get_node_instantiation(*id);
                if let Some(rty) = mem.first() {
                    let (mid, sid, inst) = rty.require_struct();
                    let l = label_for(mid.qualified_inst(sid, inst.to_owned()));
                    RewriteResult::Rewritten(
                        ExpData::Call(*id, AstOp::Global(Some(l)), args.clone()).into_exp(),
                    )
                } else {
                    RewriteResult::Unchanged(e)
                }
            },
            ExpData::Call(id, AstOp::Exists(None), args) => {
                let mem = env.get_node_instantiation(*id);
                if let Some(rty) = mem.first() {
                    let (mid, sid, inst) = rty.require_struct();
                    let l = label_for(mid.qualified_inst(sid, inst.to_owned()));
                    RewriteResult::Rewritten(
                        ExpData::Call(*id, AstOp::Exists(Some(l)), args.clone()).into_exp(),
                    )
                } else {
                    RewriteResult::Unchanged(e)
                }
            },
            _ => RewriteResult::Unchanged(e),
        })
    }

    /// Remap function parameter temporaries in an expression to their saved (pre-state)
    /// versions. This is needed when a LetPre expression (translated in pre-state context
    /// where params have their original values) is substituted into post-state conditions
    /// (ensures), where the original param temps may have been modified by the function body.
    /// Creates new saved_params entries for any params not yet saved.
    fn remap_params_to_saved(
        &mut self,
        exp: Exp,
        saved_params: &mut BTreeMap<TempIndex, TempIndex>,
    ) -> Exp {
        let param_count = self.builder.fun_env.get_parameter_count();
        let env = self.builder.global_env();
        ExpData::rewrite(exp, &mut |e| {
            if let ExpData::Temporary(id, idx) = e.as_ref() {
                if *idx < param_count {
                    let saved = *saved_params.entry(*idx).or_insert_with(|| {
                        self.builder
                            .new_temp(self.builder.get_local_type(*idx).skip_reference().clone())
                    });
                    if saved != *idx {
                        let new_id =
                            env.new_node(env.get_node_loc(*id), self.builder.get_local_type(saved));
                        return RewriteResult::Rewritten(
                            ExpData::Temporary(new_id, saved).into_exp(),
                        );
                    }
                }
            }
            RewriteResult::Unchanged(e)
        })
    }

    /// Emit well-formedness assertions for choice expressions in let bindings.
    /// This should be called AFTER preconditions are assumed, so that the
    /// preconditions can establish witnesses for the choices.
    fn emit_choice_wellformedness(&mut self, spec: &TranslatedSpec, post_state: bool) {
        use Bytecode::*;
        if !self.is_verified() {
            return;
        }
        let lets = spec
            .lets
            .iter()
            .filter(|(_, is_post, ..)| *is_post == post_state);
        for (loc, _, _, exp) in lets {
            if let Some(exists_check) = self.extract_choice_exists_check(exp) {
                self.builder
                    .set_loc_and_vc_info(loc.clone(), CHOICE_WITNESS_FAILS_MESSAGE);
                self.builder
                    .emit_with(|id| Prop(id, PropKind::Assert, exists_check));
            }
        }
    }

    /// Emit traces which are related to the `emitted_exp`.
    fn emit_traces(&mut self, spec: &TranslatedSpec, emitted_exp: &ExpData) {
        use Bytecode::*;
        // Collect all node_ids from the expression which is emitted and select those traces
        // from the spec which have those ids.
        let node_ids = emitted_exp.node_ids().into_iter().collect::<BTreeSet<_>>();
        let traces = spec
            .debug_traces
            .iter()
            .filter(|(_, _, exp)| node_ids.contains(&exp.node_id()));
        for (node_id, kind, exp) in traces {
            let env = self.builder.global_env();
            let loc = env.get_node_loc(*node_id);
            if !exp.free_vars_with_types(env).is_empty() {
                continue;
            }
            self.builder.set_loc(loc);
            let temp = if let ExpData::Temporary(_, temp) = exp.as_ref() {
                *temp
            } else {
                self.builder.emit_let(exp.clone()).0
            };
            self.builder.emit_with(|id| {
                Call(
                    id,
                    vec![],
                    Operation::TraceExp(*kind, *node_id),
                    vec![temp],
                    None,
                )
            });
        }

        // Collects related global memory.
        for memory in self.mem_info {
            self.builder.emit_with(|id| {
                Call(
                    id,
                    vec![],
                    Operation::TraceGlobalMem(memory.clone()),
                    vec![],
                    None,
                )
            });
        }
    }

    /// Checks if an expression uses memory operations or behavioral predicates.
    fn uses_memory_or_behavior(exp: &Exp) -> bool {
        use ast::Operation;
        let mut uses_memory = false;
        exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(
                _,
                Operation::Global(_)
                | Operation::Exists(_)
                | Operation::CanModify
                | Operation::ResourceDomain
                | Operation::StateDomain
                | Operation::Behavior(..),
                _,
            ) = e
            {
                uses_memory = true;
                return false; // stop traversal
            }
            !uses_memory // continue only if we haven't found memory ops
        });
        uses_memory
    }

    /// Extract an exists-check from a choice expression for well-formedness assertions.
    fn extract_choice_exists_check(&self, exp: &Exp) -> Option<Exp> {
        let env = self.builder.global_env();
        if let ExpData::Quant(node_id, kind, ranges, triggers, condition, body) = exp.as_ref()
            && kind.is_choice()
        {
            if !Self::uses_memory_or_behavior(body)
                && condition
                    .as_ref()
                    .is_none_or(|c| !Self::uses_memory_or_behavior(c))
            {
                return None;
            }
            let exists_id = env.new_node(env.get_node_loc(*node_id), BOOL_TYPE.clone());
            if let Some(inst) = env.get_node_instantiation_opt(*node_id) {
                env.set_node_instantiation(exists_id, inst);
            }
            return Some(
                ExpData::Quant(
                    exists_id,
                    QuantKind::Exists,
                    ranges.clone(),
                    triggers.clone(),
                    condition.clone(),
                    body.clone(),
                )
                .into_exp(),
            );
        }
        None
    }

    fn modify_check_fails_message(
        env: &GlobalEnv,
        mem: QualifiedId<StructId>,
        targs: &[Type],
    ) -> String {
        let targs_str = if targs.is_empty() {
            "".to_string()
        } else {
            let tctx = TypeDisplayContext::new(env);
            format!(
                "<{}>",
                targs
                    .iter()
                    .map(|ty| ty.display(&tctx).to_string())
                    .join(", ")
            )
        };
        let module_env = env.get_module(mem.module_id);
        format!(
            "caller does not have permission to modify `{}::{}{}` at given address",
            module_env.get_name().display(env),
            module_env
                .get_struct(mem.id)
                .get_name()
                .display(env.symbol_pool()),
            targs_str
        )
    }
}

// =================================================================================================
// # Block Generation

impl<'a> Instrumenter<'a> {
    fn generate_abort_block(&mut self, spec: &TranslatedSpec) {
        use Bytecode::*;
        // Set the location to the function and emit label.
        let fun_loc = self.builder.fun_env.get_loc().at_end();
        self.builder.set_loc(fun_loc);
        let abort_label = self.abort_label;
        self.builder.emit_with(|id| Label(id, abort_label));

        if self.is_verified() {
            self.generate_abort_verify(spec);
        }

        // Emit abort
        let abort_local = self.abort_local;
        self.builder.emit_with(|id| Abort(id, abort_local, None));
    }

    /// Generates verification conditions for abort block.
    fn generate_abort_verify(&mut self, spec: &TranslatedSpec) {
        // The aggregate is a disjunction over all `aborts_if`; it is boundary language
        // only when some clause travels, i.e. is not `[concrete]`.
        let aborts_travel = spec
            .aborts
            .iter()
            .any(|(loc, ..)| !self.is_concrete_cond(loc));
        let tag = self.self_is_bv_internal && aborts_travel;
        use Bytecode::*;
        use PropKind::*;

        // Emit state-label-defining assumes so labeled memory variables are
        // constrained on the abort path (not only the normal return path).
        // Without this, labeled variables like Resource_$memory#S are
        // unconstrained when checking abort coverage, causing spurious failures
        // when address aliasing makes the abort condition depend on labeled state.
        // Use abort_path=true to assume only the defining fragment of each
        // condition, not extra properties that only hold on successful returns.
        self.emit_state_label_assumes(spec, true, self.self_is_bv_internal);

        let is_partial = self
            .builder
            .fun_env
            .is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false);

        if !is_partial {
            // If not partial, emit an assertion for the overall aborts condition.
            if let Some(cond) = spec.aborts_condition(&self.builder) {
                let loc = self.builder.fun_env.get_spec_loc();
                self.emit_traces(spec, &cond);
                self.builder.set_loc_and_vc_info(loc, ABORT_NOT_COVERED);
                self.builder.emit_with(move |id| Prop(id, Assert, cond));
                self.tag_contract_prop(tag);
            }
        }

        if spec.has_aborts_code_specs() {
            // If any codes are specified, emit an assertion for the code condition.
            let actual_code = self.builder.mk_temporary(self.abort_local);
            if let Some(code_cond) = spec.aborts_code_condition(&self.builder, &actual_code) {
                let loc = self.builder.fun_env.get_spec_loc();
                self.emit_traces(spec, &code_cond);
                self.builder
                    .set_loc_and_vc_info(loc, ABORTS_CODE_NOT_COVERED);
                self.builder
                    .emit_with(move |id| Prop(id, Assert, code_cond));
                self.tag_contract_prop(tag);
            }
        }
    }

    /// Generates an abort condition for assumption in opaque calls. This returns a temporary
    /// in which the abort condition is stored, plus an optional expression which constraints
    /// the abort code. If the 1st return value is None, it indicates that the abort condition
    /// is known to be false.
    fn generate_abort_opaque_cond(
        &mut self,
        is_partial: bool,
        spec: &TranslatedSpec,
        owner_is_bv_internal: bool,
    ) -> (Option<TempIndex>, Option<Exp>) {
        let aborts_cond = if is_partial {
            None
        } else {
            spec.aborts_condition(&self.builder)
        };
        let aborts_cond_temp = if let Some(cond) = aborts_cond {
            if matches!(cond.as_ref(), ExpData::Value(_, Value::Bool(false))) {
                return (None, None);
            }
            // Introduce a temporary to hold the value of the aborts condition.
            let t = self.builder.emit_let(cond).0;
            self.tag_contract_prop(owner_is_bv_internal);
            t
        } else {
            // Introduce a havoced temporary to hold an arbitrary value for the aborts
            // condition.
            self.builder.emit_let_havoc(BOOL_TYPE.clone()).0
        };
        let aborts_code_cond = if spec.has_aborts_code_specs() {
            let actual_code = self.builder.mk_temporary(self.abort_local);
            spec.aborts_code_condition(&self.builder, &actual_code)
        } else {
            None
        };
        (Some(aborts_cond_temp), aborts_code_cond)
    }

    /// Emit the spec-decoration block at the function's unified exit: state
    /// updates, post-state lets, aborts negation, ensures assertions, and emits
    /// completeness checks. Called from the `Ret` arm of `instrument_bytecode`
    /// so the assertions execute immediately before the trailing `Ret`. The
    /// caller is responsible for emitting the `Ret` itself.
    fn emit_return_assertions(&mut self, spec: &TranslatedSpec) {
        use Bytecode::*;
        use PropKind::*;

        self.builder
            .set_loc(self.builder.fun_env.get_loc().at_end());

        // Emit specification variable updates. They are generated for both verified and inlined
        // function variants, as the evolution of state updates is always the same.
        let lets_emitted = if !spec.updates.is_empty() {
            self.emit_lets(spec, true, self.self_is_bv_internal);
            self.emit_updates(spec, None);
            true
        } else {
            false
        };

        if self.is_verified() {
            // Emit `let` bindings if not already emitted.
            if !lets_emitted {
                self.emit_lets(spec, true, self.self_is_bv_internal);
            }

            // Emit well-formedness checks for choice expressions in post-state let bindings.
            self.emit_choice_wellformedness(spec, true);

            self.emit_state_label_assumes(spec, false, self.self_is_bv_internal);

            // Definitions do not replace proof obligations. Assert the full
            // condition, including any quantified or final-state properties.
            for (loc, abort_cond, _) in &spec.aborts {
                self.emit_traces(spec, abort_cond);
                let exp = self.builder.mk_not(abort_cond.clone());
                self.builder
                    .set_loc_and_vc_info(loc.clone(), ABORTS_IF_FAILS_MESSAGE);
                self.builder.emit_with(|id| Prop(id, Assert, exp));
                self.tag_contract_prop(self.self_is_bv_internal && !self.is_concrete_cond(loc));
            }

            // Emit return-point proof actions (`post`-prefixed proof statements).
            // Verification-only — see the corresponding gate before pre_proof.
            if !self.options.inference {
                self.emit_proof_actions(&spec.post_proof, spec);
            }

            // Prove all postconditions, including mixed definition/property clauses.
            for (loc, cond) in &spec.post {
                self.emit_traces(spec, cond);
                self.builder
                    .set_loc_and_vc_info(loc.clone(), ENSURES_FAILS_MESSAGE);
                self.builder
                    .emit_with(move |id| Prop(id, Assert, cond.clone()));
                self.tag_contract_prop(self.self_is_bv_internal && !self.is_concrete_cond(loc));
            }

            // Emit all event `emits` checks.
            for (loc, cond) in spec.emits_conditions(&self.builder) {
                self.emit_traces(spec, &cond);
                self.builder.set_loc_and_vc_info(loc, EMITS_FAILS_MESSAGE);
                self.builder.emit_with(move |id| Prop(id, Assert, cond))
            }

            let emits_is_partial = self
                .builder
                .fun_env
                .is_pragma_true(EMITS_IS_PARTIAL_PRAGMA, || false);
            let emits_is_strict = self
                .builder
                .fun_env
                .is_pragma_true(EMITS_IS_STRICT_PRAGMA, || false);
            if (!spec.emits.is_empty() && !emits_is_partial)
                || (spec.emits.is_empty() && emits_is_strict)
            {
                // If not partial, emit an assertion for the completeness of the emits specs.
                let cond = spec.emits_completeness_condition(&self.builder);
                let loc = self.builder.fun_env.get_spec_loc();
                self.emit_traces(spec, &cond);
                self.builder.set_loc_and_vc_info(loc, EMITS_NOT_COVERED);
                self.builder.emit_with(move |id| Prop(id, Assert, cond));
            }
        }
    }

    /// Emit havoc+assume for intermediate state labels.
    /// Collects all conditions (ensures + aborts), finds which ones define state labels,
    /// and emits them as assumes in topological (dependency) order.
    /// Returns the set of condition indices that were emitted as assumes (defining conditions).
    /// Assume only intermediate-state definitions, never final-state properties,
    /// on either path. `abort_path` also includes definitions from abort clauses.
    fn emit_state_label_assumes(
        &mut self,
        spec: &TranslatedSpec,
        abort_path: bool,
        owner_is_bv_internal: bool,
    ) -> BTreeSet<usize> {
        use Bytecode::*;
        use PropKind::*;

        let saved_labels: BTreeSet<MemoryLabel> =
            spec.saved_memory.iter().map(|(_, l)| *l).collect();

        // Build map: label -> defining condition (the condition with range.post == Some(label))
        let mut label_definers: BTreeMap<MemoryLabel, Vec<usize>> = BTreeMap::new();
        // Only ensures and aborts conditions can define state labels (via mutation
        // builtins, behavioral predicates, or spec functions with range.post).
        // Other spec fields (modifies, emits, updates) don't carry these operations.
        let all_conditions: Vec<&Exp> = spec
            .post
            .iter()
            .map(|(_, e)| e)
            .chain(spec.aborts.iter().map(|(_, e, _)| e))
            .collect();
        // Same index space as `all_conditions`: a `[concrete]` definer stays in
        // body world, like its counterpart in the post and aborts loops.
        let condition_locs: Vec<&Loc> = spec
            .post
            .iter()
            .map(|(l, _)| l)
            .chain(spec.aborts.iter().map(|(l, _, _)| l))
            .collect();
        for (idx, cond) in all_conditions.iter().enumerate() {
            for label in Self::defined_labels(cond) {
                if !saved_labels.contains(&label) {
                    label_definers.entry(label).or_default().push(idx);
                }
            }
        }

        // Multiple conditions may define the same label (e.g., the same
        // result_of chain appearing in both ensures and aborts_if).
        // The topological sort handles this by emitting only the first
        // definer and tracking it via defining_indices.

        // Topological sort: emit definitions in dependency order.
        // Each label may have multiple defining conditions (conditional mutations).
        let mut emitted_labels: BTreeSet<MemoryLabel> = BTreeSet::new();
        let mut defining_indices: BTreeSet<usize> = BTreeSet::new();
        let mut remaining: BTreeMap<MemoryLabel, Vec<usize>> = BTreeMap::new();
        for (label, definers) in &label_definers {
            remaining.insert(*label, definers.clone());
        }
        // Iterate until all definitions are emitted (or stuck = cycle).
        while !remaining.is_empty() {
            let mut progress = false;
            let snapshot: Vec<_> = remaining
                .iter()
                .map(|(l, indices)| (*l, indices.clone()))
                .collect();
            for (label, indices) in snapshot {
                // Check all labels USED by ALL defining conditions are already defined.
                // Labels co-defined by the same condition (nested chain like
                // result_of<f>(result_of<g>(...))) are inherently ordered within
                // the expression tree, so they don't block each other.
                let all_deps_met = indices.iter().all(|&idx| {
                    let deps = Self::all_labels(all_conditions[idx]);
                    let co_defined = Self::defined_labels(all_conditions[idx]);
                    deps.iter().all(|dep| {
                        *dep == label
                            || saved_labels.contains(dep)
                            || emitted_labels.contains(dep)
                            || !label_definers.contains_key(dep)
                            || co_defined.contains(dep)
                    })
                });
                if all_deps_met {
                    for &idx in &indices {
                        // `all_conditions` is `post` followed by `aborts`.
                        let is_aborts = idx >= spec.post.len();
                        // A postcondition holds where it is assumed, on the
                        // success path. An abort condition does not: assuming
                        // it here states that the function aborts on the path
                        // where it did not, contradicting the non-aborting
                        // arithmetic the body just performed, and every
                        // postcondition then holds vacuously. Its label is
                        // still recorded as emitted so the ordering below is
                        // unaffected, and the abort path keeps its defining
                        // fragment.
                        if is_aborts && !abort_path {
                            continue;
                        }
                        self.emit_traces(spec, all_conditions[idx]);
                        let cond = self.defining_fragment(all_conditions[idx]);
                        self.builder.emit_with(move |id| Prop(id, Assume, cond));
                        self.tag_contract_prop(
                            owner_is_bv_internal && !self.is_concrete_cond(condition_locs[idx]),
                        );
                        defining_indices.insert(idx);
                    }
                    emitted_labels.insert(label);
                    remaining.remove(&label);
                    progress = true;
                }
            }
            if !progress {
                let env = self.builder.global_env();
                env.error(
                    &self.builder.fun_env.get_spec_loc(),
                    "cyclic dependency among state label definitions",
                );
                break;
            }
        }
        defining_indices
    }

    fn defined_labels(exp: &Exp) -> BTreeSet<MemoryLabel> {
        defined_state_labels(exp)
    }

    fn defining_fragment(&self, exp: &Exp) -> Exp {
        state_label_defining_fragment(&self.builder, exp)
    }

    /// Returns all labels referenced by a condition.
    fn all_labels(exp: &Exp) -> BTreeSet<MemoryLabel> {
        all_labels_in_exp(exp)
    }

    /// Generate a check whether the target can modify the given memory provided
    /// (a) the target constraints the given memory (b) the target is the verification variant.
    fn generate_modifies_check(
        &mut self,
        kind: PropKind,
        spec: &TranslatedSpec,
        loc: &Loc,
        resource_type: &Type,
        original_exp: &Exp, // this contains the right node ids for tracing
        addr: Exp,
    ) {
        let target = self.builder.get_target();
        let (mid, sid, _) = resource_type.require_struct();
        if self.is_verified()
            && target
                .get_modify_targets_for_type(&mid.qualified(sid))
                .is_some()
        {
            self.emit_traces(spec, original_exp);
            let env = self.builder.global_env();
            let node_id = env.new_node(loc.clone(), BOOL_TYPE.clone());
            env.set_node_instantiation(node_id, vec![resource_type.to_owned()]);
            let can_modify =
                ExpData::Call(node_id, ast::Operation::CanModify, vec![addr]).into_exp();
            if kind == PropKind::Assert {
                let (mid, sid, inst) = resource_type.require_struct();
                self.builder.set_loc_and_vc_info(
                    loc.clone(),
                    &Self::modify_check_fails_message(env, mid.qualified(sid), inst),
                );
            } else {
                self.builder.set_loc(loc.clone());
            }
            self.builder
                .emit_with(|id| Bytecode::Prop(id, kind, can_modify));
        }
    }

    fn generate_opaque_call(
        &mut self,
        dests: Vec<TempIndex>,
        mid: ModuleId,
        fid: FunId,
        targs: &[Type],
        srcs: Vec<TempIndex>,
        aa: Option<AbortAction>,
        is_begin: bool,
    ) {
        let opaque_op = if is_begin {
            Operation::OpaqueCallBegin(mid, fid, targs.to_vec())
        } else {
            Operation::OpaqueCallEnd(mid, fid, targs.to_vec())
        };
        self.builder
            .emit_with(|id| Bytecode::Call(id, dests, opaque_op, srcs, aa));
    }

    /// Havocs the world memory, which the backend expands to every memory, then the memory
    /// this function accesses again, to re-assume its well-formedness.
    fn emit_world_havoc(&mut self) {
        let env = self.builder.global_env();
        if let Some(world) = env.world_memory() {
            let op = Operation::HavocGlobal(world.module_id, world.id, vec![]);
            self.builder
                .emit_with(|id| Bytecode::Call(id, vec![], op, vec![], None));
        }
        let accessed = self
            .accessed_memory
            .iter()
            .filter(|mem| {
                let qid = mem.to_qualified_id();
                !env.is_world_memory(&qid)
            })
            .cloned()
            .collect_vec();
        self.emit_global_havocs(accessed);
    }

    fn emit_global_havocs(
        &mut self,
        memories: impl IntoIterator<Item = QualifiedInstId<StructId>>,
    ) {
        let env = self.builder.global_env();
        for mem in memories.into_iter().filter(|mem| {
            !env.is_wellknown_event_handle_type(&Type::Struct(mem.module_id, mem.id, vec![]))
                && !env.get_struct_qid(mem.to_qualified_id()).is_ghost_memory()
        }) {
            let op = Operation::HavocGlobal(mem.module_id, mem.id, mem.inst.clone());
            self.builder
                .emit_with(|id| Bytecode::Call(id, vec![], op, vec![], None));
            if let Some(exp) =
                self.builder
                    .mk_inst_mem_quant_opt(QuantKind::Forall, &mem, &mut |val| {
                        Some(
                            self.builder
                                .mk_call(&BOOL_TYPE, ast::Operation::WellFormed, vec![val]),
                        )
                    })
            {
                self.builder
                    .emit_with(|id| Bytecode::Prop(id, PropKind::Assume, exp));
            }
        }
    }
}

// =================================================================================================
// # Proof Emission

impl<'a> Instrumenter<'a> {
    /// Emit bytecode for a flat list of proof actions.
    fn emit_proof_actions(&mut self, actions: &[(Loc, ProofAction)], spec: &TranslatedSpec) {
        use Bytecode::*;
        for (loc, action) in actions {
            match action {
                ProofAction::Assert(exp, msg) => {
                    self.emit_traces(spec, exp);
                    self.builder.set_loc_and_vc_info(loc.clone(), msg);
                    self.builder
                        .emit_with(|id| Prop(id, PropKind::Assert, exp.clone()));
                },
                ProofAction::Assume(exp) => {
                    self.builder.set_loc(loc.clone());
                    self.builder
                        .emit_with(|id| Prop(id, PropKind::Assume, exp.clone()));
                },
                ProofAction::Split(exp, guard) => {
                    self.builder.set_loc(loc.clone());
                    let attr_id = self.builder.new_attr();
                    self.builder.emit(Bytecode::Nop(attr_id));
                    self.split_points
                        .push((attr_id, exp.clone(), guard.clone()));
                },
            }
        }
    }
}

/// Find the pre-state memory label that the enclosing function's spec uses
/// for a behavioral predicate over `(mid, fid)`. Returns `Some(label)` if
/// any condition contains a `Behavior(_, _)` whose first arg is a `Closure`
/// with the given module/function id; `None` otherwise.
///
/// Used to gate the call-site `assume bp_*_of<callee>(...)` in the opaque
/// instrumentation: the aggregate is emitted only when the enclosing spec
/// names the callee through a behavioral predicate, so that it has an
/// assertion to discharge. The aggregate itself is stated over memory saved
/// at the call, not under this label.
/// Walk `stale_lhs` and `fresh_lhs` in parallel to build a stale→fresh
/// Temporary index map, then rewrite `stale_rhs`'s Temporaries accordingly.
/// Used at ghost-field update lowering, where the fun-spec keeps a lowered
/// (`old(...)` resolved) rhs but with the pre-renumbering temporary indices;
/// the maintained Prop marker has the fresh indices in its LHS. The two
/// LHS expressions share structure by construction.
/// Whether an update of a target with declared type `target_ty` needs a
/// value-domain assert for `rhs`. The check holds by construction — and is
/// skipped — when the target is `num` (no bounds), or when the RHS is a pure
/// copy of an already-well-formed value — a function local/parameter or a
/// chain of field selections over one (possibly under `old`) — whose
/// DECLARED type equals the target's. The declared type is resolved from the
/// local's type or the selected field's declaration; the RHS node type
/// cannot be used, since expression translation coerces it to the expected
/// (target) type, which would let e.g. a `num`-declared ghost value slip
/// unchecked into a bounded target. Anything whose declared type cannot be
/// resolved keeps the check. Skipping provably-redundant checks also keeps
/// the emitted Boogie of enclosing VCs identical to what it was before this
/// feature.
fn range_check_needed(env: &GlobalEnv, target_ty: &Type, rhs: &Exp, local_types: &[Type]) -> bool {
    if matches!(target_ty, Type::Primitive(PrimitiveType::Num)) {
        return false;
    }
    fn declared_copy_type(env: &GlobalEnv, exp: &ExpData, local_types: &[Type]) -> Option<Type> {
        match exp {
            ExpData::Temporary(_, idx) => local_types.get(*idx).cloned(),
            ExpData::Call(id, ast::Operation::Select(mid, sid, fid), args) if args.len() == 1 => {
                // The base must itself be a resolvable pure copy; the leaf
                // field's declaration is the value's domain authority.
                declared_copy_type(env, args[0].as_ref(), local_types)?;
                Some(
                    env.get_module(*mid)
                        .into_struct(*sid)
                        .get_field(*fid)
                        .get_type()
                        .instantiate(&env.get_node_instantiation(*id)),
                )
            },
            ExpData::Call(_, ast::Operation::Old, args) if args.len() == 1 => {
                declared_copy_type(env, args[0].as_ref(), local_types)
            },
            _ => None,
        }
    }
    match declared_copy_type(env, rhs.as_ref(), local_types) {
        Some(declared) => declared.skip_reference() != target_ty,
        None => true,
    }
}

fn align_temps(
    env: &GlobalEnv,
    stale_lhs: &ExpData,
    fresh_lhs: &ExpData,
    fresh_rhs: &ExpData,
    stale_rhs: &Exp,
) -> Exp {
    // Collect the stale→fresh temporary index map, along with each stale
    // temporary's node id (used to preserve the original type/loc when
    // synthesizing the replacement `Temporary` expression — otherwise
    // downstream consumers reading the node's type would panic).
    // Walk both LHS and RHS in parallel: reaching-definition propagation
    // can also renumber temps referenced only in the RHS (a value read
    // from a copied local), and those wouldn't be covered by walking LHS
    // alone. The Prop marker carries a copy of both sides with fresh
    // temp indices; the fun-spec entry carries the properly lowered form
    // of `old(...)` but with stale temp indices. Combining both walks
    // gives a complete stale→fresh map without losing `old` lowering.
    let mut map: BTreeMap<TempIndex, (TempIndex, NodeId)> = BTreeMap::new();
    let mut desynced = false;
    fn collect(
        stale: &ExpData,
        fresh: &ExpData,
        map: &mut BTreeMap<TempIndex, (TempIndex, NodeId)>,
        desynced: &mut bool,
    ) {
        match (stale, fresh) {
            (ExpData::Temporary(id, s), ExpData::Temporary(_, f)) => {
                map.insert(*s, (*f, *id));
            },
            (ExpData::Call(_, op1, args1), ExpData::Call(_, op2, args2))
                if op1 == op2 && args1.len() == args2.len() =>
            {
                for (a, b) in args1.iter().zip(args2.iter()) {
                    collect(a.as_ref(), b.as_ref(), map, desynced);
                }
            },
            _ => {
                // The two copies come from the same source expression and may
                // differ only by temp renumbering; any structural divergence
                // (e.g. one side lowered differently) would leave temps in the
                // skipped subtree unmapped, silently rewriting to a stale —
                // wrong or dead — temporary. Fail loud instead: correctness
                // must not depend on an unenforced upstream shape guarantee.
                if !stale.used_temporaries().is_empty() {
                    *desynced = true;
                }
            },
        }
    }
    collect(stale_lhs, fresh_lhs, &mut map, &mut desynced);
    collect(stale_rhs.as_ref(), fresh_rhs, &mut map, &mut desynced);
    if desynced {
        env.error(
            &env.get_node_loc(stale_rhs.node_id()),
            "cannot lower this ghost update: the update expression's shape \
             diverged during translation (please report this)",
        );
        return stale_rhs.clone();
    }
    if map.is_empty() {
        return stale_rhs.clone();
    }
    let mut cb = |_id, target: RewriteTarget| match target {
        RewriteTarget::Temporary(idx) => map
            .get(&idx)
            .map(|(fresh, orig)| ExpData::Temporary(*orig, *fresh).into_exp()),
        _ => None,
    };
    let mut rewriter = ExpRewriter::new(env, &mut cb);
    rewriter.rewrite_exp(stale_rhs.clone())
}

fn find_behavior_pre_label_for_callee(
    spec: &TranslatedSpec,
    mid: ModuleId,
    fid: FunId,
) -> Option<MemoryLabel> {
    let mut found: Option<MemoryLabel> = None;
    let pre_post_exps = spec.pre.iter().chain(spec.post.iter()).map(|(_, e)| e);
    let abort_exps = spec.aborts.iter().map(|(_, e, _)| e);
    for exp in pre_post_exps.chain(abort_exps) {
        if found.is_some() {
            break;
        }
        exp.visit_pre_order(&mut |e| {
            if found.is_some() {
                return false;
            }
            if let ExpData::Call(_, ast::Operation::Behavior(_, range), args) = e {
                if let Some(first) = args.first() {
                    if let ExpData::Call(_, ast::Operation::Closure(cmid, cfid, _), _) =
                        first.as_ref()
                    {
                        if *cmid == mid && *cfid == fid {
                            if let Some(label) = range.pre {
                                found = Some(label);
                                return false;
                            }
                        }
                    }
                }
            }
            true
        });
    }
    found
}

//  ================================================================================================
/// # Modifies Checker
/// Check modifies annotations. This is depending on usage analysis and is therefore
/// invoked here from the initialize trait function of this processor.
/// Memory a closure may write and access, from the usage of its target function.
struct ClosureFootprint {
    /// Memory havoced across a call the closure is passed to: what its target's usage summary
    /// writes, and when the closure's effects are known, also what a call to the target
    /// writes as the prover encodes it (`usage_analysis::call_effects`), what the functions
    /// encoded from their contracts that its code reaches write, and the declared frames of
    /// its target's captured function-typed parameters
    /// (`usage_analysis::closures_with_known_footprint`).
    writes: BTreeSet<QualifiedInstId<StructId>>,
    /// Whether the closure's effects are known (`usage_analysis::closures_with_known_footprint`),
    /// which is what a callee leaving the frame to the call site requires, and what lets
    /// `writes` stand in for a declared frame in the havoc. For a closure whose effects are
    /// not known, the havoc covers the declared frame as well.
    known: bool,
}

/// `usage_analysis::contract_effects_reached` for every closure target and instantiation in
/// the program, computed in `initialize` from the bodies before instrumentation clears or
/// rewrites any of them. The frame check and the instrumentation both read it, so they agree
/// on every closure's footprint whatever the verification scope and processing order.
type ReachedCache = BTreeMap<(QualifiedId<FunId>, Vec<Type>), Option<ReachedEffects>>;

fn reached_cache(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
) -> ReachedCache {
    let summary_of = |id: QualifiedId<FunId>| usage.get(&id).cloned();
    let mut cache = ReachedCache::new();
    for fun_id in targets.get_funs() {
        let Some(data) = targets.get_data(&fun_id, &FunctionVariant::Baseline) else {
            continue;
        };
        for bc in &data.code {
            let Bytecode::Call(_, _, Operation::Closure(mid, fid, targs, _), _, _) = bc else {
                continue;
            };
            let target_id = mid.qualified(*fid);
            cache.entry((target_id, targs.clone())).or_insert_with(|| {
                usage_analysis::contract_effects_reached(
                    env,
                    targets,
                    &summary_of,
                    target_id,
                    targs,
                )
            });
        }
    }
    cache
}

/// The cached walk for a closure over `fun_id` at `inst`; `None` (unknown) for one the cache
/// does not cover.
fn reached_in(
    reached: &ReachedCache,
    fun_id: QualifiedId<FunId>,
    inst: &[Type],
) -> Option<ReachedEffects> {
    reached.get(&(fun_id, inst.to_vec())).cloned().flatten()
}

/// Whether `temp` is bound, in `code`, to a closure whose target's usage summary has
/// `unbounded_invoke`: the target runs a function value whose writes nothing records.
fn closure_target_has_unbounded_invoke(
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    code: &[Bytecode],
    temp: TempIndex,
) -> bool {
    code.iter().any(|bc| match bc {
        Bytecode::Call(_, dests, Operation::Closure(mid, fid, _, _), _, _) => {
            dests.contains(&temp)
                && usage
                    .get(&mid.qualified(*fid))
                    .is_some_and(|target_usage| target_usage.unbounded_invoke)
        },
        _ => false,
    })
}

/// Footprints of the closures in `code` bound to a temporary defined once, keyed by that
/// temporary.
fn closure_footprints(
    env: &GlobalEnv,
    enclosing: QualifiedId<FunId>,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    reached: &ReachedCache,
    code: &[Bytecode],
) -> BTreeMap<TempIndex, ClosureFootprint> {
    // A loop head havocs every temporary the body assigns, a closure among them, so a
    // closure built inside a loop has two definitions and no footprint here. That is
    // deliberate: the call-site havoc derived from a footprint covers the path through the
    // loop body, but a claim made after the loop is checked against the loop-exit path,
    // whose memory havoc comes from the callee's usage summary -- empty for a callee that
    // declares no frame. Treating the footprint as known there would let a false post-loop
    // claim through, so the loop case is left without one and rejected instead.
    let mut def_counts: BTreeMap<TempIndex, usize> = BTreeMap::new();
    for bc in code {
        for dest in bc.dests() {
            *def_counts.entry(dest).or_default() += 1;
        }
    }
    let summary_of = |id: QualifiedId<FunId>| usage.get(&id).cloned();
    let known = usage_analysis::closures_with_known_footprint(
        env,
        enclosing,
        code,
        &summary_of,
        &|fun_id, inst| reached_in(reached, fun_id, inst),
    );
    code.iter()
        .filter_map(|bc| match bc {
            Bytecode::Call(_, dests, Operation::Closure(mid, fid, targs, _), _, _)
                if dests.len() == 1 && def_counts[&dests[0]] == 1 =>
            {
                let target_usage = usage.get(&mid.qualified(*fid))?;
                let mut writes = target_usage.modified.get_all_inst(targs);
                writes.extend(target_usage.invoke_frame.get_all_inst(targs));
                let effects = known.get(&dests[0]);
                if let Some(effects) = effects {
                    writes.extend(effects.writes.iter().cloned());
                }
                Some((dests[0], ClosureFootprint {
                    writes,
                    known: effects.is_some(),
                }))
            },
            _ => None,
        })
        .collect()
}

/// The `modifies_of`/`reads_of` frame a function declares for a function-typed parameter,
/// instantiated at a call site.
///
/// Without a declaration the parameter may access no memory, except where the callee
/// leaves the frame to its call sites (`FunctionEnv::leaves_fun_param_frame_to_call_site`).
#[derive(Default)]
struct DeclaredFrame {
    writes: BTreeSet<QualifiedInstId<StructId>>,
    accessed: BTreeSet<QualifiedInstId<StructId>>,
    modifies_all: bool,
    reads_all: bool,
}

impl DeclaredFrame {
    fn of_fun_param(fun_env: &FunctionEnv, param: Symbol, targs: &[Type]) -> Self {
        let param_access = fun_env.get_fun_param_access_of();
        match param_access.iter().find(|a| a.fun_param == param) {
            Some(access) => Self::of_access(access, targs),
            None => Self::default(),
        }
    }

    fn of_access(access: &ast::FunParamAccessOf, targs: &[Type]) -> Self {
        let inst = |mems: &BTreeSet<QualifiedInstId<StructId>>| {
            mems.iter()
                .map(|mem| mem.instantiate_ref(targs))
                .collect::<BTreeSet<_>>()
        };
        Self {
            writes: inst(&access.old_memory),
            accessed: inst(&access.used_memory),
            modifies_all: access.frame_spec.modifies_all,
            reads_all: access.frame_spec.reads_all,
        }
    }
}

/// The function-typed parameters, by function and position, whose frames the function's
/// verification does not rely on, so a call site need not check them.
///
/// A parameter is exempt only when every use of it, and of every value copied from it or read
/// through a reference to it, is one of: a direct `Ret`; a drop; or an operand of a
/// `Pack`/`PackVariant` whose result is only returned, written through a reference parameter
/// or into global storage (`WriteRef`, `MoveTo`) or dropped, in a function that reads no field
/// of that struct. Storing hands the obligation to the frame of the field the value is stored
/// in. Every other use -- an invocation, an argument of any call, a capture, a container
/// operation, a borrow, a mention in a specification -- relies on the frame.
fn frame_exempt_params(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> BTreeSet<(QualifiedId<FunId>, usize)> {
    let mut exempt = BTreeSet::new();
    for fun_id in targets.get_funs() {
        let fun = env.get_function(fun_id);
        if fun.is_native_or_intrinsic() {
            continue;
        }
        let params = fun.get_parameters();
        let fun_params: Vec<usize> = (0..params.len())
            .filter(|idx| params[*idx].1.skip_reference().is_function())
            .collect();
        if fun_params.is_empty() {
            continue;
        }
        let Some(data) = targets.get_data(&fun_id, &FunctionVariant::Baseline) else {
            continue;
        };
        let target = FunctionTarget::new(&fun, data);
        let code = target.get_bytecode();
        let mut spec_temps = BTreeSet::new();
        let mut spec_vars = BTreeSet::new();
        spec_mentions(&fun.get_spec(), &mut spec_temps, &mut spec_vars);
        for bc in code {
            match bc {
                Bytecode::Prop(_, _, exp) => {
                    spec_temps.extend(exp.used_temporaries());
                    spec_vars.extend(exp.free_vars());
                },
                Bytecode::SpecBlock(_, spec) => {
                    spec_mentions(spec, &mut spec_temps, &mut spec_vars)
                },
                _ => {},
            }
        }
        for idx in fun_params {
            if !spec_vars.contains(&params[idx].0) && only_escapes(&target, idx, &spec_temps) {
                exempt.insert((fun_id, idx));
            }
        }
    }
    exempt
}

/// Adds the temporaries and free variables named anywhere in `spec` to `temps` and `vars`:
/// every expression of every condition and ghost update, the `modifies` targets, the proof,
/// and the code-point specs.
fn spec_mentions(spec: &ast::Spec, temps: &mut BTreeSet<TempIndex>, vars: &mut BTreeSet<Symbol>) {
    let conds = spec.conditions.iter().chain(spec.update_map.values());
    let frame_exps = spec
        .frame_spec
        .iter()
        .flat_map(|frame| frame.modifies_targets.iter());
    for exp in conds
        .flat_map(|cond| cond.all_exps())
        .chain(frame_exps)
        .chain(spec.proof_exps())
    {
        temps.extend(exp.used_temporaries());
        vars.extend(exp.free_vars());
    }
    for on_impl in spec.on_impl.values() {
        spec_mentions(on_impl, temps, vars);
    }
}

/// Whether every use of the function value in parameter `param` of `target`, and of every
/// value copied from it or read through a reference to it, is one of the escapes
/// `frame_exempt_params` allows.
fn only_escapes(
    target: &FunctionTarget,
    param: TempIndex,
    spec_temps: &BTreeSet<TempIndex>,
) -> bool {
    let code = target.get_bytecode();
    let derived = derived_by_copy(code, BTreeSet::from([param]));
    let mut packed = BTreeSet::new();
    let mut packed_structs = BTreeSet::new();
    for bc in code {
        let srcs = match bc {
            Bytecode::Prop(..) | Bytecode::SaveMem(..) | Bytecode::SaveSpecVar(..) => continue,
            Bytecode::SpecBlock(..) => continue,
            _ => bc.sources(),
        };
        if !srcs.iter().any(|src| derived.contains(src)) {
            continue;
        }
        match bc {
            Bytecode::Assign(_, dest, _, _) if derived.contains(dest) => {},
            Bytecode::Call(_, dests, Operation::ReadRef | Operation::FreezeRef(_), _, _)
                if dests.iter().all(|dest| derived.contains(dest)) => {},
            Bytecode::Ret(..) => {},
            Bytecode::Call(_, _, op, _, _) if is_neutral_use(op) => {},
            Bytecode::Call(_, dests, Operation::Pack(mid, sid, _), _, _)
            | Bytecode::Call(_, dests, Operation::PackVariant(mid, sid, _, _), _, _) => {
                packed.extend(dests.iter().copied());
                packed_structs.insert(mid.qualified(*sid));
            },
            _ => return false,
        }
    }
    if derived.iter().any(|temp| spec_temps.contains(temp)) {
        return false;
    }
    if packed.is_empty() {
        return true;
    }
    let packed = derived_by_copy(code, packed);
    if packed.iter().any(|temp| spec_temps.contains(temp)) {
        return false;
    }
    for bc in code {
        if let Bytecode::Call(_, _, op, _, _) = bc {
            let read = match op {
                Operation::Unpack(mid, sid, _)
                | Operation::UnpackVariant(mid, sid, _, _)
                | Operation::GetField(mid, sid, _, _)
                | Operation::GetVariantField(mid, sid, _, _, _)
                | Operation::BorrowField(mid, sid, _, _)
                | Operation::BorrowVariantField(mid, sid, _, _, _) => Some(mid.qualified(*sid)),
                _ => None,
            };
            if read.is_some_and(|id| packed_structs.contains(&id)) {
                return false;
            }
        }
        let srcs = match bc {
            Bytecode::Prop(..) | Bytecode::SaveMem(..) | Bytecode::SaveSpecVar(..) => continue,
            Bytecode::SpecBlock(..) => continue,
            _ => bc.sources(),
        };
        if !srcs.iter().any(|src| packed.contains(src)) {
            continue;
        }
        let escapes = match bc {
            Bytecode::Assign(_, dest, _, _) => packed.contains(dest),
            Bytecode::Ret(..) => true,
            Bytecode::Call(_, _, op, _, _) if is_neutral_use(op) => true,
            Bytecode::Call(_, _, Operation::MoveTo(..), srcs, _) => {
                srcs.get(1).is_some_and(|value| packed.contains(value))
            },
            Bytecode::Call(_, _, Operation::WriteRef, srcs, _) => {
                srcs.get(1).is_some_and(|value| packed.contains(value))
                    && !packed.contains(&srcs[0])
                    && ref_leaves_function(target, srcs[0])
            },
            _ => false,
        };
        if !escapes {
            return false;
        }
    }
    true
}

/// Whether `op` uses a value without letting it reach anything: a drop, or a trace for
/// counterexamples.
fn is_neutral_use(op: &Operation) -> bool {
    matches!(
        op,
        Operation::Drop
            | Operation::Release
            | Operation::TraceLocal(_)
            | Operation::TraceReturn(_)
            | Operation::TraceExp(..)
    )
}

/// The temporaries in `code` holding a copy of a value in `roots`, directly or read through a
/// reference to one.
fn derived_by_copy(code: &[Bytecode], roots: BTreeSet<TempIndex>) -> BTreeSet<TempIndex> {
    let mut derived = roots;
    loop {
        let more: Vec<TempIndex> = code
            .iter()
            .flat_map(|bc| match bc {
                Bytecode::Assign(_, dest, src, _) if derived.contains(src) => vec![*dest],
                Bytecode::Call(_, dests, Operation::ReadRef | Operation::FreezeRef(_), srcs, _)
                    if srcs.iter().any(|src| derived.contains(src)) =>
                {
                    dests.clone()
                },
                _ => vec![],
            })
            .filter(|temp| !derived.contains(temp))
            .collect();
        if more.is_empty() {
            return derived;
        }
        derived.extend(more);
    }
}

/// Whether every definition of the reference in `temp` borrows from a mutable reference
/// parameter of `target` or from global storage, so a value written through it outlives the
/// function.
fn ref_leaves_function(target: &FunctionTarget, temp: TempIndex) -> bool {
    let code = target.get_bytecode();
    let param_count = target.get_parameter_count();
    let mut visited = BTreeSet::new();
    let mut todo = vec![temp];
    while let Some(t) = todo.pop() {
        if !visited.insert(t) {
            continue;
        }
        let mut defined = false;
        if t < param_count {
            if !target.get_local_type(t).is_mutable_reference() {
                return false;
            }
            defined = true;
        }
        for bc in code {
            if !bc.dests().contains(&t) {
                continue;
            }
            defined = true;
            match bc {
                Bytecode::Assign(_, _, src, _) => todo.push(*src),
                Bytecode::Call(_, _, Operation::BorrowGlobal(..), _, _) => {},
                Bytecode::Call(
                    _,
                    _,
                    Operation::BorrowField(..) | Operation::BorrowVariantField(..),
                    srcs,
                    _,
                ) => todo.push(srcs[0]),
                _ => return false,
            }
        }
        if !defined {
            return false;
        }
    }
    true
}

/// Checks that every function value reaching a function-typed parameter stays within the
/// frame declared for that parameter, which is empty without a declaration. The function is
/// verified under that frame, opaque or not, and the callers of an opaque function assume the
/// result. A value reaches a parameter as a call argument, as a value captured by a closure,
/// or as an argument of an invoked function value (whose target is not known here, so the
/// empty frame, which every target accepts, applies).
///
/// The value is judged by its origins (`fun_value_origins`): a closure built in the function,
/// by what its target's code accesses and what it writes through function values whose
/// frames are declared; a parameter of the function, or a struct field, by its declared
/// frame. A value of any other origin, a field without a declared frame, or a closure whose
/// effects are not known (`usage_analysis::closure_target_effects`) is rejected, unless the
/// parameter declares `modifies_of<f> *`.
///
/// A callee that leaves the frame to its call sites
/// (`FunctionEnv::leaves_fun_param_frame_to_call_site`) is the exception: the call site
/// supplies the frame instead, provided it has a closure footprint to supply, and the
/// usage analysis carries that footprint into the summary of the function making the call.
/// A capture is not a call site, so such a parameter is held to the empty frame there.
fn check_fun_arg_frames(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    reached: &ReachedCache,
) {
    let exempt = frame_exempt_params(env, targets);
    for module_env in env.get_modules().filter(|m| m.is_target()) {
        for fun_env in module_env.get_functions() {
            if fun_env.is_not_prover_target()
                || !fun_env.is_compiled()
                || fun_env.is_native()
                || fun_env.is_intrinsic()
            {
                continue;
            }
            let target = targets.get_target(&fun_env, &FunctionVariant::Baseline);
            let code = target.get_bytecode();
            let footprints =
                closure_footprints(env, fun_env.get_qualified_id(), usage, reached, code);
            for bc in code {
                match bc {
                    Bytecode::Call(id, _, Operation::Function(mid, fid, targs), srcs, _) => {
                        let callee_env = env.get_module(*mid).into_function(*fid);
                        let callee_id = callee_env.get_qualified_id();
                        for (idx, (param, src)) in
                            callee_env.get_parameters().iter().zip(srcs).enumerate()
                        {
                            if !param.1.skip_reference().is_function()
                                || exempt.contains(&(callee_id, idx))
                                    && !callee_env.leaves_fun_param_frame_to_call_site(param.0)
                            {
                                continue;
                            }
                            let frame = DeclaredFrame::of_fun_param(&callee_env, param.0, targs);
                            if frame.modifies_all {
                                continue;
                            }
                            let loc = target.get_bytecode_loc(*id);
                            let param_name = param.0.display(env.symbol_pool());
                            // A closure built here whose target runs a function value nothing
                            // bounds has writes no footprint records, so it can be checked
                            // against neither a declared frame nor a deferred one.
                            if closure_target_has_unbounded_invoke(usage, code, *src) {
                                env.error(
                                    &loc,
                                    &format!(
                                        "cannot establish what function argument `{}` may \
                                         modify: the closure's target invokes a function \
                                         value whose effects no `modifies_of` declaration \
                                         bounds",
                                        param_name
                                    ),
                                );
                                continue;
                            }
                            if callee_env.leaves_fun_param_frame_to_call_site(param.0) {
                                // Deferring only works when this call site can answer the
                                // question the callee left open: for a closure built here
                                // whose writes are known (`closures_with_known_footprint`),
                                // which are then havoced across the call. Any other function
                                // value has no footprint to havoc, so it is rejected.
                                if !footprints.get(src).is_some_and(|f| f.known) {
                                    env.error(
                                        &loc,
                                        &format!(
                                            "cannot establish what function argument `{}` \
                                             may modify: `{}` declares no `modifies_of` for \
                                             it, and the argument is not a closure with a \
                                             known footprint built at this call site",
                                            param_name,
                                            callee_env.get_full_name_str()
                                        ),
                                    );
                                }
                                continue;
                            }
                            check_fun_value_within(
                                env,
                                targets,
                                &target,
                                usage,
                                reached,
                                *src,
                                &FrameSite {
                                    loc,
                                    kind: "function argument",
                                    value: format!("function argument `{}`", param_name),
                                    param: format!("`{}`", param_name),
                                    frame: &frame,
                                },
                            );
                        }
                    },
                    Bytecode::Call(id, _, Operation::Closure(mid, fid, targs, mask), srcs, _) => {
                        let closure_target = env.get_module(*mid).into_function(*fid);
                        let params = closure_target.get_parameters();
                        let closure_target_id = closure_target.get_qualified_id();
                        let captured = (0..params.len()).filter(|i| mask.is_captured(*i));
                        for (idx, src) in captured.zip(srcs) {
                            let param = &params[idx];
                            if !param.1.skip_reference().is_function()
                                || exempt.contains(&(closure_target_id, idx))
                            {
                                continue;
                            }
                            let frame =
                                if closure_target.leaves_fun_param_frame_to_call_site(param.0) {
                                    DeclaredFrame::default()
                                } else {
                                    DeclaredFrame::of_fun_param(&closure_target, param.0, targs)
                                };
                            let param_desc = format!(
                                "`{}` of `{}`",
                                param.0.display(env.symbol_pool()),
                                closure_target.get_full_name_str()
                            );
                            check_fun_value_within(
                                env,
                                targets,
                                &target,
                                usage,
                                reached,
                                *src,
                                &FrameSite {
                                    loc: target.get_bytecode_loc(*id),
                                    kind: "captured function value",
                                    value: format!("captured function value for {}", param_desc),
                                    param: param_desc,
                                    frame: &frame,
                                },
                            );
                        }
                    },
                    Bytecode::Call(id, _, Operation::Invoke, srcs, _) => {
                        let Some((_, args)) = srcs.split_last() else {
                            continue;
                        };
                        let empty = DeclaredFrame::default();
                        for src in args {
                            if !target.get_local_type(*src).skip_reference().is_function() {
                                continue;
                            }
                            check_fun_value_within(
                                env,
                                targets,
                                &target,
                                usage,
                                reached,
                                *src,
                                &FrameSite {
                                    loc: target.get_bytecode_loc(*id),
                                    kind: "function value passed to an invocation",
                                    value: "function value passed to an invocation".to_string(),
                                    param: "the parameter of the invoked function value"
                                        .to_string(),
                                    frame: &empty,
                                },
                            );
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}

/// Where a function value held in a temporary can come from.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FunValueOrigin {
    /// The `Closure` operation at this code offset.
    Closure(usize),
    /// The value the enclosing function received for this parameter.
    Param(TempIndex),
    /// Result `.1` of the function call at code offset `.0`.
    CallResult(usize, usize),
    /// A field of a struct or enum value: the struct, the variant and the field offset.
    Field(QualifiedInstId<StructId>, Option<Symbol>, usize),
    /// Anything else.
    Unknown,
}

/// Every origin of the function value in `temp`, following copies and borrows back to where
/// the value was made. A loop head's `Havoc` is not an origin: the values the temporary takes
/// at runtime come from its other definitions. A value a mutable reference can replace --
/// behind an incoming `&mut` parameter, a mutable borrow of a local, or a reference in the
/// chain that is written through or passed on -- is `Unknown`, as is a field read from a struct
/// that may have been replaced since it was made (`struct_may_be_replaced`).
fn fun_value_origins(target: &FunctionTarget, temp: TempIndex) -> BTreeSet<FunValueOrigin> {
    let code = target.get_bytecode();
    let param_count = target.get_parameter_count();
    let mut origins = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut todo = vec![temp];
    while let Some(t) = todo.pop() {
        if !visited.insert(t) {
            continue;
        }
        if t < param_count {
            // The caller, or this function, may have stored anything behind it.
            origins.insert(
                if target.get_local_type(t).is_mutable_reference() {
                    FunValueOrigin::Unknown
                } else {
                    FunValueOrigin::Param(t)
                },
            );
        }
        let mut defined = t < param_count;
        for (offset, bc) in code.iter().enumerate() {
            // A value behind a mutable borrow can be replaced through the reference.
            if let Bytecode::Call(_, dests, Operation::BorrowLoc, srcs, _) = bc {
                if srcs.first() == Some(&t)
                    && dests
                        .first()
                        .is_some_and(|r| target.get_local_type(*r).is_mutable_reference())
                {
                    origins.insert(FunValueOrigin::Unknown);
                }
            }
            if !bc.dests().contains(&t) {
                continue;
            }
            defined = true;
            let field_of = |mid: &ModuleId, sid: &StructId, targs: &[Type]| {
                mid.qualified_inst(*sid, targs.to_vec())
            };
            let position = |dests: &[TempIndex]| {
                dests
                    .iter()
                    .position(|d| *d == t)
                    .expect("defined temporary")
            };
            let origin = match bc {
                Bytecode::Call(_, _, Operation::Havoc(_), _, _) => continue,
                Bytecode::Assign(_, _, src, _) => {
                    todo.push(*src);
                    continue;
                },
                Bytecode::Call(_, dests, op, srcs, _) => match op {
                    Operation::Closure(..) => FunValueOrigin::Closure(offset),
                    Operation::Function(..) => FunValueOrigin::CallResult(offset, position(dests)),
                    Operation::FreezeRef(_) | Operation::ReadRef | Operation::BorrowLoc => {
                        todo.push(srcs[0]);
                        continue;
                    },
                    Operation::GetField(..)
                    | Operation::BorrowField(..)
                    | Operation::GetVariantField(..)
                    | Operation::BorrowVariantField(..)
                    | Operation::Unpack(..)
                    | Operation::UnpackVariant(..)
                        if struct_may_be_replaced(target, srcs[0]) =>
                    {
                        FunValueOrigin::Unknown
                    },
                    Operation::GetField(mid, sid, targs, field)
                    | Operation::BorrowField(mid, sid, targs, field) => {
                        FunValueOrigin::Field(field_of(mid, sid, targs), None, *field)
                    },
                    Operation::GetVariantField(mid, sid, variants, targs, field)
                    | Operation::BorrowVariantField(mid, sid, variants, targs, field) => {
                        FunValueOrigin::Field(
                            field_of(mid, sid, targs),
                            variants.first().copied(),
                            *field,
                        )
                    },
                    Operation::Unpack(mid, sid, targs) => {
                        FunValueOrigin::Field(field_of(mid, sid, targs), None, position(dests))
                    },
                    Operation::UnpackVariant(mid, sid, variant, targs) => FunValueOrigin::Field(
                        field_of(mid, sid, targs),
                        Some(*variant),
                        position(dests),
                    ),
                    _ => FunValueOrigin::Unknown,
                },
                _ => FunValueOrigin::Unknown,
            };
            origins.insert(origin);
        }
        if !defined {
            origins.insert(FunValueOrigin::Unknown);
        }
    }
    if written_through_mutable_ref(target, &visited) {
        origins.insert(FunValueOrigin::Unknown);
    }
    origins
}

/// Whether a mutable reference in `chain`, the temporaries a function value was read through,
/// or a copy of one, is written through or passed to code that may write through it.
fn written_through_mutable_ref(target: &FunctionTarget, chain: &BTreeSet<TempIndex>) -> bool {
    let code = target.get_bytecode();
    let mut refs: BTreeSet<TempIndex> = chain
        .iter()
        .copied()
        .filter(|t| target.get_local_type(*t).is_mutable_reference())
        .collect();
    loop {
        let copies: Vec<TempIndex> = code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Assign(_, dest, src, _) if refs.contains(src) => Some(*dest),
                _ => None,
            })
            .filter(|dest| !refs.contains(dest))
            .collect();
        if copies.is_empty() {
            break;
        }
        refs.extend(copies);
    }
    code.iter().any(|bc| match bc {
        Bytecode::Call(_, _, Operation::WriteRef, srcs, _) => {
            srcs.first().is_some_and(|r| refs.contains(r))
        },
        Bytecode::Call(_, _, Operation::Function(..) | Operation::Invoke, srcs, _) => {
            srcs.iter().any(|src| refs.contains(src))
        },
        _ => false,
    })
}

/// Whether the struct in `temp` may have been replaced since it was made, so a field read from
/// it is not bounded by the field's declared frame: field stores are not checked against that
/// frame (#20934). The chain the struct was built or read through is trusted only along
/// explicitly tracked definitions; everything else is replaceable. Tracked are copies,
/// borrows, field reads, `ReadRef`, `FreezeRef`, unpacks and packs (each struct operand of
/// a pack traced in turn) back to locals and parameters. Call results, globals, loop-head
/// havocs and any other definition are replaceable, as is a chain with a local that is borrowed
/// mutably or a mutable reference (or one borrowed from it) that is written through or passed
/// to a call.
fn struct_may_be_replaced(target: &FunctionTarget, temp: TempIndex) -> bool {
    let code = target.get_bytecode();
    let param_count = target.get_parameter_count();
    let mut chain = BTreeSet::new();
    let mut todo = vec![temp];
    while let Some(t) = todo.pop() {
        if !chain.insert(t) {
            continue;
        }
        let mut defined = t < param_count;
        for bc in code {
            if !bc.dests().contains(&t) {
                continue;
            }
            defined = true;
            match bc {
                Bytecode::Assign(_, _, src, _) => todo.push(*src),
                Bytecode::Call(
                    _,
                    _,
                    Operation::BorrowLoc
                    | Operation::BorrowField(..)
                    | Operation::BorrowVariantField(..)
                    | Operation::GetField(..)
                    | Operation::GetVariantField(..)
                    | Operation::Unpack(..)
                    | Operation::UnpackVariant(..)
                    | Operation::FreezeRef(_)
                    | Operation::ReadRef,
                    srcs,
                    _,
                ) => todo.push(srcs[0]),
                Bytecode::Call(_, _, Operation::Pack(..) | Operation::PackVariant(..), srcs, _) => {
                    todo.extend(
                        srcs.iter()
                            .filter(|src| target.get_local_type(**src).is_struct()),
                    )
                },
                _ => return true,
            }
        }
        if !defined {
            return true;
        }
    }
    let is_mut = |t: &TempIndex| target.get_local_type(*t).is_mutable_reference();
    let mut refs: BTreeSet<TempIndex> = chain.iter().copied().filter(is_mut).collect();
    if code.iter().any(|bc| {
        matches!(bc, Bytecode::Call(_, dests, Operation::BorrowLoc, srcs, _)
            if chain.contains(&srcs[0]) && dests.iter().any(is_mut))
    }) {
        return true;
    }
    loop {
        let more: Vec<TempIndex> = code
            .iter()
            .flat_map(|bc| match bc {
                Bytecode::Assign(_, dest, src, _) if refs.contains(src) => vec![*dest],
                Bytecode::Call(
                    _,
                    dests,
                    Operation::BorrowField(..) | Operation::BorrowVariantField(..),
                    srcs,
                    _,
                ) if refs.contains(&srcs[0]) => dests.clone(),
                _ => vec![],
            })
            .filter(|t| is_mut(t) && !refs.contains(t))
            .collect();
        if more.is_empty() {
            break;
        }
        refs.extend(more);
    }
    code.iter().any(|bc| match bc {
        Bytecode::Call(_, _, Operation::WriteRef, srcs, _) => {
            srcs.first().is_some_and(|r| refs.contains(r))
        },
        Bytecode::Call(_, _, Operation::Function(..) | Operation::Invoke, srcs, _) => {
            srcs.iter().any(|src| refs.contains(src))
        },
        _ => false,
    })
}

/// A place where a function value reaches a function-typed parameter.
struct FrameSite<'a> {
    loc: Loc,
    /// What the value is, for a value without a name of its own.
    kind: &'static str,
    /// The value, with the parameter it reaches where that names it.
    value: String,
    /// The parameter.
    param: String,
    frame: &'a DeclaredFrame,
}

impl FrameSite<'_> {
    /// Adds to `errors` what `bound`, a frame `what` is known to keep within, has beyond the
    /// parameter's frame.
    fn excess(&self, env: &GlobalEnv, what: &str, bound: &DeclaredFrame, errors: &mut Vec<String>) {
        if self.frame.modifies_all {
            return;
        }
        if bound.modifies_all {
            errors.push(format!(
                "{} may modify any resource, which is not declared in `modifies_of` for {}",
                what, self.param
            ));
            return;
        }
        for mem in bound.writes.difference(&self.frame.writes) {
            errors.push(format!(
                "{} may modify resource `{}`, which is not declared in `modifies_of` for {}",
                what,
                env.display(mem),
                self.param
            ));
        }
        if self.frame.reads_all {
            return;
        }
        if bound.reads_all {
            errors.push(format!(
                "{} may read any resource, which is not declared in `modifies_of`/`reads_of` \
                 for {}",
                what, self.param
            ));
            return;
        }
        for mem in bound
            .accessed
            .difference(&self.frame.accessed)
            .filter(|mem| !bound.writes.contains(mem))
        {
            errors.push(format!(
                "{} accesses resource `{}`, which is not declared in `modifies_of`/`reads_of` \
                 for {}",
                what,
                env.display(mem),
                self.param
            ));
        }
    }

    fn unknown(&self, reason: &str, errors: &mut Vec<String>) {
        errors.push(format!(
            "cannot establish what {} may modify: {}",
            self.value, reason
        ));
    }
}

/// What a closure built by `closure` in `enclosing` can access: what a call to its target
/// writes and accesses as the prover encodes it (`usage_analysis::closure_target_effects`),
/// with, for a target verified from its body, what that body's code accesses and writes,
/// including through function values whose frames are declared and through the functions
/// encoded from their contracts it calls (`usage_analysis::contract_effects_reached`, cached),
/// and what each captured function value may access under its parameter's declared frame.
/// `None` when its effects are not known.
fn closure_bound(
    env: &GlobalEnv,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    reached: &ReachedCache,
    enclosing: QualifiedId<FunId>,
    closure: &Bytecode,
) -> Option<DeclaredFrame> {
    let Bytecode::Call(_, _, Operation::Closure(mid, fid, targs, mask), _, _) = closure else {
        return None;
    };
    let target = mid.qualified(*fid);
    let target_env = env.get_function(target);
    let summary_of = |id: QualifiedId<FunId>| usage.get(&id).cloned();
    let usage_analysis::ClosureEffects {
        mut writes,
        accessed: contract_accessed,
    } = usage_analysis::closure_target_effects(
        env,
        enclosing,
        target,
        targs,
        Some(*mask),
        &summary_of,
    )?;
    // A target encoded from its contract does what the contract says: its summary omits the
    // declared `modifies` frame. Any other target accesses what its code and the
    // contract-encoded functions it calls do; memory its spec only mentions is not accessed.
    let mut accessed = BTreeSet::new();
    if usage_analysis::is_encoded_from_contract(&target_env) {
        accessed = contract_accessed;
    } else {
        let summary = usage.get(&target)?;
        accessed.extend(
            summary
                .code_accessed
                .iter()
                .map(|mem| mem.instantiate_ref(targs)),
        );
        let reached = reached_in(reached, target, targs)?.within(env, enclosing)?;
        writes.extend(reached.writes);
        accessed.extend(reached.accessed);
        // The code omits the captured parameters' declared reads; their declared writes, and
        // the rejection of a `*` frame, come with `closure_target_effects`.
        for access in target_env.get_fun_param_access_of() {
            accessed.extend(
                access
                    .used_memory
                    .iter()
                    .map(|mem| mem.instantiate_ref(targs)),
            );
        }
    }
    accessed.extend(writes.iter().cloned());
    Some(DeclaredFrame {
        writes,
        accessed,
        ..DeclaredFrame::default()
    })
}

/// What the function value returned as result `result` of `call` can access, when every value
/// the callee returns there is a closure built in its body with known effects. The callee must
/// be a transparent, non-native function of a target module called without type arguments:
/// its body is then the code that runs, and this check has bounded what its closures capture.
/// `None` otherwise.
fn returned_closure_bound(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    reached: &ReachedCache,
    call: &Bytecode,
    result: usize,
) -> Option<DeclaredFrame> {
    let Bytecode::Call(_, _, Operation::Function(mid, fid, targs), _, _) = call else {
        return None;
    };
    let callee = env.get_module(*mid).into_function(*fid);
    if !targs.is_empty()
        || callee.is_opaque()
        || callee.is_native_or_intrinsic()
        || callee.is_not_prover_target()
        || !callee.is_compiled()
        || !callee.module_env.is_target()
    {
        return None;
    }
    let data = targets.get_data(&callee.get_qualified_id(), &FunctionVariant::Baseline)?;
    let callee_target = FunctionTarget::new(&callee, data);
    let code = callee_target.get_bytecode();
    let mut bound: Option<DeclaredFrame> = None;
    for bc in code {
        let Bytecode::Ret(_, srcs) = bc else {
            continue;
        };
        for origin in fun_value_origins(&callee_target, *srcs.get(result)?) {
            let FunValueOrigin::Closure(offset) = origin else {
                return None;
            };
            let closure = closure_bound(
                env,
                usage,
                reached,
                callee.get_qualified_id(),
                &code[offset],
            )?;
            let acc = bound.get_or_insert_with(DeclaredFrame::default);
            acc.writes.extend(closure.writes);
            acc.accessed.extend(closure.accessed);
        }
    }
    bound
}

/// Checks the function value in `src`, used in `fun_target`, against the frame of `site`
/// over every origin of the value.
fn check_fun_value_within(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun_target: &FunctionTarget,
    usage: &BTreeMap<QualifiedId<FunId>, UsageState>,
    reached: &ReachedCache,
    src: TempIndex,
    site: &FrameSite,
) {
    if site.frame.modifies_all {
        return;
    }
    let fun_env = fun_target.func_env;
    let code = fun_target.get_bytecode();
    let mut errors = vec![];
    for origin in fun_value_origins(fun_target, src) {
        match origin {
            FunValueOrigin::Closure(offset) => {
                match closure_bound(
                    env,
                    usage,
                    reached,
                    fun_env.get_qualified_id(),
                    &code[offset],
                ) {
                    Some(bound) => site.excess(env, site.kind, &bound, &mut errors),
                    None => site.unknown("the closure's effects are not known", &mut errors),
                }
            },
            FunValueOrigin::CallResult(offset, result) => {
                match returned_closure_bound(env, targets, usage, reached, &code[offset], result) {
                    Some(bound) => site.excess(env, site.kind, &bound, &mut errors),
                    None => site.unknown(
                        "it is returned by a call, and not every value the callee returns is a \
                         closure with known effects built in a transparent function",
                        &mut errors,
                    ),
                }
            },
            FunValueOrigin::Param(idx) => {
                let param = fun_env.get_parameters()[idx].0;
                let param_name = param.display(env.symbol_pool());
                if fun_env.leaves_fun_param_frame_to_call_site(param) {
                    site.unknown(
                        &format!(
                            "parameter `{}` of `{}` leaves its frame to the call sites",
                            param_name,
                            fun_env.get_full_name_str()
                        ),
                        &mut errors,
                    );
                    continue;
                }
                // An undeclared parameter of a transparent function has the empty frame only
                // where every caller is checked against it; a dependency calling this function
                // supplies the value unchecked.
                let declared = fun_env
                    .get_fun_param_access_of()
                    .iter()
                    .any(|access| access.fun_param == param);
                if !declared
                    && !fun_env.is_opaque()
                    && !usage_analysis::all_callers_checked(fun_env)
                {
                    site.unknown(
                        &format!(
                            "it is parameter `{}` of `{}`, which declares no \
                             `modifies_of`/`reads_of` for it and may be called from a dependency, \
                             where nothing checks what reaches it",
                            param_name,
                            fun_env.get_full_name_str()
                        ),
                        &mut errors,
                    );
                    continue;
                }
                let bound = DeclaredFrame::of_fun_param(fun_env, param, &[]);
                site.excess(
                    env,
                    &format!("{} `{}`", site.kind, param_name),
                    &bound,
                    &mut errors,
                );
            },
            FunValueOrigin::Field(mem, variant, offset) => {
                let struct_env = env.get_struct(mem.to_qualified_id());
                let field_name = struct_env
                    .get_field_by_offset_optional_variant(variant, offset)
                    .get_name();
                let field_desc = format!(
                    "field `{}` of `{}`",
                    field_name.display(env.symbol_pool()),
                    struct_env.get_full_name_str()
                );
                let Some(access) = struct_env
                    .get_field_access_of()
                    .iter()
                    .find(|access| access.fun_param == field_name)
                else {
                    site.unknown(
                        &format!(
                            "it is read from {}, which declares no `modifies_of`/`reads_of`",
                            field_desc
                        ),
                        &mut errors,
                    );
                    continue;
                };
                let bound = DeclaredFrame::of_access(access, &mem.inst);
                site.excess(
                    env,
                    &format!("{} read from {}", site.kind, field_desc),
                    &bound,
                    &mut errors,
                );
            },
            FunValueOrigin::Unknown => site.unknown(
                "it is not a closure built in this function, a parameter of this function, or \
                 the value of a struct field with a declared frame",
                &mut errors,
            ),
        }
    }
    for error in errors.into_iter().unique() {
        env.error(&site.loc, &error);
    }
}
fn check_modifies(env: &GlobalEnv, targets: &FunctionTargetsHolder) {
    let mut warned_coarse_callees = BTreeSet::new();
    for module_env in env.get_modules() {
        if module_env.is_target() {
            for fun_env in module_env.get_functions() {
                if !fun_env.is_not_prover_target() && fun_env.is_compiled() {
                    check_caller_callee_modifies_relation(env, targets, &fun_env);
                    warn_coarse_opaque_calls(env, targets, &fun_env, &mut warned_coarse_callees);
                    check_reads_completeness(env, targets, &fun_env);
                }
            }
        }
    }
}

fn check_caller_callee_modifies_relation(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun_env: &FunctionEnv,
) {
    if fun_env.is_native() || fun_env.is_intrinsic() {
        return;
    }
    let caller_func_target = targets.get_target(fun_env, &FunctionVariant::Baseline);
    for callee in fun_env
        .get_called_functions()
        .expect(COMPILED_MODULE_AVAILABLE)
    {
        let callee_fun_env = env.get_function(*callee);
        if callee_fun_env.is_native()
            || callee_fun_env.is_intrinsic()
            || callee_fun_env.is_struct_api()
        {
            continue;
        }
        let callee_func_target = targets.get_target(&callee_fun_env, &FunctionVariant::Baseline);
        let callee_modified_memory = usage_analysis::get_memory_usage(&callee_func_target)
            .modified
            .get_all_uninst();
        for target in caller_func_target.get_modify_targets().keys() {
            if callee_modified_memory.contains(target)
                && callee_func_target
                    .get_modify_targets_for_type(target)
                    .is_none()
            {
                let loc = caller_func_target.get_loc();
                env.error(
                    &loc,
                    &format!(
                        "caller `{}` specifies modify targets for `{}` but callee `{}` does not",
                        fun_env.get_full_name_str(),
                        env.get_module(target.module_id)
                            .into_struct(target.id)
                            .get_full_name_str(),
                        callee_fun_env.get_full_name_str()
                    ),
                );
            }
        }
    }
}

fn warn_coarse_opaque_calls(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun_env: &FunctionEnv,
    warned: &mut BTreeSet<QualifiedId<FunId>>,
) {
    let target = targets.get_target(fun_env, &FunctionVariant::Baseline);
    let info = verification_analysis::get_info(&target);
    if !info.verified && !info.inlined {
        return;
    }
    let mut opaque_callees: BTreeSet<_> = fun_env
        .get_called_functions()
        .expect(COMPILED_MODULE_AVAILABLE)
        .iter()
        .copied()
        .collect();
    if target
        .get_bytecode()
        .iter()
        .any(|bc| matches!(bc, Bytecode::Call(_, _, Operation::Invoke, _, _)))
    {
        opaque_callees.extend(target.get_bytecode().iter().filter_map(|bc| match bc {
            Bytecode::Call(_, _, Operation::Closure(mid, fid, _, _), _, _) => {
                Some(mid.qualified(*fid))
            },
            _ => None,
        }));
    }
    for callee_qid in opaque_callees {
        let callee = env.get_function(callee_qid);
        if !callee.is_opaque() || !warned.insert(callee_qid) {
            continue;
        }
        let callee_target = targets.get_target(&callee, &FunctionVariant::Baseline);
        let framed = callee_target.get_modify_ids();
        let unframed_count = usage_analysis::get_memory_usage(&callee_target)
            .modified
            .all
            .iter()
            .filter(|mem| {
                !env.is_wellknown_event_handle_type(&Type::Struct(mem.module_id, mem.id, vec![]))
                    && !env.get_struct_qid(mem.to_qualified_id()).is_ghost_memory()
                    && !framed.iter().any(|id| *mem == id)
            })
            .count();
        if unframed_count > 0 {
            let resource_types = if unframed_count == 1 {
                "resource type"
            } else {
                "resource types"
            };
            env.warning(
                &callee.get_spec_loc(),
                &format!(
                    "opaque call to `{}` has an incomplete modifies frame; verification weakens \
                     its summary by havocing every address of {} {}",
                    callee.get_full_name_str(),
                    unframed_count,
                    resource_types
                ),
            );
        }
    }
}

/// Check that a function's actual resource access is covered by its `reads` declaration.
/// If a function declares `reads R, S;`, every resource it accesses (reads or writes)
/// must be listed. Resources in `modifies` are implicitly included.
fn check_reads_completeness(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun_env: &FunctionEnv,
) {
    let frame_spec = fun_env.get_frame_spec();
    let reads_targets = match &frame_spec {
        Some(fs) if !fs.reads_targets.is_empty() => &fs.reads_targets,
        _ => return, // No reads declaration — nothing to check
    };

    let target = targets.get_target(fun_env, &FunctionVariant::Baseline);
    let modify_ids = target.get_modify_ids();

    // All accessed memory must be in reads_targets or modify_ids
    for mem in usage_analysis::get_memory_usage(&target)
        .accessed
        .all
        .iter()
    {
        if env.is_wellknown_event_handle_type(&Type::Struct(mem.module_id, mem.id, vec![])) {
            continue;
        }
        if env.get_struct_qid(mem.to_qualified_id()).is_ghost_memory() {
            continue;
        }
        let in_reads = reads_targets
            .iter()
            .any(|r| r.to_qualified_id() == mem.to_qualified_id());
        let in_modifies = modify_ids
            .iter()
            .any(|m| m.to_qualified_id() == mem.to_qualified_id());
        if !in_reads && !in_modifies {
            let loc = fun_env.get_spec_loc();
            env.error(
                &loc,
                &format!(
                    "function `{}` accesses resource `{}` which is not covered by \
                     its `reads` or `modifies` declaration",
                    fun_env.get_full_name_str(),
                    env.display(mem)
                ),
            )
        }
    }
}

/// Saves the world memory under every label `spec` saves memory under. A state label of a
/// function that runs unknown code resolves a memory its defining operation does not modify to
/// the label before it, so a behavioral predicate over a function value of unknown target, which
/// ranges over the world memory, reads the world memory saved there. A label saving no memory
/// gets none; a predicate resolving to it is reported as reading an unsaved state.
fn add_world_saves(env: &GlobalEnv, spec: &mut TranslatedSpec) {
    let Some(world) = env.world_memory() else {
        return;
    };
    let labels: BTreeSet<MemoryLabel> = spec.saved_memory.iter().map(|(_, l)| *l).collect();
    for label in labels {
        spec.saved_memory.insert((world.clone(), label));
    }
}
