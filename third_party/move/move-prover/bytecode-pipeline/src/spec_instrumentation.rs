// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Transformation which injects specifications (Move function spec blocks) into the bytecode.

use crate::{options::ProverOptions, verification_analysis};
use itertools::Itertools;
use move_model::{
    ast,
    ast::{Exp, ExpData, TempIndex, Value},
    exp_generator::ExpGenerator,
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, QualifiedId, QualifiedInstId, StructId},
    pragmas::{ABORTS_IF_IS_PARTIAL_PRAGMA, EMITS_IS_PARTIAL_PRAGMA, EMITS_IS_STRICT_PRAGMA},
    spec_translator::{SpecTranslator, TranslatedSpec},
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

//  ================================================================================================
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
            verification_data =
                Instrumenter::run(&options, targets, fun_env, verification_data, scc_opt);
            targets.insert_target_data(
                &fun_env.get_qualified_id(),
                verification_data.variant.clone(),
                verification_data,
            );
        }

        // Instrument baseline variant only if it is inlined.
        if is_inlined {
            Instrumenter::run(&options, targets, fun_env, data, scc_opt)
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
                if fun.is_inline() {
                    continue;
                }
                for (variant, target) in targets.get_targets(fun) {
                    let spec = &*target.get_spec();
                    if !spec.conditions.is_empty() {
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
}

impl<'a> Instrumenter<'a> {
    #[allow(clippy::needless_collect)]
    fn run(
        options: &'a ProverOptions,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv<'a>,
        data: FunctionData,
        scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
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
        };
        instrumenter.instrument(&spec, &inlined_props);

        // Run copy propagation (reaching definitions) and then assignment
        // elimination (live vars). This cleans up some redundancy created by
        // the instrumentation scheme.
        let mut data = instrumenter.builder.data;
        let reach_def = ReachingDefProcessor::new();
        let live_vars = LiveVarAnalysisProcessor::new_no_annotate();
        data = reach_def.process(targets, fun_env, data, scc_opt);
        live_vars.process(targets, fun_env, data, scc_opt)
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

        if self.is_verified() {
            // Inject 'CanModify' assumptions for this function.
            for (loc, exp) in &spec.modifies {
                let struct_ty = self.builder.global_env().get_node_type(exp.node_id());
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

        // Instrument and generate new code
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
                let addr_exp = self.builder.mk_temporary(srcs[1]);
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
            Abort(id, code) => {
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

        self.builder.set_loc_from_attr(id);

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

        // Emit modify permissions as assertions if this is the verification variant. For
        // non-verification variants, we don't need to do this because they are independently
        // verified.
        if self.is_verified() {
            let loc = self.builder.get_loc(id);
            for (_, exp) in &callee_spec.modifies {
                let rty = env.get_node_type(exp.node_id());
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
        if !callee_env.is_opaque() || self.options.for_interpretation {
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

            // Emit memory state saves
            for (mem, label) in std::mem::take(&mut callee_spec.saved_memory) {
                self.builder.emit_with(|id| SaveMem(id, label, mem));
            }
            for (var, label) in std::mem::take(&mut callee_spec.saved_spec_vars) {
                self.builder.emit_with(|id| SaveSpecVar(id, label, var));
            }

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
        self.builder.emit_with(|id| Abort(id, abort_local));
    }

    /// Generates verification conditions for abort block.
    fn generate_abort_verify(&mut self, spec: &TranslatedSpec) {
        use Bytecode::*;
        use PropKind::*;

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

            // Emit the negation of all aborts conditions.
            for (loc, abort_cond, _) in &spec.aborts {
                self.emit_traces(spec, abort_cond);
                let exp = self.builder.mk_not(abort_cond.clone());
                self.builder
                    .set_loc_and_vc_info(loc.clone(), ABORTS_IF_FAILS_MESSAGE);
                self.builder.emit_with(|id| Prop(id, Assert, exp))
            }

            // Emit all post-conditions which must hold as we do not abort.
            for (loc, cond) in &spec.post {
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
                    &modify_check_fails_message(env, mid.qualified(sid), inst),
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

//  ================================================================================================
// # Modifies Checker

/// Check modifies annotations. This is depending on usage analysis and is therefore
/// invoked here from the initialize trait function of this processor.
fn check_modifies(env: &GlobalEnv, targets: &FunctionTargetsHolder) {
    for module_env in env.get_modules() {
        if module_env.is_target() {
            for fun_env in module_env.get_functions() {
                if !fun_env.is_inline() {
                    check_caller_callee_modifies_relation(env, targets, &fun_env);
                    check_opaque_modifies_completeness(env, targets, &fun_env);
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
        if callee_fun_env.is_native() || callee_fun_env.is_intrinsic() {
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
