// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    compositional_analysis::{CompositionalAnalysis, SummaryCache},
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult, SetDomain},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{BorrowNode, Bytecode, Operation, PropKind},
};
use abstract_domain_derive::AbstractDomain;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{ConditionKind, Spec, TempIndex},
    model::{FunId, FunctionEnv, GlobalEnv, QualifiedId, QualifiedInstId, StructId},
    pragmas::VERIFY_PRAGMA,
    ty::Type,
};
use paste::paste;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::Formatter,
};

pub fn get_memory_usage<'env>(target: &FunctionTarget<'env>) -> &'env UsageState {
    target
        .get_annotations()
        .get::<UsageState>()
        .expect("Invariant violation: target not analyzed")
}

/// A summary of the memory accessed / modified per function, both directly and transitively.
#[derive(AbstractDomain, Default, Clone)]
pub struct MemoryUsage {
    // The memory directly used in the function.
    pub direct: SetDomain<QualifiedInstId<StructId>>,
    // The memory transitively used in either the function itself or at least one of its callees.
    pub transitive: SetDomain<QualifiedInstId<StructId>>,
    // The union of the above sets
    pub all: SetDomain<QualifiedInstId<StructId>>,
}

#[derive(AbstractDomain, Default, Clone)]
pub struct UsageState {
    /// The memory accessed by this function. This is the union of the three individual fields
    /// below.
    pub accessed: MemoryUsage,
    /// The memory modified by this function.
    pub modified: MemoryUsage,
    /// The memory declared as invocable-through by `modifies_of` frame
    /// specifications reachable in this function's body — the enclosing
    /// function's own `modifies_of` parameter frames, the struct-field
    /// `modifies_of` declarations that any Invoke may reach, and (via
    /// `subsume_callee`) the same for all callees. Distinct from
    /// `modified` so downstream checks over declared modifies (e.g.
    /// opacity coverage) are not perturbed; consumed by loop-header havoc
    /// collection.
    pub invoke_frame: MemoryUsage,
    /// True if any `modifies_of<f> *` wildcard is reachable in this
    /// function's body or its callees. Deliberately unresolved at
    /// summary time: a pure forwarder's own accessed footprint is empty,
    /// so callers must resolve the wildcard against *their* accessed
    /// footprint at each use site (loop-header havoc, Invoke, etc.).
    pub invoke_frame_wildcard: bool,
    /// True if this function's code, or the code of any function it calls, invokes a
    /// function value whose effects nothing bounds. An invocation is bounded only when it
    /// invokes the function's own, never reassigned parameter and every call site checks the
    /// argument against a bound: the function declares `modifies_of`/`reads_of` for the
    /// parameter, or it is opaque (an undeclared frame is then empty and enforced, or left to
    /// the call site, which must supply a closure with known effects). A summary with this
    /// flag set does not account for everything a call to the function does.
    pub unbounded_invoke: bool,
    /// `invoke_frame` without the enclosing function's own parameter frames: the
    /// struct-field frames and the callees' `invoke_frame`s. At a call site, the
    /// parameter frames can be replaced by what the arguments actually modify.
    pub invoke_frame_other: SetDomain<QualifiedInstId<StructId>>,
    /// The memory read or written by the code of this function or its callees, excluding
    /// memory only mentioned by specifications.
    pub code_accessed: SetDomain<QualifiedInstId<StructId>>,
    /// The memory mentioned by the assume expressions in this function.
    pub assumed: MemoryUsage,
    /// The memory mentioned by the assert expressions in this function.
    pub asserted: MemoryUsage,
}

impl MemoryUsage {
    //
    // setters that insert element(s) to related sets
    //

    fn add_direct(&mut self, mem: QualifiedInstId<StructId>) {
        self.direct.insert(mem.clone());
        self.all.insert(mem);
    }

    fn add_transitive(&mut self, mem: QualifiedInstId<StructId>) {
        self.transitive.insert(mem.clone());
        self.all.insert(mem);
    }

    //
    // accessors that further instantiate the memories
    //

    pub fn get_direct_inst(&self, inst: &[Type]) -> BTreeSet<QualifiedInstId<StructId>> {
        self.direct
            .iter()
            .map(|mem| mem.instantiate_ref(inst))
            .collect()
    }

    pub fn get_transitive_inst(&self, inst: &[Type]) -> BTreeSet<QualifiedInstId<StructId>> {
        self.transitive
            .iter()
            .map(|mem| mem.instantiate_ref(inst))
            .collect()
    }

    pub fn get_all_inst(&self, inst: &[Type]) -> BTreeSet<QualifiedInstId<StructId>> {
        self.all
            .iter()
            .map(|mem| mem.instantiate_ref(inst))
            .collect()
    }

    //
    // accessors that uninstantiate the memories
    //

    pub fn get_direct_uninst(&self) -> BTreeSet<QualifiedId<StructId>> {
        self.direct
            .iter()
            .map(|mem| mem.module_id.qualified(mem.id))
            .collect()
    }

    pub fn get_transitive_uninst(&self) -> BTreeSet<QualifiedId<StructId>> {
        self.transitive
            .iter()
            .map(|mem| mem.module_id.qualified(mem.id))
            .collect()
    }

    pub fn get_all_uninst(&self) -> BTreeSet<QualifiedId<StructId>> {
        self.all
            .iter()
            .map(|mem| mem.module_id.qualified(mem.id))
            .collect()
    }
}

macro_rules! generate_inserter {
    ($field:ident, $method:ident) => {
        paste! {
            #[allow(dead_code)]
            fn [<$method _ $field>](&mut self, mem: QualifiedInstId<StructId>) {
                self.$field.$method(mem.clone());
                self.accessed.$method(mem);
            }

            #[allow(dead_code)]
            fn [<$method _ $field _iter>](
                &mut self,
                mems: impl Iterator<Item = QualifiedInstId<StructId>>
            ) {
                for mem in mems {
                    self.[<$method _ $field>](mem);
                }
            }
        }
    };
}

/// Generated functions
impl UsageState {
    generate_inserter!(accessed, add_direct);

    generate_inserter!(accessed, add_transitive);

    generate_inserter!(modified, add_direct);

    generate_inserter!(modified, add_transitive);

    generate_inserter!(assumed, add_direct);

    generate_inserter!(assumed, add_transitive);

    generate_inserter!(asserted, add_direct);

    generate_inserter!(asserted, add_transitive);
}

/// Direct-only inserters for `invoke_frame`: it does not participate in the
/// `accessed` union (unlike `modified`, an invocable-through frame is a
/// distinct concept from an actually-accessed memory).
impl UsageState {
    fn add_direct_invoke_frame(&mut self, mem: QualifiedInstId<StructId>) {
        self.invoke_frame.direct.insert(mem.clone());
        self.invoke_frame.all.insert(mem);
    }

    fn add_transitive_invoke_frame_iter(
        &mut self,
        mems: impl Iterator<Item = QualifiedInstId<StructId>>,
    ) {
        for mem in mems {
            self.invoke_frame.transitive.insert(mem.clone());
            self.invoke_frame.all.insert(mem);
        }
    }
}

/// Helpers for the abstract interpretation process
impl UsageState {
    fn subsume_callee(&mut self, callee: &Self, inst: &[Type]) {
        self.add_transitive_accessed_iter(callee.accessed.get_all_inst(inst).into_iter());
        self.add_transitive_modified_iter(callee.modified.get_all_inst(inst).into_iter());
        self.add_transitive_assumed_iter(callee.assumed.get_all_inst(inst).into_iter());
        self.add_transitive_asserted_iter(callee.asserted.get_all_inst(inst).into_iter());
        self.code_accessed.extend(
            callee
                .code_accessed
                .iter()
                .map(|mem| mem.instantiate_ref(inst)),
        );
        let callee_invoke_frame = callee.invoke_frame.get_all_inst(inst);
        self.invoke_frame_other
            .extend(callee_invoke_frame.iter().cloned());
        self.add_transitive_invoke_frame_iter(callee_invoke_frame.into_iter());
        self.invoke_frame_wildcard |= callee.invoke_frame_wildcard;
        self.unbounded_invoke |= callee.unbounded_invoke;
    }
}

struct MemoryUsageAnalysis<'a> {
    cache: SummaryCache<'a>,
}

impl DataflowAnalysis for MemoryUsageAnalysis<'_> {}

impl CompositionalAnalysis<UsageState> for MemoryUsageAnalysis<'_> {
    fn to_summary(&self, state: UsageState, _fun_target: &FunctionTarget) -> UsageState {
        state
    }
}

impl TransferFunctions for MemoryUsageAnalysis<'_> {
    type State = UsageState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, code: &Bytecode, _offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;
        use PropKind::*;

        match code {
            // memory accesses in operations
            Call(_, _, oper, _, _) => match oper {
                Function(mid, fid, inst)
                | OpaqueCallBegin(mid, fid, inst)
                | OpaqueCallEnd(mid, fid, inst) => {
                    let callee_id = mid.qualified(*fid);
                    if let Some(summary) = self
                        .cache
                        .get::<UsageState>(callee_id, &FunctionVariant::Baseline)
                    {
                        state.subsume_callee(summary, inst);
                    }
                },
                MoveTo(mid, sid, inst)
                | MoveFrom(mid, sid, inst)
                | BorrowGlobal(mid, sid, inst) => {
                    let mem = mid.qualified_inst(*sid, inst.to_owned());
                    state.code_accessed.insert(mem.clone());
                    state.add_direct_modified(mem);
                },
                WriteBack(BorrowNode::GlobalRoot(mem), _) => {
                    state.code_accessed.insert(mem.clone());
                    state.add_direct_modified(mem.clone());
                },
                Exists(mid, sid, inst) | GetGlobal(mid, sid, inst) => {
                    let mem = mid.qualified_inst(*sid, inst.to_owned());
                    state.code_accessed.insert(mem.clone());
                    state.add_direct_accessed(mem);
                },
                _ => {},
            },
            // memory accesses in expressions
            Prop(_, kind, exp) => match kind {
                Assume => state.add_direct_assumed_iter(
                    exp.used_memory(self.cache.global_env())
                        .into_iter()
                        .map(|(usage, _)| usage),
                ),
                Assert => state.add_direct_asserted_iter(
                    exp.used_memory(self.cache.global_env())
                        .into_iter()
                        .map(|(usage, _)| usage),
                ),
                Modifies => {
                    // do nothing, as the `modifies` memories are captured by other sets
                },
            },
            _ => {},
        }
    }
}

impl MemoryUsageAnalysis<'_> {
    /// Compute usage information for the given spec. This spec maybe injected in later
    /// phases into the code, but we need to account for it's memory usage already here
    /// as spec injection itself depends on this information.
    fn compute_spec_usage(&self, spec: &Spec, state: &mut UsageState) {
        use ConditionKind::*;
        for cond in &spec.conditions {
            let mut used_memory = cond.exp.used_memory(self.cache.global_env());
            for exp in &cond.additional_exps {
                used_memory.extend(exp.used_memory(self.cache.global_env()));
            }
            match &cond.kind {
                Ensures | AbortsIf | Emits => {
                    state.add_direct_asserted_iter(used_memory.into_iter().map(|(usage, _)| usage));
                },
                _ => {
                    state.add_direct_assumed_iter(used_memory.into_iter().map(|(usage, _)| usage));
                },
            }
            if matches!(cond.kind, Update) {
                // Add target of spec update to modified memory
                if let Some((mem, _, _)) =
                    cond.additional_exps[0].extract_ghost_mem_access(self.cache.global_env())
                {
                    state.add_direct_modified(mem);
                }
            }
        }

        // Handle memory update of the specs in the function body
        for impl_spec in spec.on_impl.values() {
            for cond in &impl_spec.conditions {
                if matches!(cond.kind, Update) && !cond.additional_exps.is_empty() {
                    if let Some((mem, _, _)) =
                        cond.additional_exps[0].extract_ghost_mem_access(self.cache.global_env())
                    {
                        state.add_direct_modified(mem);
                    }
                }
            }
        }
    }

    /// Add memory declared as invocable-through by `modifies_of` frame
    /// specifications to the modified summary of any function that invokes a
    /// function value. Without this, a helper whose only effect is invoking
    /// its declared parameter has an empty `modified` summary, and callers
    /// that inherit that summary (e.g. loop-header havoc collection) miss
    /// the frame — the effect propagates through arbitrary call depth via
    /// the existing `subsume_callee` transitive closure.
    ///
    /// Sources of Invoke frame memory (all unioned; provenance of the
    /// invoked value is not tracked at this stage):
    /// - The enclosing function's `modifies_of` parameter frames — a
    ///   `modifies_of<f> *` wildcard falls back to the current summary's
    ///   accessed footprint.
    /// - Struct fields with `modifies_of` declarations, whose field type
    ///   may match the invoked value's type — same wildcard treatment.
    ///
    /// Foreign type-parameter contexts (a struct spec's or the enclosing
    /// function's `*` fallback) set the wildcard flag instead of being
    /// inserted with a mis-scoped instantiation.
    fn compute_invoke_usage(&self, target: &FunctionTarget, state: &mut UsageState) {
        use Bytecode::*;
        use Operation::*;
        let env = self.cache.global_env();
        let has_invoke = target
            .get_bytecode()
            .iter()
            .any(|bc| matches!(bc, Call(_, _, Invoke, _, _)));
        if !has_invoke {
            return;
        }
        if has_unbounded_invoke(target) {
            state.unbounded_invoke = true;
        }
        let mem_is_closed =
            |mem: &QualifiedInstId<StructId>| mem.inst.iter().all(|ty| !ty.is_open());
        let mut wildcard = false;
        // Parameter-declared frames of the enclosing function.
        for access in target.func_env.get_fun_param_access_of() {
            if access.frame_spec.modifies_all {
                wildcard = true;
            }
            for mem in &access.old_memory {
                if mem_is_closed(mem) {
                    state.add_direct_invoke_frame(mem.clone());
                } else {
                    wildcard = true;
                }
            }
        }
        // Struct-field-declared frames from anywhere in the program.
        //
        // The type-match gate would need per-invocation-operand-type
        // filtering (fat_loop.rs applies that at loop granularity); at the
        // function-summary layer we don't know which Invoke reaches which
        // caller, so we union the frames of every field whose type is a
        // function type at all. Safe over-approximation for `invoke_frame`.
        for module_env in env.get_modules() {
            for struct_env in module_env.get_structs() {
                for access in struct_env.get_field_access_of() {
                    if !struct_env
                        .find_field(access.fun_param)
                        .is_some_and(|f| matches!(f.get_type().skip_reference(), Type::Fun(..)))
                    {
                        continue;
                    }
                    if access.frame_spec.modifies_all {
                        wildcard = true;
                    }
                    for mem in &access.old_memory {
                        if mem_is_closed(mem) {
                            state.add_direct_invoke_frame(mem.clone());
                            state.invoke_frame_other.insert(mem.clone());
                        } else {
                            wildcard = true;
                        }
                    }
                }
            }
        }
        // The wildcard is intentionally NOT resolved here against the
        // summarized function's accessed footprint: for a pure forwarder
        // (`fun apply(f, s) { f(s) }` with `modifies_of<f> *`) that
        // footprint is empty, and a caller inheriting the summary via
        // `subsume_callee` would then havoc nothing. Propagate the flag
        // and let each use site resolve it against its own footprint.
        if wildcard {
            state.invoke_frame_wildcard = true;
        }
    }

    /// Accounts for closures handed to a callee that leaves their frame to the call site.
    ///
    /// See [`deferred_closure_targets`]: the callee's own summary records none of what such
    /// a closure does, so the function making the call carries it, exactly as if it had
    /// invoked the closure itself.
    fn compute_deferred_closure_usage(&self, target: &FunctionTarget, state: &mut UsageState) {
        let env = self.cache.global_env();
        let code = target.get_bytecode();
        let summaries = |id: QualifiedId<FunId>| {
            self.cache
                .get::<UsageState>(id, &FunctionVariant::Baseline)
                .cloned()
        };
        let closures = closures_with_known_footprint(
            env,
            target.func_env.get_qualified_id(),
            code,
            &summaries,
        );
        for bc in code {
            let Bytecode::Call(_, _, Operation::Function(mid, fid, _), srcs, _) = bc else {
                continue;
            };
            let callee = env.get_function(mid.qualified(*fid));
            for closure in deferred_closure_targets(&callee, srcs, &closures) {
                state.add_transitive_modified_iter(closure.writes.iter().cloned());
                state.add_transitive_accessed_iter(closure.accessed.iter().cloned());
            }
        }
    }
}

/// What a closure built in some function writes and accesses, as the prover sees a call to
/// its target.
#[derive(Clone, Debug)]
pub struct ClosureEffects {
    /// Every resource a call to the target can write, instantiated.
    pub writes: BTreeSet<QualifiedInstId<StructId>>,
    /// Every resource a call to the target can read or write, instantiated.
    pub accessed: BTreeSet<QualifiedInstId<StructId>>,
}

/// What a call to `fun_id` at `inst` writes and accesses, as the prover encodes the call:
/// `None` when that is not known.
///
/// - A native, or an opaque function under `pragma verify = false`, is encoded from its
///   contract, so its declared `modifies` frame is what it writes; for the opaque function,
///   a call also havocs what its body writes outside that frame, which is added from its
///   usage summary. A frame that is a wildcard, or that names memory through a bare type
///   parameter, is not a known set.
/// - Any other function is described by its usage summary, which `summary_of` supplies. It
///   is not known when there is no summary (a member of a recursive cycle whose summary is
///   still being computed), or when the summary has `unbounded_invoke` or a wildcard invoke
///   frame: some function value it runs has effects nothing records.
pub fn call_effects(
    env: &GlobalEnv,
    fun_id: QualifiedId<FunId>,
    inst: &[Type],
    summary_of: &dyn Fn(QualifiedId<FunId>) -> Option<UsageState>,
) -> Option<ClosureEffects> {
    let fun = env.get_function(fun_id);
    if fun.is_native() || (fun.is_opaque() && fun.is_pragma_false(VERIFY_PRAGMA)) {
        let mut writes = BTreeSet::new();
        if let Some(frame) = fun.get_frame_spec() {
            if frame.modifies_all {
                return None;
            }
            for target in &frame.modifies_targets {
                let Type::Struct(mid, sid, targs) =
                    env.get_node_type(target.node_id()).skip_reference().clone()
                else {
                    return None;
                };
                let mem = mid.qualified_inst(sid, targs).instantiate(inst);
                if mem.inst.iter().any(|ty| ty.is_open()) {
                    return None;
                }
                writes.insert(mem);
            }
        }
        let mut accessed = writes.clone();
        accessed.extend(fun.get_spec_used_memory_instantiated(inst));
        // A direct call also havocs what the body writes outside the frame, so a closure
        // over this target must be charged with the same.
        if !fun.is_native() {
            let summary = summary_of(fun_id)?;
            if summary.unbounded_invoke || summary.invoke_frame_wildcard {
                return None;
            }
            writes.extend(summary.modified.get_all_inst(inst));
            writes.extend(summary.invoke_frame.get_all_inst(inst));
            accessed.extend(summary.accessed.get_all_inst(inst));
            accessed.extend(writes.iter().cloned());
        }
        return Some(ClosureEffects { writes, accessed });
    }
    let summary = summary_of(fun_id)?;
    if summary.unbounded_invoke || summary.invoke_frame_wildcard {
        return None;
    }
    let mut writes = summary.modified.get_all_inst(inst);
    writes.extend(summary.invoke_frame.get_all_inst(inst));
    let mut accessed = summary.accessed.get_all_inst(inst);
    accessed.extend(writes.iter().cloned());
    Some(ClosureEffects { writes, accessed })
}

/// Whether `target` invokes a function value whose effects nothing bounds (see
/// `UsageState::unbounded_invoke`).
///
/// An invocation is bounded only when its operand is one of the function's own parameters,
/// never reassigned in the body, whose argument is checked against a bound at every call
/// site. The frame checks enforce that exactly when the function declares
/// `modifies_of`/`reads_of` for the parameter (the argument must stay within it; the writes
/// are in `invoke_frame`), or when the function is opaque: an undeclared frame is then
/// empty and enforced, or left to the call site, which must supply a closure with known
/// effects. Any other operand -- a parameter of a transparent function without a
/// declaration, a value read from storage or a struct field, a local of any other origin --
/// is unbounded.
fn has_unbounded_invoke(target: &FunctionTarget) -> bool {
    let fun = target.func_env;
    let params = fun.get_parameters();
    let code = target.get_bytecode();
    let reassigned: BTreeSet<TempIndex> = code.iter().flat_map(|bc| bc.dests()).collect();
    let bounded_param = |idx: TempIndex| {
        idx < params.len()
            && !reassigned.contains(&idx)
            && params[idx].1.skip_reference().is_function()
            && (fun.is_opaque()
                || fun
                    .get_fun_param_access_of()
                    .iter()
                    .any(|access| access.fun_param == params[idx].0))
    };
    code.iter().any(|bc| match bc {
        Bytecode::Call(_, _, Operation::Invoke, srcs, _) => {
            srcs.last().is_none_or(|operand| !bounded_param(*operand))
        },
        _ => false,
    })
}

/// The closures built in `code` whose effects the prover can state, by the temporary each is
/// bound to.
///
/// A closure qualifies when:
///
/// - its temporary is defined exactly once, by the `Closure` operation (a closure rebuilt in
///   a loop, or overwritten, has no single target);
/// - its target is not `enclosing`, the function being analysed, whose summary is still
///   being computed;
/// - its target leaves none of its own function-typed parameters' frames to its call sites:
///   such a target's effects omit the closure it runs, so a closure that partially applies it
///   (`|y| apply(h, y)`, lowered to `Closure(apply, [h])`) would hide the captured `h`;
/// - its target has no function-typed parameter: the target's summary bounds what invoking
///   such a parameter writes by the check a direct call makes on its argument (against the
///   declared frame, or the empty one), and a closure's target is not called directly -- the
///   value comes from the closure's captures or from whoever invokes the closure, and neither
///   is checked;
/// - a call to its target has known effects ([`call_effects`]): for a native or an
///   unverified opaque function, its declared frame; otherwise its usage summary;
/// - those effects write no spec variable (ghost memory): a call-site havoc never covers
///   one, so deferring would drop the write.
///
/// Every analysis that needs a closure's effects, and every check that decides whether a
/// callee may leave a closure's frame to its call site, must use this one set, so that a
/// closure is deferred exactly when its effects are accounted for. Any other closure has no
/// footprint, and an undeclared frame stays empty for it.
pub fn closures_with_known_footprint(
    env: &GlobalEnv,
    enclosing: QualifiedId<FunId>,
    code: &[Bytecode],
    summary_of: &dyn Fn(QualifiedId<FunId>) -> Option<UsageState>,
) -> BTreeMap<TempIndex, ClosureEffects> {
    let mut def_counts: BTreeMap<TempIndex, usize> = BTreeMap::new();
    for bc in code {
        for dest in bc.dests() {
            *def_counts.entry(dest).or_default() += 1;
        }
    }
    let effects_of = |target_id: QualifiedId<FunId>, targs: &[Type]| {
        let target = env.get_function(target_id);
        let defers_itself = target
            .get_parameters()
            .iter()
            .any(|param| target.leaves_fun_param_frame_to_call_site(param.0));
        if target_id == enclosing || defers_itself {
            return None;
        }
        call_effects(env, target_id, targs, summary_of).filter(|effects| {
            !effects
                .writes
                .iter()
                .any(|mem| env.get_struct_qid(mem.to_qualified_id()).is_ghost_memory())
        })
    };
    let takes_fun_value = |target_id: QualifiedId<FunId>| {
        env.get_function(target_id)
            .get_parameters()
            .iter()
            .any(|param| param.1.skip_reference().is_function())
    };
    code.iter()
        .filter_map(|bc| match bc {
            Bytecode::Call(_, dests, Operation::Closure(mid, fid, targs, _), _, _)
                if dests.len() == 1
                    && def_counts[&dests[0]] == 1
                    && !takes_fun_value(mid.qualified(*fid)) =>
            {
                effects_of(mid.qualified(*fid), targs).map(|effects| (dests[0], effects))
            },
            _ => None,
        })
        .collect()
}

/// The effects of the closures a call to `callee` with arguments `srcs` runs on the callee's
/// behalf without the callee's own summary accounting for them.
///
/// A callee that leaves a function-typed parameter's frame to its call sites
/// (`FunctionEnv::leaves_fun_param_frame_to_call_site`) records nothing for that parameter.
/// Whatever the closure passed for it does is therefore part of what *this call* does, and
/// every analysis that asks what a call modifies -- the summary of the function making it,
/// the havoc at a loop head that contains it -- must add the effects returned here.
/// `closures` is [`closures_with_known_footprint`] of the calling function; an argument of
/// any other origin has no footprint and is rejected by the frame check, so it never reaches
/// the prover.
pub fn deferred_closure_targets<'a>(
    callee: &FunctionEnv,
    srcs: &[TempIndex],
    closures: &'a BTreeMap<TempIndex, ClosureEffects>,
) -> Vec<&'a ClosureEffects> {
    if !callee.is_opaque() {
        return vec![];
    }
    callee
        .get_parameters()
        .iter()
        .zip(srcs)
        .filter(|(param, _)| callee.leaves_fun_param_frame_to_call_site(param.0))
        .filter_map(|(_, src)| closures.get(src))
        .collect()
}

pub struct UsageProcessor();

impl UsageProcessor {
    pub fn new() -> Box<Self> {
        Box::new(UsageProcessor())
    }

    pub fn analyze(
        targets: &FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: &FunctionData,
    ) -> UsageState {
        let func_target = FunctionTarget::new(func_env, data);
        let cache = SummaryCache::new(targets, func_env.module_env.env);
        let analysis = MemoryUsageAnalysis { cache };
        let mut summary = analysis.summarize(&func_target, UsageState::default());
        analysis.compute_spec_usage(&func_env.get_spec(), &mut summary);
        analysis.compute_invoke_usage(&func_target, &mut summary);
        analysis.compute_deferred_closure_usage(&func_target, &mut summary);
        summary
    }
}

impl FunctionTargetProcessor for UsageProcessor {
    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        let summary = Self::analyze(targets, func_env, &data);
        // TODO(mengxu): re-check the code on whether recursion have an impact on the analysis here.
        data.annotations.set(summary, true);
        data
    }

    fn name(&self) -> String {
        "usage_analysis".to_string()
    }

    fn dump_result(
        &self,
        f: &mut Formatter<'_>,
        env: &GlobalEnv,
        targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        writeln!(f, "\n\n********* Result of usage analysis *********\n\n")?;
        for module in env.get_modules() {
            if !module.is_target() {
                continue;
            }
            for fun in module.get_functions() {
                if fun.is_not_prover_target() {
                    continue;
                }
                for (_, ref target) in targets.get_targets(&fun) {
                    let usage = get_memory_usage(target);
                    writeln!(
                        f,
                        "function {} [{}] {{",
                        target.func_env.get_full_name_str(),
                        target.data.variant
                    )?;

                    let mut print_usage = |set: &MemoryUsage, name: &str| -> fmt::Result {
                        writeln!(
                            f,
                            "  {} = {{{}}}",
                            name,
                            set.all
                                .iter()
                                .map(|qid| env.display(qid).to_string())
                                .join(", ")
                        )?;
                        writeln!(
                            f,
                            "  directly {} = {{{}}}",
                            name,
                            set.direct
                                .iter()
                                .map(|qid| env.display(qid).to_string())
                                .join(", ")
                        )
                    };

                    print_usage(&usage.accessed, "accessed")?;
                    print_usage(&usage.modified, "modified")?;
                    print_usage(&usage.assumed, "assumed")?;
                    print_usage(&usage.asserted, "asserted")?;

                    writeln!(f, "}}")?;
                }
            }
        }
        writeln!(f)?;
        Ok(())
    }
}
