// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "common subexpression elimination" (CSE) transformation,
//! inspired by the Global Value Numbering (GVN) algorithms used by LLVM:
//! - https://github.com/llvm/llvm-project/blob/main/llvm/lib/Transforms/Scalar/GVN.cpp
//!
//! Prerequisites:
//! - Variable liveness information is available
//! - Reaching definition information is available
//!
//! Motivating Example:
//! ```Move
//! 1. fun test(data: S, a: u64, b: u64): u64 {
//! 2.       if (data.x != 0) {
//! 3.           a / data.x
//! 4.       } else {
//! 5.           data.x + 1
//! 6.       }
//! 7.   }
//! ```
//! At the stackless bytecode level, `data.x` is translated into a seq of `BorrowLoc` + `BorrowField` + `ReadRef` instructions.
//! *Without* CSE, all occurance of `data.x` (line 2, line 3, line 5) will be translated into the seq above, despite `data.x` at line 3 and
//! line 5 share the same result of line 2 and the computations are not necessary.
//!
//! CSE aims to eliminate such redundant computations by reusing the result of previous computations.
//! Specifically, in the example above, assuming the `BorrowLoc` + `BorrowField` + `ReadRef` sequence at line 2 is assigned to temp `t1`,
//! then the occurrences at line 3 and line 5 can both be replaced by `t1`, eliminating the redundant computations.
//! The optimized bytecode would look like:
//!
//!  0: $t6 := borrow_local($t0)
//!  1: $t7 := borrow_field<0x8675::M::S>.x($t6)
//!  2: $t5 := read_ref($t7) // `data.x` at line 2 assigned to $t5
//!  3: $t8 := 0
//!  4: $t4 := !=($t5, $t8)
//!  5: if ($t4) goto 6 else goto 11
//!  6: label L0
//!  7: $t9 := move($t1)
//!  8: $t3 := /($t9, $t5) // line 3 reuses $t5
//!  9: label L2
//! 10: return $t3
//! 11: label L1
//! 12: $t16 := 1
//! 13: $t3 := +($t5, $t16) // line 5 reuses $t5
//! 14: goto 9
//!
//!
//! Implementation Details:
//! Step 1: Build the Control Flow Graph (CFG) and Domination Tree of a target function.
//!
//! Step 2: Traverse the Domination Tree in preorder, and for each basic block, for each instruction:
//! - If the instruction is *PURE*, canonicalize the expression represented by the instruction into an `ExprKey` structure
//!   - `ExprKey` contains the operation and its arguments, represented as `ExpArg`,
//!   - `ExpArg` can be either a constant, a variable (temp) or another `ExprKey` to nest expressions recursively
//!      - Motivation to nest expression: consider the expression `ReadRef(BorrowField(BorrowLoc(x)))`, we want to
//!        represent it as a single expression rather than three separate ones, so that we can eliminate
//!        the entire expression at once.
//!      - Conditions to nest `t1 = Op1(t0); t2 = Op2(t1);` as `Op2(Op1(t0))`:
//!         - The definition at `Op1` is the only definition of of `t1` that can reach the instruction of `Op2`
//!         - `t1` is only used once and exactly by `Op2`.
//!      - For commutative operations, the arguments are sorted to get a canonical order
//! - Why pre-order traversal: ensure that all dominating blocks have been processed before the dominated ones,
//!   hencing not missing opportunities for replacement
//!
//! Step 3: Check if the `ExprKey` from Step 2 has been seen before in a dominating block.
//! Given a candidate replacement `ExprKey` (annotated as `src_expr`) for the current expression (annotated as `dest_expr`)
//!
//! Assuming the two expressions have the following formats:
//! - `src_expr`: `(src_temp1, src_temp2, ...) = src_op(src_ope1, src_ope2, ...)` defined at `src_inst`, where `src_ope1` and `src_ope2` can be nested expressions.
//! - `dest_expr`: `(dest_temp1, dest_temp2, ...) = dest_op(dest_ope1, dest_ope2, ...)` defined at `dest_inst`, where `dest_ope1` and `dest_ope2` can be nested expressions.
//!
//! we take a set of conservative conditions to check safety of the replacement:
//! - Condition 1. `src_inst` dominates `dest_inst`
//! - Condition 2. All temps in `src_temps` are copyable and dropable
//! - Condition 3: `src_temps` and `dest_temps` share the same types
//! - Condition 4: Operands used in `src_inst` are safe to reuse at `dest_inst`:
//!   - None of the operands used in `src_inst` are references
//!   - None of the operands used in `src_inst` are possibly re-defined in a path between `src_inst` and `dest_inst` (without going through `src_inst` again)
//!
//! Step 4: for each `src_expr` passing the conditions to replace `dest_expr` in Step 3, we gather the replacement information:
//!
//! For each `dest_temp` (temporaries defined by `dest_expr`), we visit *EACH* of its uses, annotated as `use(dest_temp)`,
//!  and check if `use(dest_temp)` can be replaced by `use(src_temp)` (`src_temp` is the counterpart of `dest_temp` defined at `src_inst`):
//!  - Condition 1: `dest_temp` at `dest_expr` is the only definition of `dest_temp` that can reach `use(dest_temp)`
//!  - Condition 2: the definition at `src_expr` is the only one of `src_temp` that can reach `use(dest_temp)`
//!  - Condition 3: reference safety [see detailed comments in function `collect_replace_info()`]
//! - If *ALL* uses of `dest_temp` pass the checks above, we can safely replace `use(dest_temp)` by `use(src_temp)`
//!
//! If *ALL* `dest_temps` defined at `dest_expr` can be safely replaced as above, we record the replacement information.
//! We also mark `dest_expr` and all its nested expressions for elimination.
//!
//! Step 5: After processing all basic blocks, we perform the recorded replacements and eliminate the marked code.
//!
//! The algorithm above is designed to handle PURE instructions, defined as blow
//! - the results only depend on the operands
//! - has no side effects on `memory` (including write via references), control flow (including `abort`), or external state (global storage)
//! - recomputing it multiple times has no semantic effect.
//!
//! For better optimizations, we also introduce the `aggressive` mode to support special, non-pure instructions.
//!
//! Case 1: operations that are pure if no arithmetic errors like overflows happen (+, -, *, /, %, etc):
//! - such operations are dealt as pure in `aggressive` mode
//! - their side effects are safe as, if those happen, they are guaranteed to happen in the `src_inst`
//!   due to the domination and no-redefinition of operands requirement
//!
//! Case 2: the sequence of `borrowloc` + `borrowfield` + `readref`
//! - In principle, `readref` is not pure as it depends on memory states.
//! - However, when appearing in the above sequence, its effects only depend on the operands of `borrowloc`
//!   Thus, we treat the entire sequence as pure in `aggressive` mode.
//! - Safety: as we guarantee that the operands to `borrowloc` are not changed between `src_inst` and `dst_inst`,
//!   we rule out potential side effects.
//!
//! To add support for other instructions, please extend `BytecodeSanitizer` to enable support and extend the checks accordingly.

use crate::pipeline::{
    livevar_analysis_processor::LiveVarAnnotation,
    reaching_def_analysis_processor::{Def, ReachingDefAnnotation},
};
use im::ordset::OrdSet;
use log::info;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    graph::{DomRelation, Graph},
    stackless_bytecode::{AbortAction, AssignKind, Bytecode, Constant, Operation},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fmt::Formatter,
    vec,
};

/// Enum to represent the operation of an expression; We only consider `Call(_, Operation, args)` instructions at present.
#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum ExpOp {
    Op(Operation),
    Load,
    Assign(AssignKind),
}

/// Canonicalized representation of an expression argument
#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum ExpArg {
    Const(Constant),
    Var(TempIndex),
    Expr(Box<ExprKey>),
}

impl ExpArg {
    pub fn display<'env>(
        &'env self,
        func_target: &'env FunctionTarget<'env>,
        verbose: bool,
    ) -> ExprArgDisplay<'env> {
        ExprArgDisplay {
            expr_arg: self,
            func_target,
            verbose,
        }
    }
}

/// A display object for a bytecode.
pub struct ExprArgDisplay<'env> {
    expr_arg: &'env ExpArg,
    func_target: &'env FunctionTarget<'env>,
    verbose: bool,
}

impl fmt::Display for ExprArgDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self.expr_arg {
            ExpArg::Const(c) => format!("{}", c),
            ExpArg::Var(idx) => format!("t{}", idx),
            ExpArg::Expr(expr) => format!("{}", expr.display(self.func_target, self.verbose)),
        };
        write!(f, "{}", str)?;
        Ok(())
    }
}

/// Canonicalized representation of an expression
/// - `op`: the operation of the expression
/// - `args`: the arguments of the expression, recursively represented as ExprKey when applicable
/// - `temps`: the temps defined by this expression
/// - `temp_tys`: the types of the temps defined by this expression
/// - `offset`: the code offset of this expression
/// The data structure is used as both a key and a value in BTreeMap:
/// - When used as a key, only `op` and `args` are used to represent the canonicalized expression, so that common subexpressions can be identified
/// - When used as a value, the full `ExprKey` (including `temps` and `offset`) is used to represent the specific expression instance
///
/// Consider the following example:
/// ```Move
///  1. t1 = pure_computation_1(t0)
///  2. t2 = pure_computation_1(t0)
/// ```
/// Here, the `ExprKey` for line 1 is `ExprKey {op = pure_computation_1, args = [Var(t0)], temps = [t1], offset = 1}`,
/// and the `ExprKey` for line 2 is `ExprKey {op = pure_computation_1, args = [Var(t0)], temps = [t2], offset = 2}`.
/// When their `ExprKey`s are used as keys in a BTreeMap, only `op` and `args` are considered, so they are treated as the same key.
/// When used as values, the full `ExprKey`s are retained to distinguish between the two instances.
///
#[derive(Clone, Debug)]
pub struct ExprKey {
    op: ExpOp,
    args: Vec<ExpArg>,
    temps: Vec<TempIndex>,
    offset: CodeOffset,
}

impl PartialEq for ExprKey {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op && self.args == other.args
    }
}

impl Eq for ExprKey {}

impl PartialOrd for ExprKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExprKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare only the fields that constitute the key, in the desired order
        self.op
            .cmp(&other.op)
            .then_with(|| self.args.cmp(&other.args))
    }
}

impl ExprKey {
    pub fn new(op: ExpOp, args: Vec<ExpArg>, temps: Vec<TempIndex>, offset: CodeOffset) -> Self {
        Self {
            op,
            args,
            temps,
            offset,
        }
    }

    /// Collect all operands (temps) used in this expression
    pub fn collect_operands(&self) -> Vec<(TempIndex, CodeOffset)> {
        let mut operands = Vec::new();
        for arg in self.args.iter() {
            match arg {
                ExpArg::Var(temp) => operands.push((*temp, self.offset)),
                ExpArg::Expr(boxed_expr) => {
                    operands.extend(boxed_expr.collect_operands().into_iter());
                },
                ExpArg::Const(_) => { /* do nothing */ },
            }
        }
        operands
    }

    pub fn collect_exps(&self) -> Vec<CodeOffset> {
        let mut exps = vec![self.offset];
        for arg in self.args.iter() {
            if let ExpArg::Expr(arg_expr) = arg {
                exps.extend(arg_expr.collect_exps().into_iter());
            }
        }
        exps
    }

    /// Creates a format object for a bytecode in context of a function target.
    pub fn display<'env>(
        &'env self,
        func_target: &'env FunctionTarget<'env>,
        verbose: bool,
    ) -> ExprKeyDisplay<'env> {
        ExprKeyDisplay {
            expr_key: self,
            func_target,
            verbose,
        }
    }
}

/// A display object for a bytecode.
pub struct ExprKeyDisplay<'env> {
    expr_key: &'env ExprKey,
    func_target: &'env FunctionTarget<'env>,
    verbose: bool,
}

impl fmt::Display for ExprKeyDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let args_str = self
            .expr_key
            .args
            .iter()
            .map(|arg| format!("{}", arg.display(self.func_target, false)))
            .collect::<Vec<_>>()
            .join(", ");

        if self.verbose {
            write!(f, "`{}()` @ ", self.func_target.func_env.get_full_name_str())?;

            let loc = self
                .func_target
                .get_bytecode_loc_at_offset(self.expr_key.offset);
            let file_name = self
                .func_target
                .global_env()
                .get_file(loc.file_id())
                .to_string_lossy()
                .to_string();
            let file_loc = self.func_target.global_env().get_location(&loc);

            if let Some(file_loc) = file_loc {
                write!(f, "{}: {},{}: ", file_name, file_loc.line, file_loc.column)?;
            } else {
                write!(f, "{}: <unknown>: ", file_name)?;
            }
        }

        write!(f, "{}", "[")?;
        match &self.expr_key.op {
            ExpOp::Load => write!(f, "load({})", args_str)?,
            ExpOp::Assign(kind) => write!(f, "assign[{:?}]({})", kind, args_str)?,
            ExpOp::Op(op) => write!(f, "{}({})", op.display(self.func_target), args_str)?,
        };
        write!(f, "{}", "]")?;
        Ok(())
    }
}

/// The processor to perform Common Subexpression Elimination (CSE)
pub struct CommonSubexpElimination {
    aggressive_mode: bool,
}

impl CommonSubexpElimination {
    pub fn new(aggressive_mode: bool) -> Self {
        Self { aggressive_mode }
    }
}

/// Implements the CSE transformation as a FunctionTargetProcessor
impl FunctionTargetProcessor for CommonSubexpElimination {
    /// Entry point
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
        // CSE depends on variable liveness analysis and reaching definition analysis!!!
        let (Some(live_var_annotation), Some(reach_def_annotation)) = (
            target.get_annotations().get::<LiveVarAnnotation>(),
            target.get_annotations().get::<ReachingDefAnnotation>(),
        ) else {
            return data;
        };
        let analyzer = CSEAnalyzer::new(
            target,
            live_var_annotation,
            reach_def_annotation,
            self.aggressive_mode,
        );
        let new_code = analyzer.transform();
        data.code = new_code;
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "CommonSubexpElimination".to_string()
    }
}

struct CSEAnalyzer<'env> {
    target: FunctionTarget<'env>,
    live_var_annotation: &'env LiveVarAnnotation,
    reach_def_annotation: &'env ReachingDefAnnotation,
    aggressive_mode: bool,
}

impl CSEAnalyzer<'_> {
    fn new<'env>(
        target: FunctionTarget<'env>,
        live_var_annotation: &'env LiveVarAnnotation,
        reach_def_annotation: &'env ReachingDefAnnotation,
        aggressive_mode: bool,
    ) -> CSEAnalyzer<'env> {
        CSEAnalyzer {
            target,
            live_var_annotation,
            reach_def_annotation,
            aggressive_mode,
        }
    }

    fn transform(&self) -> Vec<Bytecode> {
        // Step 1: Build the control flow graph and the domination tree
        let code = self.target.get_bytecode();
        let forward_cfg = StacklessControlFlowGraph::new_forward(code);
        let graph = Graph::new(
            forward_cfg.entry_block(),
            forward_cfg.blocks(),
            forward_cfg.edges(),
        );
        let dom_relation = DomRelation::new(&graph);

        // Maps from temps to the vector of expressions that define them
        // - Why using a vector: we are not SSA, so a temp may be re-defined multiple times
        // - Here `ExprKey` is used as the value to capture the full expression info (including `temps` and `offset`)
        let mut tempid_to_exprkey = BTreeMap::<Vec<TempIndex>, Vec<ExprKey>>::new();

        // Maps from key to the vector of expressions that share the key
        // - Why using a vector: different expressions may share the same key but define different temps
        // - Here `ExprKey` is used as both the key and value
        //   - When used as key, only `op` and `args` are used (see the customized `PartialEq` and `Ord` implementations) to represent the canonicalized expression
        //   - When used as value, the full `ExprKey` (including `temps` and `offset`) is used to represent the specific expression instance
        let mut expr_table = BTreeMap::<Box<ExprKey>, Vec<ExprKey>>::new();

        // Maps recording the replacements to be made: (code offset: dst_temp) -> src_temp,
        // meaning that `dst_temp` defined at `code offset` can be replaced by `src_temp`
        let mut expr_replacements = BTreeMap::new();
        // Set of code to be eliminated
        let mut eliminate_code = BTreeSet::new();

        // helper to transform a basic block
        let mut transform_bbl = |block_id: BlockId| {
            let bbl_range = forward_cfg.code_range(block_id);
            let bbl = &code[bbl_range.clone()];
            for (offset, inst) in bbl_range.zip(bbl) {
                // Step 2: get a canonicalized representation of the current expression
                let Some(expr_key) =
                    self.canonicalize_expr(&tempid_to_exprkey, inst, offset as CodeOffset)
                else {
                    continue;
                };

                // cache the mapping from defined temps to `ExprKey`
                tempid_to_exprkey
                    .entry(expr_key.temps.clone())
                    .or_default()
                    .push(expr_key.clone());

                // Step 3: get the most recent expression that share the same key and qualifies for replacement
                if let Some(src_expr) = self.get_qualified_replacement(
                    &expr_key,
                    &forward_cfg,
                    &dom_relation,
                    &expr_table,
                ) {
                    // Step 4: record the replacement info
                    self.collect_replace_info(
                        &src_expr,
                        &expr_key,
                        &mut expr_replacements,
                        &mut eliminate_code,
                    );
                    continue;
                }
                // if not replaced, record the `ExprKey` for checking future re-occurrences
                expr_table
                    .entry(Box::new(expr_key.clone()))
                    .or_default()
                    .push(expr_key);
            }
        };

        // Traverse the domination tree in preorder
        // Why preorder? we need to ensure that dominators are processed before their dominated blocks
        //  so that we do not miss opportunities for replacement
        for block_id in dom_relation.traverse_preorder() {
            transform_bbl(block_id);
        }

        // Step 5: perform the replacements and eliminate the marked code
        self.perform_replacement(&mut expr_replacements, &mut eliminate_code)
    }

    /// Create a canonicalized representation (`ExprKey`), for the expression `inst` at `offset`.
    /// - `ExprKey` contains the operation represented as `ExpOp` and its arguments represented as `ExpArg`,
    /// - `ExpArg` can be either a variable (temp) or another `ExprKey` to nest expressions recursively
    /// - Why and when to nest expressions: see the doc comments at the beginning of this file.
    /// - For commutative operations, the arguments are sorted to get a canonical order
    fn canonicalize_expr(
        &self,
        tempid_to_exprkey: &BTreeMap<Vec<TempIndex>, Vec<ExprKey>>,
        inst: &Bytecode,
        offset: CodeOffset,
    ) -> Option<ExprKey> {
        // Check if the bytecode is allowed to consider
        // - see BytecodeSanitizer for details
        let bytecode_sanitizer = BytecodeSanitizer::new_from_bytecode(inst);
        if !bytecode_sanitizer.is_allowed(self.aggressive_mode) {
            return None;
        }

        // Helper to check if `src_temp` defined at `src_inst` is used as an immediate temp at `dest_inst`
        // condition 1: `src_inst` is the only definition of `src_temp` that can reach `dest_inst`;
        // - this ensures that the value of `src_temp` at `dest_inst` is exactly the one defined at `src_inst`
        // condition 2: the usage is single and exactly at `dest_inst`
        // - this ensures that, when nesting `src_inst` into `dest_inst` and consequently removing `src_temp`, no other uses of `src_temp` are affected
        let used_as_imm = |src_inst, src_temp, dest_inst| {
            self.single_def_reach(src_inst, src_temp, dest_inst)
                && self.single_use_at(src_inst, src_temp, dest_inst)
        };

        // Helper to find the most recent expression that defines `arg`, which is used as an immediate temp at `offset`
        let find_recent_expr = |arg: TempIndex, offset: CodeOffset| {
            tempid_to_exprkey.get(&vec![arg]).and_then(|expr_keys| {
                expr_keys
                    .iter()
                    .rev()
                    .find(|arg_key| used_as_imm(arg_key.offset, arg, offset))
                    .map(|arg_key| ExpArg::Expr(Box::new(arg_key.clone())))
            })
        };

        let res = match inst {
            Bytecode::Load(_, dest, constant) => {
                // Nothing can go wrong with `Load` (?)
                Some(ExprKey::new(
                    ExpOp::Load,
                    vec![ExpArg::Const(constant.clone())],
                    vec![*dest],
                    offset,
                ))
            },

            Bytecode::Assign(_, dest, src, kind) => {
                let mut arg_vec = vec![];
                if let Some(expr_arg) = find_recent_expr(*src, offset) {
                    arg_vec.push(expr_arg);
                } else {
                    arg_vec.push(ExpArg::Var(*src));
                }

                Some(ExprKey::new(
                    ExpOp::Assign(*kind),
                    arg_vec,
                    vec![*dest],
                    offset,
                ))
            },

            Bytecode::Call(_, dests, ops, args, _) => {
                // replace arguments that are used as immediate temps with their canonicalized expressions
                let mut arg_vec = Vec::new();
                for arg in args.iter() {
                    if let Some(expr_arg) = find_recent_expr(*arg, offset) {
                        arg_vec.push(expr_arg);
                    } else {
                        arg_vec.push(ExpArg::Var(*arg));
                    }
                }

                // if the operation is commutative, sort the arguments to get a canonical order
                if ops.is_commutative() {
                    arg_vec.sort();
                }

                Some(ExprKey::new(
                    ExpOp::Op(ops.clone()),
                    arg_vec,
                    dests.clone(),
                    offset,
                ))
            },

            // these do not use temps
            Bytecode::Branch(..)
            | Bytecode::Ret(..)
            | Bytecode::Abort(..)
            | Bytecode::Jump(..)
            | Bytecode::Label(..)
            | Bytecode::Nop(..)
            | Bytecode::SaveMem(..)
            | Bytecode::SaveSpecVar(..)
            | Bytecode::SpecBlock(..)
            | Bytecode::Prop(..) => None,
        };

        // finally check if the generated `ExprKey` is safe to use
        res.filter(|expr_key| bytecode_sanitizer.sanitize(expr_key, &self.target))
    }

    /// Get a qualified replacement for the expression represented by `expr_key`
    fn get_qualified_replacement(
        &self,
        target_expr: &ExprKey,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
        expr_table: &BTreeMap<Box<ExprKey>, Vec<ExprKey>>,
    ) -> Option<ExprKey> {
        // check all previous occurrences of the same expression and return the first qualified one
        if let Some(src_exprs) = expr_table.get(target_expr) {
            for src_expr in src_exprs.iter().rev() {
                if self.is_qualified_replacement(cfg, dom_relation, src_expr, target_expr) {
                    return Some(src_expr.clone());
                }
            }
        }
        None
    }

    /// Check if `src_temps` defined at `src_inst` can be used to replace the expressions defined at `dest_inst`
    /// Context:
    /// - `src_temps` = `src_inst`
    /// - `src_inst` can be nested, namely `src_inst` = `op(inner_op1(operand1), inner_op2(operand2), ...)`,
    ///    where `operand1` and `operand2` might be further nested, and if not, they are called operands of `src_inst`
    /// - `dest_temps` = `dest_inst`
    /// - `dest_inst` can be similarly nested as `src_inst`
    ///
    /// We take a set of conservative conditions to ensure the safety of the replacement:
    /// Condition 1. `src_inst` dominates `dest_inst`
    /// - This ensures that `src_inst` is always executed before `dest_inst`
    /// Condition 2. All temps in `src_temps` are copyable and dropable
    /// - This ensures that reusing the temps does not violate ability constraints
    /// Condition 3: `src_temps` and `dest_temps` share the same types
    /// - This ensures type safety when replacing `dest_temps` with `src_temps`
    /// Condition 4: Operands used in `src_inst` are safe to reuse at `dest_inst`:
    /// - None of the operands used in `src_inst` are references
    ///   - given a reference operand, we may have an opaque understanding of its memory underneath (e.g., the reference is a parameter),
    ///     preventing us from understanding definitions to that piece of memory, which is critical in our aggressive mode
    /// - None of the operands used in `src_inst` are possibly re-defined in a path between `src_inst` and `dest_inst` (without going through `src_inst` again)
    ///   - This ensures that the values of the operands used in `src_inst` remain unchanged when reaching `dest_inst`
    ///
    fn is_qualified_replacement(
        &self,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
        src_expr: &ExprKey,
        dest_expr: &ExprKey,
    ) -> bool {
        // helper to check if `src_inst` dominates `dest_inst`
        let src_dominate_dst = |src, dst| {
            let src_bbl = cfg.enclosing_block(src);
            let dst_bbl = cfg.enclosing_block(dst);
            if src_bbl == dst_bbl {
                // same block, check offset
                src < dst
            } else {
                // different blocks, check domination
                dom_relation.is_dominated_by(dst_bbl, src_bbl)
            }
        };

        // helper to check if source temps and destination temps share the same type
        let src_dest_same_type = |src_temps: &Vec<TempIndex>, dst_temps: &Vec<TempIndex>| {
            src_temps
                .iter()
                .zip(dst_temps.iter())
                .all(|(src_temp, dest_temp)| {
                    self.target.get_local_type(*src_temp) == self.target.get_local_type(*dest_temp)
                })
        };

        // helper to check if all srcs are copyable and dropable
        let srcs_copyable_dropable = |srcs: &Vec<TempIndex>| {
            srcs.iter().all(|&temp| {
                let ty = self.target.get_local_type(temp);
                let abilities = self
                    .target
                    .global_env()
                    .type_abilities(ty, &self.target.get_type_parameters());
                abilities.has_copy() && abilities.has_drop()
            })
        };

        // helper to check if all operands are safe to use:
        // - Condition 1: operands deriving the src are identical to those deriving the dst
        // - Condition 2: none of the operands is reference
        // - Condition 3: none of the operands is possibly re-defined between `src_inst` and `dst_inst`
        let operands_safe_to_reuse =
            |src_operands: &[(_, CodeOffset)], dst_operands: &[(_, CodeOffset)]| {
                src_operands.iter().zip(dst_operands).all(
                    |((src_operand, src_offset), (dst_operand, dst_offset))| {
                        src_operand == dst_operand
                            && !self.target.get_local_type(*src_operand).is_reference()
                            && !Self::temp_killed_between(
                                self,
                                *src_operand,
                                *src_offset,
                                *dst_offset,
                                cfg,
                            )
                    },
                )
            };

        let perf_estimator = PerfEstimator::new(&self.target, self.live_var_annotation);

        src_dominate_dst(src_expr.offset, dest_expr.offset)
            && src_dest_same_type(&src_expr.temps, &dest_expr.temps)
            && srcs_copyable_dropable(&src_expr.temps)
            && operands_safe_to_reuse(&src_expr.collect_operands(), &dest_expr.collect_operands())
            && perf_estimator.gain_perf(src_expr, dest_expr)
    }

    /// Collects the information for the expression defined at `dest_expr` to be replaced by `src_expr`.
    ///
    /// Example:
    /// ``` Move
    /// t0 = src_expr()
    /// ...
    /// t1 = dest_expr()
    /// t2 = use(t1)
    /// if (c) {
    ///  t1 = other_expr()
    /// }
    /// t3 = use(t1)
    /// ```
    /// Here, we aim to replace `t1` defined at `dest_expr` with `t0` defined at `src_expr`.
    /// Step 1: For each temp defined by `dest_expr` (`t1`), we collect its usages (`t2 = use(t1)` and `t3 = use(t1)`).
    /// Step 2: For each usage, we check the following conditions
    ///  - Condition 1: the definition of `t1` at `dest_expr` is the only one that can reach the usage
    ///  - Condition 2: the definition of `t0` at `src_expr` is the only one that can reach the usage
    ///  - Condition 3: the replacement does not introduce reference safety issues
    /// If *ALL* usages pass the check, we can safely replace the usages of `t1` by `t0`.
    /// - In the example above, `t3 = use(t1)` does not meet Condition 1, as `t1` has an additional definition by `t1 = other_expr()` that can reach this use.
    ///   Thus, in this case, we cannot perform the replacement.
    ///Step 3: We also need to collect the code to be eliminated: including `dest_expr` and all its nested expressions.
    fn collect_replace_info(
        &self,
        src_expr: &ExprKey,
        dest_expr: &ExprKey,
        expr_replacements: &mut BTreeMap<(CodeOffset, TempIndex), TempIndex>,
        eliminate_code: &mut BTreeSet<CodeOffset>,
    ) {
        // get the temps defined by src_expr (to replace) and dest_expr (be replaced)
        let src_temps = &src_expr.temps;
        let dest_temps = &dest_expr.temps;

        // temporary map to collect local replacement info
        let mut local_expr_replacement = BTreeMap::new();

        // helper to get the usage info of a temp at a given offset
        let usage_of =
            |temp: &TempIndex, offset| self.live_var_annotation.get_info_at(offset).after.get(temp);

        for (src_temp, dest_temp) in src_temps.iter().zip(dest_temps.iter()) {
            // get after usage info of `dst_temp` (a temp defined at `dest_expr.offset`)
            let Some(usage) = usage_of(dest_temp, dest_expr.offset) else {
                // no usage; a bit wired but safe to ignore
                continue;
            };

            // helper to check reference safety of reusing `src_temp` at `use_site`
            // - `src_temp` is not a reference
            // - `src_temp` is not used as a reference at the `use_site`
            // - no existing usage of `src_temp` after `src_site` is a reference
            // TODO: the rules above are crazily restrictive; make them more permissive later to allow better optimizations
            let ref_safety = |src_temp, src_site, use_site, code: &[Bytecode]| {
                let src_ty = self.target.get_local_type(src_temp);
                !src_ty.is_reference()
                    && !code[use_site as usize].is_borrowing()
                    && usage_of(&src_temp, src_site).is_none_or(|existing_uses| {
                        existing_uses
                            .usage_offsets()
                            .iter()
                            .all(|existing_use| !code[*existing_use as usize].is_borrowing())
                    })
            };

            for use_offset in usage.usage_offsets() {
                if self.single_def_reach(dest_expr.offset, *dest_temp, use_offset)
                    && self.single_def_reach(src_expr.offset, *src_temp, use_offset)
                    && ref_safety(
                        *src_temp,
                        src_expr.offset,
                        use_offset,
                        self.target.get_bytecode(),
                    )
                {
                    local_expr_replacement.insert((use_offset, *dest_temp), *src_temp);
                } else {
                    return;
                }
            }
        }

        if !local_expr_replacement.is_empty() {
            info!(
                "CSE: replacing \n \t {} \n ===> \n \t {}\n",
                dest_expr.display(&self.target, true),
                src_expr.display(&self.target, true),
            );
        }

        // It's prove that the replacement is safe for all temps defined at `dest_expr`
        // so let's finalize the replacement info
        expr_replacements.extend(local_expr_replacement);

        // We also need to collect the code to be eliminated: including `dest_expr` and all its nested expressions
        eliminate_code.insert(dest_expr.offset);
        let mut stack = dest_expr.args.clone();
        while let Some(arg) = stack.pop() {
            if let ExpArg::Expr(boxed_expr) = arg {
                eliminate_code.insert(boxed_expr.offset);
                stack.extend(boxed_expr.args.iter().cloned());
            }
        }
    }

    /// Perform the actual replacement in the bytecode
    fn perform_replacement(
        &self,
        expr_replacements: &mut BTreeMap<(CodeOffset, TempIndex), TempIndex>,
        eliminate_code: &mut BTreeSet<CodeOffset>,
    ) -> Vec<Bytecode> {
        let mut new_code = Vec::new();
        for (offset, inst) in self.target.get_bytecode().iter().enumerate() {
            let code_offset = offset as CodeOffset;
            // Skip expressions marked for elimination
            // This helps ensure we always replace the deeply nested expressions
            // Consider the following example
            // ```Move
            // t0 = pure_compute_1
            // t1 = pure_compute_2(t0)
            // t2 = pure_compute_1
            // t3 = pure_compute_2(t2)
            //```
            // Here, `t0` and `t2` are represented as `ExprKey {op = pure_compute_1, args = []}`,
            // and `t1` and `t3` are represented as nested `ExprKey {op = pure_compute_2, args = [ExprKey {op = pure_compute_1, args = []}]}`.
            // Our algorithm will mark that `t2` can be replaced by `t0`, and `t3` can be replaced by `t1`; Also both `t1` and `t3` are marked for elimination.
            // Here replacement of t2 will not happen, because the use of t2, which is `t3 = pure_compute_2(t2)`, is marked for elimination.
            // - Further, as we guaranteed that `t3 = pure_compute_2(t2)` is the only use of `t2` when nesting the expressions, no other uses of `t2` will be affected.

            // if an inst should be removed, we skip it.
            // We also remove it from the record so that we can check if all insts are removed correctly at the end.
            // Further, we remove the inst from the replacement targets.
            if eliminate_code.remove(&code_offset) {
                // also remove all entries in the replacement map related to this offset
                expr_replacements.retain(|(offset, _), _| *offset != code_offset);
                continue;
            }

            // helper to replace a temp if needed
            let get_new_temp =
                |temp, offset| *expr_replacements.get(&(offset, temp)).unwrap_or(&temp);

            // perform replacements as needed
            match inst {
                Bytecode::Call(id, dests, ops, srcs, abort) => {
                    let new_srcs = srcs
                        .iter()
                        .map(|&src| get_new_temp(src, code_offset))
                        .collect::<Vec<_>>();

                    let new_abort = abort.as_ref().map(|abort_act| {
                        let new_abort_temp = get_new_temp(abort_act.1, code_offset);
                        let new_abort_label = abort_act.0;
                        AbortAction(new_abort_label, new_abort_temp)
                    });

                    new_code.push(Bytecode::Call(
                        *id,
                        dests.clone(),
                        ops.clone(),
                        new_srcs,
                        new_abort,
                    ));

                    // remove the used replacements so that we can check if all replacements happened!!!
                    for src in srcs {
                        expr_replacements.remove(&(code_offset, *src));
                    }
                    if let Some(abort_act) = abort {
                        expr_replacements.remove(&(code_offset, abort_act.1));
                    }
                },
                Bytecode::Abort(id, arg) => {
                    let new_arg = get_new_temp(*arg, code_offset);
                    new_code.push(Bytecode::Abort(*id, new_arg));
                    // remove the used replacement
                    expr_replacements.remove(&(code_offset, *arg));
                },
                Bytecode::Branch(id, branch1, branch2, cond) => {
                    let new_cond = get_new_temp(*cond, code_offset);
                    new_code.push(Bytecode::Branch(*id, *branch1, *branch2, new_cond));
                    // remove the used replacement
                    expr_replacements.remove(&(code_offset, *cond));
                },
                Bytecode::Assign(id, dst, src, kind) => {
                    let new_src = get_new_temp(*src, code_offset);
                    new_code.push(Bytecode::Assign(*id, *dst, new_src, *kind));
                    // remove the used replacement
                    expr_replacements.remove(&(code_offset, *src));
                },
                Bytecode::Ret(id, ret_vals) => {
                    let new_ret_vals = ret_vals
                        .iter()
                        .map(|&ret_val| get_new_temp(ret_val, code_offset))
                        .collect::<Vec<_>>();

                    new_code.push(Bytecode::Ret(*id, new_ret_vals));

                    // remove the used replacements so that we can check if all replacements happened!!!
                    for ret_val in ret_vals {
                        expr_replacements.remove(&(code_offset, *ret_val));
                    }
                },

                Bytecode::Load(..)
                | Bytecode::Jump(..)
                | Bytecode::Label(..)
                | Bytecode::Nop(..)
                | Bytecode::SaveMem(..)
                | Bytecode::SaveSpecVar(..)
                | Bytecode::SpecBlock(..)
                | Bytecode::Prop(..) => new_code.push(inst.clone()),
            }
        }
        assert!(
            expr_replacements
                .keys()
                .all(|(offset, _)| !eliminate_code.contains(offset)),
            "no replacements should be left for eliminated code"
        );
        assert!(
            expr_replacements.is_empty(),
            "all replacements must have been completed {:?}",
            expr_replacements
        );
        new_code
    }

    /// Checks if the definition of `src_temp` at `src_inst` is only used once and exactly at `dest_inst`.
    fn single_use_at(
        &self,
        src_inst: CodeOffset,
        src_temp: TempIndex,
        dest_inst: CodeOffset,
    ) -> bool {
        self.live_var_annotation
            .get_info_at(src_inst)
            .after
            .get(&src_temp)
            .is_some_and(|uses| uses.usage_offsets() == OrdSet::unit(dest_inst))
    }

    /// Checks if the definition of `src_temp` is the only definition of `src_inst`that can reach `dest_inst`.
    fn single_def_reach(
        &self,
        src_inst: CodeOffset,
        src_temp: TempIndex,
        dest_inst: CodeOffset,
    ) -> bool {
        self.reach_def_annotation
            .get_info_at(dest_inst)
            .map
            .get(&src_temp)
            .is_some_and(|defs| defs == &BTreeSet::from([Def::Loc(src_inst)]))
    }

    /// Checks if `temp` is possibly re-defined in a path between `src` and `dest` (without going through `src` again)
    fn temp_killed_between(
        &self,
        temp: TempIndex,
        src_inst: CodeOffset,
        dest_inst: CodeOffset,
        cfg: &StacklessControlFlowGraph,
    ) -> bool {
        // get all definitions of `temp` that can reach `dest`
        let Some(dest_defs) = self
            .reach_def_annotation
            .get_info_at(dest_inst)
            .map
            .get(&temp)
        else {
            // TODO: this would only happen if `temp` is a function parameter; so add a check later
            return false;
        };

        // helper to check if there is a path from `start` to `end` without going through `block`
        // if `block` is `start`, it means we are checking paths that do not go through `start` again
        let can_reach_without = |start, end, block| {
            let mut queue = vec![start];
            let mut visited = BTreeSet::new();
            visited.insert(start);

            while let Some(cur) = queue.pop() {
                if cur == end {
                    return true;
                }
                for succ in cfg.successor_insts(cur) {
                    if succ == block || visited.contains(&succ) {
                        continue;
                    }
                    visited.insert(succ);
                    queue.push(succ);
                }
            }
            false
        };

        // check if any definition can reach `dest` on a path starting at `src` but without going through `src` again
        // if so, it means `temp` is killed in between
        dest_defs.iter().any(|def| match def {
            // Logic here: `src_inst` is the start, `dest_inst` is the end, and `def_inst` is where `temp` is defined
            // If there is a path from `src_inst` to `def_inst` without going through `src_inst` again,
            // and there is a path from `def_inst` to `dest_inst` without going through `src_inst`,
            // it means there is a path from `src_inst` to `dest_inst` where `temp` is re-defined at `def_inst` without going through `src_inst` again
            Def::Loc(def_inst) => {
                can_reach_without(src_inst, *def_inst, src_inst)
                    && can_reach_without(*def_inst, dest_inst, src_inst)
            },
        })
    }
}

/// Data structure to help determine the qualification of instructions for CSE
enum BytecodeSanitizer {
    Pure,
    PureIfNoArithError,
    Forbidden,
    ReadRef,
    Assign,
}

/// Macro to match enum variants with or without inner data.
macro_rules! match_operand {
    ($e:expr, $enum:ident:: $variant:ident) => {
        matches!($e, $enum::$variant)
    };
    ($e:expr, $enum:ident:: $variant:ident(..)) => {
        matches!($e, $enum::$variant(..))
    };
}

impl BytecodeSanitizer {
    /// Create a BytecodeSanitizer from a bytecode instruction
    /// It can be extended to support other impure instructions
    pub fn new_from_bytecode(inst: &Bytecode) -> Self {
        if inst.is_pure() {
            BytecodeSanitizer::Pure
        } else if inst.pure_if_no_arith_error() {
            BytecodeSanitizer::PureIfNoArithError
        } else if matches!(inst, Bytecode::Call(_, _, Operation::ReadRef, _, _)) {
            BytecodeSanitizer::ReadRef
        } else if matches!(
            inst,
            Bytecode::Assign(_, _, _, AssignKind::Copy | AssignKind::Inferred)
        ) {
            BytecodeSanitizer::Assign
        } else {
            BytecodeSanitizer::Forbidden
        }
    }

    /// Check if the instruction is allowed to consider for CSE
    pub fn is_allowed(&self, aggressive_mode: bool) -> bool {
        match self {
            BytecodeSanitizer::Pure => true,
            BytecodeSanitizer::PureIfNoArithError => aggressive_mode,
            BytecodeSanitizer::ReadRef => aggressive_mode,
            BytecodeSanitizer::Assign => aggressive_mode,
            BytecodeSanitizer::Forbidden => false,
        }
    }

    /// Further sanitize the expression represented by `expr_key` in the context of `function_target`
    pub fn sanitize(&self, expr_key: &ExprKey, function_target: &FunctionTarget) -> bool {
        match self {
            BytecodeSanitizer::Pure => true,
            BytecodeSanitizer::PureIfNoArithError => true,
            BytecodeSanitizer::ReadRef => {
                // define postfix patterns
                // - each postfix is a list of functions that take an Operation and return true if it matches the desired operation.
                let postfixes: Vec<Vec<fn(&Operation) -> bool>> = vec![vec![
                    |op: &Operation| match_operand!(op, Operation::ReadRef),
                    |op: &Operation| match_operand!(op, Operation::BorrowField(..)),
                    |op: &Operation| match_operand!(op, Operation::BorrowLoc),
                ]];
                // only allow certain patterns
                Self::has_op_postfix(expr_key, function_target, &postfixes)
            },
            BytecodeSanitizer::Assign => true,
            BytecodeSanitizer::Forbidden => false,
        }
    }

    /// Checks if the expression, after expanding the args, matches certain postfix patterns.
    fn has_op_postfix(
        expr_key: &ExprKey,
        target: &FunctionTarget,
        postfixes: &[Vec<fn(&Operation) -> bool>],
    ) -> bool {
        // depth-first search to match the postfix
        fn dfs(
            expr: &ExprKey,
            postfix: &[fn(&Operation) -> bool],
            idx: usize,
            _target: &FunctionTarget,
        ) -> bool {
            if postfix.is_empty() {
                return true;
            }
            // Given an operation that matches the current postfix function,
            if matches!(&expr.op, ExpOp::Op(cur_op) if postfix[idx](cur_op)) {
                // all have been matched
                if idx + 1 >= postfix.len() {
                    return true;
                }
                // we continue checking its arguments
                return expr
                    .args
                    .iter()
                    .any(|arg| {
                        matches!(arg, ExpArg::Expr(boxed_expr) if dfs(boxed_expr, postfix, idx + 1, _target))
                    });
            }
            false
        }
        postfixes
            .iter()
            .any(|postfix| dfs(expr_key, postfix, 0, target))
    }
}

/// Data structure to help estimate the performance impact of CSE
struct PerfEstimator<'env> {
    target: &'env FunctionTarget<'env>,
    live_var_annotation: &'env LiveVarAnnotation,
}

impl PerfEstimator<'_> {
    const COPY_MAX: usize = 294;
    const MOVE_MAX: usize = 441;
    const ST_LOC_MAX: usize = 441;

    pub fn new<'env>(
        target: &'env FunctionTarget<'env>,
        live_var_annotation: &'env LiveVarAnnotation,
    ) -> PerfEstimator<'env> {
        PerfEstimator {
            target,
            live_var_annotation,
        }
    }

    /// Check if replacing `dest_expr` with `src_expr` can gain performance
    /// In general, we need to consider three types of costs:
    /// - old_cost: The cost of usages of the original `src_temps` (temps defined by `src_expr`) and `dest_temps` (temps defined by ``dest_expr``) before replacement
    /// - new_cost: The cost of original and new usages of `src_temps` after replacement
    /// - gain: The cost saved by eliminating `dest_expr`
    ///
    /// Here, we take a conservative estimation of the perf gain:
    /// - old_cost: in the optimal case, the original usage of `scr_temps` and `dest_temps` are completely removed by optimizations, so we simply assume old_cost = 0
    /// - new_cost: in the most costly case, we need to store the temps defined by `src_expr` and copy/move them to each usage (both original and new usages after replacement)
    /// - gain: we consider the minimum cost of each instruction defining `dest_expr`
    pub fn gain_perf(&self, src_expr: &ExprKey, dest_expr: &ExprKey) -> bool {
        let original_cost = 0;
        let mut new_cost = 0;
        let mut gain = 0;

        // Assuming we need to store every temp defined at `src_expr`
        new_cost += Self::ST_LOC_MAX * src_expr.temps.len();

        let usage_of =
            |temp: &TempIndex, offset| self.live_var_annotation.get_info_at(offset).after.get(temp);

        for src_temp in src_expr.temps.iter() {
            let Some(usage) = usage_of(src_temp, src_expr.offset) else {
                continue;
            };
            // Assuming we need to copy/move the temp to each original usage after replacement
            new_cost += usage.usage_offsets().len() * Self::COPY_MAX.max(Self::MOVE_MAX);
        }

        for dest_temp in dest_expr.temps.iter() {
            let Some(usage) = usage_of(dest_temp, dest_expr.offset) else {
                continue;
            };
            // Assuming we need to copy/move the temp to each original usage after replacement
            new_cost += usage.usage_offsets().len() * Self::COPY_MAX.max(Self::MOVE_MAX);
        }

        // here we calculate the gain from eliminating the bytecode instructions defining dest_expr
        // we take the minimum cost of each instruction as the gain
        for offset in dest_expr.collect_exps().iter() {
            if let Some(inst) = self.target.get_bytecode().get(*offset as usize) {
                let (min_cost, _) = Self::bytecoode_cost(inst);
                gain += min_cost;
            }
        }

        gain + original_cost > new_cost
    }

    /// Estimate the bytecode cost of an instruction
    /// based on the gas metrics defined in `aptos-move/aptos-gas-schedule/src/gas_schedule/instr.rs`
    /// The returned tuple represents (min_cost, max_cost)
    fn bytecoode_cost(code: &Bytecode) -> (usize, usize) {
        match code {
            // min: StLoc(dst): 441
            // max: MoveLoc(src) + StLoc(dst): 441 + 441 = 882
            Bytecode::Assign(..) => (441, 882),
            // min: LdU8: 220
            // max: LdConst: 2389
            Bytecode::Load(..) => (220, 2389),
            Bytecode::Ret(..) => (220, 220),
            // BrTrue or BrFalse
            Bytecode::Branch(..) => (441, 441),
            Bytecode::Jump(..) => (294, 294),
            Bytecode::Label(..) => (0, 0),
            Bytecode::Nop(..) => (36, 36),
            Bytecode::Abort(..) => (220, 220),
            Bytecode::SpecBlock(..) => (0, 0),
            Bytecode::Prop(..) => (0, 0),
            Bytecode::SaveMem(..) => (1100, 1100),
            Bytecode::SaveSpecVar(..) => (1100, 1100),
            Bytecode::Call(_, _, op, _, _) => {
                match op {
                    // min: Call + Ret: 3676 + 220 = 3896
                    // max: unknown
                    Operation::Function(..) | Operation::Invoke => (3896, usize::MAX),
                    Operation::Pack(..)
                    | Operation::Closure(..)
                    | Operation::PackVariant(..)
                    | Operation::Unpack(..)
                    | Operation::UnpackVariant(..) => (808, usize::MAX),
                    Operation::MoveTo(..) => (1838, 1838),
                    Operation::MoveFrom(..) => (1286, 1286),
                    Operation::Exists(..) => (919, 919),
                    Operation::TestVariant(..) => (535, 535),
                    Operation::BorrowLoc => (220, 220),
                    Operation::BorrowField(..) => (735, 735),
                    Operation::BorrowVariantField(..) => (835, 835),
                    Operation::BorrowGlobal(..) => (1838, 1838),
                    // deemed as `Move`?
                    Operation::Drop | Operation::Release => (441, 441),
                    Operation::ReadRef => (735, usize::MAX),
                    Operation::WriteRef => (735, 735),
                    Operation::FreezeRef(..) => (36, 36),
                    Operation::Vector => (0, usize::MAX),
                    Operation::CastU8
                    | Operation::CastU16
                    | Operation::CastU32
                    | Operation::CastU64
                    | Operation::CastU128
                    | Operation::CastU256
                    | Operation::CastI8
                    | Operation::CastI16
                    | Operation::CastI32
                    | Operation::CastI64
                    | Operation::CastI128
                    | Operation::CastI256 => (441, 441),
                    Operation::Not
                    | Operation::Negate
                    | Operation::Add
                    | Operation::Sub
                    | Operation::Mul
                    | Operation::Div
                    | Operation::Mod
                    | Operation::BitOr
                    | Operation::BitAnd
                    | Operation::Xor
                    | Operation::Shl
                    | Operation::Shr
                    | Operation::Or
                    | Operation::And
                    | Operation::Le
                    | Operation::Lt
                    | Operation::Ge
                    | Operation::Gt => (588, 588),
                    Operation::Eq | Operation::Neq => (367, 367),
                    Operation::OpaqueCallBegin(..)
                    | Operation::OpaqueCallEnd(..)
                    | Operation::IsParent(..)
                    | Operation::WriteBack(..)
                    | Operation::UnpackRef
                    | Operation::PackRef
                    | Operation::UnpackRefDeep
                    | Operation::PackRefDeep
                    | Operation::GetField(..)
                    | Operation::GetVariantField(..)
                    | Operation::GetGlobal(..)
                    | Operation::Uninit
                    | Operation::Havoc(..)
                    | Operation::Stop
                    | Operation::TraceLocal(..)
                    | Operation::TraceReturn(..)
                    | Operation::TraceAbort
                    | Operation::TraceExp(..)
                    | Operation::TraceGlobalMem(..)
                    | Operation::EmitEvent
                    | Operation::EventStoreDiverge => (0, 0),
                }
            },
        }
    }
}
