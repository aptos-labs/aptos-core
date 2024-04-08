// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implements a live-variable analysis processor, annotating lifetime information about locals.
//! See also https://en.wikipedia.org/wiki/Live-variable_analysis
//!
//! This processor assumes that the CFG of the code has no critical edges.

use abstract_domain_derive::AbstractDomain;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::TempIndex,
    model::{FunctionEnv, Loc},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult, MapDomain},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    iter::once,
};

/// Annotation which is attached to function data.
#[derive(Default, Clone)]
pub struct LiveVarAnnotation(pub BTreeMap<CodeOffset, LiveVarInfoAtCodeOffset>);

impl LiveVarAnnotation {
    /// Get the live var info at the given code offset
    pub fn get_live_var_info_at(
        &self,
        code_offset: CodeOffset,
    ) -> Option<&LiveVarInfoAtCodeOffset> {
        self.0.get(&code_offset)
    }

    /// Get the live var info at the given code offset, expecting it to be defined.
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &LiveVarInfoAtCodeOffset {
        self.get_live_var_info_at(code_offset).expect("live_var_at")
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

    /// Check whether temp is used after bc
    pub fn is_temp_used_after(&self, temp: &TempIndex, bc: &Bytecode) -> bool {
        self.after.contains_key(temp) && !bc.dests().contains(temp)
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

pub struct LiveVarAnalysisProcessor {}

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
        let mut live_info = self.analyze(&target);
        // Add parameters to the before set at offset 0
        if live_info.contains_key(&0) {
            let live_info_at_zero = live_info.get_mut(&0).unwrap();
            for (i, param) in fun_env.get_parameters().iter().enumerate() {
                if let Entry::Vacant(e) = live_info_at_zero.before.entry(i) {
                    let mut usages = BTreeSet::new();
                    usages.insert(param.clone().2);
                    let live_info = LiveVarInfo { usages };
                    e.insert(live_info);
                } else {
                    let live_info = live_info_at_zero.before.get_mut(&i).unwrap();
                    live_info.usages.insert(param.clone().2); // use the location info for the parameter
                }
            }
        }
        data.annotations.set(LiveVarAnnotation(live_info), true);
        data
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
            analyzer.state_per_instruction_with_default(state_map, code, &cfg, |before, after| {
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
