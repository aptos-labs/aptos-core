// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Reaching definition analysis with subsequent copy propagation.
//
// This analysis and transformation only propagates definitions, leaving dead assignments
// in the code. The subsequent livevar_analysis takes care of removing those.

use crate::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AbortAction, BorrowNode, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use std::collections::{BTreeMap, BTreeSet};

/// The reaching definitions we are capturing. Currently we only capture
/// aliases (assignment).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Def {
    Alias(TempIndex),
}

type DefMap = BTreeMap<TempIndex, BTreeSet<Def>>;
type HavocSet = BTreeSet<TempIndex>;

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Default)]
pub struct ReachingDefState {
    pub map: DefMap,
    pub havoced: HavocSet,
}

/// The annotation for reaching definitions. For each code position, we have a map of local
/// indices to the set of definitions reaching the code position.
#[derive(Default)]
pub struct ReachingDefAnnotation(BTreeMap<CodeOffset, ReachingDefState>);

pub struct ReachingDefProcessor {
    /// Whether copies of mutable references may be alias-propagated. The
    /// prophecy reference model turns every `&mut`-to-`&mut` assignment into a
    /// reborrow whose handles carry independent current values, so propagating
    /// a use of the copy to the source would read a diverged handle; the
    /// write-back (path) model keeps the propagation, which its copy coherence
    /// relies on.
    propagate_mut_refs: bool,
}

impl ReachingDefProcessor {
    pub fn new() -> Box<Self> {
        Box::new(ReachingDefProcessor {
            propagate_mut_refs: true,
        })
    }

    /// A variant for the prophecy reference model: `&mut` copies are reborrows
    /// with their own state and are never alias-propagated.
    pub fn new_no_mut_ref_propagation() -> Box<Self> {
        Box::new(ReachingDefProcessor {
            propagate_mut_refs: false,
        })
    }

    /// Returns Some(temp, def) if temp has a unique reaching definition and None otherwise.
    fn get_unique_def(
        temp: TempIndex,
        defs: &BTreeSet<Def>,
        havoc_vars: &HavocSet,
    ) -> Option<(TempIndex, TempIndex)> {
        if defs.len() != 1 {
            return None;
        }
        let Def::Alias(def) = defs.iter().next().unwrap();
        if havoc_vars.contains(def) {
            return None;
        }
        Some((temp, *def))
    }

    /// Gets the propagated local resolving aliases using the reaching definitions.
    fn get_propagated_local(temp: TempIndex, state: &ReachingDefState) -> TempIndex {
        // For being robust, we protect this function against cycles in alias definitions. If
        // a cycle is detected, alias resolution stops.
        fn get(
            temp: TempIndex,
            state: &ReachingDefState,
            visited: &mut BTreeSet<TempIndex>,
        ) -> TempIndex {
            if let Some(defs) = state.map.get(&temp) {
                if let Some((_, def_temp)) =
                    ReachingDefProcessor::get_unique_def(temp, defs, &state.havoced)
                {
                    if visited.insert(def_temp) {
                        return get(def_temp, state, visited);
                    }
                }
            }
            temp
        }
        let mut visited = BTreeSet::new();
        get(temp, state, &mut visited)
    }

    /// Perform copy propagation based on reaching definitions analysis results.
    pub fn copy_propagation(
        target: &FunctionTarget<'_>,
        code: Vec<Bytecode>,
        defs: &ReachingDefAnnotation,
    ) -> Vec<Bytecode> {
        let default_state = ReachingDefState::default();

        let mut res = vec![];
        for (pc, bytecode) in code.into_iter().enumerate() {
            let state = defs.0.get(&(pc as CodeOffset)).unwrap_or(&default_state);
            let mut propagate = |local| Self::get_propagated_local(local, state);
            res.push(bytecode.remap_src_vars(target, &mut propagate));
        }
        res
    }

    /// Compute the set of locals which are borrowed from or which are otherwise used to refer to.
    /// We can't alias such locals to other locals because of reference semantics.
    fn borrowed_locals(&self, code: &[Bytecode]) -> BTreeSet<TempIndex> {
        use Bytecode::*;
        code.iter()
            .filter_map(|bc| match bc {
                Call(_, _, Operation::BorrowLoc, srcs, _) => Some(srcs[0]),
                Call(_, _, Operation::WriteBack(BorrowNode::LocalRoot(src), ..), ..)
                | Call(_, _, Operation::IsParent(BorrowNode::LocalRoot(src), ..), ..)
                | Call(_, _, Operation::ProphecyBorrow(BorrowNode::LocalRoot(src), ..), ..)
                | Call(_, _, Operation::ProphecySyncCurrent(BorrowNode::LocalRoot(src), ..), ..)
                | Call(_, _, Operation::ProphecySyncFinal(BorrowNode::LocalRoot(src), ..), ..) => {
                    Some(*src)
                },
                Call(_, _, Operation::WriteBack(BorrowNode::Reference(src), ..), ..)
                | Call(_, _, Operation::IsParent(BorrowNode::Reference(src), ..), ..)
                | Call(_, _, Operation::ProphecyBorrow(BorrowNode::Reference(src), ..), ..)
                | Call(_, _, Operation::ProphecySyncCurrent(BorrowNode::Reference(src), ..), ..)
                | Call(_, _, Operation::ProphecySyncFinal(BorrowNode::Reference(src), ..), ..) => {
                    Some(*src)
                },
                _ => None,
            })
            .collect()
    }
}

impl FunctionTargetProcessor for ReachingDefProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if !func_env.is_native() {
            let cfg = StacklessControlFlowGraph::new_forward(&data.code);
            let analyzer = ReachingDefAnalysis {
                _target: FunctionTarget::new(func_env, &data),
                borrowed_locals: self.borrowed_locals(&data.code),
                propagate_mut_refs: self.propagate_mut_refs,
            };
            let block_state_map = analyzer.analyze_function(
                ReachingDefState {
                    map: BTreeMap::new(),
                    havoced: BTreeSet::new(),
                },
                &data.code,
                &cfg,
            );
            let per_bytecode_state =
                analyzer.state_per_instruction(block_state_map, &data.code, &cfg, |before, _| {
                    before.clone()
                });

            // Run copy propagation transformation.
            let annotations = ReachingDefAnnotation(per_bytecode_state);
            let code = std::mem::take(&mut data.code);
            let target = FunctionTarget::new(func_env, &data);
            let new_code = Self::copy_propagation(&target, code, &annotations);
            data.code = new_code;

            // Currently we do not need reaching defs after this phase. If so in the future, we
            // need to uncomment this statement.
            //data.annotations.set(annotations);
        }

        data
    }

    fn name(&self) -> String {
        "reaching_def_analysis".to_string()
    }
}

struct ReachingDefAnalysis<'a> {
    _target: FunctionTarget<'a>,
    borrowed_locals: BTreeSet<TempIndex>,
    propagate_mut_refs: bool,
}

impl ReachingDefAnalysis<'_> {}

impl TransferFunctions for ReachingDefAnalysis<'_> {
    type State = ReachingDefState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut ReachingDefState, instr: &Bytecode, _offset: CodeOffset) {
        use BorrowNode::*;
        use Bytecode::*;
        use Operation::*;
        match instr {
            Assign(_, dest, src, _) => {
                state.kill(*dest);
                // Under the prophecy model `&mut` assignments are reborrows (renames
                // arise only later, in `normalize_exits`' merged-exit moves), so
                // redirecting a later use from the copy to the source would address
                // a diverged handle.
                let excluded_mut_ref = !self.propagate_mut_refs
                    && self._target.get_local_type(*dest).is_mutable_reference();
                if !excluded_mut_ref
                    && !self.borrowed_locals.contains(dest)
                    && !self.borrowed_locals.contains(src)
                {
                    state.def_alias(*dest, *src);
                }
            },
            Load(_, dest, ..) => {
                state.kill(*dest);
            },
            Call(_, dests, oper, _, on_abort) => {
                // generic kills
                for dest in dests {
                    state.kill(*dest);
                }
                if let Some(AbortAction(_, dest)) = on_abort {
                    state.kill(*dest);
                }
                // op-specific actions
                match oper {
                    WriteBack(LocalRoot(local_root), ..) => {
                        state.kill(*local_root);
                    },
                    Havoc(_) => {
                        state.havoc(dests[0]);
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
        for idx in self.map.keys().cloned().collect_vec() {
            if let Some(other_defs) = other.map.get(&idx) {
                // Union of definitions
                let defs = self.map.get_mut(&idx).unwrap();
                for d in other_defs {
                    if defs.insert(d.clone()) {
                        result = JoinResult::Changed;
                    }
                }
            } else {
                // Kill this definition as it is not contained in both incoming states.
                self.map.remove(&idx);
                result = JoinResult::Changed;
            }
        }
        result
    }
}

impl ReachingDefState {
    fn def_alias(&mut self, dest: TempIndex, src: TempIndex) {
        // ensure that the previous def is killed
        assert!(!self.map.contains_key(&dest));

        // update the new alias
        self.map.entry(dest).or_default().insert(Def::Alias(src));
    }

    fn kill(&mut self, dest: TempIndex) {
        self.map.remove(&dest);
        self.havoced.remove(&dest);
        // A redefinition of `dest` invalidates every alias to it recorded for other
        // temps: their values were copied from the overwritten definition, and a
        // later use would propagate to the redefined temp and read the wrong value.
        // (Source-level reassignments arrive as `Assign`s from fresh temps;
        // pass-emitted code redefines temps directly via `Load`/`Call` dests.) The
        // whole entry is removed, not the one alias: a multi-element set stems from a
        // join, so a surviving element holds only on some paths; and removal, unlike
        // an emptied set, survives the join — union would resurrect an empty set.
        let stale: Vec<TempIndex> = self
            .map
            .iter()
            .filter(|(_, defs)| defs.contains(&Def::Alias(dest)))
            .map(|(temp, _)| *temp)
            .collect();
        for temp in stale {
            self.map.remove(&temp);
        }
    }

    fn havoc(&mut self, dest: TempIndex) {
        self.havoced.insert(dest);
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
        if let Some(map_at) = map.get(&code_offset) {
            let mut res = map_at
                .map
                .iter()
                .map(|(idx, defs)| {
                    let name = target.get_local_name(*idx);
                    format!(
                        "{} -> {{{}}}",
                        name.display(target.symbol_pool()),
                        defs.iter()
                            .map(|def| {
                                match def {
                                    Def::Alias(a) => {
                                        let local_name = format!(
                                            "{}",
                                            target.get_local_name(*a).display(target.symbol_pool())
                                        );
                                        if map_at.havoced.contains(a) {
                                            format!("{}, {}*", local_name, local_name)
                                        } else {
                                            local_name
                                        }
                                    },
                                }
                            })
                            .join(", ")
                    )
                })
                .join(", ");
            res.insert_str(0, "reach: ");
            return Some(res);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn propagated(state: &ReachingDefState, temp: TempIndex) -> TempIndex {
        ReachingDefProcessor::get_propagated_local(temp, state)
    }

    #[test]
    fn kill_invalidates_aliases_to_redefined_temp() {
        // saved := a; a := <load/call>; use saved  — must NOT propagate to a.
        let mut state = ReachingDefState::default();
        state.def_alias(1, 0); // saved(1) := a(0)
        assert_eq!(propagated(&state, 1), 0);
        state.kill(0); // a redefined
        assert_eq!(propagated(&state, 1), 1);
    }

    #[test]
    fn kill_invalidates_alias_chains() {
        // y := x; z := y; x := <load>; use z — must stop at y, not reach x.
        let mut state = ReachingDefState::default();
        state.def_alias(1, 0); // y(1) := x(0)
        state.def_alias(2, 1); // z(2) := y(1)
        assert_eq!(propagated(&state, 2), 0);
        state.kill(0);
        assert_eq!(propagated(&state, 2), 1);
    }

    #[test]
    fn join_preserves_kill_of_stale_alias() {
        // Path A: copy := x intact. Path B: x redefined after the copy.
        // After the join the alias must not survive (it only holds on path A).
        let mut a = ReachingDefState::default();
        a.def_alias(1, 0);
        let mut b = ReachingDefState::default();
        b.def_alias(1, 0);
        b.kill(0);
        a.join(&b);
        assert_eq!(propagated(&a, 1), 1);
    }

    #[test]
    fn joined_alias_sets_do_not_propagate() {
        // y aliases x on one path and w on the other; neither may propagate.
        let mut a = ReachingDefState::default();
        a.def_alias(1, 0);
        let mut b = ReachingDefState::default();
        b.def_alias(1, 2);
        a.join(&b);
        assert_eq!(propagated(&a, 1), 1);
    }
}
