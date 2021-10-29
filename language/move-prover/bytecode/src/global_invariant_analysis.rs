// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

// Analysis pass which analyzes how to injects global invariants into the bytecode.

use crate::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{
        FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant, VerificationFlavor,
    },
    stackless_bytecode::{BorrowNode, Bytecode, Operation},
    usage_analysis,
    verification_analysis::{is_invariant_suspendable, InvariantAnalysisData},
};

use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::ConditionKind,
    model::{FunctionEnv, GlobalEnv, GlobalId, QualifiedInstId, StructId},
    ty::{Type, TypeDisplayContext, TypeInstantiationDerivation, TypeUnificationAdapter, Variance},
};

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

/// A named struct for holding the information on how an invariant is relevant to a bytecode.
#[derive(Default, Clone)]
pub struct PerBytecodeRelevance {
    /// for each `inst_fun` (instantiation of function type parameters) in the key set, the
    /// associated value is a set of `inst_inv` (instantiation of invariant type parameters) that
    /// are applicable to the concrete function instance `F<inst_fun>`.
    pub insts: BTreeMap<Vec<Type>, BTreeSet<Vec<Type>>>,
}

/// A named struct for holding the information on how invariants are relevant to a function.
#[derive(Clone)]
pub struct PerFunctionRelevance {
    /// Invariants that needs to be assumed at function entrypoint
    /// - Key: global invariants that needs to be assumed before the first instruction,
    /// - Value: the instantiation information per each related invariant.
    pub entrypoint_assumptions: BTreeMap<GlobalId, PerBytecodeRelevance>,

    /// For each bytecode at given code offset, the associated value is a map of
    /// - Key: global invariants that needs to be asserted after the bytecode instruction and
    /// - Value: the instantiation information per each related invariant.
    pub per_bytecode_assertions: BTreeMap<CodeOffset, BTreeMap<GlobalId, PerBytecodeRelevance>>,

    /// Invariants that needs to be asserted at function exitpoint
    /// - Key: global invariants that needs to be assumed before the first instruction,
    /// - Value: the instantiation information per each related invariant.
    pub exitpoint_assertions: BTreeMap<GlobalId, PerBytecodeRelevance>,
}

/// Get verification information for this function.
pub fn get_info<'env>(target: &FunctionTarget<'env>) -> &'env PerFunctionRelevance {
    target
        .get_annotations()
        .get::<PerFunctionRelevance>()
        .expect("Global invariant analysis not performed")
}

// The function target processor
pub struct GlobalInvariantAnalysisProcessor {}

impl GlobalInvariantAnalysisProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl FunctionTargetProcessor for GlobalInvariantAnalysisProcessor {
    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv<'_>,
        mut data: FunctionData,
    ) -> FunctionData {
        if fun_env.is_native() || fun_env.is_intrinsic() {
            // Nothing to do
            return data;
        }
        if !data.variant.is_verified() {
            // Only need to instrument if this is a verification variant
            return data;
        }

        // Analyze invariants
        let target = FunctionTarget::new(fun_env, &data);
        let analysis_result = PerFunctionRelevance::analyze(&target, targets);
        data.annotations.set(analysis_result);

        // This is an analysis pass, nothing gets changed
        data
    }

    fn name(&self) -> String {
        "global_invariant_analysis".to_string()
    }

    fn dump_result(
        &self,
        f: &mut fmt::Formatter<'_>,
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        // utils
        let type_display_ctxt = TypeDisplayContext::WithEnv {
            env,
            type_param_names: None,
        };

        let display_type_slice = |tys: &[Type]| -> String {
            let content = tys
                .iter()
                .map(|t| t.display(&type_display_ctxt).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("<{}>", content)
        };

        let make_indent = |indent: usize| "  ".repeat(indent);

        let display_inv_relevance = |f: &mut fmt::Formatter,
                                     invs: &BTreeMap<GlobalId, PerBytecodeRelevance>,
                                     header: &str,
                                     assert_or_assume: &str|
         -> fmt::Result {
            let mut indent = 1;

            // oneliner for empty invs
            if invs.is_empty() {
                return writeln!(f, "{}{} {{}}", make_indent(indent), header);
            }

            writeln!(f, "{}{} {{", make_indent(indent), header)?;
            indent += 1;

            for (inv_id, inv_rel) in invs {
                writeln!(
                    f,
                    "{}{} {} = [",
                    make_indent(indent),
                    assert_or_assume,
                    inv_id
                )?;
                indent += 1;

                for (rel_inst, inv_insts) in &inv_rel.insts {
                    writeln!(
                        f,
                        "{}{} -> [",
                        make_indent(indent),
                        display_type_slice(rel_inst)
                    )?;
                    indent += 1;

                    for inv_inst in inv_insts {
                        writeln!(f, "{}{}", make_indent(indent), display_type_slice(inv_inst))?;
                    }

                    indent -= 1;
                    writeln!(f, "{}]", make_indent(indent))?;
                }

                indent -= 1;
                writeln!(f, "{}]", make_indent(indent))?;
            }

            indent -= 1;
            writeln!(f, "{}}}", make_indent(indent))
        };

        writeln!(
            f,
            "\n********* Result of global invariant instrumentation *********\n"
        )?;
        for (fun_id, fun_variant) in targets.get_funs_and_variants() {
            if !matches!(
                fun_variant,
                FunctionVariant::Verification(VerificationFlavor::Regular)
            ) {
                // the analysis results are available in the regular verification variant
                continue;
            }

            let fenv = env.get_function(fun_id);
            let target = targets.get_target(&fenv, &fun_variant);
            let result = target
                .get_annotations()
                .get::<PerFunctionRelevance>()
                .expect("Analysis not performed");

            writeln!(f, "{}: [", fenv.get_full_name_str())?;

            // display entrypoint assumptions
            display_inv_relevance(f, &result.entrypoint_assumptions, "entrypoint", "assume")?;

            // display per-bytecode assertions
            for (code_offset, code_invs) in &result.per_bytecode_assertions {
                let bc = target.data.code.get(*code_offset as usize).unwrap();
                let header = format!("{}: {}", code_offset, bc.display(&target, &BTreeMap::new()));
                display_inv_relevance(f, code_invs, &header, "assert")?;
            }

            // display exitpoint assertions
            display_inv_relevance(f, &result.exitpoint_assertions, "exitpoint", "assert")?;

            writeln!(f, "]")?;
        }

        writeln!(f, "\n********* Global invariants by ID *********\n")?;
        let mut all_invs = BTreeSet::new();
        for menv in env.get_modules() {
            all_invs.extend(env.get_global_invariants_by_module(menv.get_id()));
        }
        for inv_id in all_invs {
            let inv = env.get_global_invariant(inv_id).unwrap();
            let inv_src = env.get_source(&inv.loc).unwrap_or("<unknown invariant>");
            writeln!(f, "{} => {}", inv_id, inv_src)?;
        }
        writeln!(f)
    }
}

/// This impl block is about the analysis pass
impl PerFunctionRelevance {
    /// Collect and build the relevance analysis information for this function target.
    fn analyze(target: &FunctionTarget, targets: &FunctionTargetsHolder) -> Self {
        use BorrowNode::*;
        use Bytecode::*;
        use Operation::*;

        // collect information
        let fid = target.func_env.get_qualified_id();
        let env = target.global_env();
        let inv_analysis = env
            .get_extension::<InvariantAnalysisData>()
            .expect("Verification analysis not performed");

        let check_suspendable_inv_on_return =
            inv_analysis.fun_set_with_inv_check_on_exit.contains(&fid);
        let inv_applicability = inv_analysis
            .fun_to_inv_map
            .get(&fid)
            .expect("Invariant applicability not available");

        let mem_analysis = usage_analysis::get_memory_usage(target);

        let fun_type_params = target.get_type_parameters();
        let fun_type_params_arity = fun_type_params.len();

        // collect invariant applicability and instantiation information for entrypoint assumptions
        //
        // NOTE: why do we use the `InvariantRelevance::accessed` set instead of other sets?
        //
        // - The reason we choose `accessed` over `direct_accessed` is that sometimes we need to
        //   assume invariants that are applicable to callees only and not applicable to the caller.
        //   The reason is that if we inline a callee function, proving properties about the inlined
        //   function might require assumptions about the memories it touches (e.g., proving the
        //   `borrow_global_mut<R>(addr)` does not abort with the invariant that resource `R` must
        //   exist under account `addr` after operation has started).
        //
        //   It does not hurt (in terms of soundness or completeness of the proofs) to assume extra
        //   invariants in the `accessed` set even when these assumptions are not actually used in
        //   proofs of any asserts. We might re-consider this when performance (due to too many
        //   assumptions added to the proof system) becomes an issue.
        //
        // - The reason we choose `direct_accessed` over `direct_modified` is that we may need
        //   assumptions from global invariants to prove properties in the code.
        //
        //   For example, we may have an `invariant exists<A>(0x1) ==> exists<B>(0x1);` while in
        //   the code, we have `if (exists<A>(0x1)) { borrow_global<B>(0x1); }`. With the global
        //   invariant, we know that the `borrow_global` won't abort. But we won't be able to prove
        //   this property without the global invariant.
        let entrypoint_invariants: BTreeSet<_> = inv_applicability
            .accessed
            .iter()
            .filter_map(|&inv_id| {
                let inv = env.get_global_invariant(inv_id).unwrap();
                // update invariants should not be assumed at function entrypoint.
                matches!(inv.kind, ConditionKind::GlobalInvariant(..)).then(|| inv_id)
            })
            .collect();
        let entrypoint_assumptions = Self::calculate_invariant_relevance(
            env,
            mem_analysis.accessed.all.iter(),
            &entrypoint_invariants,
            fun_type_params_arity,
        );

        // if this function defers invariant checking on return, filter out invariants that are
        // suspended in body.
        //
        // NOTE: why do we use the `InvariantRelevance::direct_modified` set instead of other sets?
        //
        // First, be reminded that in the rest of this function, we aim to find which invariants
        // should be *asserted* at each bytecode instruction. Therefore, if a bytecode instruction
        // only reads some memory but never modifies one, we don't need to assert the invariant.
        // This rules out the `direct_accessed` and `accessed` sets.
        //
        // Second, similar to the reason why we choose `direct_accessed` set over `accessed` for
        // invariants that constitute entrypoint assumptions, we choose `direct_modified` over
        // `modified` is that we don't want to assert invariants that are applicable to callees
        // only and not applicable to the caller. The reason is still: if a suspendable invariant is
        // delegated to the caller, that invariant will appear in the `direct_modified` set on the
        // caller side.
        let (inv_related_return, inv_related_normal): (BTreeSet<_>, BTreeSet<_>) =
            if check_suspendable_inv_on_return {
                inv_applicability
                    .direct_modified
                    .iter()
                    .cloned()
                    .partition(|inv_id| is_invariant_suspendable(env, *inv_id))
            } else {
                (BTreeSet::new(), inv_applicability.direct_modified.clone())
            };

        // collect invariant applicability and instantiation information per bytecode, i.e.,
        // - which invariants should be instrumented after each instruction and
        // - per each invariant applicable, how to instantiate them.
        let mut per_bytecode_assertions = BTreeMap::new();
        let mut mem_related_on_return = BTreeSet::new();
        let mut exitpoint_assertions = None;

        for (code_offset, bc) in target.data.code.iter().enumerate() {
            let code_offset = code_offset as CodeOffset;

            // collect memory modified in operations
            let mem_related = match bc {
                Call(_, _, oper, _, _) => match oper {
                    Function(mid, fid, inst) | OpaqueCallEnd(mid, fid, inst) => {
                        let callee_fid = mid.qualified(*fid);

                        // shortcut the call if the callee does not delegate invariant checking.
                        //
                        // NOTE: in this case, memories modified by the callee are NOT back
                        // propagated to the caller in the `verification_analysis.rs`, which means,
                        // the `InvariantRelevance::direct_modified` set for the caller does NOT
                        // necessarily cover invariants that are related to the callee.
                        if !inv_analysis.fun_set_with_no_inv_check.contains(&callee_fid) {
                            continue;
                        }

                        let callee_env = env.get_function(callee_fid);
                        let callee_target =
                            targets.get_target(&callee_env, &FunctionVariant::Baseline);
                        let callee_usage = usage_analysis::get_memory_usage(&callee_target);

                        // NOTE: it is important to include *ALL* memories modified by the callee
                        // instead of just the direct ones --- if a function `F` delegates
                        // suspendable invariant checking to its caller, all the functions that `F`
                        // calls will not check suspendable invariants anymore.
                        callee_usage.modified.get_all_inst(inst)
                    }

                    MoveTo(mid, sid, inst) | MoveFrom(mid, sid, inst) => {
                        let mem = mid.qualified_inst(*sid, inst.to_owned());
                        std::iter::once(mem).collect()
                    }
                    WriteBack(GlobalRoot(mem), _) => std::iter::once(mem.clone()).collect(),

                    // shortcut other operations
                    _ => continue,
                },

                Ret(..) if check_suspendable_inv_on_return => {
                    std::mem::take(&mut mem_related_on_return)
                }

                // shortcut other bytecodes
                _ => continue,
            };

            // mark whether we are processing a return instruction
            let is_return = matches!(bc, Ret(..));

            // select the related invariants based on whether this bytecode instruction is a return
            let inv_related = if is_return {
                &inv_related_return
            } else {
                &inv_related_normal
            };

            // collect instantiation information
            let relevance = Self::calculate_invariant_relevance(
                env,
                mem_related.iter(),
                inv_related,
                fun_type_params_arity,
            );

            if is_return {
                // capture invariants asserted before return
                if exitpoint_assertions.is_some() {
                    panic!("Expect at most one return instruction in the function body");
                }
                exitpoint_assertions = Some(relevance);
            } else {
                // capture invariants asserted after the bytecode
                per_bytecode_assertions.insert(code_offset, relevance);

                // save the related memories for return point if the function defers that
                if check_suspendable_inv_on_return {
                    mem_related_on_return.extend(mem_related);
                }
            }
        }

        // sanity check: the deferred memory is indeed consumed by a return instruction, UNLESS
        // the deferred memory do not touch anything that is checked in any suspendable invariant.
        if !mem_related_on_return.is_empty() {
            let mut deferred_invs = vec![];
            'error_check: for inv_id in inv_related_return {
                let inv = env.get_global_invariant(inv_id).unwrap();
                for inv_mem in &inv.mem_usage {
                    let inv_ty = inv_mem.to_type();
                    for rel_mem in &mem_related_on_return {
                        let rel_ty = rel_mem.to_type();
                        let adapter =
                            TypeUnificationAdapter::new_pair(&rel_ty, &inv_ty, true, true);
                        if adapter.unify(Variance::Allow, false).is_none() {
                            deferred_invs.push(inv_id);
                            continue 'error_check;
                        }
                    }
                }
            }
            if !deferred_invs.is_empty() {
                env.error(
                    &target.get_loc(),
                    &format!(
                        "Function `{}` defers the checking of {} suspendable invariants to the \
                        return point, but the function never returns",
                        target.func_env.get_full_name_str(),
                        deferred_invs.len(),
                    ),
                );
            }
        }

        // wrap and return the analysis result
        Self {
            entrypoint_assumptions,
            per_bytecode_assertions,
            exitpoint_assertions: exitpoint_assertions.unwrap_or_default(),
        }
    }

    /// Given a set of memories, calculate the global invariants that are related to this memory
    /// set and for each related global invariant, derive how to instantiate the invariant to make
    /// it relevant.
    fn calculate_invariant_relevance<'a>(
        env: &GlobalEnv,
        mem_related: impl Iterator<Item = &'a QualifiedInstId<StructId>>,
        inv_related: &BTreeSet<GlobalId>,
        fun_type_params_arity: usize,
    ) -> BTreeMap<GlobalId, PerBytecodeRelevance> {
        let mut result = BTreeMap::new();

        for rel_mem in mem_related {
            let rel_ty = rel_mem.to_type();
            for inv_id in inv_related {
                let inv = env.get_global_invariant(*inv_id).unwrap();
                let inv_type_params = match &inv.kind {
                    ConditionKind::GlobalInvariant(params) => params,
                    ConditionKind::GlobalInvariantUpdate(params) => params,
                    _ => unreachable!(
                        "A global invariant must have a condition kind of either \
                            `GlobalInvariant` or `GlobalInvariantUpdate`"
                    ),
                };
                let inv_type_params_arity = inv_type_params.len();

                for inv_mem in &inv.mem_usage {
                    let inv_ty = inv_mem.to_type();

                    // make sure these two types unify before trying to instantiate them
                    let adapter = TypeUnificationAdapter::new_pair(&rel_ty, &inv_ty, true, true);
                    if adapter.unify(Variance::Allow, false).is_none() {
                        continue;
                    }

                    // instantiate the bytecode first
                    //
                    // NOTE: in fact, in this phase, we don't intend to instantiation the function
                    // nor do we want to collect information on how this function (or this bytecode)
                    // needs to be instantiated. All we care is how the invariant should be
                    // instantiated in order to be instrumented at this code point, with a generic
                    // function and generic code.
                    //
                    // But unfortunately, based on how the type unification logic is written now,
                    // this two-step instantiation is needed in order to find all possible
                    // instantiations of the invariant. I won't deny that there might be a way to
                    // collect invariant instantiation combinations without instantiating the
                    // function type parameters, but I haven't iron out one so far.
                    let rel_insts = TypeInstantiationDerivation::progressive_instantiation(
                        std::iter::once(&rel_ty),
                        std::iter::once(&inv_ty),
                        true,
                        true,
                        true,
                        false,
                        fun_type_params_arity,
                        true,
                        false,
                    );

                    // for each instantiation of the bytecode, instantiate the invariants
                    for rel_inst in rel_insts {
                        let inst_rel_ty = rel_ty.instantiate(&rel_inst);
                        let inv_insts = TypeInstantiationDerivation::progressive_instantiation(
                            std::iter::once(&inst_rel_ty),
                            std::iter::once(&inv_ty),
                            false,
                            true,
                            false,
                            true,
                            inv_type_params_arity,
                            false,
                            true,
                        );

                        let mut wellformed_inv_inst = vec![];
                        for inv_inst in inv_insts {
                            if inv_inst.iter().any(|t| matches!(t, Type::Error)) {
                                // TODO(mengxu): handle uninstantiable generic invariants.
                                // One possibility is to handle them via phantom type parameters.
                            } else {
                                wellformed_inv_inst.push(inv_inst);
                            }
                        }

                        // record the relevance information
                        result
                            .entry(*inv_id)
                            .or_insert_with(PerBytecodeRelevance::default)
                            .insts
                            .entry(rel_inst)
                            .or_insert_with(BTreeSet::new)
                            .extend(wellformed_inv_inst);
                    }
                }
            }
        }

        result
    }
}
