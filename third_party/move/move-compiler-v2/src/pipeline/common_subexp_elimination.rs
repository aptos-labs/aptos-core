// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "common subexpression elimination" transformation.
//!
//! //! prerequisites:
//! //! side effect:
//!
//! This implementation follows the Global Value Numbering (GVN) and Common Subexpression Elimination (CSE)
//! algorithms used by LLVM. Pseudocode:
//!
//! procedure COMMON_SUBEXPRESSION_ELIMINATION(function F):
//!    # Map from (opcode, operands) → value
//!    value_table ← empty map
//!
//!    # Process basic blocks in dominance order
//!    for block in DOMINANCE_ORDER(F):
//!
//!        for instr in block.instructions:
//!            if not is_pure(instr):
//!                # Side-effecting ops invalidate local vars
//!                invalidate_local_dependent_entries(value_table, inst)
//!                continue
//!
//!            key ← canonical_key(instr) # operands are recursively replaced by their keys
//!
//!            if key in value_table and dominates(value_table[key], instr):
//!                # Redundant instruction found
//!                replace_all_uses(instr, value_table[key])
//!                remove_instruction(instr)
//!            else:
//!                # New unique expression
//!                value_table[key] ← instr
//!
//!    return F

use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    graph::{DomRelation, Graph},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::collections::BTreeMap;

/// Macro to match enum variants with or without payloads.
macro_rules! match_operand {
    // Match variant without payload
    ($e:expr, $enum:ident:: $variant:ident) => {
        matches!($e, $enum::$variant)
    };
    // Match variant with payload (ignore inner data)
    ($e:expr, $enum:ident:: $variant:ident(..)) => {
        matches!($e, $enum::$variant(..))
    };
}


#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum ExpOp {
    Var(TempIndex),
    Op(Operation),
}

/// Structure to represent the key of an expression
/// - We only consider expressions that are temps or `Call(_, Operation, args)` instructions.
/// - The `args` are recursively represented as ExprKey.
///   - If an arg is a temp and the temp has a known ExprKey, we use that ExprKey. Otherwise, we wrap it as `ExpOp::Var(temp)`.
#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct ExprKey {
    op: ExpOp,
    args: Vec<ExprKey>,
}

impl ExprKey {
    pub fn new(op: ExpOp, args: Vec<ExprKey>) -> Self {
        Self { op, args }
    }

    pub fn should_eliminate(&self, target: &FunctionTarget) -> bool {
        let data: Vec<Vec<fn(&Operation) -> bool>> = vec![
            // vec![
            //    |op: &Operation| match_operand!(op, Operation::ReadRef),
            //    |op: &Operation| match_operand!(op, Operation::BorrowField(..)),
            //    |op: &Operation| match_operand!(op, Operation::BorrowLoc),
            // ],
            vec![
                |op: &Operation| match_operand!(op, Operation::BorrowGlobal(..)),
            ]
        ];
        data.iter().any(|prefix| self.has_op_prefix(prefix, target))
    }

    /// Checks if the sequence of Operation in this ExprKey matches the given prefix,
    /// considering all args at each level.
    pub fn has_op_prefix(
        &self,
        prefix: &[fn(&Operation) -> bool],
        target: &FunctionTarget,
    ) -> bool {
        fn helper(
            expr: &ExprKey,
            prefix: &[fn(&Operation) -> bool],
            idx: usize,
            target: &FunctionTarget,
        ) -> bool {
            if idx >= prefix.len() {
                return true;
            }
            match &expr.op {
                ExpOp::Op(cur_op) if prefix[idx](cur_op) => {
                    if idx + 1 == prefix.len() {
                        true
                    } else {
                        expr.args
                            .iter()
                            .any(|arg| helper(arg, prefix, idx + 1, target))
                    }
                },
                _ => false,
            }
        }
        helper(self, prefix, 0, target)
    }

    pub fn display(&self) -> String {
        match &self.op {
            ExpOp::Var(t) => format!("t{}", t),
            ExpOp::Op(op) => {
                let args_str = self
                    .args
                    .iter()
                    .map(|arg| arg.display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{:?}({})", op, args_str)
            },
        }
    }
}

pub struct CommonSubexpElimination {
    apply_filter: bool,
}

impl CommonSubexpElimination {
    pub fn new(apply_filter: bool) -> Self {
        Self { apply_filter }
    }

    fn transform(&self, target: &FunctionTarget) {
        // Build the control flow graph
        let code = target.get_bytecode();
        let forward_cfg = StacklessControlFlowGraph::new_forward(code);
        // Build the domination tree
        let graph = Graph::new(
            forward_cfg.entry_block(),
            forward_cfg.blocks(),
            forward_cfg.edges(),
        );
        let dom_relation = DomRelation::new(&graph);

        let mut tempid_to_exprkey = BTreeMap::<Vec<usize>, ExprKey>::new();
        let mut expr_table = BTreeMap::<ExprKey, (Vec<usize>, BlockId, Bytecode)>::new();

        let mut transform_bbl = |block_id: BlockId| {
            let bbl = &code[forward_cfg.code_range(block_id)];
            for inst in bbl {
                let Some((expr_key, dests)) = Self::canonicalize_expr(&tempid_to_exprkey, inst)
                else {
                    continue;
                };
                tempid_to_exprkey.insert(dests.clone(), expr_key.clone());
                if let Some((_, bbl_prev, prev_inst)) = expr_table.get(&expr_key) {
                    if dom_relation.is_dominated_by(block_id, *bbl_prev) {
                        println!(
                            "[Debug] inst {} at {} can be replaced by {} at {}",
                            inst.display(target, &BTreeMap::new()),
                            target
                                .data
                                .locations
                                .get(&inst.get_attr_id())
                                .expect("")
                                .display(target.global_env())
                                .to_string(),
                            prev_inst.display(target, &BTreeMap::new()),
                            target
                                .data
                                .locations
                                .get(&prev_inst.get_attr_id())
                                .expect("")
                                .display(target.global_env())
                                .to_string(),
                        );
                        continue;
                    }
                }
                if !self.apply_filter || expr_key.should_eliminate(target) {
                    expr_table.insert(expr_key.clone(), (dests.clone(), block_id, inst.clone()));
                }
            }
        };

        // traverse the domination tree in preorder
        for block_id in dom_relation.traverse_preorder() {
            transform_bbl(block_id);
        }
    }

    fn canonicalize_expr(
        tempid_to_exprkey: &BTreeMap<Vec<usize>, ExprKey>,
        inst: &Bytecode,
    ) -> Option<(ExprKey, Vec<usize>)> {
        if !inst.is_pure() && !matches!(inst, Bytecode::Call(_, _, Operation::BorrowGlobal(..), _, _)) {
            return None;
        }
        match inst {
            Bytecode::Call(_, dests, ops, args, _) => {
                let arg_vec = args
                    .iter()
                    .map(|n| {
                        tempid_to_exprkey
                            .get(&vec![*n])
                            .cloned()
                            .unwrap_or(ExprKey {
                                op: ExpOp::Var(*n),
                                args: vec![],
                            })
                    })
                    .collect::<Vec<_>>();

                let inst_key = ExprKey::new(ExpOp::Op(ops.clone()), arg_vec.clone());
                Some((inst_key, dests.clone()))
            },
            _ => None,
        }
    }
}

impl FunctionTargetProcessor for CommonSubexpElimination {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() || !func_env.module_env.is_target() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        self.transform(&target);
        data
    }

    fn name(&self) -> String {
        "CommonSubexpElimination".to_string()
    }
}
