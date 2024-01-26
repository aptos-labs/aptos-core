// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implements a live-variable analysis processor, annotating lifetime information about locals.
//! See also https://en.wikipedia.org/wiki/Live-variable_analysis
//!
//! After transformation, this also runs copy inference transformation, which inserts
//! copies as needed, and reports errors for invalid copies.
//!
//! This processor assumes that the CFG of the code has no critical edges.

use super::ability_checker::check_copy;
use crate::pipeline::ability_checker::has_ability;
use abstract_domain_derive::AbstractDomain;
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use move_binary_format::file_format::{Ability, CodeOffset};
use move_model::{
    ast::TempIndex,
    model::{FunctionEnv, Loc},
    ty::Type,
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult, MapDomain},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AssignKind, AttrId, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    iter::{empty, once},
};

/// Annotation which is attached to function data.
#[derive(Default, Clone)]
pub struct LiveVarAnnotation(BTreeMap<CodeOffset, LiveVarInfoAtCodeOffset>);

impl LiveVarAnnotation {
    /// Get the live var info at the given code offset
    pub fn get_live_var_info_at(
        &self,
        code_offset: CodeOffset,
    ) -> Option<&LiveVarInfoAtCodeOffset> {
        self.0.get(&code_offset)
    }
}

/// The annotation for live variable analysis per code offset.
#[derive(Debug, Default, Clone)]
pub struct LiveVarInfoAtCodeOffset {
    /// Usage before this program point.
    pub before: BTreeMap<TempIndex, LiveVarInfo>,
    /// Usage after this program point.
    pub after: BTreeMap<TempIndex, LiveVarInfo>,
}

impl LiveVarInfoAtCodeOffset {
    /// Returns the temporaries that are alive before the program point and dead after.
    pub fn released_temps(&self) -> impl Iterator<Item = TempIndex> + '_ {
        // TODO: make this linear
        self.before
            .keys()
            .filter(|t| !self.after.contains_key(t))
            .cloned()
    }

    /// Returns the temporaries that are alive before the program point and dead after, or introduced
    /// by the given bytecode and dead after.
    pub fn released_and_unused_temps(&self, bc: &Bytecode) -> BTreeSet<TempIndex> {
        let mut result: BTreeSet<_> = self.released_temps().collect();
        for dest in bc.dests() {
            if !self.after.contains_key(&dest) {
                result.insert(dest);
            }
        }
        result
    }

    /// Creates a set of the temporaries alive before this program point.
    pub fn before_set(&self) -> BTreeSet<TempIndex> {
        self.before.keys().cloned().collect()
    }

    /// Creates a set of the temporaries alive after this program point.
    pub fn after_set(&self) -> BTreeSet<TempIndex> {
        self.after.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct LiveVarInfo {
    /// The usage of a given temporary after this program point, inclusive of locations where
    /// the usage happens. This set contains at least one element.
    pub usages: BTreeSet<Loc>,
}

// =================================================================================================
// Processor

pub struct LiveVarAnalysisProcessor {
    // If set, run copy and move inference. Otherwise only compute livevar annotation.
    pub with_copy_inference: bool,
}

impl FunctionTargetProcessor for LiveVarAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        let offset_to_live_refs = LiveVarAnnotation(self.analyze(&target));
        if self.with_copy_inference {
            let mut transformer = CopyTransformation { fun_env, data };
            transformer.transform(&offset_to_live_refs);
            // Now run the analyze a 2nd time, as we modified the code
            let target = FunctionTarget::new(fun_env, &transformer.data);
            let offset_to_live_refs = LiveVarAnnotation(self.analyze(&target));
            // Annotate the result on the function data.
            transformer.data.annotations.set(offset_to_live_refs, true);
            transformer.data
        } else {
            data.annotations.set(offset_to_live_refs, true);
            data
        }
    }

    fn name(&self) -> String {
        "LiveVarAnalysisProcessor".to_owned()
    }
}

impl LiveVarAnalysisProcessor {
    /// Run the live var analysis.
    fn analyze(
        &self,
        func_target: &FunctionTarget,
    ) -> BTreeMap<CodeOffset, LiveVarInfoAtCodeOffset> {
        let code = func_target.get_bytecode();
        // Perform backward analysis from all blocks just in case some block
        // cannot reach an exit block
        let cfg = StacklessControlFlowGraph::new_backward(code, /*from_all_blocks*/ true);
        let analyzer = LiveVarAnalysis { func_target };
        let state_map = analyzer.analyze_function(
            LiveVarState {
                livevars: MapDomain::default(),
            },
            code,
            &cfg,
        );
        // Prepare the result as a map from CodeOffset to LiveVarInfo
        let mut code_map =
            analyzer.state_per_instruction(state_map, code, &cfg, |before, after| {
                LiveVarInfoAtCodeOffset {
                    before: before.livevars.clone().into_iter().collect(),
                    after: after.livevars.clone().into_iter().collect(),
                }
            });

        // Now propagate to all branches in the code the `after` set of the branch instruction. Consider code as follows:
        // ```
        // L0: if c goto L1 else L2
        // <x alive>
        // L1: ..
        //     goto L0
        // L2: ..
        // ```
        // The backwards analysis will not populate the before state of `L1` and `L2` with `x` being alive unless it
        // is used in the branch. However, from the forward program flow it follows that `x` is alive before
        // `L1` and `L2` regardless of its usage. More specifically, it may have to be _dropped_ if it goes out
        // of scope after the branch.
        //
        // This problem of values which "are lost on the edge" of the control graph can be dealt with by
        // introducing extra edges. However, assuming that there are no critical edges, a simpler
        // solution is the join `pre(L1) := pre(L1) join after(L0)`, and similar for `L2`.
        let label_to_offset = Bytecode::label_offsets(code);
        for (offs, bc) in code.iter().enumerate() {
            let offs = offs as CodeOffset;
            if let Bytecode::Branch(_, then_label, else_label, _) = bc {
                let this = code_map[&offs].clone();
                let then = code_map.get_mut(&label_to_offset[then_label]).unwrap();
                Self::join_maps(&mut then.before, &this.after);
                let else_ = code_map.get_mut(&label_to_offset[else_label]).unwrap();
                Self::join_maps(&mut else_.before, &this.after);
            }
        }
        code_map
    }

    fn join_maps(m1: &mut BTreeMap<TempIndex, LiveVarInfo>, m2: &BTreeMap<TempIndex, LiveVarInfo>) {
        for (k, v) in m2 {
            match m1.entry(*k) {
                Entry::Vacant(e) => {
                    e.insert(v.clone());
                },
                Entry::Occupied(mut e) => {
                    e.get_mut().join(v);
                },
            }
        }
    }

    /// Registers annotation formatter at the given function target. This is for debugging and
    /// testing.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_livevar_annotation))
    }
}

// =================================================================================================
// Dataflow Analysis

/// State of the livevar analysis,
#[derive(AbstractDomain, Debug, Clone, Eq, PartialEq, PartialOrd)]
struct LiveVarState {
    livevars: MapDomain<TempIndex, LiveVarInfo>,
}

impl AbstractDomain for LiveVarInfo {
    fn join(&mut self, other: &Self) -> JoinResult {
        let count = self.usages.len();
        self.usages.extend(other.usages.iter().cloned());
        if self.usages.len() != count {
            JoinResult::Changed
        } else {
            JoinResult::Unchanged
        }
    }
}

struct LiveVarAnalysis<'a> {
    func_target: &'a FunctionTarget<'a>,
}

/// Implements the necessary transfer function to instantiate the data flow framework
impl<'a> TransferFunctions for LiveVarAnalysis<'a> {
    type State = LiveVarState;

    const BACKWARD: bool = true;

    fn execute(&self, state: &mut LiveVarState, instr: &Bytecode, _idx: CodeOffset) {
        use Bytecode::*;
        match instr {
            Assign(id, dst, src, _) => {
                state.livevars.remove(dst);
                state.livevars.insert(*src, self.livevar_info(id));
            },
            Load(_, dst, _) => {
                state.livevars.remove(dst);
            },
            Call(id, dsts, _, srcs, _) => {
                for dst in dsts {
                    state.livevars.remove(dst);
                }
                for src in srcs {
                    state.livevars.insert(*src, self.livevar_info(id));
                }
            },
            Ret(id, srcs) => {
                for src in srcs {
                    state.livevars.insert(*src, self.livevar_info(id));
                }
            },
            Abort(id, src) | Branch(id, _, _, src) => {
                state.livevars.insert(*src, self.livevar_info(id));
            },
            Prop(id, _, exp) => {
                for (idx, _) in exp.used_temporaries(self.func_target.global_env()) {
                    state.livevars.insert(idx, self.livevar_info(id));
                }
            },
            _ => {},
        }
    }
}

/// Implements various entry points to the framework based on the transfer function.
impl<'a> DataflowAnalysis for LiveVarAnalysis<'a> {}

impl<'a> LiveVarAnalysis<'a> {
    fn livevar_info(&self, id: &AttrId) -> LiveVarInfo {
        LiveVarInfo {
            usages: once(self.func_target.get_bytecode_loc(*id)).collect(),
        }
    }
}

// =================================================================================================
// Bytecode Transformation

/// State for copy inference transformation.
struct CopyTransformation<'a> {
    fun_env: &'a FunctionEnv<'a>,
    data: FunctionData,
}

impl<'a> CopyTransformation<'a> {
    /// Runs copy inference transformation. This transformation inserts implicit copies. It also
    /// checks correctness of copies, whether explicit or implicit.
    fn transform(&mut self, alive: &LiveVarAnnotation) {
        let code = std::mem::take(&mut self.data.code);
        for (i, bc) in code.into_iter().enumerate() {
            self.transform_bytecode(
                alive
                    .get_live_var_info_at(i as CodeOffset)
                    .expect("live var info"),
                bc,
            )
        }
    }

    /// Transforms a bytecode. This handles `Assign` and `Call` instructions.
    /// For the former, it infers the `AssignKind` (copy or move) and for the later,
    /// it implicitly copies arguments if needed. Implicit copy is needed
    /// if a non-primitive value is used after the given program point.
    fn transform_bytecode(&mut self, alive: &LiveVarInfoAtCodeOffset, bc: Bytecode) {
        use Bytecode::*;
        match bc {
            Assign(id, dst, src, kind) => match kind {
                AssignKind::Inferred => {
                    if self.check_implicit_copy(alive, id, false, src) {
                        self.data.code.push(Assign(id, dst, src, AssignKind::Copy))
                    } else {
                        self.data.code.push(Assign(id, dst, src, AssignKind::Move))
                    }
                },
                AssignKind::Copy | AssignKind::Store => {
                    self.check_explicit_copy(id, src);
                    self.data.code.push(Assign(id, dst, src, AssignKind::Copy))
                },
                AssignKind::Move => {
                    self.check_explicit_move(alive, id, src);
                    self.data.code.push(Assign(id, dst, src, AssignKind::Move))
                },
            },
            Call(_, _, Operation::BorrowLoc, _, _)
            | Call(_, _, Operation::BorrowField(..), _, _)
            | Call(_, _, Operation::ReadRef, _, _) => {
                // Borrow and ReadRef does not consume its operand and need no copy
                self.data.code.push(bc)
            },
            Call(id, dsts, Operation::WriteRef, srcs, ai) => {
                // The reference parameter is not consumed and does not need copy
                let mut new_srcs = self.copy_arg_if_needed(alive, id, vec![srcs[1]]);
                new_srcs.insert(0, srcs[0]);
                self.data
                    .code
                    .push(Call(id, dsts, Operation::WriteRef, new_srcs, ai))
            },
            Call(id, dsts, oper, srcs, ai) => {
                let srcs = self.copy_arg_if_needed(alive, id, srcs);
                self.data.code.push(Call(id, dsts, oper, srcs, ai))
            },
            _ => self.data.code.push(bc),
        }
    }

    /// Walks over the argument list and inserts copies if needed.
    fn copy_arg_if_needed(
        &mut self,
        alive: &LiveVarInfoAtCodeOffset,
        id: AttrId,
        srcs: Vec<TempIndex>,
    ) -> Vec<TempIndex> {
        use Bytecode::*;
        let mut new_srcs = vec![];
        for (i, src) in srcs.iter().enumerate() {
            let is_prim = matches!(self.target().get_local_type(*src), Type::Primitive(_));
            if !is_prim
                && (self.check_implicit_copy(alive, id, true, *src)
                    || self.check_implicit_copy_in_arglist(id, *src, &srcs[i + 1..srcs.len()]))
            {
                let temp = self.clone_local(*src);
                self.data
                    .code
                    .push(Assign(id, temp, *src, AssignKind::Copy));
                new_srcs.push(temp)
            } else {
                new_srcs.push(*src)
            }
        }
        new_srcs
    }

    /// Checks whether an implicit copy is needed because the value is used afterwards.
    /// This produces an error if copy is not allowed.
    fn check_implicit_copy(
        &self,
        alive: &LiveVarInfoAtCodeOffset,
        id: AttrId,
        _is_updated: bool,
        temp: TempIndex,
    ) -> bool {
        let target = self.target();
        let ty = target.get_local_type(temp);
        if !ty.is_reference()
            && has_ability(&target, ty, Ability::Copy)
            && has_ability(&target, ty, Ability::Drop)
        {
            // TODO(#11223): Until we have info about whether a var may have had a reference
            // taken, be very conservative here and always copy if the type has both drop
            // and copy ability. Notice we also need drop ability as with too many copies we
            // may end up with the need to destroy a value, which requires drop.
            // If conditions don't hold, reference analysis should give us an error if we move
            // the value and references still exist.
            true
        } else {
            let needed = alive.after.contains_key(&temp);
            if needed {
                check_copy(
                    &target,
                    ty,
                    &target.get_bytecode_loc(id),
                    &format!(
                        "cannot copy {} implicitly",
                        target.get_local_name_for_error_message(temp)
                    ),
                );
            }
            needed
        }
    }

    fn make_hints_from_usage(
        &self,
        info: &'a LiveVarInfo,
    ) -> impl Iterator<Item = (Loc, String)> + 'a {
        info.usages
            .iter()
            .map(|loc| (loc.clone(), "used here".to_owned()))
    }

    /// Checks whether the given temp has copy ability
    /// add diagnostics if not
    fn check_copy_for_temp(&self, target: &FunctionTarget, temp: TempIndex, id: AttrId) {
        check_copy(
            target,
            target.get_local_type(temp),
            &target.get_bytecode_loc(id),
            &format!(
                "cannot copy {} implicitly",
                target.get_local_name_for_error_message(temp)
            ),
        );
    }

    /// Checks whether an implicit copy is needed because the value is used again in
    /// an argument list. This cannot be determined from the livevar analysis result
    /// because the 2nd usage appears at the same program point.
    fn check_implicit_copy_in_arglist(
        &self,
        id: AttrId,
        temp: TempIndex,
        args: &[TempIndex],
    ) -> bool {
        if args.contains(&temp) {
            // If this is a &mut, produce an error
            let target = self.target();
            if target.get_local_type(temp).is_mutable_reference() {
                self.error_with_hints(
                    &target.get_bytecode_loc(id),
                    format!(
                        "implicit copy of mutable reference in {} which \
                    is used later in argument list",
                        target.get_local_name_for_error_message(temp)
                    ),
                    "implicitly copied here",
                    empty(),
                );
                false
            } else {
                self.check_copy_for_temp(&target, temp, id);
                true
            }
        } else {
            false
        }
    }

    /// Checks whether an explicit copy is allowed.
    fn check_explicit_copy(&self, id: AttrId, temp: TempIndex) {
        let target = self.target();
        if !target.get_local_type(temp).is_mutable_reference() {
            // Copy of mutable refs is checked in reference analysis
            self.check_copy_for_temp(&target, temp, id)
        }
    }

    /// Checks whether an explicit move is allowed.
    fn check_explicit_move(&self, alive: &LiveVarInfoAtCodeOffset, id: AttrId, temp: TempIndex) {
        if let Some(info) = alive.after.get(&temp) {
            let target = self.target();
            self.error_with_hints(
                &target.get_bytecode_loc(id),
                format!(
                    "cannot move {} since it is used later",
                    target.get_local_name_for_error_message(temp)
                ),
                "attempted to move here",
                self.make_hints_from_usage(info),
            );
        }
    }

    /// Makes a new temporary with the same type as the given one.
    fn clone_local(&mut self, temp: TempIndex) -> TempIndex {
        let ty = self.target().get_local_type(temp).clone();
        self.data.local_types.push(ty);
        self.data.local_types.len() - 1
    }

    /// Produces an error with primary message and secondary hints.
    fn error_with_hints(
        &self,
        loc: &Loc,
        msg: impl AsRef<str>,
        primary: impl AsRef<str>,
        hints: impl Iterator<Item = (Loc, String)>,
    ) {
        self.fun_env.module_env.env.diag_with_primary_and_labels(
            Severity::Error,
            loc,
            msg.as_ref(),
            primary.as_ref(),
            hints.collect(),
        )
    }

    /// Constructs a function target for temporary use. Since we need to mutate `self.data`
    /// we cannot store the target in `self`, so construct it as needed.
    fn target(&self) -> FunctionTarget<'_> {
        FunctionTarget::new(self.fun_env, &self.data)
    }
}

// =================================================================================================
// Formatting

/// Format a live variable annotation.
pub fn format_livevar_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(LiveVarAnnotation(map)) = target.get_annotations().get::<LiveVarAnnotation>() {
        if let Some(map_at) = map.get(&code_offset) {
            let mut res = map_at
                .before
                .keys()
                .map(|idx| {
                    let name = target.get_local_raw_name(*idx);
                    format!("{}", name.display(target.symbol_pool()))
                })
                .join(", ");
            res.insert_str(0, "live vars: ");
            return Some(res);
        }
    }
    None
}
