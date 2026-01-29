// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! # Number Operation Dataflow Analysis
//!
//! This module implements a dataflow analysis that propagates NumOperation classifications
//! throughout the program to determine whether each variable, field, and expression should
//! use mathematical integers or bitvectors in the generated Boogie code.
//!
//! ## Analysis Strategy
//!
//! 1. **Initialization**: Seed classifications from `pragma bv` annotations (see number_operation.rs)
//! 2. **Propagation**: Run dataflow analysis to propagate operations through:
//!    - Assignment statements
//!    - Arithmetic/bitwise operations
//!    - Function calls (bidirectional: actual ↔ formal parameters)
//!    - Struct pack/unpack operations
//!    - Spec expressions
//! 3. **Fixed Point**: Iterate until no more changes occur
//! 4. **Conflict Detection**: Report errors when values are used in both arithmetic and bitwise contexts
//!
//! ## Key Principles
//!
//! - **Bitwise operations** (&, |, ^) force Bitwise classification
//! - **Arithmetic operations** (+, -, *, /, %) can use either, but propagate existing classification
//! - **Shift operations** (<<, >>) propagate the existing classification without forcing Bitwise
//! - **Function calls** unify actual and formal parameter types
//! - **Struct operations** ensure fields have consistent types across all uses
//!
//! ## Example Propagation
//!
//! ```move
//! fun example(x: u8) {  // x starts as Bottom
//!     let y = x & 0xFF;  // Forces x and y to Bitwise
//!     let z = y + 1;     // z becomes Bitwise (propagated from y)
//!     // x, y, z all end up as Bitwise
//! }
//! ```

use crate::number_operation::{
    GlobalNumberOperationState, NumOperation,
    NumOperation::{Arithmetic, Bitwise, Bottom},
};
use itertools::Either;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{Exp, ExpData, Operation as ASTOperation, TempIndex},
    model::{FieldId, FunId, GlobalEnv, ModuleId, Parameter, StructId},
    ty::{PrimitiveType, Type},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::FunctionTarget,
    function_target_pipeline::{
        FunctionTargetPipeline, FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant,
    },
    stackless_bytecode::{AttrId, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{collections::BTreeMap, str};

/// Error message shown when a value is used in conflicting contexts (both arithmetic and bitwise).
static CONFLICT_ERROR_MSG: &str = "cannot appear in both arithmetic and bitwise operation, please refer to https://aptos.dev/en/build/smart-contracts/prover/spec-lang#bitwise-operators for more information";

/// Main processor that orchestrates the number operation analysis.
/// This runs as part of the bytecode pipeline and iterates until reaching a fixed point.
pub struct NumberOperationProcessor {}

impl NumberOperationProcessor {
    pub fn new() -> Box<Self> {
        Box::new(NumberOperationProcessor {})
    }

    /// Create initial number operation state for expressions
    pub fn create_initial_exp_oper_state(&self, env: &GlobalEnv) {
        let mut default_exp = BTreeMap::new();
        let exp_info_map = env.get_nodes();
        for id in exp_info_map {
            default_exp.insert(id, Bottom);
        }
        let mut global_state = env.get_cloned_extension::<GlobalNumberOperationState>();
        global_state.exp_operation_map = default_exp;
        env.set_extension(global_state);
    }

    /// Entry point of the analysis. Iterates over all functions in reverse topological order
    /// (callees before callers) until reaching a fixed point where no more types change.
    ///
    /// This ensures that:
    /// - Callee signatures are analyzed before callers
    /// - Changes in callees propagate back to callers in subsequent iterations
    /// - Strongly connected components (mutual recursion) are handled correctly
    fn analyze<'a>(&self, env: &'a GlobalEnv, targets: &'a FunctionTargetsHolder) {
        self.create_initial_exp_oper_state(env);
        let fun_env_vec = FunctionTargetPipeline::sort_in_reverse_topological_order(env, targets);
        let init_state = env
            .get_extension::<GlobalNumberOperationState>()
            .unwrap_or_default();
        let mut pre_state = init_state.clone();
        // run until fixed point is reached
        loop {
            for item in &fun_env_vec {
                match item {
                    Either::Left(fid) => {
                        let func_env = env.get_function(*fid);
                        if func_env.is_inline() {
                            continue;
                        }
                        for (_, target) in targets.get_targets(&func_env) {
                            if target.data.code.is_empty() {
                                continue;
                            }
                            self.analyze_fun(target.clone());
                        }
                    },
                    Either::Right(scc) => {
                        for fid in scc {
                            let func_env = env.get_function(*fid);
                            if func_env.is_inline() {
                                continue;
                            }
                            for (_, target) in targets.get_targets(&func_env) {
                                if target.data.code.is_empty() {
                                    continue;
                                }
                                self.analyze_fun(target.clone());
                            }
                        }
                    },
                }
            }
            let post_state = env
                .get_extension::<GlobalNumberOperationState>()
                .unwrap_or_default();
            if pre_state == post_state {
                break;
            }
            pre_state = post_state.clone();
        }
    }

    fn analyze_fun(&self, target: FunctionTarget) {
        if !target.func_env.is_native_or_intrinsic() {
            let cfg = StacklessControlFlowGraph::one_block(target.get_bytecode());
            let analyzer = NumberOperationAnalysis {
                func_target: target,
            };
            analyzer.analyze_function(
                NumberOperationState::create_initial_state(),
                analyzer.func_target.get_bytecode(),
                &cfg,
            );
        }
    }
}

impl FunctionTargetProcessor for NumberOperationProcessor {
    fn is_single_run(&self) -> bool {
        true
    }

    fn run(&self, env: &GlobalEnv, targets: &mut FunctionTargetsHolder) {
        self.analyze(env, targets);
    }

    fn name(&self) -> String {
        "number_operation_analysis".to_string()
    }
}

/// Per-function analysis state for the dataflow analysis.
struct NumberOperationAnalysis<'a> {
    func_target: FunctionTarget<'a>,
}

/// Local analysis state tracking whether changes occurred during this pass.
/// Used by the dataflow framework to determine when a fixed point is reached.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
struct NumberOperationState {
    /// Flag indicating whether the global state changed during this pass.
    /// If true, another iteration is needed.
    pub changed: bool,
}

impl NumberOperationState {
    /// Create a default NumberOperationState
    fn create_initial_state() -> Self {
        NumberOperationState { changed: false }
    }
}

/// Check if a vector/table function propagates element type to its destination (return value).
/// These functions return elements from the collection, so the return type should match the element type.
fn vector_table_funs_name_propagate_to_dest(callee_name: &str) -> bool {
    callee_name.contains("borrow")
        || callee_name.contains("borrow_mut")
        || callee_name.contains("pop_back")
        || callee_name.contains("singleton")
        || callee_name.contains("remove")
        || callee_name.contains("swap_remove")
        || callee_name.contains("spec_get")
}

/// Check if a vector function propagates element type from source arguments.
/// These functions add elements to the vector, so the element type should match the argument type.
fn vector_funs_name_propagate_to_srcs(callee_name: &str) -> bool {
    callee_name == "contains"
        || callee_name == "index_of"
        || callee_name == "append"
        || callee_name == "push_back"
        || callee_name == "insert"
}

/// Check if a table function propagates value type from source arguments.
/// These functions insert values into the table, so the value type should match the argument type.
fn table_funs_name_propagate_to_srcs(callee_name: &str) -> bool {
    callee_name == "add"
        || callee_name == "borrow_mut_with_default"
        || callee_name == "borrow_with_default"
        || callee_name == "upsert"
}

impl NumberOperationAnalysis<'_> {
    /// Helper to get current function's module and function IDs
    fn func_ids(&self) -> (ModuleId, FunId) {
        (
            self.func_target.func_env.module_env.get_id(),
            self.func_target.func_env.get_id(),
        )
    }

    /// Helper to check if this is a baseline variant
    fn is_baseline(&self) -> bool {
        self.func_target.data.variant == FunctionVariant::Baseline
    }

    /// Helper to get the operation for a temp index
    fn get_temp_oper(
        &self,
        idx: TempIndex,
        global_state: &GlobalNumberOperationState,
    ) -> NumOperation {
        let (mid, fid) = self.func_ids();
        *global_state
            .get_temp_index_oper(mid, fid, idx, self.is_baseline())
            .unwrap_or(&Bottom)
    }

    /// Helper to update temp index operation and track if it changed
    fn update_temp_oper(
        &self,
        idx: TempIndex,
        oper: NumOperation,
        global_state: &mut GlobalNumberOperationState,
        state: &mut NumberOperationState,
    ) {
        let (mid, fid) = self.func_ids();
        let current_oper = self.get_temp_oper(idx, global_state);
        if current_oper != oper {
            state.changed = true;
            *global_state
                .get_mut_temp_index_oper(mid, fid, idx, self.is_baseline())
                .unwrap() = oper;
        }
    }

    /// Helper to merge and update two temp indices to have the same operation
    fn merge_temp_opers(
        &self,
        idx1: TempIndex,
        idx2: TempIndex,
        global_state: &mut GlobalNumberOperationState,
        state: &mut NumberOperationState,
    ) {
        let oper1 = self.get_temp_oper(idx1, global_state);
        let oper2 = self.get_temp_oper(idx2, global_state);
        let merged = oper1.merge(&oper2);

        if merged != oper1 {
            self.update_temp_oper(idx1, merged, global_state, state);
        }
        if merged != oper2 {
            self.update_temp_oper(idx2, merged, global_state, state);
        }
    }

    /// Handle vector/table function calls in spec expressions.
    /// Propagates NumOperation between collection instances and their elements.
    ///
    /// # Parameters
    /// - `callee_name`: Name of the vector/table function being called
    /// - `args`: Arguments to the function call
    /// - `result_node_id`: Node ID of the function call result
    /// - `global_state`: Global state to update
    /// - `state`: Local state tracking changes
    /// - `allow_merge`: Whether to allow merging operations
    fn handle_vector_table_spec_call(
        &self,
        callee_name: &str,
        args: &[Exp],
        result_node_id: move_model::model::NodeId,
        global_state: &mut GlobalNumberOperationState,
        state: &mut NumberOperationState,
        allow_merge: bool,
    ) {
        if args.is_empty() {
            return;
        }

        let (cur_mid, cur_fid) = self.func_ids();
        let baseline_flag = self.is_baseline();

        // Helper to update a temporary variable from an expression
        let update_temporary = |arg: &Exp,
                                oper: &NumOperation,
                                global_state: &mut GlobalNumberOperationState,
                                state: &mut NumberOperationState| {
            if let ExpData::Temporary(_, idx) = arg.as_ref() {
                let cur_oper = global_state
                    .get_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                    .unwrap_or(&Bottom);
                if *cur_oper != *oper {
                    state.changed = true;
                    *global_state
                        .get_mut_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                        .unwrap() = *oper;
                }
            }
        };

        // Get the operation of the first argument (the collection instance)
        let oper_first = global_state.get_node_num_oper(args[0].node_id());

        // Functions that return elements from the collection propagate the collection's element type to the result
        if vector_table_funs_name_propagate_to_dest(callee_name) {
            global_state.update_node_oper(result_node_id, oper_first, true);
        } else {
            global_state.update_node_oper(result_node_id, Bottom, allow_merge);
        }

        // Special handling for borrow_mut_with_default which provides a default value
        // The default value (3rd argument) must match the collection's element type
        if callee_name.contains("borrow_mut_with_default") && args.len() >= 3 {
            update_temporary(&args[2], &oper_first, global_state, state);
        }
    }

    /// Handle vector/table function calls in bytecode.
    /// Propagates NumOperation between collection instances and their elements,
    /// with bidirectional propagation when Bitwise is involved.
    ///
    /// # Parameters
    /// - `module_env`: Module environment of the callee
    /// - `callee_name`: Name of the vector/table function being called
    /// - `srcs`: Source temp indices (arguments)
    /// - `dests`: Destination temp indices (return values)
    /// - `global_state`: Global state to update
    /// - `state`: Local state tracking changes
    fn handle_vector_table_bytecode_call(
        &self,
        module_env: &move_model::model::ModuleEnv,
        callee_name: &str,
        srcs: &[TempIndex],
        dests: &[TempIndex],
        global_state: &mut GlobalNumberOperationState,
        state: &mut NumberOperationState,
    ) {
        if srcs.is_empty() {
            return;
        }

        let (cur_mid, cur_fid) = self.func_ids();
        let baseline_flag = self.is_baseline();

        // Helper closure to update a temp index to Bitwise if it's not already
        let check_and_update_bitwise =
            |idx: &TempIndex,
             global_state: &mut GlobalNumberOperationState,
             state: &mut NumberOperationState| {
                let cur_oper = global_state
                    .get_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                    .unwrap();

                if *cur_oper != Bitwise {
                    state.changed = true;
                    *global_state
                        .get_mut_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                        .unwrap() = Bitwise;
                }
            };

        // Get the operation of the first argument (the collection instance)
        let first_oper = *global_state
            .get_temp_index_oper(cur_mid, cur_fid, srcs[0], baseline_flag)
            .unwrap();

        // If the function returns elements and the collection has Bitwise elements,
        // propagate Bitwise to all return values
        if vector_table_funs_name_propagate_to_dest(callee_name) && first_oper == Bitwise {
            for dest in dests.iter() {
                check_and_update_bitwise(dest, global_state, state);
            }
        }

        // Determine which source argument contains the element being added to the collection
        let mut second_oper = first_oper;
        let mut src_idx = 0;

        if module_env.is_std_vector() && vector_funs_name_propagate_to_srcs(callee_name) {
            // For vector functions like push_back, the element is the 2nd argument (index 1)
            assert!(srcs.len() > 1);
            second_oper = *global_state
                .get_temp_index_oper(cur_mid, cur_fid, srcs[1], baseline_flag)
                .unwrap();
            src_idx = 1;
        } else if table_funs_name_propagate_to_srcs(callee_name) {
            // For table functions like add, the value is the 3rd argument (index 2)
            assert!(srcs.len() > 2);
            second_oper = *global_state
                .get_temp_index_oper(cur_mid, cur_fid, srcs[2], baseline_flag)
                .unwrap();
            src_idx = 2;
        }

        // Bidirectional propagation: if either the collection or the element is Bitwise,
        // both must be Bitwise for consistency
        if first_oper == Bitwise || second_oper == Bitwise {
            check_and_update_bitwise(&srcs[0], global_state, state);
            check_and_update_bitwise(&srcs[src_idx], global_state, state);
        }
    }

    /// Analyze and propagate NumOperation through a spec expression.
    /// This recursively visits all sub-expressions and:
    /// - Propagates operations through operators (arithmetic, bitwise, comparison)
    /// - Handles special operations (Cast, Index, Select, Pack, SpecFunction calls)
    /// - Detects conflicts between arithmetic and bitwise usage
    /// - Updates the global state with inferred types
    ///
    /// # Parameters
    /// - `attr_id`: Source location for error reporting
    /// - `e`: The expression to analyze
    /// - `global_state`: Global state to update with inferred operations
    /// - `state`: Local state tracking if changes occurred
    fn handle_exp(
        &self,
        attr_id: AttrId,
        e: &Exp,
        global_state: &mut GlobalNumberOperationState,
        state: &mut NumberOperationState,
    ) {
        // Currently we don't support automatic int-to-bv conversion in specs
        let allow_merge = false;
        let opers_for_propagation = |oper: &move_model::ast::Operation| {
            use move_model::ast::Operation::*;
            matches!(
                *oper,
                Add | Sub
                    | Mul
                    | Div
                    | Mod
                    | BitOr
                    | BitAnd
                    | Xor
                    | Shr
                    | Shl
                    | Lt
                    | Le
                    | Gt
                    | Ge
                    | Neq
                    | Eq
            )
        };
        let bitwise_oper = |oper: &move_model::ast::Operation| {
            use move_model::ast::Operation::*;
            matches!(*oper, |BitOr| BitAnd | Xor)
        };
        let baseline_flag = self.func_target.data.variant == FunctionVariant::Baseline;
        let cur_mid = self.func_target.func_env.module_env.get_id();
        let cur_fid = self.func_target.func_env.get_id();
        let update_temporary = |arg: &Exp,
                                oper: &NumOperation,
                                global_state: &mut GlobalNumberOperationState,
                                state: &mut NumberOperationState| {
            if let ExpData::Temporary(_, idx) = arg.as_ref() {
                let cur_oper = global_state
                    .get_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                    .unwrap_or(&Bottom);
                if *cur_oper != *oper {
                    state.changed = true;
                    *global_state
                        .get_mut_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                        .unwrap() = *oper;
                }
            }
        };
        let visitor = &mut |exp: &ExpData| {
            match exp {
                ExpData::Temporary(id, idx) => {
                    let baseline_flag = self.func_target.data.variant == FunctionVariant::Baseline;
                    let oper = global_state
                        .get_temp_index_oper(cur_mid, cur_fid, *idx, baseline_flag)
                        .unwrap_or(&Bottom);
                    // Update num_oper for the node for the temporary variable
                    global_state.update_node_oper(*id, *oper, true);
                },
                ExpData::Block(id, pattern, opt_exp, exp) => {
                    // Assume that the pattern is a single variable because spec does not support
                    // tuple or function that returns a tuple for now
                    if let move_model::ast::Pattern::Var(pid, _) = pattern {
                        if let Some(exp) = opt_exp {
                            let source_ty = self.func_target.global_env().get_node_type(*pid);
                            if matches!(source_ty, Type::Primitive(PrimitiveType::Num)) {
                                self.func_target.global_env().update_node_type(
                                    *pid,
                                    self.func_target
                                        .global_env()
                                        .get_node_type(exp.node_id())
                                        .skip_reference()
                                        .clone(),
                                );
                            }
                        }
                    }
                    let exp_oper = global_state.get_node_num_oper(exp.node_id());
                    global_state.update_node_oper(*id, exp_oper, true);
                },
                ExpData::IfElse(id, _, true_exp, false_exp) => {
                    let true_oper = global_state.get_node_num_oper(true_exp.node_id());
                    let false_oper = global_state.get_node_num_oper(false_exp.node_id());
                    if !allow_merge && true_oper.conflict(&false_oper) {
                        self.func_target.global_env().error(
                            &self.func_target.get_bytecode_loc(attr_id),
                            CONFLICT_ERROR_MSG,
                        );
                    }
                    let merged = true_oper.merge(&false_oper);
                    global_state.update_node_oper(true_exp.node_id(), merged, true);
                    global_state.update_node_oper(false_exp.node_id(), merged, true);
                    global_state.update_node_oper(*id, merged, true);
                },
                ExpData::Call(id, oper, args) => {
                    let mut arg_oper = vec![];
                    for arg in args {
                        arg_oper.push(global_state.get_node_num_oper(arg.node_id()));
                    }
                    match oper {
                        move_model::ast::Operation::Identical => {
                            let num_oper_0 = global_state.get_node_num_oper(args[0].node_id());
                            let num_oper_1 = global_state.get_node_num_oper(args[1].node_id());
                            if !num_oper_0.conflict(&num_oper_1) {
                                let merged = num_oper_0.merge(&num_oper_1);
                                global_state.update_node_oper(*id, merged, true);
                                for arg in args {
                                    global_state.update_node_oper(arg.node_id(), merged, true);
                                    update_temporary(arg, &merged, global_state, state);
                                }
                            }
                        },
                        // Update node for index
                        move_model::ast::Operation::Index => {
                            global_state.update_node_oper(*id, arg_oper[0], true);
                        },
                        // Update node for return value
                        move_model::ast::Operation::Result(i) => {
                            let oper = global_state
                                .get_ret_map()
                                .get(&(cur_mid, cur_fid))
                                .unwrap()
                                .get(i)
                                .unwrap_or(&Bottom);
                            global_state.update_node_oper(*id, *oper, true);
                        },
                        // Update node for field operation
                        move_model::ast::Operation::Select(mid, sid, field_id)
                        | move_model::ast::Operation::UpdateField(mid, sid, field_id) => {
                            let field_oper =
                                global_state.get_num_operation_field(mid, sid, field_id);
                            global_state.update_node_oper(*id, *field_oper, true);
                        },
                        move_model::ast::Operation::SelectVariants(mid, sid, field_ids) => {
                            for field_id in field_ids {
                                let field_oper =
                                    global_state.get_num_operation_field(mid, sid, field_id);
                                global_state.update_node_oper(*id, *field_oper, true);
                            }
                        },
                        move_model::ast::Operation::Cast => {
                            // Obtained the updated num_oper of the expression
                            let num_oper = global_state.get_node_num_oper(args[0].node_id());
                            // Update the node of cast
                            global_state.update_node_oper(*id, num_oper, true);
                        },
                        move_model::ast::Operation::Int2Bv => {
                            global_state.update_node_oper(*id, Bitwise, true);
                        },
                        move_model::ast::Operation::Bv2Int => {
                            global_state.update_node_oper(*id, Arithmetic, true);
                        },
                        move_model::ast::Operation::SpecFunction(mid, sid, _) => {
                            let module_env = &self.func_target.global_env().get_module(*mid);
                            let callee_name = module_env
                                .get_spec_fun(*sid)
                                .name
                                .display(self.func_target.global_env().symbol_pool())
                                .to_string();
                            if module_env.is_std_vector() || module_env.is_table() {
                                self.handle_vector_table_spec_call(
                                    &callee_name,
                                    args,
                                    *id,
                                    global_state,
                                    state,
                                    allow_merge,
                                );
                            } else {
                                // Analysis for general spec functions.
                                let module = &self.func_target.global_env().get_module(*mid);
                                let callee_spec_fun = module.get_spec_fun(*sid);
                                // Try to get num_oper for signatures
                                // If not exists, compute num_oper for this spec fun and update the exp_operation_map and spec_fun_map
                                if let std::collections::btree_map::Entry::Vacant(_) =
                                    global_state.spec_fun_operation_map.entry((*mid, *sid))
                                {
                                    let mut para_vec = vec![];
                                    let mut ret_vec = vec![];
                                    // Default num oper is determined by the actual arguments
                                    para_vec.append(&mut arg_oper);
                                    ret_vec.push(Bottom);
                                    if callee_spec_fun.body.is_some() {
                                        let body_exp = callee_spec_fun.body.as_ref().unwrap();
                                        let local_map = body_exp.bound_local_vars_with_node_id();
                                        for (i, Parameter(sym, _, loc)) in
                                            callee_spec_fun.params.iter().enumerate()
                                        {
                                            if local_map.contains_key(sym) {
                                                let sym_node_id = local_map.get(sym).unwrap();
                                                let oper_opt =
                                                    global_state.exp_operation_map.get(sym_node_id);
                                                if let Some(oper) = oper_opt {
                                                    // Still need to check compatibility
                                                    if !allow_merge && oper.conflict(&para_vec[i]) {
                                                        self.func_target
                                                            .global_env()
                                                            .error(loc, CONFLICT_ERROR_MSG);
                                                    } else {
                                                        let merged = oper.merge(&para_vec[i]);
                                                        para_vec[i] = merged;
                                                    }
                                                }
                                                global_state.update_node_oper(
                                                    *sym_node_id,
                                                    para_vec[i],
                                                    true,
                                                );
                                            }
                                        }
                                        global_state
                                            .spec_fun_operation_map
                                            .insert((*mid, *sid), (para_vec, ret_vec));

                                        // Check compatibility between formal and actual arguments
                                        self.handle_exp(attr_id, body_exp, global_state, state);
                                        global_state.update_node_oper(
                                            *id,
                                            global_state.get_node_num_oper(body_exp.node_id()),
                                            allow_merge,
                                        );
                                        global_state.update_spec_ret(
                                            mid,
                                            sid,
                                            global_state.get_node_num_oper(body_exp.node_id()),
                                        );
                                    } else {
                                        global_state
                                            .spec_fun_operation_map
                                            .insert((*mid, *sid), (para_vec, ret_vec));
                                    }
                                } else {
                                    // Check compatibility between formal and actual arguments
                                    let para_oper_vec = &global_state
                                        .spec_fun_operation_map
                                        .get(&(*mid, *sid))
                                        .unwrap()
                                        .0;
                                    assert_eq!(para_oper_vec.len(), arg_oper.len());
                                    for (formal_oper, actual_oper) in
                                        para_oper_vec.iter().zip(arg_oper.iter())
                                    {
                                        // For simplicity, only check compatibility
                                        if !allow_merge && formal_oper.conflict(actual_oper) {
                                            self.func_target.global_env().error(
                                                &self.func_target.get_bytecode_loc(attr_id),
                                                CONFLICT_ERROR_MSG,
                                            );
                                        }
                                    }
                                }
                                // Update number oper for this node based on the return value of the spec fun
                                let ret_num_oper_vec = &global_state
                                    .spec_fun_operation_map
                                    .get(&(*mid, *sid))
                                    .unwrap()
                                    .1;
                                if !ret_num_oper_vec.is_empty() {
                                    global_state.update_node_oper(
                                        *id,
                                        ret_num_oper_vec[0],
                                        allow_merge,
                                    );
                                }
                            }
                        },
                        move_model::ast::Operation::WellFormed => {
                            global_state.update_node_oper(*id, arg_oper[0], true);
                        },
                        move_model::ast::Operation::Pack(mid, sid, None) => {
                            let struct_env = self
                                .func_target
                                .global_env()
                                .get_module(*mid)
                                .into_struct(*sid);
                            for (i, field) in struct_env.get_fields().enumerate() {
                                let field_oper =
                                    global_state.get_num_operation_field(mid, sid, &field.get_id());
                                let arg_oper = global_state.get_node_num_oper(args[i].node_id());
                                if !allow_merge && field_oper.conflict(&arg_oper) {
                                    self.func_target.global_env().error(
                                        &self.func_target.get_bytecode_loc(attr_id),
                                        CONFLICT_ERROR_MSG,
                                    );
                                }
                                let merged = field_oper.merge(&arg_oper);
                                global_state.update_node_oper(args[i].node_id(), merged, true);
                                global_state
                                    .struct_operation_map
                                    .get_mut(&(*mid, *sid))
                                    .unwrap()
                                    .insert(field.get_id(), merged);
                            }
                        },
                        _ => {
                            // All args must have compatible number operations
                            // TODO(tengzhang): support converting int to bv
                            if opers_for_propagation(oper) {
                                let mut merged = if bitwise_oper(oper) { Bitwise } else { Bottom };
                                for num_oper in &arg_oper {
                                    if !allow_merge && num_oper.conflict(&merged) {
                                        self.func_target.global_env().error(
                                            &self.func_target.get_bytecode_loc(attr_id),
                                            CONFLICT_ERROR_MSG,
                                        );
                                    }
                                    merged = num_oper.merge(&merged);
                                }
                                // If operation involve operands with bv type, check and update concrete integer type if possible
                                if merged == Bitwise {
                                    let exp_ty = self
                                        .func_target
                                        .global_env()
                                        .get_node_type(exp.node_id())
                                        .skip_reference()
                                        .clone();
                                    let concrete_num_ty_oper_0 = self
                                        .func_target
                                        .global_env()
                                        .get_node_type(args[0].node_id());
                                    let concrete_num_ty_oper_1 = self
                                        .func_target
                                        .global_env()
                                        .get_node_type(args[1].node_id());
                                    if concrete_num_ty_oper_0.is_number() {
                                        let if_shift = matches!(
                                            oper,
                                            move_model::ast::Operation::Shl
                                                | move_model::ast::Operation::Shr
                                        );
                                        // For shift operation, we don't need to check compatibility between the two operands
                                        let concrete_num_ty = if if_shift {
                                            Some(concrete_num_ty_oper_0.clone())
                                        } else {
                                            concrete_num_ty_oper_0
                                                .is_compatible_num_type(&concrete_num_ty_oper_1)
                                        };
                                        if concrete_num_ty.is_none() {
                                            self.func_target.global_env().error(
                                                    &self.func_target.global_env().get_node_loc(exp.node_id()),
                                                    &format!("integer type mismatch between two operands, one has type `{}` while the other one has type `{}`, consider using explicit type cast",
                                                    concrete_num_ty_oper_0.display(&
                                                        self.func_target.global_env().get_type_display_ctx()),
                                                    concrete_num_ty_oper_1.display(&
                                                        self.func_target.global_env().get_type_display_ctx())),
                                                );
                                        }
                                        if exp_ty == Type::Primitive(PrimitiveType::Num)
                                            && concrete_num_ty.as_ref().is_some_and(|ty| {
                                                *ty != Type::Primitive(PrimitiveType::Num)
                                            })
                                        {
                                            self.func_target.global_env().update_node_type(
                                                exp.node_id(),
                                                concrete_num_ty.unwrap(),
                                            );
                                        }
                                    }
                                }
                                for (arg, arg_oper) in args.iter().zip(arg_oper.iter()) {
                                    if merged != *arg_oper {
                                        // need to update the num_oper type to avoid insertion of int2bv conversion
                                        // which is inefficient during SMT solving
                                        let update_flag = match arg.clone().into() {
                                            ExpData::Temporary(..)
                                            | ExpData::LocalVar(..)
                                            | ExpData::Value(..)
                                            | ExpData::Call(_, ASTOperation::Cast, _) => true,
                                            ExpData::Call(
                                                _,
                                                ASTOperation::SpecFunction(mid, sid, _),
                                                _,
                                            ) => {
                                                // if the current argument is a call to a recursive spec function
                                                // we need to update num_oper type, otherwise the boogie generator
                                                // will incorrectly insert inv2bv conversion
                                                let module_env =
                                                    &self.func_target.global_env().get_module(mid);
                                                let spec_f = module_env.get_spec_fun(sid);
                                                !spec_f.is_move_fun
                                                    && self
                                                        .func_target
                                                        .global_env()
                                                        .is_spec_fun_recursive(mid.qualified(sid))
                                            },
                                            _ => false,
                                        };
                                        if update_flag {
                                            global_state.update_node_oper(
                                                arg.node_id(),
                                                merged,
                                                allow_merge,
                                            );
                                            update_temporary(arg, &merged, global_state, state);
                                        }
                                    }
                                }
                                global_state.update_node_oper(*id, merged, allow_merge);
                            }
                        },
                    }
                },
                _ => {},
            }
            true // keep going
        };
        e.visit_post_order(visitor);
    }

    /// Update NumOperation for destinations and sources to a specific operation.
    /// Used for binary operations where all operands must have the same type.
    ///
    /// # Example
    /// ```
    /// let z = x + y;  // All three (x, y, z) must have compatible operations
    /// ```
    fn check_and_update_oper(
        &self,
        state: &mut NumberOperationState,
        dests: &[TempIndex],
        srcs: &[TempIndex],
        oper: NumOperation,
        _mid: ModuleId,
        _fid: FunId,
        global_state: &mut GlobalNumberOperationState,
        _baseline_flag: bool,
    ) {
        self.update_temp_oper(dests[0], oper, global_state, state);
        self.update_temp_oper(srcs[0], oper, global_state, state);
        self.update_temp_oper(srcs[1], oper, global_state, state);
    }

    fn check_and_update_oper_dest(
        &self,
        state: &mut NumberOperationState,
        dests: &[TempIndex],
        oper: NumOperation,
        _mid: ModuleId,
        _fid: FunId,
        global_state: &mut GlobalNumberOperationState,
        _baseline_flag: bool,
    ) {
        self.update_temp_oper(dests[0], oper, global_state, state);
    }

    /// Generate default num_oper for all non-parameter locals
    fn populate_non_param_oper(&self, global_state: &mut GlobalNumberOperationState) {
        let (mid, fid) = self.func_ids();
        let non_param_range = self.func_target.get_non_parameter_locals();
        let local_map = global_state.get_mut_non_param_local_map(mid, fid, self.is_baseline());

        for i in non_param_range {
            local_map.entry(i).or_insert(Bottom);
        }
    }

    /// Handle pack/unpack operations - merge field types with temporary variables
    fn handle_pack_unpack(
        &self,
        msid: ModuleId,
        sid: StructId,
        _field_idx: usize,
        field_id: FieldId,
        temp_idx: TempIndex,
        state: &mut NumberOperationState,
        global_state: &mut GlobalNumberOperationState,
    ) {
        let field_oper = *global_state.get_num_operation_field(&msid, &sid, &field_id);
        let temp_oper = self.get_temp_oper(temp_idx, global_state);
        let merged = field_oper.merge(&temp_oper);

        if merged != field_oper {
            state.changed = true;
            global_state
                .struct_operation_map
                .get_mut(&(msid, sid))
                .unwrap()
                .insert(field_id, merged);
        }
        if merged != temp_oper {
            self.update_temp_oper(temp_idx, merged, global_state, state);
        }
    }
}

impl TransferFunctions for NumberOperationAnalysis<'_> {
    type State = NumberOperationState;

    const BACKWARD: bool = false;

    /// Core transfer function: propagate NumOperation through a single bytecode instruction.
    ///
    /// Handles different instruction types:
    /// - **Assign**: Propagate from source to destination
    /// - **Ret**: Unify return values with function return type
    /// - **Arithmetic ops** (Add, Sub, Mul, Div, Mod): Propagate and merge all operands
    /// - **Bitwise ops** (BitOr, BitAnd, Xor): Force all operands to Bitwise
    /// - **Shift ops** (Shl, Shr): Propagate without forcing Bitwise
    /// - **Comparison ops**: Unify compared values
    /// - **Pack/Unpack**: Unify field types with struct field declarations
    /// - **GetField/BorrowField**: Propagate field type to destination
    /// - **Function calls**: Unify actual and formal parameters bidirectionally
    fn execute(&self, state: &mut NumberOperationState, instr: &Bytecode, _offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;
        let mut global_state = self
            .func_target
            .global_env()
            .get_cloned_extension::<GlobalNumberOperationState>();
        self.populate_non_param_oper(&mut global_state);
        let (cur_mid, cur_fid) = self.func_ids();
        let baseline_flag = self.is_baseline();

        match instr {
            Assign(_, dest, src, _) => {
                self.merge_temp_opers(*dest, *src, &mut global_state, state);
            },
            // Check and update operations of rets in temp_index_operation_map and operations in ret_operation_map
            Ret(_, rets) => {
                let ret_types = self.func_target.get_return_types();
                for ((i, _), ret) in ret_types.iter().enumerate().zip(rets) {
                    let ret_oper = *global_state
                        .get_ret_map()
                        .get(&(cur_mid, cur_fid))
                        .unwrap()
                        .get(&i)
                        .unwrap();
                    let idx_oper = self.get_temp_oper(*ret, &global_state);
                    let merged = idx_oper.merge(&ret_oper);

                    if merged != idx_oper {
                        self.update_temp_oper(*ret, merged, &mut global_state, state);
                    }
                    if merged != ret_oper {
                        state.changed = true;
                        global_state
                            .get_mut_ret_map()
                            .get_mut(&(cur_mid, cur_fid))
                            .unwrap()
                            .insert(i, merged);
                    }
                }
            },
            Call(_, dests, oper, srcs, _) => {
                match oper {
                    BorrowLoc | ReadRef | CastU8 | CastU16 | CastU32 | CastU64 | CastU128
                    | CastU256 => {
                        self.merge_temp_opers(dests[0], srcs[0], &mut global_state, state);
                    },
                    WriteRef | Lt | Le | Gt | Ge | Eq | Neq => {
                        self.merge_temp_opers(srcs[0], srcs[1], &mut global_state, state);
                    },
                    Add | Sub | Mul | Div | Mod | Shl | Shr => {
                        // Merge operations of all operands (dest, src1, src2)
                        let op_srcs_0 = self.get_temp_oper(srcs[0], &global_state);
                        let op_srcs_1 = self.get_temp_oper(srcs[1], &global_state);
                        let op_dests_0 = self.get_temp_oper(dests[0], &global_state);
                        let merged = op_srcs_0.merge(&op_srcs_1).merge(&op_dests_0);

                        self.check_and_update_oper(
                            state,
                            dests,
                            srcs,
                            merged,
                            cur_mid,
                            cur_fid,
                            &mut global_state,
                            baseline_flag,
                        );
                    },

                    BitOr | BitAnd | Xor => {
                        // Bitwise operations force Bitwise classification
                        self.check_and_update_oper_dest(
                            state,
                            dests,
                            Bitwise,
                            cur_mid,
                            cur_fid,
                            &mut global_state,
                            baseline_flag,
                        );
                    },
                    // Checking and operations in the struct_operation_map when packing
                    Pack(msid, sid, _) => {
                        let struct_env = self
                            .func_target
                            .global_env()
                            .get_module(*msid)
                            .into_struct(*sid);
                        for (i, field) in struct_env.get_fields().enumerate() {
                            self.handle_pack_unpack(
                                *msid,
                                *sid,
                                i,
                                field.get_id(),
                                srcs[i],
                                state,
                                &mut global_state,
                            );
                        }
                    },
                    // Checking and operations in the struct_operation_map when packing an enum type
                    PackVariant(msid, sid, variant, _) => {
                        let struct_env = self
                            .func_target
                            .global_env()
                            .get_module(*msid)
                            .into_struct(*sid);
                        for (i, field) in struct_env.get_fields_of_variant(*variant).enumerate() {
                            let pool = struct_env.symbol_pool();
                            let new_field_id =
                                FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                                    pool.string(*variant).as_str(),
                                    pool.string(field.get_name()).as_str(),
                                )));
                            if srcs.len() > i {
                                self.handle_pack_unpack(
                                    *msid,
                                    *sid,
                                    i,
                                    new_field_id,
                                    srcs[i],
                                    state,
                                    &mut global_state,
                                );
                            }
                        }
                    },
                    // Checking and operations in the struct_operation_map when unpacking
                    Unpack(msid, sid, _) => {
                        let struct_env = self
                            .func_target
                            .global_env()
                            .get_module(*msid)
                            .into_struct(*sid);
                        for (i, field) in struct_env.get_fields().enumerate() {
                            self.handle_pack_unpack(
                                *msid,
                                *sid,
                                i,
                                field.get_id(),
                                dests[i],
                                state,
                                &mut global_state,
                            );
                        }
                    },
                    // Checking and operations in the struct_operation_map when unpacking an enum
                    UnpackVariant(msid, sid, variant, _) => {
                        let struct_env = self
                            .func_target
                            .global_env()
                            .get_module(*msid)
                            .into_struct(*sid);
                        for (i, field) in struct_env.get_fields_of_variant(*variant).enumerate() {
                            let pool = struct_env.symbol_pool();
                            let new_field_id =
                                FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                                    pool.string(*variant).as_str(),
                                    pool.string(field.get_name()).as_str(),
                                )));
                            if dests.len() > i {
                                self.handle_pack_unpack(
                                    *msid,
                                    *sid,
                                    i,
                                    new_field_id,
                                    dests[i],
                                    state,
                                    &mut global_state,
                                );
                            }
                        }
                    },
                    GetField(msid, sid, _, offset) | BorrowField(msid, sid, _, offset) => {
                        let field_id = self
                            .func_target
                            .func_env
                            .module_env
                            .get_struct(*sid)
                            .get_field_by_offset(*offset)
                            .get_id();

                        self.handle_pack_unpack(
                            *msid,
                            *sid,
                            *offset,
                            field_id,
                            dests[0],
                            state,
                            &mut global_state,
                        );
                    },
                    GetVariantField(msid, sid, variants, _, offset)
                    | BorrowVariantField(msid, sid, variants, _, offset) => {
                        let struct_env = self
                            .func_target
                            .global_env()
                            .get_module(*msid)
                            .into_struct(*sid);
                        let pool = struct_env.symbol_pool();
                        let field_name = &self
                            .func_target
                            .func_env
                            .module_env
                            .get_struct(*sid)
                            .get_field_by_offset_optional_variant(Some(variants[0]), *offset)
                            .get_name();
                        let new_field_id =
                            FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                                pool.string(variants[0]).as_str(),
                                pool.string(*field_name).as_str(),
                            )));
                        let dests_oper = global_state
                            .get_temp_index_oper(cur_mid, cur_fid, dests[0], baseline_flag)
                            .unwrap();
                        let field_oper =
                            global_state.get_num_operation_field(msid, sid, &new_field_id);

                        let merged_oper = dests_oper.merge(field_oper);
                        if merged_oper != *field_oper || merged_oper != *dests_oper {
                            state.changed = true;
                        }
                        *global_state
                            .get_mut_temp_index_oper(cur_mid, cur_fid, dests[0], baseline_flag)
                            .unwrap() = merged_oper;
                        for variant in variants {
                            let field_name = &self
                                .func_target
                                .func_env
                                .module_env
                                .get_struct(*sid)
                                .get_field_by_offset_optional_variant(Some(*variant), *offset)
                                .get_name();
                            let new_field_id =
                                FieldId::new(pool.make(&FieldId::make_variant_field_id_str(
                                    pool.string(*variant).as_str(),
                                    pool.string(*field_name).as_str(),
                                )));
                            global_state
                                .struct_operation_map
                                .get_mut(&(*msid, *sid))
                                .unwrap()
                                .insert(new_field_id, merged_oper);
                        }
                    },
                    Function(msid, fsid, _) => {
                        let module_env = &self.func_target.global_env().get_module(*msid);
                        // Vector functions are handled separately
                        if !module_env.is_std_vector() && !module_env.is_table() {
                            for (i, src) in srcs.iter().enumerate() {
                                let cur_oper = global_state
                                    .get_temp_index_oper(cur_mid, cur_fid, *src, baseline_flag)
                                    .unwrap();
                                let callee_oper = global_state
                                    .get_temp_index_oper(*msid, *fsid, i, true)
                                    .unwrap();

                                let merged = cur_oper.merge(callee_oper);
                                if merged != *cur_oper || merged != *callee_oper {
                                    state.changed = true;
                                }
                                *global_state
                                    .get_mut_temp_index_oper(cur_mid, cur_fid, *src, baseline_flag)
                                    .unwrap() = merged;
                                *global_state
                                    .get_mut_temp_index_oper(*msid, *fsid, i, true)
                                    .unwrap() = merged;
                            }
                            for (i, dest) in dests.iter().enumerate() {
                                let cur_oper = global_state
                                    .get_temp_index_oper(cur_mid, cur_fid, *dest, baseline_flag)
                                    .unwrap();
                                let callee_oper = global_state
                                    .get_ret_map()
                                    .get(&(*msid, *fsid))
                                    .unwrap()
                                    .get(&i)
                                    .unwrap();
                                let merged = cur_oper.merge(callee_oper);
                                if merged != *cur_oper || merged != *callee_oper {
                                    state.changed = true;
                                }
                                *global_state
                                    .get_mut_temp_index_oper(cur_mid, cur_fid, *dest, baseline_flag)
                                    .unwrap() = merged;
                                global_state
                                    .get_mut_ret_map()
                                    .get_mut(&(*msid, *fsid))
                                    .unwrap()
                                    .insert(i, merged);
                            }
                        } else {
                            // Handle vector/table function calls
                            let callee = module_env.get_function(*fsid);
                            let callee_name = callee.get_name_str();
                            self.handle_vector_table_bytecode_call(
                                module_env,
                                &callee_name,
                                srcs,
                                dests,
                                &mut global_state,
                                state,
                            );
                        }
                    },
                    // TODO(#14349): add support for enum type related operation and
                    //   function values.
                    _ => {},
                }
            },
            Prop(id, _, exp) => {
                self.handle_exp(*id, exp, &mut global_state, state);
            },
            _ => {},
        }
        self.func_target.global_env().set_extension(global_state);
    }
}

impl DataflowAnalysis for NumberOperationAnalysis<'_> {}

impl AbstractDomain for NumberOperationState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;
        self.changed = false;
        if other.changed {
            result = JoinResult::Changed;
        }
        result
    }
}
