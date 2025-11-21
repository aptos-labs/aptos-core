use super::{cfg_utils::collect_branch_info, get_function_name, is_table_function, ContainsCall};
use move_binary_format::file_format::CodeOffset;
use move_linter::temp_equivalence_analyzer::TempEquivalenceState;
use move_model::ast::TempIndex;
use move_stackless_bytecode::{
    dataflow_analysis::{BlockState, DataflowAnalysis, StateMap, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Label, Operation},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Default)]
pub(super) struct ContainsDomain {
    pub(super) known_present: BTreeSet<usize>,
    pub(super) known_absent: BTreeSet<usize>,
}

impl ContainsDomain {
    fn add_present(&mut self, idx: usize) {
        self.known_present.insert(idx);
        self.known_absent.remove(&idx);
    }

    fn add_absent(&mut self, idx: usize) {
        self.known_absent.insert(idx);
        self.known_present.remove(&idx);
    }

    fn invalidate_for_table(
        &mut self,
        contains_calls: &[ContainsCall],
        alias_state: &TempEquivalenceState,
        table_temp: TempIndex,
    ) {
        self.remove_matching(|idx| {
            let call = &contains_calls[idx];
            alias_state.are_equivalent(call.table, table_temp)
        });
    }

    fn invalidate_for_entry(
        &mut self,
        contains_calls: &[ContainsCall],
        alias_state: &TempEquivalenceState,
        table_temp: TempIndex,
        key_temp: TempIndex,
    ) {
        self.remove_matching(|idx| {
            let call = &contains_calls[idx];
            alias_state.are_equivalent(call.table, table_temp)
                && alias_state.are_equivalent(call.key, key_temp)
        });
    }

    fn remove_matching<F>(&mut self, predicate: F)
    where
        F: Fn(usize) -> bool,
    {
        let to_remove: Vec<usize> = self
            .known_present
            .iter()
            .chain(self.known_absent.iter())
            .copied()
            .filter(|idx| predicate(*idx))
            .collect();
        for idx in to_remove {
            self.known_present.remove(&idx);
            self.known_absent.remove(&idx);
        }
    }
}

impl AbstractDomain for ContainsDomain {
    fn join(&mut self, other: &Self) -> JoinResult {
        let present: BTreeSet<_> = self
            .known_present
            .intersection(&other.known_present)
            .copied()
            .collect();
        let absent: BTreeSet<_> = self
            .known_absent
            .intersection(&other.known_absent)
            .copied()
            .collect();

        let mut changed = JoinResult::Unchanged;
        if present != self.known_present {
            self.known_present = present;
            changed = JoinResult::Changed;
        }
        if absent != self.known_absent {
            self.known_absent = absent;
            changed = JoinResult::Changed;
        }
        changed
    }
}

pub(super) struct ContainsTransfer<'a> {
    target: &'a FunctionTarget<'a>,
    contains_calls: &'a [ContainsCall],
    contains_index: &'a BTreeMap<TempIndex, Vec<usize>>,
    negated_map: &'a HashMap<TempIndex, TempIndex>,
    equiv_states: &'a BTreeMap<CodeOffset, TempEquivalenceState>,
}

impl<'a> ContainsTransfer<'a> {
    pub(super) fn new(
        target: &'a FunctionTarget<'a>,
        contains_calls: &'a [ContainsCall],
        contains_index: &'a BTreeMap<TempIndex, Vec<usize>>,
        negated_map: &'a HashMap<TempIndex, TempIndex>,
        equiv_states: &'a BTreeMap<CodeOffset, TempEquivalenceState>,
    ) -> Self {
        Self {
            target,
            contains_calls,
            contains_index,
            negated_map,
            equiv_states,
        }
    }
}

impl TransferFunctions for ContainsTransfer<'_> {
    type State = ContainsDomain;
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, offset: CodeOffset) {
        if let Bytecode::Call(_, _, Operation::Function(module_id, function_id, _), srcs, _) = instr
        {
            let name = get_function_name(self.target, *module_id, *function_id);
            if srcs.is_empty() {
                return;
            }
            let Some(alias_state) = self.equiv_states.get(&offset) else {
                return;
            };
            let table = srcs[0];
            if is_table_function(&name, "add")
                || is_table_function(&name, "remove")
                || is_table_function(&name, "upsert")
            {
                if srcs.len() >= 2 {
                    let key = srcs[1];
                    state.invalidate_for_entry(self.contains_calls, alias_state, table, key);
                } else {
                    state.invalidate_for_table(self.contains_calls, alias_state, table);
                }
            } else if is_table_function(&name, "destroy_empty")
                || is_table_function(&name, "drop_unchecked")
            {
                state.invalidate_for_table(self.contains_calls, alias_state, table);
            }
        }
    }
}

impl DataflowAnalysis for ContainsTransfer<'_> {}

pub(super) fn analyze_contains_function(
    transfer: &ContainsTransfer,
    code: &[Bytecode],
    cfg: &StacklessControlFlowGraph,
    label_to_block: &HashMap<Label, BlockId>,
    equiv_states: &BTreeMap<CodeOffset, TempEquivalenceState>,
) -> StateMap<ContainsDomain> {
    let branch_info = collect_branch_info(code, cfg, label_to_block);
    let mut state_map: StateMap<ContainsDomain> = StateMap::new();
    let mut work_list = VecDeque::new();
    let entry = cfg.entry_block();
    state_map.insert(
        entry,
        BlockState {
            pre: ContainsDomain::default(),
            post: ContainsDomain::default(),
        },
    );
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
            let (idx, is_negated) = resolve_contains_condition(
                alias_state,
                info.cond,
                transfer.contains_index,
                transfer.negated_map,
            )?;
            let mut then_state = post.clone();
            let mut else_state = post.clone();
            if is_negated {
                then_state.add_absent(idx);
                else_state.add_present(idx);
            } else {
                then_state.add_present(idx);
                else_state.add_absent(idx);
            }
            Some((info.then_block, then_state, info.else_block, else_state))
        });

        let successors = cfg.successors(block_id).clone();
        for succ in successors {
            let succ_state =
                if let Some((then_block, then_state, else_block, else_state)) = &branch_update {
                    if succ == *then_block {
                        then_state.clone()
                    } else if succ == *else_block {
                        else_state.clone()
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
                    state_map.insert(
                        succ,
                        BlockState {
                            pre: succ_state.clone(),
                            post: ContainsDomain::default(),
                        },
                    );
                    work_list.push_back(succ);
                },
            }
        }

        state_map.get_mut(&block_id).expect("block state").post = post;
    }

    state_map
}

pub(super) fn resolve_contains_condition(
    alias_state: &TempEquivalenceState,
    cond_temp: TempIndex,
    contains_index: &BTreeMap<TempIndex, Vec<usize>>,
    negated_map: &HashMap<TempIndex, TempIndex>,
) -> Option<(usize, bool)> {
    let mut stack = vec![(cond_temp, false)];
    let mut visited = HashSet::new();

    while let Some((temp, is_negated)) = stack.pop() {
        if !visited.insert((temp, is_negated)) {
            continue;
        }
        if let Some(indices) = contains_index.get(&temp) {
            if let Some(&idx) = indices.last() {
                return Some((idx, is_negated));
            }
        }

        for alias in alias_state.equivalence_class(temp).iter() {
            if *alias != temp {
                stack.push((*alias, is_negated));
            }
            if let Some(indices) = contains_index.get(alias) {
                if let Some(&idx) = indices.last() {
                    return Some((idx, is_negated));
                }
            }
            if let Some(original) = negated_map.get(alias) {
                stack.push((*original, !is_negated));
            }
        }

        if let Some(original) = negated_map.get(&temp) {
            stack.push((*original, !is_negated));
        }
    }

    None
}
