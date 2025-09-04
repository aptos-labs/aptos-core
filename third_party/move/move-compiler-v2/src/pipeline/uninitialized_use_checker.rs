// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements a checker which verifies that all locals are initialized before any use.
//! This intra-procedural checker does not require any other analysis to be run before.
//! As a side effect of running this checker, function targets are annotated with
//! the initialized state (yes, no, maybe) of all locals at each reachable program point.
//!
//! There are two parts to this checker:
//! * `InitializedStateAnalysis` which computes the initialized state of all locals at each
//!   program point via a forward dataflow analysis.
//! * `UninitializedUseChecker` which checks that all locals are initialized before use.

use im::Vector;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::BTreeMap;

/// State of initialization of a local at a given program point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Initialized {
    No,    // definitely not initialized
    Maybe, // maybe initialized
    Yes,   // definitely initialized
}

impl AbstractDomain for Initialized {
    /// Implements `join` for the initialized state lattice:
    /// ```diagram
    ///             +-------+
    ///             | Maybe |
    ///             +-------+
    ///               /   \
    ///              /     \
    ///      +-----+/       \+----+
    ///      | Yes |         | No |
    ///      +-----+\       /+----+
    ///              \     /
    ///               \   /
    ///            +--------+
    ///            | bottom |
    ///            +--------+
    /// ```
    /// Note that bottom is not explicitly represented in the enum `Initialized`.
    /// Instead, it is implicit: it represents the initialized state for locals at unreachable program points.
    fn join(&mut self, other: &Self) -> JoinResult {
        if *self == *other {
            return JoinResult::Unchanged;
        }
        if *self != Initialized::Maybe {
            *self = Initialized::Maybe;
            JoinResult::Changed
        } else {
            JoinResult::Unchanged
        }
    }
}

/// Initialization state of all the locals at a program point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializedState(Vector<Initialized>);

impl InitializedState {
    /// Create a new initialized state with:
    /// * all param locals set to `Yes`
    /// * all other locals set to `No`.
    /// Note: `num_locals` is the total number of locals, including params.
    pub fn new(num_params: usize, num_locals: usize) -> Self {
        if num_locals < num_params {
            panic!("ICE: num_locals must be >= num_params");
        }
        Self(Vector::from_iter(
            std::iter::repeat(Initialized::Yes)
                .take(num_params)
                .chain(std::iter::repeat(Initialized::No).take(num_locals - num_params)),
        ))
    }

    /// Mark `local` as initialized.
    fn mark_as_initialized(&mut self, local: usize) {
        self.0.set(local, Initialized::Yes);
    }

    /// Get the initialization state of `local`.
    /// Panics if `local` does not exist in this state.
    fn get_initialized_state(&self, local: usize) -> Initialized {
        self.0.get(local).expect("local must exist").clone()
    }
}

impl AbstractDomain for InitializedState {
    /// `join` of two initialized states is the point-wise join for each local.
    /// The result is `JoinResult::Changed` if any of the local joins causes a change.
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;
        // Do a shallow check if the two states are the same.
        if self.0.ptr_eq(&other.0) {
            return result;
        }
        // Otherwise, join each element.
        for (l, r) in self.0.iter_mut().zip(other.0.iter()) {
            result = result.combine(l.join(r));
        }
        result
    }
}

/// Initialized state of all locals, both before and after various program points.
#[derive(Clone)]
struct InitializedStateAnnotation(
    BTreeMap<
        CodeOffset, // program point
        (
            /*before*/ InitializedState,
            /*after*/ InitializedState,
        ),
    >,
);

impl InitializedStateAnnotation {
    /// Get the initialized state of `local` just before the instruction at `offset`, if available.
    fn get_initialized_state(&self, local: TempIndex, offset: CodeOffset) -> Option<Initialized> {
        self.0
            .get(&offset)
            .map(|(before, _)| before.get_initialized_state(local))
    }
}

/// Analysis to compute the initialized state of all locals at each reachable program point.
/// This is an intra-procedural forward dataflow analysis.
pub struct InitializedStateAnalysis {
    num_params: usize, // number of parameters in the analyzed function
    num_locals: usize, // number of locals in the analyzed function, including parameters
}

impl InitializedStateAnalysis {
    /// Create a new instance of the initialized state analysis for a function.
    /// The function's `num_params` and `num_locals` are provided.
    ///
    /// Note: `num_locals` is the total number of locals, including params.
    /// Thus, `num_locals` must be >= `num_params`.
    pub fn new(num_params: usize, num_locals: usize) -> Self {
        Self {
            num_params,
            num_locals,
        }
    }

    /// Analyze the given function and return the initialized state of all locals before and
    /// after each reachable program point.
    fn analyze(&self, func_target: &FunctionTarget) -> InitializedStateAnnotation {
        let code = func_target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let block_state_map = self.analyze_function(
            InitializedState::new(self.num_params, self.num_locals),
            code,
            &cfg,
        );
        let per_bytecode_state =
            self.state_per_instruction(block_state_map, code, &cfg, |before, after| {
                (before.clone(), after.clone())
            });
        InitializedStateAnnotation(per_bytecode_state)
    }
}

impl TransferFunctions for InitializedStateAnalysis {
    type State = InitializedState;

    // This is a forward analysis.
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, _offset: CodeOffset) {
        // Once you write to a local, it is considered initialized.
        instr.dests().iter().for_each(|dst| {
            state.mark_as_initialized(*dst);
        });
    }
}

impl DataflowAnalysis for InitializedStateAnalysis {}

/// Checker which verifies that all locals are definitely initialized before use.
/// Violations are reported as errors.
pub struct UninitializedUseChecker {
    pub keep_annotations: bool,
}

impl UninitializedUseChecker {
    /// Check whether all locals are definitely initialized before use in the function `target`.
    /// Information about initialized state is provided in `annotation`.
    /// Violations are reported as errors in the `target`'s global environment.
    fn perform_checks(&self, target: &FunctionTarget, annotation: &InitializedStateAnnotation) {
        for (offset, bc) in target.get_bytecode().iter().enumerate() {
            let check = |temp: &usize| {
                if let Some(state @ (Initialized::Maybe | Initialized::No)) =
                    annotation.get_initialized_state(*temp, offset as CodeOffset)
                {
                    target.global_env().error(
                        &target.get_bytecode_loc(bc.get_attr_id()),
                        &format!(
                            "use of {}unassigned {}",
                            match state {
                                Initialized::Maybe => "possibly ",
                                _ => "",
                            },
                            target.get_local_name_for_error_message(*temp)
                        ),
                    );
                }
            };
            if let Bytecode::SpecBlock(_, spec) = bc {
                // `update_map` is not yet filled in this phase
                // only need to handle expressions in `conditions`
                for cond in &spec.conditions {
                    for exp in cond.all_exps() {
                        exp.used_temporaries().iter().for_each(check);
                    }
                }
            } else if bc.is_spec_only() {
                // We don't check spec-only instructions here because
                // they don't exist in the compilation phase
                continue;
            } else {
                bc.sources().iter().for_each(check);
            }
        }
    }

    /// Registers initialized state annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_initialized_state_annotation));
    }
}

impl FunctionTargetProcessor for UninitializedUseChecker {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            // We don't have to look inside native functions.
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        let analysis =
            InitializedStateAnalysis::new(target.get_parameter_count(), target.get_local_count());
        let annotation = analysis.analyze(&target);
        self.perform_checks(&target, &annotation);
        if self.keep_annotations {
            data.annotations.set(annotation, true); // for testing.
        }
        data
    }

    fn name(&self) -> String {
        "uninitialized_use_checker".to_string()
    }
}

// ====================================================================
// Formatting functionality for initialized state annotation.

/// Format the initialized state annotation for a given function target.
pub fn format_initialized_state_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let InitializedStateAnnotation(map) = target
        .get_annotations()
        .get::<InitializedStateAnnotation>()?;
    let (before, after) = map.get(&code_offset)?;
    let mut s = String::new();
    s.push_str("before: ");
    s.push_str(&format_initialized_state(before, target));
    s.push_str(", after: ");
    s.push_str(&format_initialized_state(after, target));
    Some(s)
}

/// Format a vector of `locals`.
/// `header` is added as a prefix.
/// `target` is used to get the name of each local symbol.
fn format_vector_of_locals(
    header: &str,
    locals: Vec<TempIndex>,
    target: &FunctionTarget,
) -> String {
    let mut s = String::new();
    s.push_str(&format!("{{ {}: ", header));
    s.push_str(
        &locals
            .into_iter()
            .map(|tmp| {
                let name = target.get_local_raw_name(tmp);
                name.display(target.symbol_pool()).to_string()
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str(" }");
    s
}

/// Format the initialized state for a given function `target`.
fn format_initialized_state(state: &InitializedState, target: &FunctionTarget) -> String {
    let mut s = String::new();
    let mut nos = vec![];
    let mut maybes = vec![];
    for (i, v) in state.0.iter().enumerate() {
        match v {
            Initialized::No => nos.push(i),
            Initialized::Maybe => maybes.push(i),
            Initialized::Yes => {},
        }
    }
    let mut all_initialized = true;
    if !nos.is_empty() {
        s.push_str(&format_vector_of_locals("no", nos, target));
        all_initialized = false;
    }
    if !maybes.is_empty() {
        s.push_str(&format_vector_of_locals("maybe", maybes, target));
        all_initialized = false;
    }
    if all_initialized {
        s.push_str("all initialized");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialized_join() {
        let mut state = Initialized::No;
        assert_eq!(state.join(&Initialized::No), JoinResult::Unchanged);
        assert_eq!(state, Initialized::No);

        state = Initialized::No;
        assert_eq!(state.join(&Initialized::Maybe), JoinResult::Changed);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::No;
        assert_eq!(state.join(&Initialized::Yes), JoinResult::Changed);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Maybe;
        assert_eq!(state.join(&Initialized::No), JoinResult::Unchanged);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Maybe;
        assert_eq!(state.join(&Initialized::Maybe), JoinResult::Unchanged);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Maybe;
        assert_eq!(state.join(&Initialized::Yes), JoinResult::Unchanged);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Yes;
        assert_eq!(state.join(&Initialized::No), JoinResult::Changed);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Yes;
        assert_eq!(state.join(&Initialized::Maybe), JoinResult::Changed);
        assert_eq!(state, Initialized::Maybe);

        state = Initialized::Yes;
        assert_eq!(state.join(&Initialized::Yes), JoinResult::Unchanged);
        assert_eq!(state, Initialized::Yes);
    }

    #[test]
    fn test_initialized_state_join() {
        let mut state = InitializedState::new(1, 2);
        let mut other = InitializedState::new(1, 2);
        assert_eq!(state.join(&other), JoinResult::Unchanged);
        assert_eq!(state, InitializedState::new(1, 2));

        state.mark_as_initialized(1);
        assert_eq!(state.join(&other), JoinResult::Changed);

        state = InitializedState::new(1, 2);
        other.mark_as_initialized(1);
        assert_eq!(state.join(&other), JoinResult::Changed);
    }
}
