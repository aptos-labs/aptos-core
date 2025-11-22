// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Reaching definition analysis

use crate::pipeline::{
    livevar_analysis_processor::LiveVarAnnotation, reference_safety::LifetimeAnnotation,
};
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::TempIndex,
    model::FunctionEnv,
    ty::{ReferenceKind, Type},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AbortAction, AssignKind, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

/// The reaching definitions we are capturing.
///  As stackless bytecode is not SSA, we identify a definition by its code offset.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Def {
    Loc(CodeOffset),
}

type DefMap = BTreeMap<TempIndex, BTreeSet<Def>>;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Default)]
pub struct ReachingDefState {
    pub map: DefMap,
}

/// The annotation for reaching definitions. For each code position, we have a map of local
/// indices to the set of definitions reaching the code position.
#[derive(Clone, Default)]
pub struct ReachingDefAnnotation(BTreeMap<CodeOffset, ReachingDefState>);

impl ReachingDefAnnotation {
    /// Returns information for code offset.
    pub fn get_info_at(&self, code_offset: CodeOffset) -> &ReachingDefState {
        self.0.get(&code_offset).expect("reaching def info")
    }
}

pub struct ReachingDefProcessor {}

impl FunctionTargetProcessor for ReachingDefProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }

        let target = FunctionTarget::new(func_env, &data);
        // reaching definition depends on reference analysis!!!
        let Some(ref_annotation) = target.get_annotations().get::<LifetimeAnnotation>() else {
            return data;
        };
        let Some(livevar_annotation) = target.get_annotations().get::<LiveVarAnnotation>() else {
            return data;
        };
        // init and run the analyzer
        let analyzer = ReachingDefAnalysis {
            _target: target,
            livevar_annotation,
            ref_annotation,
        };
        let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        let block_state_map = analyzer.analyze_function(
            ReachingDefState {
                map: BTreeMap::new(),
            },
            &data.code,
            &cfg,
        );
        let per_bytecode_state =
            analyzer.state_per_instruction(block_state_map, &data.code, &cfg, |before, _| {
                before.clone()
            });

        let annotations = ReachingDefAnnotation(per_bytecode_state);
        data.annotations.set(annotations, true);

        data
    }

    fn name(&self) -> String {
        "reaching_def_analysis".to_string()
    }
}

impl ReachingDefProcessor {
    /// Registers annotation formatter at the given function target. This is for debugging and
    /// testing.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_reaching_def_annotation))
    }
}

struct ReachingDefAnalysis<'env> {
    _target: FunctionTarget<'env>,
    livevar_annotation: &'env LiveVarAnnotation,
    ref_annotation: &'env LifetimeAnnotation,
}

impl TransferFunctions for ReachingDefAnalysis<'_> {
    type State = ReachingDefState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut ReachingDefState, instr: &Bytecode, offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;
        match instr {
            Assign(_, dest, src, kind) => {
                state.kill(*dest);
                state.def(*dest, offset);
                // Move semantics invalidate the source, essentially killing it
                match kind {
                    AssignKind::Move => {
                        state.kill(*src);
                    },
                    AssignKind::Inferred => {
                        let lifetime = self.ref_annotation.get_info_at(offset);
                        let live_info = self.livevar_annotation.get_info_at(offset);
                        // If the source temp is not used after this instruction and is not borrowed,
                        // it will be moved, and we need to consider it killed.
                        if !live_info.is_temp_used_after(src, instr) && !lifetime.is_borrowed(*src)
                        {
                            state.kill(*src);
                        }
                    },
                    _ => {},
                }
            },
            Load(_, dest, ..) => {
                state.kill(*dest);
                state.def(*dest, offset);
            },
            Call(_, dests, oper, srcs, on_abort) => {
                // generic kills
                for dest in dests {
                    state.kill(*dest);
                    state.def(*dest, offset);
                }
                if let Some(AbortAction(_, dest)) = on_abort {
                    state.kill(*dest);
                }
                // op-specific actions
                match oper {
                    WriteRef => {
                        let ref_info = self.ref_annotation.get_info_at(offset);
                        for temp in ref_info.referenced_temps_before(srcs[0]) {
                            state.kill(temp);
                            state.def(temp, offset);
                        }
                    },
                    // If a mut ref is passed to a callee, we consider all the object it points to is redefined
                    Function(_, _, arg_tys) => {
                        for (src, ty) in srcs.iter().zip(arg_tys.iter()) {
                            if ty.is_mutable_reference() {
                                for temp in self
                                    .ref_annotation
                                    .get_info_at(offset)
                                    .referenced_temps_before(*src)
                                {
                                    state.kill(temp);
                                    state.def(temp, offset);
                                }
                            }
                        }
                    },
                    // If a mut ref is passed to an invoked closure, we consider all the object it points to is redefined
                    Invoke => {
                        let fun_type = self
                            ._target
                            .get_local_type(*srcs.last().expect("closure expected"));
                        if let Type::Fun(args_ty, _, _) = fun_type {
                            let ref_info = self.ref_annotation.get_info_at(offset);
                            match args_ty.as_ref() {
                                // A single arg closure
                                Type::Reference(ReferenceKind::Mutable, _) => {
                                    for temp in ref_info.referenced_temps_before(srcs[0]) {
                                        state.kill(temp);
                                        state.def(temp, offset);
                                    }
                                },
                                Type::Tuple(x) => {
                                    assert!(
                                        srcs.len() == x.len() + 1,
                                        "{} arguments expected for invoke",
                                        x.len()
                                    );
                                    for (i, ty) in x.iter().enumerate() {
                                        if ty.is_mutable_reference() {
                                            for temp in ref_info.referenced_temps_before(srcs[i]) {
                                                state.kill(temp);
                                                state.def(temp, offset);
                                            }
                                        }
                                    }
                                },
                                _ => {},
                            }
                        }
                    },
                    _ => (),
                }
            },
            _ => {},
        }
    }
}

impl DataflowAnalysis for ReachingDefAnalysis<'_> {}

impl AbstractDomain for ReachingDefState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;
        // why we simply merge them:
        // - we want to collect a sound set of may-reach definitions
        for (idx, other_defs) in &other.map {
            let defs = self.map.entry(*idx).or_default();
            for d in other_defs {
                if defs.insert(d.clone()) {
                    result = JoinResult::Changed;
                }
            }
        }
        result
    }
}

impl ReachingDefState {
    fn def(&mut self, dest: TempIndex, loc: CodeOffset) {
        // ensure that the previous def is killed
        assert!(!self.map.contains_key(&dest));

        // update the new defs
        self.map.entry(dest).or_default().insert(Def::Loc(loc));
    }

    fn kill(&mut self, dest: TempIndex) {
        self.map.remove(&dest);
    }
}

// =================================================================================================
// Formatting

/// Format a reaching definition annotation.
pub fn format_reaching_def_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(ReachingDefAnnotation(map)) =
        target.get_annotations().get::<ReachingDefAnnotation>()
    {
        let mut str = String::new();
        str.insert_str(0, &format!("reaching instruction #{}: ", code_offset));
        if let Some(map_at) = map.get(&code_offset) {
            let res = map_at
                .map
                .iter()
                .map(|(idx, defs)| {
                    format!(
                        "`t{}` @ {{{}}}",
                        *idx,
                        defs.iter()
                            .map(|def| {
                                match def {
                                    Def::Loc(loc) => loc,
                                }
                            })
                            .join(", ")
                    )
                })
                .join(", ");
            return Some(str + &res);
        }
    }
    None
}
