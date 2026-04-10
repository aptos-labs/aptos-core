// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Transformation which injects specifications (Move function spec blocks) into the bytecode.

use crate::{options::ProverOptions, verification_analysis};
use itertools::Itertools;
use move_model::{
    ast,
    ast::{Exp, ExpData, MemoryLabel, QuantKind, RewriteResult, TempIndex, Value},
    exp_generator::ExpGenerator,
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    memory_labels::all_labels_in_exp,
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, QualifiedId, QualifiedInstId, StructId},
    pragmas::{ABORTS_IF_IS_PARTIAL_PRAGMA, EMITS_IS_PARTIAL_PRAGMA, EMITS_IS_STRICT_PRAGMA},
    spec_translator::{ProofAction, SpecTranslator, TranslatedSpec},
    symbol::Symbol,
    ty::{ReferenceKind, Type, TypeDisplayContext, BOOL_TYPE, NUM_TYPE},
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
    usage_analysis, COMPILED_MODULE_AVAILABLE,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const REQUIRES_FAILS_MESSAGE: &str = "precondition does not hold at this call";
const ENSURES_FAILS_MESSAGE: &str = "post-condition does not hold";
const ABORTS_IF_FAILS_MESSAGE: &str = "function does not abort under this condition";
const ABORT_NOT_COVERED: &str = "abort not covered by any of the `aborts_if` clauses";
const ABORTS_CODE_NOT_COVERED: &str =
    "abort code not covered by any of the `aborts_if` or `aborts_with` clauses";
const EMITS_FAILS_MESSAGE: &str = "function does not emit the expected event";
const EMITS_NOT_COVERED: &str = "emitted event not covered by any of the `emits` clauses";
const CHOICE_WITNESS_FAILS_MESSAGE: &str = "choice expression requires a witness to exist";

// ================================================================================================
// # Spec Instrumenter

pub struct SpecInstrumentationProcessor {}

impl SpecInstrumentationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl FunctionTargetProcessor for SpecInstrumentationProcessor {
    fn initialize(&self, env: &GlobalEnv, targets: &mut FunctionTargetsHolder) {
        // Perform static analysis part of modifies check.
        check_modifies(env, targets);
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

        if is_verified {
            // Create a clone of the function data, moving annotations
            // out of this data and into the clone.
            let mut verification_data =
                data.fork(FunctionVariant::Verification(VerificationFlavor::Regular));
            let (instrumented, split_points) =
                Instrumenter::run(&options, targets, fun_env, verification_data, scc_opt);
            verification_data = instrumented;

            if split_points.is_empty() {
                // No splits — insert the single verification variant.
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
            let (data, _) = Instrumenter::run(&options, targets, fun_env, data, scc_opt);
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

struct Instrumenter<'a> {
    options: &'a ProverOptions,
    builder: FunctionDataBuilder<'a>,
    ret_locals: Vec<TempIndex>,
    ret_label: Label,
    can_return: bool,
    abort_local: TempIndex,
    abort_label: Label,
    can_abort: bool,
    mem_info: &'a BTreeSet<QualifiedInstId<StructId>>,
    /// Map from Nop AttrId to split expression and optional guard (for `split` proof items).
    /// AttrIds are stable across optimization passes, unlike bytecode offsets.
    /// The optional guard is a path condition from enclosing `if` in the proof block.
    split_points: Vec<(AttrId, Exp, Option<Exp>)>,
    /// Counter for deterministic label freshening across opaque call sites.
    /// Starts at 0 so that freshened labels are independent of the global counter.
    freshen_counter: usize,
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
    ) -> (FunctionData, Vec<(AttrId, Exp, Option<Exp>)>) {
        // Pre-collect properties in the original function data
        let props: Vec<_> = data
            .code
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Prop(id, PropKind::Assume, exp)
                | Bytecode::Prop(id, PropKind::Assert, exp) => Some((*id, exp.clone())),
                _ => None,
            })
            .collect();

        let mut builder = FunctionDataBuilder::new(fun_env, data);

        // Create label and locals for unified return exit point. We translate each `Ret(t..)`
        // instruction into `Assign(r.., t..); Jump(RetLab)`.
        let ret_locals = builder
            .data
            .result_type
            .clone()
            .flatten()
            .into_iter()
            .map(|ty| builder.new_temp(ty))
            .collect_vec();
        let ret_label = builder.new_label();

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
            ret_label,
            can_return: false,
            abort_local,
            abort_label,
            can_abort: false,
            mem_info: &mem_info,
            split_points: vec![],
            freshen_counter: 0,
        };
        if ProverOptions::get(instrumenter.builder.global_env()).inline_spec_lets {
            let mut spec = spec;
            instrumenter.inline_lets(&mut spec, false);
            instrumenter.instrument(&spec, &inlined_props);
        } else {
            instrumenter.instrument(&spec, &inlined_props);
        }

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

        // Emit `let` bindings.
        self.emit_lets(spec, false);

        // Inject preconditions as assumes. This is done for all self.variant values.
        self.builder
            .set_loc(self.builder.fun_env.get_loc().at_start()); // reset to function level
        for (loc, exp) in spec.pre_conditions(&self.builder) {
            self.builder.set_loc(loc);
            self.builder
                .emit_with(move |attr_id| Prop(attr_id, Assume, exp))
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

            // For the verification variant, we generate post-conditions. Inject any state
            // save instructions needed for this.
            for translated_spec in
                std::iter::once(spec).chain(inlined_props.values().map(|(s, _)| s))
            {
                for (mem, label) in &translated_spec.saved_memory {
                    let mem = mem.clone();
                    self.builder
                        .emit_with(|attr_id| SaveMem(attr_id, *label, mem));
                }
                for (spec_var, label) in &translated_spec.saved_spec_vars {
                    let spec_var = spec_var.clone();
                    self.builder
                        .emit_with(|attr_id| SaveSpecVar(attr_id, *label, spec_var));
                }
                let saved_params = translated_spec.saved_params.clone();
                self.emit_save_for_old(&saved_params);
            }
        } else {
            // The inlined variant may have an inlined spec that used "old" values - these need to
            // be saved in both cases
            assert!(
                verification_analysis::get_info(&FunctionTarget::new(
                    self.builder.fun_env,
                    &self.builder.data
                ))
                .inlined
            );
            for translated_spec in inlined_props.values().map(|(s, _)| s) {
                let saved_params = translated_spec.saved_params.clone();
                self.emit_save_for_old(&saved_params);
            }
        }

        // Instrument and generate new code.
        // Peel leading TraceLocal instructions and emit them before pre_proof
        // so that pre-proof assertion errors can display model values in counter-examples.
        let mut old_code = old_code.into_iter();
        let mut pre_proof_emitted = false;
        for bc in old_code.by_ref() {
            if matches!(&bc, Call(_, _, Operation::TraceLocal(_), _, _)) {
                self.builder.emit(bc);
            } else {
                // First non-trace instruction: emit pre_proof, then this instruction.
                self.emit_proof_actions(&spec.pre_proof, spec);
                pre_proof_emitted = true;
                self.instrument_bytecode(spec, inlined_props, bc);
                break;
            }
        }
        if !pre_proof_emitted {
            self.emit_proof_actions(&spec.pre_proof, spec);
        }
        // Continue with remaining old_code.
        for bc in old_code {
            self.instrument_bytecode(spec, inlined_props, bc);
        }

        // Generate return and abort blocks
        if self.can_return {
            self.generate_return_block(spec);
        }
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
                for (i, r) in self.ret_locals.clone().into_iter().enumerate() {
                    self.builder
                        .emit_with(|id| Assign(id, r, results[i], AssignKind::Move));
                }
                let ret_label = self.ret_label;
                self.builder.emit_with(|id| Jump(id, ret_label));
                self.can_return = true;
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
                self.instrument_call(id, dests, mid, fid, targs, srcs, aa);
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
        let mut callee_spec = SpecTranslator::translate_fun_spec(
            self.options.auto_trace_level.functions(),
            true,
            &mut self.builder,
            &callee_env,
            targs,
            Some(&srcs),
            &dests,
        );

        // Freshen state labels to avoid collisions between different inlining sites.
        // Uses a function-scoped counter for deterministic label IDs.
        callee_spec.freshen_labels(&mut self.freshen_counter);

        self.builder.set_loc_from_attr(id);

        if ProverOptions::get(self.builder.global_env()).inline_spec_lets {
            self.inline_lets(&mut callee_spec, true);
        }

        // Emit `let` assignments.
        self.emit_lets(&callee_spec, false);
        self.builder.set_loc_from_attr(id);

        // Emit pre conditions if this is the verification variant or if the callee
        // is opaque. For inlined callees outside of verification entry points, we skip
        // emitting any pre-conditions because they are assumed already at entry into the
        // function.
        if self.is_verified() || callee_opaque {
            for (loc, cond) in callee_spec.pre_conditions(&self.builder) {
                self.emit_traces(&callee_spec, &cond);
                // Determine whether we want to emit this as an assertion or an assumption.
                let prop_kind = match self.builder.data.variant {
                    FunctionVariant::Verification(..) => {
                        self.builder
                            .set_loc_and_vc_info(loc, REQUIRES_FAILS_MESSAGE);
                        Assert
                    },
                    FunctionVariant::Baseline => Assume,
                };
                self.builder.emit_with(|id| Prop(id, prop_kind, cond));
            }
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

            // Emit saves for parameters used in old(..) context. Those can be referred
            // to in aborts conditions, and must be initialized before evaluating those.
            self.emit_save_for_old(&callee_spec.saved_params);

            // Emit memory and spec var saves BEFORE label defines. The label-defining
            // ensures conditions reference these saved labels (e.g., global[@0] maps to
            // a SaveMem'd memory). They must be initialized before the label defines
            // and abort condition evaluation — on BOTH the abort and success paths.
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
            let defining_indices = self.emit_state_label_assumes(&callee_spec, true);
            // Remove defining conditions from post so they aren't double-emitted
            // later when assumes for ensures are emitted on the non-abort path.
            if !defining_indices.is_empty() {
                let post_count = callee_spec.post.len();
                callee_spec.post = callee_spec
                    .post
                    .drain(..)
                    .enumerate()
                    .filter(|(i, _)| !defining_indices.contains(i))
                    .map(|(_, v)| v)
                    .collect();
                // Adjust aborts indices (they start after post in the combined list)
                // No adjustment needed — aborts are separate from post in callee_spec
                let _ = post_count; // suppress unused warning
            }

            let callee_aborts_if_is_partial =
                callee_env.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false);

            // Translate the abort condition. If the abort_cond_temp_opt is None, it indicates
            // that the abort condition is known to be false, so we can skip the abort handling.
            let (abort_cond_temp_opt, code_cond) =
                self.generate_abort_opaque_cond(callee_aborts_if_is_partial, &callee_spec);
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

            // Emit modifies properties which havoc memory at the modified location.
            for (_, exp) in std::mem::take(&mut callee_spec.modifies) {
                self.emit_traces(&callee_spec, &exp);
                self.builder.emit_with(|id| Prop(id, Modifies, exp));
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
            self.emit_lets(&callee_spec, true);

            // Emit well-formedness checks for choice expressions in callee's post-state let bindings.
            self.emit_choice_wellformedness(&callee_spec, true);

            // Emit spec var updates.
            self.emit_updates(&callee_spec, None);

            // Emit post conditions as assumptions.
            for (_, cond) in std::mem::take(&mut callee_spec.post) {
                self.emit_traces(&callee_spec, &cond);
                self.builder.emit_with(|id| Prop(id, Assume, cond));
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

            // Generate OpaqueCallEnd instruction if invariant_v2.
            self.generate_opaque_call(dests, mid, fid, targs, srcs, aa, false);
        }
    }
}

// =================================================================================================
// # Spec Condition Emission

impl<'a> Instrumenter<'a> {
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

            // Extract the ghost mem from lhs
            let (ghost_mem, _field_id, addr) = lhs
                .extract_ghost_mem_access(self.builder.global_env())
                .expect("lhs of update valid");
            let ghost_mem_ty = ghost_mem.to_type();

            // Construct new ghost mem struct value from rhs. We assign the struct value
            // directly, ignoring the `_field_id`. This is currently possible because
            // each ghost memory struct contains exactly one field. Should this change,
            // this code here needs to be generalized.
            let (rhs_temp, _) = self.builder.emit_let(self.builder.mk_call_with_inst(
                &ghost_mem_ty,
                ghost_mem.inst.clone(),
                ast::Operation::Pack(ghost_mem.module_id, ghost_mem.id, None),
                vec![new_rhs.clone()],
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

    fn emit_lets(&mut self, spec: &TranslatedSpec, post_state: bool) {
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
                let annotated =
                    Self::annotate_pre_state_memory(env, exp.clone(), &mut spec.saved_memory);
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
    /// memory references, this is a no-op.
    fn annotate_pre_state_memory(
        env: &GlobalEnv,
        exp: Exp,
        saved_memory: &mut BTreeMap<QualifiedInstId<StructId>, MemoryLabel>,
    ) -> Exp {
        use ast::Operation as AstOp;
        ExpData::rewrite(exp, &mut |e| match e.as_ref() {
            ExpData::Call(id, AstOp::Global(None), args) => {
                let mem = env.get_node_instantiation(*id);
                if let Some(rty) = mem.first() {
                    let (mid, sid, inst) = rty.require_struct();
                    let l = *saved_memory
                        .entry(mid.qualified_inst(sid, inst.to_owned()))
                        .or_insert_with(|| MemoryLabel::new(env.new_global_id().as_usize()));
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
                    let l = *saved_memory
                        .entry(mid.qualified_inst(sid, inst.to_owned()))
                        .or_insert_with(|| MemoryLabel::new(env.new_global_id().as_usize()));
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
        use Bytecode::*;
        use PropKind::*;

        // Emit state-label-defining assumes so labeled memory variables are
        // constrained on the abort path (not only the normal return path).
        // Without this, labeled variables like Resource_$memory#S are
        // unconstrained when checking abort coverage, causing spurious failures
        // when address aliasing makes the abort condition depend on labeled state.
        // Use abort_path=true to assume only the defining fragment of each
        // condition, not extra properties that only hold on successful returns.
        self.emit_state_label_assumes(spec, true);

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
            self.builder.emit_let(cond).0
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

    fn generate_return_block(&mut self, spec: &TranslatedSpec) {
        use Bytecode::*;
        use PropKind::*;

        // Set the location to the function and emit label.
        self.builder
            .set_loc(self.builder.fun_env.get_loc().at_end());
        let ret_label = self.ret_label;
        self.builder.emit_with(|id| Label(id, ret_label));

        // Emit specification variable updates. They are generated for both verified and inlined
        // function variants, as the evolution of state updates is always the same.
        let lets_emitted = if !spec.updates.is_empty() {
            self.emit_lets(spec, true);
            self.emit_updates(spec, None);
            true
        } else {
            false
        };

        if self.is_verified() {
            // Emit `let` bindings if not already emitted.
            if !lets_emitted {
                self.emit_lets(spec, true);
            }

            // Emit well-formedness checks for choice expressions in post-state let bindings.
            self.emit_choice_wellformedness(spec, true);

            let defining_indices = self.emit_state_label_assumes(spec, false);

            // Emit the negation of all aborts conditions.
            // For defining conditions (already assumed), assert the non-defining
            // residual so mixed conditions like `mutation(...) && property` still
            // verify the property part.
            let post_count = spec.post.len();
            for (i, (loc, abort_cond, _)) in spec.aborts.iter().enumerate() {
                if defining_indices.contains(&(post_count + i)) {
                    if let Some(residual) = self.non_defining_residual(abort_cond) {
                        self.emit_traces(spec, abort_cond);
                        let exp = self.builder.mk_not(residual);
                        self.builder
                            .set_loc_and_vc_info(loc.clone(), ABORTS_IF_FAILS_MESSAGE);
                        self.builder.emit_with(|id| Prop(id, Assert, exp));
                    }
                    continue;
                }
                self.emit_traces(spec, abort_cond);
                let exp = self.builder.mk_not(abort_cond.clone());
                self.builder
                    .set_loc_and_vc_info(loc.clone(), ABORTS_IF_FAILS_MESSAGE);
                self.builder.emit_with(|id| Prop(id, Assert, exp))
            }

            // Emit return-point proof actions (`post`-prefixed proof statements).
            self.emit_proof_actions(&spec.post_proof, spec);

            // Emit all post-conditions which must hold.
            // For defining conditions (already assumed), assert the non-defining
            // residual so mixed conditions like `mutation(...) && result == 0`
            // still verify the `result == 0` part.
            for (i, (loc, cond)) in spec.post.iter().enumerate() {
                if defining_indices.contains(&i) {
                    if let Some(residual) = self.non_defining_residual(cond) {
                        self.emit_traces(spec, cond);
                        self.builder
                            .set_loc_and_vc_info(loc.clone(), ENSURES_FAILS_MESSAGE);
                        self.builder.emit_with(move |id| Prop(id, Assert, residual));
                    }
                    continue;
                }
                self.emit_traces(spec, cond);
                self.builder
                    .set_loc_and_vc_info(loc.clone(), ENSURES_FAILS_MESSAGE);
                self.builder
                    .emit_with(move |id| Prop(id, Assert, cond.clone()))
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

        // Emit return
        let ret_locals = self.ret_locals.clone();
        self.builder.emit_with(move |id| Ret(id, ret_locals))
    }

    /// Emit havoc+assume for intermediate state labels.
    /// Collects all conditions (ensures + aborts), finds which ones define state labels,
    /// and emits them as assumes in topological (dependency) order.
    /// Returns the set of condition indices that were emitted as assumes (defining conditions).
    /// If `abort_path` is true, assume only the defining fragment of each condition
    /// (conjuncts that contain label-defining operations), not extra properties
    /// that only hold on successful returns.
    fn emit_state_label_assumes(
        &mut self,
        spec: &TranslatedSpec,
        abort_path: bool,
    ) -> BTreeSet<usize> {
        use Bytecode::*;
        use PropKind::*;

        let saved_labels: BTreeSet<MemoryLabel> = spec.saved_memory.values().copied().collect();

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
                        self.emit_traces(spec, all_conditions[idx]);
                        let cond = if abort_path {
                            Self::defining_fragment(all_conditions[idx])
                        } else {
                            all_conditions[idx].clone()
                        };
                        self.builder.emit_with(move |id| Prop(id, Assume, cond));
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

    /// Returns the set of labels DEFINED by a condition (all range.post labels).
    fn defined_labels(exp: &Exp) -> BTreeSet<MemoryLabel> {
        exp.as_ref().all_defined_labels()
    }

    /// Extract the defining fragment of a condition: only the conjuncts that
    /// contain label-defining operations. Non-defining conjuncts (properties
    /// that only hold on successful returns) are dropped. Used on the abort
    /// path to avoid assuming postcondition properties.
    fn defining_fragment(exp: &Exp) -> Exp {
        use move_model::exp_simplifier::flatten_conjunction_owned;
        let conjuncts = flatten_conjunction_owned(exp);
        let defining: Vec<Exp> = conjuncts
            .into_iter()
            .filter(|c| !c.as_ref().all_defined_labels().is_empty())
            .collect();
        if defining.is_empty() {
            // No defining conjuncts — return true (no constraint).
            let id = exp.as_ref().node_id();
            ExpData::Value(id, Value::Bool(true)).into_exp()
        } else {
            defining
                .into_iter()
                .reduce(|a, b| {
                    let id = a.as_ref().node_id();
                    ExpData::Call(id, ast::Operation::And, vec![a, b]).into_exp()
                })
                .unwrap()
        }
    }

    /// Returns all labels referenced by a condition.
    fn all_labels(exp: &Exp) -> BTreeSet<MemoryLabel> {
        all_labels_in_exp(exp)
    }

    /// Compute the non-defining residual of a condition: the condition with all
    /// label-defining operations neutralized. Mutation builtins are replaced by `true`
    /// (boolean predicates about state transitions). For behavioral predicates and spec
    /// functions with `range.post`, stripping the label would change the state context
    /// (evaluating at exit instead of the labeled intermediate state), producing
    /// semantically incorrect assertions. For those, no residual is returned.
    /// Returns `None` if the residual is trivially `true` (pure definition) or if
    /// the condition contains label-defining Behavior/SpecFunction operations.
    fn non_defining_residual(&self, exp: &Exp) -> Option<Exp> {
        // If the condition contains label-defining Behavior or SpecFunction operations,
        // there is no meaningful residual: stripping the label changes the state context.
        // The assume already covers the verification obligation.
        let has_non_mutation_definers = {
            let mut found = false;
            exp.as_ref().visit_pre_order(&mut |e| {
                if let ExpData::Call(_, op, _) = e {
                    match op {
                        ast::Operation::Behavior(_, range)
                        | ast::Operation::SpecFunction(_, _, range)
                            if range.post.is_some() =>
                        {
                            found = true;
                            return false;
                        },
                        _ => {},
                    }
                }
                !found
            });
            found
        };
        if has_non_mutation_definers {
            return None;
        }
        struct MutationStripper;
        impl ExpRewriterFunctions for MutationStripper {
            fn rewrite_call(
                &mut self,
                id: move_model::model::NodeId,
                oper: &ast::Operation,
                _args: &[Exp],
            ) -> Option<Exp> {
                match oper {
                    ast::Operation::SpecPublish(_)
                    | ast::Operation::SpecRemove(_)
                    | ast::Operation::SpecUpdate(_) => {
                        Some(ExpData::Value(id, Value::Bool(true)).into_exp())
                    },
                    _ => None,
                }
            }
        }
        let mut stripper = MutationStripper;
        let residual = stripper.rewrite_exp(exp.clone());
        if matches!(residual.as_ref(), ExpData::Value(_, Value::Bool(true))) {
            None // Pure definition, no residual to assert
        } else {
            Some(residual)
        }
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

//  ================================================================================================
/// # Modifies Checker
/// Check modifies annotations. This is depending on usage analysis and is therefore
/// invoked here from the initialize trait function of this processor.
fn check_modifies(env: &GlobalEnv, targets: &FunctionTargetsHolder) {
    for module_env in env.get_modules() {
        if module_env.is_target() {
            for fun_env in module_env.get_functions() {
                if !fun_env.is_not_prover_target() && fun_env.is_compiled() {
                    check_caller_callee_modifies_relation(env, targets, &fun_env);
                    check_opaque_modifies_completeness(env, targets, &fun_env);
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

fn check_opaque_modifies_completeness(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    fun_env: &FunctionEnv,
) {
    let target = targets.get_target(fun_env, &FunctionVariant::Baseline);
    if !target.is_opaque() {
        return;
    }
    // All memory directly or indirectly modified by this opaque function must be captured by
    // a modifies clause. Otherwise we could introduce unsoundness.
    // TODO: we currently except Event::EventHandle from this, because this is treated as
    //   an immutable reference. We should find a better way how to deal with event handles.
    for mem in usage_analysis::get_memory_usage(&target)
        .modified
        .all
        .iter()
    {
        if env.is_wellknown_event_handle_type(&Type::Struct(mem.module_id, mem.id, vec![])) {
            continue;
        }
        if env.get_struct_qid(mem.to_qualified_id()).is_ghost_memory() {
            continue;
        }
        let found = target.get_modify_ids().iter().any(|id| mem == id);
        if !found {
            let loc = fun_env.get_spec_loc();
            env.error(&loc,
            &format!("function `{}` is opaque but its specification does not have a modifies clause for `{}`",
                fun_env.get_full_name_str(),
                env.display(mem))
            )
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
