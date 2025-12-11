use super::{cfg_utils::collect_branch_info, ZeroComparison, ZeroComparisonOp};
use crate::temp_equivalence_analyzer::TempEquivalenceState;
use move_binary_format::file_format::CodeOffset;
use move_model::ast::TempIndex;
use move_stackless_bytecode::{
    dataflow_analysis::{BlockState, DataflowAnalysis, StateMap, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Default)]
pub(super) struct ZeroDomain {
    proven_non_zero: BTreeSet<TempIndex>,
}

impl ZeroDomain {
    pub(super) fn is_non_zero(&self, temp: TempIndex) -> bool {
        self.proven_non_zero.contains(&temp)
    }

    pub(super) fn mark_non_zero(&mut self, temp: TempIndex) {
        self.proven_non_zero.insert(temp);
    }

    pub(super) fn clear(&mut self, temp: TempIndex) {
        self.proven_non_zero.remove(&temp);
    }
}

impl AbstractDomain for ZeroDomain {
    fn join(&mut self, other: &Self) -> JoinResult {
        let intersection: BTreeSet<_> = self
            .proven_non_zero
            .intersection(&other.proven_non_zero)
            .copied()
            .collect();

        if intersection == self.proven_non_zero {
            if intersection == other.proven_non_zero {
                JoinResult::Unchanged
            } else {
                self.proven_non_zero = intersection;
                JoinResult::Changed
            }
        } else {
            self.proven_non_zero = intersection;
            JoinResult::Changed
        }
    }
}

pub(super) struct ZeroTransfer<'a> {
    param_lookup: HashMap<TempIndex, TempIndex>,
    comparisons: &'a BTreeMap<TempIndex, ZeroComparison>,
    negated_map: &'a HashMap<TempIndex, TempIndex>,
    equiv_states: &'a BTreeMap<CodeOffset, TempEquivalenceState>,
}

impl<'a> ZeroTransfer<'a> {
    pub(super) fn new(
        param_temps: &[TempIndex],
        comparisons: &'a BTreeMap<TempIndex, ZeroComparison>,
        negated_map: &'a HashMap<TempIndex, TempIndex>,
        equiv_states: &'a BTreeMap<CodeOffset, TempEquivalenceState>,
    ) -> Self {
        let param_lookup = param_temps.iter().map(|temp| (*temp, *temp)).collect();
        Self {
            param_lookup,
            comparisons,
            negated_map,
            equiv_states,
        }
    }

    fn param_id_for(
        &self,
        alias_state: &TempEquivalenceState,
        temp: TempIndex,
    ) -> Option<TempIndex> {
        for alias in alias_state.equivalence_class(temp) {
            if let Some(param) = self.param_lookup.get(&alias) {
                return Some(*param);
            }
        }
        None
    }

    pub(super) fn resolve_condition(
        &self,
        alias_state: &TempEquivalenceState,
        cond_temp: TempIndex,
    ) -> Option<(TempIndex, bool, bool)> {
        let mut stack = vec![(cond_temp, false)];
        let mut visited = HashSet::new();

        while let Some((temp, is_negated)) = stack.pop() {
            if !visited.insert((temp, is_negated)) {
                continue;
            }

            if let Some(comparison) = self.comparisons.get(&temp) {
                let mut then_non_zero = matches!(comparison.op, ZeroComparisonOp::NeqZero);
                let mut else_non_zero = matches!(comparison.op, ZeroComparisonOp::EqZero);

                if is_negated {
                    std::mem::swap(&mut then_non_zero, &mut else_non_zero);
                }

                let param_temp = self.param_id_for(alias_state, comparison.address)?;

                return Some((param_temp, then_non_zero, else_non_zero));
            }

            for alias in alias_state.equivalence_class(temp) {
                if alias != temp {
                    stack.push((alias, is_negated));
                }
                if let Some(original) = self.negated_map.get(&alias) {
                    stack.push((*original, !is_negated));
                }
            }

            if let Some(original) = self.negated_map.get(&temp) {
                stack.push((*original, !is_negated));
            }
        }

        None
    }

    fn clear_if_param(
        &self,
        state: &mut ZeroDomain,
        alias_state: Option<&TempEquivalenceState>,
        temp: TempIndex,
    ) {
        if let Some(alias_state) = alias_state {
            if let Some(param_temp) = self.param_id_for(alias_state, temp) {
                state.clear(param_temp);
                return;
            }
        }

        if let Some(param_temp) = self.param_lookup.get(&temp) {
            state.clear(*param_temp);
        }
    }
}

impl TransferFunctions for ZeroTransfer<'_> {
    type State = ZeroDomain;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, offset: CodeOffset) {
        use Bytecode::*;

        let alias_state = self.equiv_states.get(&offset);

        match instr {
            Assign(_, dest, _, _) => self.clear_if_param(state, alias_state, *dest),
            Load(_, dest, _) => self.clear_if_param(state, alias_state, *dest),
            Call(_, dests, _, _, _) => {
                for dest in dests {
                    self.clear_if_param(state, alias_state, *dest);
                }
            },
            _ => {},
        }
    }
}

impl DataflowAnalysis for ZeroTransfer<'_> {}

pub(super) fn analyze_zero_function(
    transfer: &ZeroTransfer,
    code: &[Bytecode],
    cfg: &StacklessControlFlowGraph,
    label_to_block: &HashMap<Label, BlockId>,
    equiv_states: &BTreeMap<CodeOffset, TempEquivalenceState>,
) -> StateMap<ZeroDomain> {
    let branch_info = collect_branch_info(code, cfg, label_to_block);
    let mut state_map: StateMap<ZeroDomain> = StateMap::new();
    let mut work_list = VecDeque::new();
    let entry = cfg.entry_block();
    state_map.insert(entry, BlockState {
        pre: ZeroDomain::default(),
        post: ZeroDomain::default(),
    });
    work_list.push_back(entry);

    while let Some(block_id) = work_list.pop_front() {
        let pre = state_map
            .get(&block_id)
            .expect("block pre state")
            .pre
            .clone();
        let post = transfer.execute_block(block_id, pre, code, cfg);

        let branch_update = branch_info.get(&block_id).and_then(|info| {
            let alias_state = equiv_states.get(&info.last_offset)?;
            transfer
                .resolve_condition(alias_state, info.cond)
                .map(|resolution| (info.then_block, info.else_block, resolution))
        });

        let successors = cfg.successors(block_id).clone();
        for succ in successors {
            let succ_state =
                if let Some((then_block, else_block, (param_temp, then_non_zero, else_non_zero))) =
                    &branch_update
                {
                    if succ == *then_block {
                        let mut then_state = post.clone();
                        if *then_non_zero {
                            then_state.mark_non_zero(*param_temp);
                        } else {
                            then_state.clear(*param_temp);
                        }
                        then_state
                    } else if succ == *else_block {
                        let mut else_state = post.clone();
                        if *else_non_zero {
                            else_state.mark_non_zero(*param_temp);
                        } else {
                            else_state.clear(*param_temp);
                        }
                        else_state
                    } else {
                        post.clone()
                    }
                } else {
                    post.clone()
                };

            match state_map.get_mut(&succ) {
                Some(block_state) => {
                    let mut joined = block_state.pre.clone();
                    if joined.join(&succ_state) == JoinResult::Changed {
                        block_state.pre = joined;
                        work_list.push_back(succ);
                    }
                },
                None => {
                    state_map.insert(succ, BlockState {
                        pre: succ_state.clone(),
                        post: ZeroDomain::default(),
                    });
                    work_list.push_back(succ);
                },
            }
        }

        state_map.get_mut(&block_id).expect("block state").post = post;
    }

    state_map
}
