// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "common subexpression elimination" (CSE) transformation
//!
//! Prerequisites:
//! - Variable liveness information is available
//! - Reaching definition information is available
//! - Flush writes information is available
//!
//! Side effects:
//! - Certain instructions are rewritten/removed
//! - Annotations are cleared
//! - AbilityProcessor need to run after this to check variable abilities and insert Copy/Move as needed
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
//! *Without* CSE, all occurance of the same expression `data.x` (line 2, line 3, line 5) will be translated into the seq above,
//! despite `data.x` at line 3 and line 5 share the same result of line 2 and the computations are not necessary.
//!
//! CSE aims to eliminate such repeated computations by reusing the result of previous computations.
//! Specifically, in the example above, assuming the `BorrowLoc` + `BorrowField` + `ReadRef` sequence at line 2 is assigned to temp `t1`,
//! then the occurrences at line 3 and line 5 can both be replaced by `t1`, eliminating the repeated computations.
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
//!  8: $t10 := copy($t5)
//!  9: $t3 := /($t9, $t10) // line 3 reuses $t5
//!  10: label L2
//!  11: return $t3
//!  12: label L1
//!  13: $t16 := 1
//!  14: $t11 := copy($t5)
//!  15: $t3 := +($t11, $t16) // line 5 reuses $t5
//!  16: goto 9
//!
//! ============================ Implementation Details ============================
//!
//! Step 1: Build the Control Flow Graph (CFG) and Domination Tree of a target function.
//!
//! Step 2: Traverse the Domination Tree in preorder, and for each basic block, for each instruction:
//! - If the instruction is *PURE*, canonicalize the expression represented by the instruction into an `ExprKey` structure
//!   - `ExprKey` contains the operation and its arguments, represented as `ExpArg`,
//!   - `ExpArg` can be either a constant, a variable (temp), or another `ExprKey` to nest expressions recursively
//!      - Motivation to nest expression: consider the expression `ReadRef(BorrowField(BorrowLoc(x)))`, we want to
//!        represent it as a single one rather than three separate ones, so that we can eliminate
//!        the entire sequence at once upon reoccurance.
//!      - Conditions to nest `t1 = Op1(t0); t2 = Op2(t1);` as `Op2(Op1(t0))`:
//!         - The definition at `Op1` is the only definition of `t1` that can reach `Op2`
//!         - `t1` is only used once and exactly by `Op2`.
//!      - For commutative operations, the arguments are sorted to get a canonical order
//! - Why pre-order traversal: ensure that all dominating blocks have been processed before the dominated ones,
//!   hencing not missing opportunities for replacement
//!
//! Step 3: Check if the `ExprKey` from Step 2 has been seen before in a dominating block.
//! Given a seen-before `ExprKey` (annotated as `src_expr`) of the current expression (annotated as `dest_expr`),
//! and assuming the two expressions have the following formats:
//!   - `src_expr`: `(src_temp1, src_temp2, ...) = src_op(src_ope1, src_ope2, ...)` defined at `src_inst`, where `src_ope1` and `src_ope2` can be nested expressions
//!   - `dest_expr`: `(dest_temp1, dest_temp2, ...) = dest_op(dest_ope1, dest_ope2, ...)` defined at `dest_inst`, where `dest_ope1` and `dest_ope2` can be nested expressions
//! reusing the results of `src_expr` to replace `dest_expr` can incur safety issues, which we defail below with corresponding solutions:
//!
//! Safety 1: execution may reach `dest_inst` without going through `src_inst` first
//! - This can lead to using incorrect values at `dest_inst`
//! - Solution: check that `src_expr` dominates `dest_expr`
//!
//! Safety 2: type issues
//! - `src_temps` and `dest_temps` can have different mutability when both are references (stackless bytecode does not encode mutability status)
//!   - This can lead to type conflict when copying `src_temp` to `dest_temp`
//!   - Solution: check that when `src_temps` and `dest_temps` have identical types
//! - `stc_temps` can be mutably borrowed
//!   - This can create reference safety violations when copying `src_temps` to `dest_temps`
//!   - Solution: check that none of `src_temps` are mutably borrowed
//!
//! Safety 3: `src_temps` may not be copyable
//! - This can lead to ability violations when copying `src_temps` to `dest_temps`
//! - Solution: check that all `src_temps` are copyable
//!
//! Safety 4: `src_temps` may be re-defined before reaching `dest_expr`
//! - This can lead to using incorrect values at `dest_expr`
//! - Solution: check that the definitions at `src_expr` are the only definitions of `src_temps` that can reach `dest_expr`
//!
//! Safety 5: resources accessed by `src_expr` (via `BorrowGlobal` and `Exists`) may be changed before reaching `dest_expr`
//! - This can lead to accessing different resource status/values at `dest_expr`
//! - Solution: check that the resources accessed by `src_expr` are not changed before reaching `dest_expr`
//!
//! Safety 6: operands used in `src_expr` (i.e., `src_ope1, src_ope2, ...`) may be changed before reaching `dest_expr`
//! - This means that `dest_expr` may produce different results from `src_expr`
//! - Solution: check that operands used in `src_expr` are safe to reuse at `dest_expr`
//!   1. Operands used in `src_expr` are identical to those used in `dest_expr`
//!   2. None of the operands used in `src_expr` are possibly re-defined in a path between `src_expr` and `dest_expr` (without going through `src_expr` again)
//!   3. None of the operands used in `src_expr` are mutable references
//!      - In special cases (e.g., the operand is directly from function argument), our reaching definition cannot trace the memory underneath,
//!        and we may miss possible modifications to the memory states via the mutable reference.
//!
//! Besides safety, we also need to ensure that the replacement can bring performance gains. See comments above `gain_perf` for details
//!
//! Step 4: for each `src_expr` passing the conditions to replace `dest_expr` in Step 3, we check gather necessary information to perform replacement like below:
//!
//! Example:
//! ```Move
//! 1. src_temp = pure_computation_1(t0)      // src_inst
//! 2. ...
//! 3. use(src_temp)
//! 4. dest_temp = pure_computation_1(t0)      // dest_inst
//! 5. ...
//! 6. use(dest_temp)
//! ```
//! ==>
//! ```Move
//! 1. src_temp = pure_computation_1(t0)      // src_inst
//! 2. ...
//! 3. use(src_temp)
//! 4. dest_temp = copy(src_temp)      // inserted copy
//! 5. ...
//! 6. use(dest_temp)
//! ```
//!
//! Step 5: After processing all basic blocks, we perform the recorded replacements and eliminate the marked code.
//!
//! ============================ Extensions ============================
//!
//! In principle, the algorithm above is designed to handle PURE instructions, defined as blow
//! - the results only depend on the operands
//! - has no side effects on `memory` (including write via references), control flow (including `abort`), or external state (global storage)
//! - recomputing it multiple times yields no semantic effect.
//!
//! Yet, we found that some non-pure instructions can be safely handled under certain conditions.
//!
//! Group 1: operations that are pure if no arithmetic errors like overflows happen (`+`, `-`, `*`, `/`, `%`, etc):
//! - their side effects (i.e., aborts) are safe because those, if happening, are guaranteed to happen earlier in the `src_inst`
//!
//! Group 2: operations that are pure if no type errors happen (`UnpackVariant`):
//! - their side effects (i.e., aborts) are safe because those, if happening, are guaranteed to happen earlier in the `src_inst`
//!
//! Group 3: local borrow operations: `BorrowLoc`, `BorrowField`, `BorrowVariantField`
//! - In principle, borrow operations are not pure as they depend on memory states.
//! - Yet, our `Safety 6` guarantees that the memory underneath are not changed and, hence, their "pureness".
//! - We also note that borrowing constants (e.g., `&42`) cannot be reused, as the same constant will actually reside at different memory locations.
//!
//! Group 4: `Assign`
//! - It can be treated as pure when the assign kind is `Copy` or `Inferred` (TODO(#18203): reasoning more about `Inferred`)
//!
//! Group 5: `readref`
//! - `readref` also depends on memory states
//! - But similar to local borrow operations, our `Safety 6` guarantees that the memory states are not changed.
//!
//! Group 6: `Function` calls
//! - A function call can be treated as pure if the callee
//!   - Does not modify any memory via mutable references
//!   - Does not access global resources
//!
//! Group 7: `BorrowGlobal` and `Exists`
//! - They can be treated as pure as our `Safety 5` guarantees that the resources accessed are not modified between `src_inst` and `dst_inst`
//!
//! To add support for other instructions, please extend `BytecodeSanitizer` to enable support and extend the safety rules accordingly.

use crate::{
    bytecode_generator::generate_bytecode,
    pipeline::{
        flush_writes_processor::FlushWritesAnnotation,
        livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfo},
        reaching_def_analysis_processor::ReachingDefAnnotation,
        reference_safety::Object,
    },
};
use im::ordset::OrdSet;
use log::info;
use move_binary_format::file_format::CodeOffset;
use move_core_types::account_address::AccountAddress;
use move_model::{
    ast::{Address, TempIndex},
    model::{FunId, FunctionEnv, ModuleId, QualifiedId, StructId},
    ty::Type,
    well_known::{BORROW_NAME, EMPTY_NAME, LENGTH_NAME, VECTOR_MODULE},
};
use move_stackless_bytecode::{
    function_data_builder::FunctionDataBuilder,
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    graph::{DomRelation, Graph},
    stackless_bytecode::{AssignKind, Bytecode, Constant, Operation},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Formatter},
};

/// Enum to represent an expression operation
/// ExpOp ::= Op(Operation) | Load | Assign(AssignKind)
#[derive(Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum ExpOp {
    Op(Operation),
    Load,
    Assign(AssignKind),
}

/// Canonicalized representation of an expression argument
/// ExpArg ::= Const(c) | Var(t) | Expr(e)
///    where c ∈ Constant, t ∈ TempIndex, e ∈ ExprKey
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

/// A display object for an `ExprArg`.
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

/// Canonicalized representation of an expression, which consists of one or more stackless bytecode instructions.
/// - `op`: the expression operation
/// - `args`: the arguments of the expression, recursively represented as ExprKey when applicable
/// - `temps`: the temps defined by this expression
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
/// ExprKey ::= (op, args, temps, offset)
///   where
///     op ∈ ExpOp
///       args = [ExpArg₁, ExpArg₂, ..., ExpArgₙ], each ExpArgᵢ ∈ ExpArg
///       temps = [t₁, t₂, ..., tₖ], each tᵢ ∈ TempIndex
///       offset ∈ CodeOffset
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

    /// Collect all operands used in this expression
    pub fn collect_operands(&self) -> (Vec<(TempIndex, CodeOffset)>, Vec<(Constant, CodeOffset)>) {
        let mut vars = Vec::new();
        let mut consts = Vec::new();
        for arg in self.args.iter() {
            match arg {
                ExpArg::Var(temp) => vars.push((*temp, self.offset)),
                ExpArg::Expr(boxed_expr) => {
                    let (nested_vars, nested_consts) = boxed_expr.collect_operands();
                    vars.extend(nested_vars.into_iter());
                    consts.extend(nested_consts.into_iter());
                },
                ExpArg::Const(c) => consts.push((c.clone(), self.offset)),
            }
        }
        (vars, consts)
    }

    /// Collect all globals used in this expression
    pub fn collect_resources(&self) -> Vec<(QualifiedId<StructId>, CodeOffset)> {
        let mut globals = Vec::new();
        if let ExpOp::Op(Operation::BorrowGlobal(mid, fid, _)) = &self.op {
            globals.push((mid.qualified(*fid), self.offset));
        }
        if let ExpOp::Op(Operation::Exists(mid, fid, _)) = &self.op {
            globals.push((mid.qualified(*fid), self.offset));
        }
        for arg in self.args.iter() {
            if let ExpArg::Expr(arg_expr) = arg {
                globals.extend(arg_expr.collect_resources().into_iter());
            }
        }
        globals
    }

    /// Collect all bytecode offsets constituting this expression, including nested ones
    pub fn collect_exps(&self) -> Vec<CodeOffset> {
        let mut exps = vec![self.offset];
        for arg in self.args.iter() {
            if let ExpArg::Expr(arg_expr) = arg {
                exps.extend(arg_expr.collect_exps().into_iter());
            }
        }
        exps
    }

    /// Creates a format object for an expression in context of a function target.
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

/// A display object for an `ExprKey`.
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
            write!(
                f,
                "`{}()` @ ",
                self.func_target.func_env.get_full_name_str()
            )?;

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

        write!(f, "[L{}: ", self.expr_key.offset)?;
        match &self.expr_key.op {
            ExpOp::Load => write!(f, "load({})", args_str)?,
            ExpOp::Assign(kind) => write!(f, "assign[{:?}]({})", kind, args_str)?,
            ExpOp::Op(op) => write!(f, "{}({})", op.display(self.func_target), args_str)?,
        };
        write!(f, "]")?;
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
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        // CSE depends on variable liveness analysis, reaching definition analysis, and flush writes analysis!!!
        let (Some(live_var_annotation), Some(reach_def_annotation), Some(flush_writes_annotation)) = (
            target.get_annotations().get::<LiveVarAnnotation>(),
            target.get_annotations().get::<ReachingDefAnnotation>(),
            target.get_annotations().get::<FlushWritesAnnotation>(),
        ) else {
            return data;
        };
        let analyzer = CSEAnalyzer::new(
            target,
            live_var_annotation,
            reach_def_annotation,
            flush_writes_annotation,
            self.aggressive_mode,
        );
        let mut new_data = analyzer.transform();
        new_data.annotations.clear();
        new_data
    }

    fn name(&self) -> String {
        "CommonSubexpElimination".to_string()
    }
}

struct CSEAnalyzer<'env> {
    target: FunctionTarget<'env>,
    live_var_annotation: &'env LiveVarAnnotation,
    reach_def_annotation: &'env ReachingDefAnnotation,
    flush_writes_annotation: &'env FlushWritesAnnotation,
    aggressive_mode: bool,
}

impl CSEAnalyzer<'_> {
    fn new<'env>(
        target: FunctionTarget<'env>,
        live_var_annotation: &'env LiveVarAnnotation,
        reach_def_annotation: &'env ReachingDefAnnotation,
        flush_writes_annotation: &'env FlushWritesAnnotation,
        aggressive_mode: bool,
    ) -> CSEAnalyzer<'env> {
        CSEAnalyzer {
            target,
            live_var_annotation,
            reach_def_annotation,
            flush_writes_annotation,
            aggressive_mode,
        }
    }

    fn transform(&self) -> FunctionData {
        // Step 1: Build the control flow graph and the domination tree as well as the post-domination tree
        let code = self.target.get_bytecode();
        let forward_cfg = StacklessControlFlowGraph::new_forward(code);
        let graph = Graph::new(
            forward_cfg.entry_block(),
            forward_cfg.blocks(),
            forward_cfg.edges(),
        );
        let dom_relation = DomRelation::new(&graph);

        let backward_cfg = StacklessControlFlowGraph::new_backward(code, true);
        let backward_graph = Graph::new(
            backward_cfg.entry_block(),
            backward_cfg.blocks(),
            backward_cfg.edges(),
        );
        let post_dom_relation = DomRelation::new(&backward_graph);

        // Maps from temps to the vector of expressions that define them
        // - Why using a vector: we are not SSA, so a temp may be re-defined multiple times
        // - Here `ExprKey` is used as the value to capture the full expression info (including `temps` and `offset`)
        let mut tempid_to_exprkey = BTreeMap::<Vec<TempIndex>, Vec<ExprKey>>::new();

        // Maps from key to the vector of expressions that share the key
        // - Why using a vector: different expressions may appear multiple times (i.e., sharing the same key) to define different temps
        // - Here `ExprKey` is used as both the key and value
        //   - When used as key, only `op` and `args` are used (see the customized `PartialEq` and `Ord` implementations) to represent the canonicalized expression
        //   - When used as value, the full `ExprKey` (including `temps` and `offset`) is used to represent the specific expression instance
        let mut expr_table = BTreeMap::<ExprKey, Vec<ExprKey>>::new();

        // Maps recording the replacements to be made: (dst_offset: dst_temp) -> (src_offset: src_temp),
        // meaning that `dst_temp` defined at `dst_offset` can be replaced by `src_temp` at `src_offset`
        let mut expr_replacements = BTreeMap::new();
        // Set of code to be eliminated
        let mut eliminate_code = BTreeSet::new();

        // helper to transform a basic block
        let mut transform_bbl = |block_id: BlockId| {
            let bbl_range = forward_cfg.code_range(block_id);
            let bbl = &code[bbl_range.clone()];
            for (offset, inst) in bbl_range.clone().zip(bbl) {
                // Step 2: get a canonicalized representation of the current expression
                let Some(expr_key) = self.canonicalize_expr(
                    &tempid_to_exprkey,
                    inst,
                    offset as CodeOffset,
                    &forward_cfg,
                    &dom_relation,
                ) else {
                    continue;
                };

                // cache the mapping from defined temps to `ExprKey`
                tempid_to_exprkey
                    .entry(expr_key.temps.clone())
                    .or_default()
                    .push(expr_key.clone());

                // Step 3: get the top-most expression that share the same key and qualifies for replacement
                // why top-most: maximize the chances of reusing the same expression in multiple places
                if let Some(src_expr) = self.get_qualified_replacement(
                    &expr_key,
                    &forward_cfg,
                    &dom_relation,
                    &backward_cfg,
                    &post_dom_relation,
                    &expr_table,
                ) {
                    // Step 4: record the replacement info
                    if self.collect_replace_info(
                        &src_expr,
                        &expr_key,
                        &mut expr_replacements,
                        &mut eliminate_code,
                    ) {
                        continue;
                    }
                }
                // if not to be replaced, record the `ExprKey` for checking future re-occurrences
                expr_table
                    .entry(expr_key.clone())
                    .or_default()
                    .push(expr_key);
            }
        };

        // Traverse the domination tree in preorder
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
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
    ) -> Option<ExprKey> {
        // Check if the bytecode is allowed to consider
        // - see BytecodeSanitizer for details
        let bytecode_sanitizer = BytecodeSanitizer::new_from_bytecode(inst);
        if !bytecode_sanitizer.is_allowed(self.aggressive_mode, &self.target) {
            return None;
        }

        // Helper to check if `src_temp` defined at `src_inst` is used as an immediate temp at `dest_inst`
        // condition 1: `src_inst` is the only definition of `src_temp` that can reach `dest_inst`;
        // - this ensures that the value of `src_temp` at `dest_inst` is exactly the one defined at `src_inst`
        // condition 2: the usage is single and exactly at `dest_inst`
        // - this ensures that, when nesting `src_inst` into `dest_inst` and consequently removing `src_temp`, no other uses of `src_temp` are affected
        let used_as_imm = |src_inst, src_temp, dest_inst| {
            self.single_def_reach(src_inst, src_temp, dest_inst, cfg, dom_relation)
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

        // converting the bytecode into an `ExprKey`, possibly nesting their arguments recursively
        let res = match inst {
            Bytecode::Load(_, dest, constant) => Some(ExprKey::new(
                ExpOp::Load,
                vec![ExpArg::Const(constant.clone())],
                vec![*dest],
                offset,
            )),

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

            // TODO(#18203): handle AbortAction
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

            // these do not define temps
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
        backward_cfg: &StacklessControlFlowGraph,
        post_dom_relation: &DomRelation<u16>,
        expr_table: &BTreeMap<ExprKey, Vec<ExprKey>>,
    ) -> Option<ExprKey> {
        // check all previous occurrences of the same expression and return the first qualified one
        if let Some(src_exprs) = expr_table.get(target_expr) {
            for src_expr in src_exprs.iter() {
                if self.is_qualified_replacement(
                    cfg,
                    dom_relation,
                    backward_cfg,
                    post_dom_relation,
                    src_expr,
                    target_expr,
                ) {
                    return Some(src_expr.clone());
                }
            }
        }
        None
    }

    /// Check if `src_temps` defined at `src_expr` can be used to replace the expressions defined at `dest_expr` as follows
    /// ```Move
    /// 1. src_temp = pure_computation_1(t0)      // src_expr
    /// 2. ...
    /// 3. use(src_temp)
    /// 4. dest_temp = pure_computation_1(t0)      // dest_expr
    /// 5. ...
    /// 6. use(dest_temp)
    /// ```
    /// ==>
    /// ```Move
    /// 1. src_temp = pure_computation_1(t0)      // src_expr
    /// 2. ...
    /// 3. use(src_temp)
    /// 4. dest_temp = copy(src_temp)      // inserted copy
    /// 5. ...
    /// 6. use(dest_temp)
    /// ```
    /// Notes:
    /// - `src_temps` are temps produced by `src_expr`
    /// - `src_expr` can be nested, namely `src_expr` := `op(inner_op1(operand1), inner_op2(operand2), ...)`,
    ///    where `operand1` and `operand2` are called *operands* of `src_expr`
    /// - `dest_temps` and `dest_expr` are defined similarly
    ///
    /// Specifically, we check the following conditions to rule out safety issues described at the beginning of this file:
    ///
    /// Condition 1. `src_expr` dominates `dest_expr`
    ///
    /// Condition 2: type safety
    /// - `src_temps` and `dest_temps` share the same types
    /// - `stc_temp` is not mutably borrowed
    ///
    /// Condition 3: `src_temps` are copyable
    ///
    /// Condition 4: Temps defined at `src_expr` are safe to reuse at `dest_expr`:
    /// - `src_temps` are not mutably borrowed elsewhere
    /// - `src_temps` at `src_expr` are the only definitions of `src_temps` that can reach `dest_expr`:
    ///
    /// Condition 5: Resources used in `src_expr` are safe to reuse at `dest_expr`:
    /// - Resources used in `src_expr` are identical to those used in `dest_expr`
    /// - Locations where resources are used in `src_expr` dominate those in `dest_expr`
    /// - None of the resources used in `src_expr` are possibly re-defined in a path between `src_expr` and `dest_expr` (without going through `src_expr` again)
    ///
    /// Condition 6: Operands used in `src_expr` are safe to reuse at `dest_expr`:
    /// - Operands used in `src_expr` are identical to those used in `dest_expr`
    /// - Locations where operands are used in `src_expr` dominate those in `dest_expr`
    /// - None of the operands used in `src_expr` are possibly re-defined in a path between `src_expr` and `dest_expr` (without going through `src_expr` again)
    /// - None of the operands used in `src_expr` are mutable references
    /// - None of the operands used in `src_expr` are mutably borrowed elsewhere
    ///
    /// Condition 7: The replacement will bring performance gains! See comments above `gain_perf` for details
    ///
    fn is_qualified_replacement(
        &self,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
        backward_cfg: &StacklessControlFlowGraph,
        post_dom_relation: &DomRelation<u16>,
        src_expr: &ExprKey,
        dest_expr: &ExprKey,
    ) -> bool {
        // helper to check Condition 2
        let type_safe = |src_temps: &[TempIndex], dst_temps: &[TempIndex], src_site: CodeOffset| {
            src_temps
                .iter()
                .zip(dst_temps)
                .all(|(src_temp, dest_temp)| {
                    self.get_local_type(src_temp) == self.get_local_type(dest_temp)
                        && match self.get_usage_of(src_temp, src_site) {
                            Some(live_var) => live_var
                                .usage_offsets()
                                .iter()
                                .all(|use_site| !self.is_mutable_borrow(*use_site)),
                            None => true,
                        }
                })
        };

        // helper to check Condition 3
        let srcs_copyable = |srcs: &Vec<TempIndex>| {
            srcs.iter().all(|temp| {
                let ty = self.get_local_type(temp);
                let abilities = self
                    .target
                    .global_env()
                    .type_abilities(ty, &self.target.get_type_parameters());
                abilities.has_copy()
            })
        };

        // helper to check Condition 4
        let temps_safe_to_reuse = |src_temps: &[TempIndex], src_offset, dst_offset| {
            src_temps.iter().all(|src_temp| {
                !self.get_local_type(src_temp).is_mutable_reference()
                    && self.single_def_reach(src_offset, *src_temp, dst_offset, cfg, dom_relation)
            })
        };

        // helper to check Condition 5
        let resources_safe_to_reuse =
            |src_resources: &[(QualifiedId<StructId>, CodeOffset)],
             dst_resources: &[(QualifiedId<StructId>, CodeOffset)]| {
                src_resources.iter().zip(dst_resources).all(
                    |((src_resource, _src_offset), (dst_resource, _dst_offset))| {
                        Self::src_dominate_dst(*_src_offset, *_dst_offset, cfg, dom_relation)
                            && src_resource == dst_resource
                            && !self.obj_killed_between(
                                Object::Global(*src_resource),
                                *_src_offset,
                                *_dst_offset,
                                cfg,
                            )
                    },
                )
            };

        // helper to check Condition 6
        let operands_safe_to_reuse =
            |src_operands: &[(TempIndex, CodeOffset)], dst_operands: &[(TempIndex, CodeOffset)]| {
                src_operands.iter().zip(dst_operands).all(
                    |((src_operand, src_ope_offset), (dst_operand, dst_ope_offset))| {
                        Self::src_dominate_dst(*src_ope_offset, *dst_ope_offset, cfg, dom_relation)
                            && src_operand == dst_operand
                            && !self.obj_killed_between(
                                Object::Local(*src_operand),
                                *src_ope_offset,
                                *dst_ope_offset,
                                cfg,
                            )
                            && !self.get_local_type(src_operand).is_mutable_reference()
                    },
                )
            };

        Self::src_dominate_dst(src_expr.offset, dest_expr.offset, cfg, dom_relation) // check Condition 1
            && type_safe(&src_expr.temps, &dest_expr.temps, src_expr.offset) // check Condition 2
            && srcs_copyable(&src_expr.temps) // check Condition 3
            && temps_safe_to_reuse(
                &src_expr.temps, src_expr.offset, dest_expr.offset
            ) // check Condition 4
            && resources_safe_to_reuse(
                &src_expr.collect_resources(),
                &dest_expr.collect_resources(),
            ) // check Condition 5
            && operands_safe_to_reuse(
                &src_expr.collect_operands().0,
                &dest_expr.collect_operands().0,
            ) // check Condition 6
            && self.gain_perf(src_expr, dest_expr, backward_cfg, post_dom_relation, true)
        // check Condition 7
    }

    /// Collect information needed for performing the replacement
    /// Given `src_temps = src_expr` that will replace `dest_temps = dest_expr`,
    /// record the replacement info in the format of `dest_expr.offset: dest_temp -> (src_expr.offset, src_temp)`
    fn collect_replace_info(
        &self,
        src_expr: &ExprKey,
        dest_expr: &ExprKey,
        expr_replacements: &mut BTreeMap<CodeOffset, Vec<(TempIndex, (CodeOffset, TempIndex))>>,
        eliminate_code: &mut BTreeSet<CodeOffset>,
    ) -> bool {
        // get the temps defined by src_expr (to replace) and dest_expr (be replaced)
        let src_temps = &src_expr.temps;
        let dest_temps = &dest_expr.temps;

        // it's a lazy impl at present to only support expressions defining a single temp
        // TODO(#18203): extend to support multiple temps
        if src_temps.len() != 1 {
            return false;
        }

        // If any nested expression in `src_expr` has been recorded for elimination, we cannot do the replacement
        if src_expr
            .collect_exps()
            .iter()
            .any(|offset| expr_replacements.contains_key(offset))
        {
            return false;
        }

        // record the replacement info
        for (src_temp, dest_temp) in src_temps.iter().zip(dest_temps.iter()) {
            expr_replacements
                .entry(dest_expr.offset)
                .or_default()
                .push((*dest_temp, (src_expr.offset, *src_temp)));

            info!(
                "CSE: replacing \n \t {} ===> \t {}\n",
                dest_expr.display(&self.target, true),
                src_expr.display(&self.target, true),
            );
        }

        // We also need to collect the code to be eliminated: including `dest_expr` and all its nested expressions
        let to_be_eliminated = dest_expr.collect_exps();
        // any nested expression that is not `dest_expr` itself does not need to be replaced
        // Example:
        // - `src_expr` := `op(inner_op1(operand1), inner_op2(operand2), ...)`
        // - `dest_expr` := `op(inner_op1(operand1), inner_op2(operand2), ...)`
        // we replace `dest_expr` entirely as a whole, so `inner_op1(operand1)` and `inner_op2(operand2)` do not need to be replaced
        for offset in to_be_eliminated.iter() {
            if offset != &dest_expr.offset {
                expr_replacements.remove(offset);
            }
        }
        eliminate_code.extend(to_be_eliminated);
        true
    }

    /// Perform the actual replacement in the bytecode
    ///
    /// Given the following stackless bytecode:
    /// ```Move stackless bytecode without CSE
    /// 1. t1 = pure_computation_1()
    /// 2. ...
    /// 3. use1(t1, ...)
    /// 4. t2 = pure_computation_1()
    /// 5. ...
    /// 6. use2(t2, ...)
    /// ```
    /// intuitively, we should simply replace line 6 with `use2(t1, ...)`, resulting in:
    /// ```Move stackless bytecode after intuitive CSE
    /// 1. t1 = pure_computation_1()
    /// 2. ...
    /// 3. use1(t1, ...)
    /// 4. ...
    /// 5. use2(t1, ...)
    /// ```
    ///
    /// This transformation, after translating to the file format bytecode, however can often downgrade the performance.
    /// Let's consider the following example in file format bytecode:
    ///
    /// ```Move File Format Bytecode without CSE
    /// 1. PURE_COMPUTATION_1 // defines `t1` and keeps `t1` on stack
    /// 2. OP1 ...  // defines another temp on the stack (`t2`), without consuming `t1` or flushing it out of stack
    /// 3. OP2 ...  // defines another temp on the stack (`t3`), without consuming `t1` and `t2` or flushing them out of stack
    /// 4. USE1     // the original use of `t1`, consuming three temps on stack: [`t1`, `t2`, `t3`]
    /// 5. PURE_COMPUTATION_1 // the redundant computation to be replaced, defines `t4` and keeps `t4` on stack
    /// 6. USE2    // the original use of `t4`, consuming `t4` on the stack
    /// ```
    ///
    /// If in CSE, we add a reuse of `t1` at line 6, the file format generator will flush `t1` after line 1,
    /// and then copy it back to the stack before line 4.
    /// Before the copy, it will find that the stack becomes [`t2`, `t3`], missing `t1`.
    /// To restore the stack layout, it has to
    /// - pop `t2` and `t3` off the stack,
    /// - copy `t1` back to the stack,
    /// - push `t2` and `t3` back to the stack.
    /// and eventually have file format bytecode like below:
    ///
    /// ```Move File Format Bytecode
    /// 1. PURE_COMPUTATION_1 // defines `t1` and keeps `t1` on stack
    /// 2. STLOC t1 // flush t1 off stack to a local
    /// 3. OP1 ...  // defines another temp on the stack (`t2`)
    /// 4. OP2 ...  // defines another temp on the stack (`t3`)
    /// 5. STLOC t3 // pop t3 to a local
    /// 6. STLOC t2 // pop t2 to a local
    /// 7. COPYLOC t1 // copy t1 back to stack
    /// 8. COPYLOC t2 // copy t2 back to stack
    /// 9. COPYLOC t3 // copy t3 back to stack
    /// 10. USE1  // the original use of `t1`, taking three temps on stack: [`t1`, `t2`, `t3`]
    /// 11. COPYLOC t1
    /// 12. USE2  // the original use of `t4`, taking `t1` on stack
    /// ```
    /// To avoid this problem, we need to make a reuse of `t1` while not affecting the stack layout before its original use.
    /// Similarly, we should also avoid affecting the stack layout before the original use of `t4` (`t4` represents the result of the replaced expression).
    ///
    /// As such, we will transform the stackless bytecode to:
    ///
    /// ```Move stackless bytecode after our deployed CSE
    /// 1. t1 = pure_computation_1()
    /// 2. t1 = dup(t1)
    ///    // flush the value of `t1` from the stack to a local and meanwhile keep `t1` on the stack for its original use.
    ///    // This ensures the stack layout before `use1` is not affected
    ///    // Note: we do not need to do this if `t1` is going to be flushed without CSE
    /// 3. ...
    /// 4. use1(t1, ...)
    /// 5. t2 = dup(t1) // copy the value of `t1` from the local to the stack to work as `t4`. This ensures the stack layout before `use2` is not affected
    /// 6. ...
    /// 7. use2(t2, ...)
    ///
    fn perform_replacement(
        &self,
        expr_replacements: &mut BTreeMap<CodeOffset, Vec<(TempIndex, (CodeOffset, TempIndex))>>,
        eliminate_code: &mut BTreeSet<CodeOffset>,
    ) -> FunctionData {
        let mut builder = FunctionDataBuilder::new(self.target.func_env, self.target.data.clone());
        let code = std::mem::take(&mut builder.data.code);

        // collect the set of `src_expr`s whose temps need a `dup`
        // specifically, if the temp defined at `src_expr` will not be flushed without CSE,
        // we need to `dup` it to keep a copy on stack for its original use
        // Remark: `expr_replacements` records the replacement info in the format of
        //   `dest_expr.offset: dest_temp -> (src_expr.offset, src_temp)`
        let dup_set = expr_replacements
            .values()
            .flatten()
            .filter_map(|(_, (src_offset, src_temp))| {
                if self.def_needs_flush(src_offset, src_temp) {
                    None
                } else {
                    Some(*src_offset)
                }
            })
            .collect::<BTreeSet<CodeOffset>>();

        // iterate through the original code and perform replacements and eliminations
        for (offset, inst) in code.into_iter().enumerate() {
            let code_offset = offset as CodeOffset;
            let id = inst.get_attr_id();

            // if replacement found, perform it
            if let Some(replacements) = expr_replacements.get(&code_offset) {
                assert!(
                    replacements.len() == 1 && eliminate_code.contains(&code_offset),
                    "only one replacement supported for now and the code must be recorded for elimination"
                );

                // if the `src_temp` needs a `dup`, we replace the `dest_expr` with a `dup` of `src_temp`
                // otherwise, we simply replace it with an `Assign` from `src_temp`
                let assign_kind = if dup_set.contains(&replacements[0].1 .0) {
                    AssignKind::Dup
                } else {
                    AssignKind::Inferred
                };
                builder.emit(Bytecode::Assign(
                    id,
                    replacements[0].0,
                    replacements[0].1 .1,
                    assign_kind,
                ));

                // record that the replacement has been done
                // and the elimination has been performed
                expr_replacements.remove(&code_offset);
                eliminate_code.remove(&code_offset);
                continue;
            }

            // if no replacement found, we try to see if it needs to be eliminated
            // if so, simply skip it
            if eliminate_code.remove(&code_offset) {
                continue;
            }
            // otherwise, we emit the original instruction
            let temp = inst.dests();
            builder.emit(inst);
            // if this is a `src_expr` whose temp needs a `dup`, we emit a `dup` instruction as well
            if dup_set.contains(&code_offset) {
                let new_id = builder.new_attr_with_cloned_info(id);
                builder.emit(Bytecode::Assign(new_id, temp[0], temp[0], AssignKind::Dup));
            }
        }

        // finally, check all replacements and eliminations have been performed
        assert!(
            expr_replacements.is_empty() && eliminate_code.is_empty(),
            "all replacements must have been completed {:?}",
            expr_replacements
        );
        builder.data
    }

    /// get the bytecode at the given offset
    fn get_bytecode_at(&self, offset: &CodeOffset) -> &Bytecode {
        &self.target.get_bytecode()[*offset as usize]
    }

    /// get the usage info of a temp at a given bytecode offset
    fn get_usage_of(&self, temp: &TempIndex, offset: CodeOffset) -> Option<&LiveVarInfo> {
        self.live_var_annotation.get_info_at(offset).after.get(temp)
    }

    /// get the type of a local temp
    fn get_local_type(&self, temp: &TempIndex) -> &Type {
        self.target.get_local_type(*temp)
    }

    /// check if `src` dominates `dst`
    fn src_dominate_dst(
        src: CodeOffset,
        dst: CodeOffset,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
    ) -> bool {
        let src_bbl = cfg.enclosing_block(src);
        let dst_bbl = cfg.enclosing_block(dst);
        if src_bbl == dst_bbl {
            // same block, check offset
            src < dst
        } else {
            // different blocks, check domination
            dom_relation.is_dominated_by(dst_bbl, src_bbl)
        }
    }

    /// check if `dst` post-dominates `dst`
    fn dst_post_dominate_src(
        src: CodeOffset,
        dst: CodeOffset,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
    ) -> bool {
        let src_bbl = cfg.enclosing_block(src);
        let dst_bbl = cfg.enclosing_block(dst);
        if src_bbl == dst_bbl {
            // same block, check offset
            src < dst
        } else {
            // different blocks, check domination
            dom_relation.is_dominated_by(src_bbl, dst_bbl)
        }
    }

    /// check if a temp defined at offset will be flushed before its use
    fn def_needs_flush(&self, offset: &CodeOffset, temp: &TempIndex) -> bool {
        self.target.get_pinned_temps(true).contains(temp)
            || self
                .flush_writes_annotation
                .0
                .get(offset)
                .is_some_and(|temps| temps.contains(temp))
    }

    /// check if the instruction at offset creates a mutable borrow
    fn is_mutable_borrow(&self, offset: CodeOffset) -> bool {
        let inst = self.get_bytecode_at(&offset);
        inst.is_borrowing()
            && inst
                .dests()
                .iter()
                .any(|t| self.get_local_type(t).is_mutable_reference())
    }

    /// Checks if the definition of `src_temp` at `src_inst` is only used once and exactly at `dest_inst`.
    fn single_use_at(
        &self,
        src_inst: CodeOffset,
        src_temp: TempIndex,
        dest_inst: CodeOffset,
    ) -> bool {
        self.get_usage_of(&src_temp, src_inst)
            .is_some_and(|uses| uses.usage_offsets() == OrdSet::unit(dest_inst))
    }

    /// Checks if the definition of `src_temp` is the only definition of `src_inst`that can reach `dest_inst`.
    fn single_def_reach(
        &self,
        src_inst: CodeOffset,
        src_temp: TempIndex,
        dest_inst: CodeOffset,
        cfg: &StacklessControlFlowGraph,
        dom_relation: &DomRelation<u16>,
    ) -> bool {
        Self::src_dominate_dst(src_inst, dest_inst, cfg, dom_relation)
            && self
                .reach_def_annotation
                .get_info_at(dest_inst)
                .map
                .get(&Object::Local(src_temp))
                .is_some_and(|defs| defs == &BTreeSet::from([src_inst]))
    }

    /// Checks if `obj` is possibly re-defined in a path between `src` and `dest` (without going through `src` again)
    fn obj_killed_between(
        &self,
        obj: Object,
        src_inst: CodeOffset,
        dest_inst: CodeOffset,
        cfg: &StacklessControlFlowGraph,
    ) -> bool {
        // get all definitions of `obj` that can reach `dest`
        let Some(dest_defs) = self
            .reach_def_annotation
            .get_info_at(dest_inst)
            .map
            .get(&obj)
        else {
            // TODO(#18203): this would only happen if `obj` is a function parameter; so add a check later
            return false;
        };

        // helper to check if there is a path from `start` to `end` without going through `blocker`
        // if `blocker` == `start`, it means we are checking paths that do not go through `start` again
        let can_reach_without = |start, end, blocker| {
            let mut queue = vec![start];
            let mut visited = BTreeSet::new();
            visited.insert(start);

            while let Some(cur) = queue.pop() {
                if cur == end {
                    return true;
                }
                for succ in cfg.successor_insts(cur) {
                    if succ == blocker || visited.contains(&succ) {
                        continue;
                    }
                    visited.insert(succ);
                    queue.push(succ);
                }
            }
            false
        };

        // check if any definition can reach `dest` on a path starting at `src` without going through `src` again
        dest_defs.iter().any(|def_inst| {
            // Logic here: `src_inst` is the start, `dest_inst` is the end, and `def_inst` is where `obj` is defined
            // If there is a path from `src_inst` to `def_inst` without going through `src_inst` again,
            // and there is a path from `def_inst` to `dest_inst` without going through `src_inst`,
            // there is a path from `src_inst` to `dest_inst` where `obj` is re-defined at `def_inst` without going through `src_inst` again
            can_reach_without(src_inst, *def_inst, src_inst)
                && can_reach_without(*def_inst, dest_inst, src_inst)
        })
    }

    /// Check if replacing `dest_expr` with `src_expr` can gain performance
    /// In general, we need to consider two types of costs:
    /// - new_cost: new instructions introduced by the replacements
    /// - gain: the cost saved by eliminating `dest_expr`
    ///
    /// Here, we take a conservative estimation of the perf gain:
    /// - new_cost: maximize
    /// - gain: minimize
    ///
    /// The calculation of `new_cost` needs to happen at the file format level. Yet, we only have the stackless bytecode here.
    /// As a workaround, we estimate the cost based on how the stackless bytecode would be translated to file format bytecode.
    ///
    /// Based on the comments above `perform_replacement`:
    /// - If `src_temp` defined at `src_expr` is going to be flushed anyway,
    ///   we only need to copy `src_temp` at `dest_expr` for reuse, usually via a `CopyLoc` instruction.
    /// - If `src_temp` defined at `src_expr` is not going to be flushed,
    ///   we need to flush `src_temp` at `src_expr` (via a `StLoc` instruction),
    ///   `dup` it back to the stack for its original uses (via a `CopyLoc` instruction),
    ///   and also `dup` it back to the stack at `dest_expr` for reuse (via a `CopyLoc` instruction).
    ///
    /// We have two modes to estimate the costs:
    /// - instruction count mode: each bytecode instruction has a unit cost of 1, except for those very expensive ones like `Call` and `BorrowGlobal`
    /// - gas cost mode: each bytecode instruction has a cost based on gas metrics
    ///   - TODO(#18203): this is based on gas metrics defined in `aptos-move/aptos-gas-schedule/src/gas_schedule/instr.rs`.
    ///   - Once we have a more accurate gas model, we need to refine the estimation here.
    ///
    pub fn gain_perf(
        &self,
        src_expr: &ExprKey,
        dest_expr: &ExprKey,
        backward_cfg: &StacklessControlFlowGraph,
        post_dom_relation: &DomRelation<u16>,
        use_gas_cost: bool,
    ) -> bool {
        let mut new_cost = 0;
        let mut gain = 0;
        let mut risk = 0;
        let mut post_dominate = true;

        // helper to get the cost of copying a local temp
        let get_copy_loc_cost = |temp: &TempIndex| {
            if use_gas_cost {
                let temp_size = self
                    .get_local_type(temp)
                    .estimate_size(self.target.global_env(), Self::VEC_SIZE);
                Self::COPY_LOC_COST + Self::BYTE_COST * temp_size
            } else {
                1
            }
        };

        // helper to get the cost of storing a local temp
        let get_st_loc_cost = || {
            if use_gas_cost {
                Self::ST_LOC_COST
            } else {
                1
            }
        };

        for src_temp in src_expr.temps.iter() {
            // part 4: we always need to make a copy of each `src_temp` at `dest_expr` for reuse
            let copy_src_temp_cost = get_copy_loc_cost(src_temp);
            new_cost += copy_src_temp_cost;

            // if `src_temp` is going to be flushed anyway, no extra cost is introduced
            if self.def_needs_flush(&src_expr.offset, src_temp) {
                continue;
            }

            // check if `dest_expr` post-dominates `src_expr`
            if !Self::dst_post_dominate_src(
                src_expr.offset,
                dest_expr.offset,
                backward_cfg,
                post_dom_relation,
            ) {
                post_dominate = false;
            }

            // part 1: flushing `src_temp` from the stack
            let st_loc_cost = get_st_loc_cost();
            new_cost += st_loc_cost;
            risk += st_loc_cost;

            // part 2: dup `src_temp` back to the stack for its original uses
            let Some(usage) = self.get_usage_of(src_temp, src_expr.offset) else {
                continue;
            };

            let use_cost = usage.usage_offsets().len() * get_copy_loc_cost(src_temp);
            new_cost += use_cost;
            risk += use_cost;
        }

        // here we calculate the gain from eliminating the bytecode instructions defining dest_expr
        // we take the minimum cost of each instruction as the gain
        for offset in dest_expr.collect_exps().iter() {
            if let Some(inst) = self.target.get_bytecode().get(*offset as usize) {
                let code_cost = self.bytecode_cost(inst);
                if use_gas_cost {
                    gain += code_cost.min_gas;
                } else {
                    gain += code_cost.min_inst_num;
                }
            }
        }

        // if any `dest_expr` does not post-dominate `src_expr`, we need to consider the risk level
        // (i.e., the extra cost on paths which does not go through `dest_expr`);
        // otherwise, we only need to compare gain and new_cost
        if !post_dominate {
            risk <= Self::RISK_LEVEL && gain > new_cost + Self::MIN_GAP
        } else {
            gain > new_cost + Self::MIN_GAP
        }
    }
}

struct BytecodeCost {
    min_inst_num: usize,
    _max_inst_num: usize,
    min_gas: usize,
    _max_gas: usize,
}

impl CSEAnalyzer<'_> {
    const ABORT_COST: usize = 220;
    const ARITH_LOGIC_COST: usize = 588;
    const BORROW_FIELD_COST: usize = 735;
    const BORROW_GLOBAL_COST: usize = 1838;
    const BORROW_LOC_COST: usize = 220;
    const BORROW_VARIANT_FIELD_COST: usize = 835;
    const BRANCH_COST: usize = 441;
    // extra cost for each byte in operands
    // TODO(#18203): tune this value
    const BYTE_COST: usize = 14;
    const CAST_COST: usize = 441;
    const COPY_LOC_COST: usize = 294;
    const DROP_RELEASE_COST: usize = 441;
    const EQUALITY_COST: usize = 367;
    const EXISTS_COST: usize = 919;
    const FREEZE_REF_COST: usize = 36;
    const FUN_CALL_COST: usize = 3676;
    const JUMP_COST: usize = 294;
    const LABEL_COST: usize = 0;
    const LD_CONST_COST: usize = 2389;
    const LD_U8_COST: usize = 220;
    // A minimum gain to justify the replacement
    // TODO(#18203): tune this value
    const MIN_GAP: usize = Self::ST_LOC_COST + Self::COPY_LOC_COST;
    const MOVE_FROM_COST: usize = 1286;
    const MOVE_LOC_COST: usize = 441;
    const MOVE_TO_COST: usize = 1838;
    const NOP_COST: usize = 36;
    const PACK_UNPACK_CLOSURE_COST: usize = 808;
    const READ_REF_COST: usize = 735;
    const RET_COST: usize = 220;
    // A risk level we can afford when any `dest_expr` does not post-dominate `src_expr`
    // Here, `risk level` means the extra cost we may introduce in paths not going through `dest_expr`
    // Currently, we do not take any risk
    // TODO(#18203): tune this value
    const RISK_LEVEL: usize = 0;
    const SPEC_COST: usize = 0;
    const ST_LOC_COST: usize = 441;
    const TEST_VARIANT_COST: usize = 535;
    const VEC_BORROW_COST: usize = 1213;
    const VEC_ELE_COST: usize = 147;
    const VEC_EMPTY_COST: usize = 2205;
    const VEC_LEN_COST: usize = 808;
    const VEC_PACK_COST: usize = 2205;
    // default size for vector types
    // TODO(#18203): tune this value
    const VEC_SIZE: usize = 1;
    const WRITE_REF_COST: usize = 735;

    /// Estimate the bytecode cost of an instruction
    /// based on its number of instructions and gas metric
    /// The returned tuple represents (min_inst_num, _max_inst_num, min_gas, _max_gas)
    fn bytecode_cost(&self, code: &Bytecode) -> BytecodeCost {
        match code {
            // min: nothing happens
            // max: StLoc + CopyLoc/MoveLoc
            Bytecode::Assign(..) => BytecodeCost {
                min_inst_num: 0,
                _max_inst_num: 2,
                min_gas: 0,
                _max_gas: Self::MOVE_LOC_COST + Self::ST_LOC_COST,
            },
            // min: LdU8
            // max: LdConst
            Bytecode::Load(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::LD_U8_COST,
                _max_gas: Self::LD_CONST_COST,
            },
            // BrTrue or BrFalse
            Bytecode::Branch(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::BRANCH_COST,
                _max_gas: Self::BRANCH_COST,
            },
            Bytecode::Jump(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::JUMP_COST,
                _max_gas: Self::JUMP_COST,
            },
            Bytecode::Label(..) => BytecodeCost {
                min_inst_num: 0,
                _max_inst_num: 0,
                min_gas: Self::LABEL_COST,
                _max_gas: Self::LABEL_COST,
            },
            Bytecode::Nop(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::NOP_COST,
                _max_gas: Self::NOP_COST,
            },
            Bytecode::Abort(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::ABORT_COST,
                _max_gas: Self::ABORT_COST,
            },
            Bytecode::SpecBlock(..)
            | Bytecode::Prop(..)
            | Bytecode::SaveMem(..)
            | Bytecode::SaveSpecVar(..) => BytecodeCost {
                min_inst_num: 0,
                _max_inst_num: 0,
                min_gas: Self::SPEC_COST,
                _max_gas: Self::SPEC_COST,
            },
            Bytecode::Ret(..) => BytecodeCost {
                min_inst_num: 1,
                _max_inst_num: 1,
                min_gas: Self::RET_COST,
                _max_gas: Self::RET_COST,
            },
            Bytecode::Call(_, dests, op, _, _) => {
                match op {
                    Operation::Function(mid, fid, _) => {
                        // Several functions are compiled into special bytecodes instead of normal call/ret
                        let global_env = self.target.global_env();
                        let module_env = global_env.get_module(*mid);
                        let module_addr = module_env.self_address();
                        let module_name = global_env
                            .symbol_pool()
                            .string(module_env.get_name().name());
                        let func_name = global_env
                            .symbol_pool()
                            .string(module_env.get_function(*fid).get_name());
                        match (module_addr, module_name.as_str(), func_name.as_str()) {
                            (
                                Address::Numerical(AccountAddress::ONE),
                                VECTOR_MODULE,
                                LENGTH_NAME,
                            ) => BytecodeCost {
                                min_inst_num: 1,
                                _max_inst_num: 1,
                                min_gas: Self::VEC_LEN_COST,
                                _max_gas: Self::VEC_LEN_COST,
                            },
                            (
                                Address::Numerical(AccountAddress::ONE),
                                VECTOR_MODULE,
                                BORROW_NAME,
                            ) => BytecodeCost {
                                min_inst_num: 1,
                                _max_inst_num: 1,
                                min_gas: Self::VEC_BORROW_COST,
                                _max_gas: Self::VEC_BORROW_COST,
                            },
                            (
                                Address::Numerical(AccountAddress::ONE),
                                VECTOR_MODULE,
                                EMPTY_NAME,
                            ) => BytecodeCost {
                                min_inst_num: 1,
                                _max_inst_num: 1,
                                min_gas: Self::VEC_EMPTY_COST,
                                _max_gas: Self::VEC_EMPTY_COST,
                            },
                            // min: Call + at least 2 inst (?) + Ret
                            // max: unknown
                            _ => BytecodeCost {
                                min_inst_num: 4,
                                _max_inst_num: usize::MAX,
                                min_gas: Self::FUN_CALL_COST + Self::RET_COST,
                                _max_gas: usize::MAX,
                            },
                        }
                    },
                    Operation::Invoke => BytecodeCost {
                        min_inst_num: 4,
                        _max_inst_num: usize::MAX,
                        min_gas: Self::FUN_CALL_COST + Self::RET_COST,
                        _max_gas: usize::MAX,
                    },
                    Operation::Pack(..)
                    | Operation::Closure(..)
                    | Operation::PackVariant(..)
                    | Operation::Unpack(..)
                    | Operation::UnpackVariant(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::PACK_UNPACK_CLOSURE_COST,
                        _max_gas: usize::MAX,
                    },
                    Operation::MoveTo(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::MOVE_TO_COST,
                        _max_gas: Self::MOVE_TO_COST,
                    },
                    Operation::MoveFrom(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::MOVE_FROM_COST,
                        _max_gas: Self::MOVE_FROM_COST,
                    },
                    Operation::Exists(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::EXISTS_COST,
                        _max_gas: Self::EXISTS_COST,
                    },
                    Operation::TestVariant(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::TEST_VARIANT_COST,
                        _max_gas: Self::TEST_VARIANT_COST,
                    },
                    Operation::BorrowLoc => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::BORROW_LOC_COST,
                        _max_gas: Self::BORROW_LOC_COST,
                    },
                    Operation::BorrowField(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::BORROW_FIELD_COST,
                        _max_gas: Self::BORROW_FIELD_COST,
                    },
                    Operation::BorrowVariantField(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::BORROW_VARIANT_FIELD_COST,
                        _max_gas: Self::BORROW_VARIANT_FIELD_COST,
                    },
                    Operation::BorrowGlobal(..) => BytecodeCost {
                        min_inst_num: 3, // too much?
                        _max_inst_num: 3,
                        min_gas: Self::BORROW_GLOBAL_COST,
                        _max_gas: Self::BORROW_GLOBAL_COST,
                    },
                    // deemed as `Move`?
                    Operation::Drop | Operation::Release => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::DROP_RELEASE_COST,
                        _max_gas: Self::DROP_RELEASE_COST,
                    },
                    Operation::ReadRef => {
                        let dest_size = dests
                            .iter()
                            .map(|t| {
                                self.get_local_type(t)
                                    .estimate_size(self.target.global_env(), Self::VEC_SIZE)
                            })
                            .sum::<usize>();
                        BytecodeCost {
                            min_inst_num: 1,
                            _max_inst_num: 1,
                            min_gas: Self::READ_REF_COST + Self::BYTE_COST * dest_size,
                            _max_gas: usize::MAX,
                        }
                    },
                    Operation::WriteRef => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::WRITE_REF_COST,
                        _max_gas: Self::WRITE_REF_COST,
                    },
                    Operation::FreezeRef(..) => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::FREEZE_REF_COST,
                        _max_gas: Self::FREEZE_REF_COST,
                    },
                    Operation::Vector => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::VEC_PACK_COST + Self::VEC_ELE_COST * Self::VEC_SIZE,
                        _max_gas: usize::MAX,
                    },
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
                    | Operation::CastI256 => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::CAST_COST,
                        _max_gas: Self::CAST_COST,
                    },
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
                    | Operation::Gt => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::ARITH_LOGIC_COST,
                        _max_gas: Self::ARITH_LOGIC_COST,
                    },
                    Operation::Eq | Operation::Neq => BytecodeCost {
                        min_inst_num: 1,
                        _max_inst_num: 1,
                        min_gas: Self::EQUALITY_COST,
                        _max_gas: Self::EQUALITY_COST,
                    },
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
                    | Operation::EventStoreDiverge => BytecodeCost {
                        min_inst_num: 0,
                        _max_inst_num: 0,
                        min_gas: usize::MAX,
                        _max_gas: usize::MAX,
                    },
                }
            },
        }
    }
}

/// Data structure to help determine the qualification of instructions for CSE
enum BytecodeSanitizer {
    Pure,
    PureIfNoArithError,
    PureIfNoTypeError,
    LocalBorrow,
    ReadRef,
    Assign,
    Function(ModuleId, FunId),
    GlobalBorrow,
    Exists,
    Forbidden,
}

impl BytecodeSanitizer {
    /// Create a BytecodeSanitizer from a bytecode instruction
    /// It can be extended to support other impure instructions
    pub fn new_from_bytecode(inst: &Bytecode) -> Self {
        use BytecodeSanitizer::*;
        match inst {
            i if i.is_pure() => Pure,
            i if i.pure_if_no_arith_error() => PureIfNoArithError,
            i if i.pure_if_no_type_error() => PureIfNoTypeError,
            i if i.is_loc_borrowing() => LocalBorrow,
            Bytecode::Assign(_, _, _, AssignKind::Copy | AssignKind::Inferred) => Assign,
            Bytecode::Call(_, _, Operation::ReadRef, _, _) => ReadRef,
            Bytecode::Call(_, _, Operation::Exists(..), _, _) => Exists,
            Bytecode::Call(_, _, Operation::BorrowGlobal(..), _, _) => GlobalBorrow,
            Bytecode::Call(_, _, Operation::Function(mid, fid, _), _, _) => Function(*mid, *fid),
            _ => Forbidden,
        }
    }

    /// Check if the instruction is allowed to consider for CSE
    /// See the doc comments of different types for details
    pub fn is_allowed(&self, aggressive_mode: bool, function_target: &FunctionTarget) -> bool {
        match self {
            BytecodeSanitizer::Pure => true,
            BytecodeSanitizer::PureIfNoArithError => aggressive_mode,
            BytecodeSanitizer::PureIfNoTypeError => aggressive_mode,
            BytecodeSanitizer::LocalBorrow => aggressive_mode,
            BytecodeSanitizer::ReadRef => aggressive_mode,
            BytecodeSanitizer::Assign => aggressive_mode,
            BytecodeSanitizer::Function(mid, fid) => {
                aggressive_mode && Self::is_child_allowed(function_target, mid, fid)
            },
            BytecodeSanitizer::GlobalBorrow => aggressive_mode,
            BytecodeSanitizer::Exists => aggressive_mode,

            BytecodeSanitizer::Forbidden => false,
        }
    }

    /// Recursively check if the called function is allowed to consider for CSE
    /// Condition 1: the called function do not modify its parent's memories
    /// Condition 2: the called function and its deccendents do not access global storages
    pub fn is_child_allowed(function_target: &FunctionTarget, mid: &ModuleId, fid: &FunId) -> bool {
        let global_env = function_target.global_env();
        let module_env = global_env.get_module(*mid);
        let func_env = module_env.get_function(*fid);

        // helper check if the bytecode of a function accesses global storage
        let code_access_global = |qualified_fid: QualifiedId<FunId>| {
            let code = generate_bytecode(global_env, qualified_fid).code;
            code.is_empty() // no code available, conservatively assuming it accesses global storage
                    || code.iter().any(|inst| {
                        matches!(
                            inst,
                            Bytecode::Call(_, _, Operation::BorrowGlobal(..), _, _)
                                | Bytecode::Call(_, _, Operation::MoveFrom(..), _, _)
                                | Bytecode::Call(_, _, Operation::MoveTo(..), _, _)
                                | Bytecode::Call(_, _, Operation::Exists(..), _, _)
                        )
                    })
        };

        // Checks if any bytecode in the function or its transitive callees accesses global storage
        let accesses_global_resource =
            |func_env: &FunctionEnv, qualified_fid: QualifiedId<FunId>| {
                if code_access_global(qualified_fid) {
                    return true;
                }
                // now we need to check all the functions called or used (possibly called through `Invoke`) transitively by this function.
                func_env
                    .get_transitive_closure_of_used_functions()
                    .iter()
                    .any(|child_fid| {
                        let child_module_env = global_env.get_module(child_fid.module_id);
                        let child_func_env = child_module_env.get_function(child_fid.id);
                        // child is not native or inline, and its code accesses global storage
                        !child_func_env.is_native()
                            && !child_func_env.is_inline()
                            && code_access_global(*child_fid)
                    })
            };

        !func_env.is_mutating()
            && (func_env.is_native() // native functions do not access global storage
                || !accesses_global_resource(&func_env, mid.qualified(*fid)))
    }

    /// Further sanitize the non-pure expression represented by `expr_key` in the context of `function_target`
    /// See the doc comments of different types for details
    pub fn sanitize(&self, expr_key: &ExprKey, function_target: &FunctionTarget) -> bool {
        match self {
            BytecodeSanitizer::Pure => true,
            BytecodeSanitizer::PureIfNoArithError => true,
            BytecodeSanitizer::PureIfNoTypeError => true,
            BytecodeSanitizer::LocalBorrow => {
                let no_const_operands =
                    |const_vars: &Vec<(Constant, CodeOffset)>, _: &FunctionTarget| {
                        const_vars.is_empty()
                    };
                Self::sanitize_const_operands(expr_key, function_target, no_const_operands)
            },
            BytecodeSanitizer::ReadRef => true,
            BytecodeSanitizer::Assign => true,
            BytecodeSanitizer::Function(..) => true,
            BytecodeSanitizer::GlobalBorrow => true,
            BytecodeSanitizer::Exists => true,
            BytecodeSanitizer::Forbidden => false,
        }
    }

    // Helper to sanitize all temp operands used in `expr_key` with a given predicate
    #[allow(dead_code)]
    fn sanitize_var_operands<F>(
        expr_key: &ExprKey,
        function_target: &FunctionTarget,
        predicate: F,
    ) -> bool
    where
        F: Fn(&Vec<(TempIndex, CodeOffset)>, &FunctionTarget) -> bool,
    {
        predicate(&expr_key.collect_operands().0, function_target)
    }

    // Helper to sanitize all constant operands used in `expr_key` with a given predicate
    fn sanitize_const_operands<F>(
        expr_key: &ExprKey,
        function_target: &FunctionTarget,
        predicate: F,
    ) -> bool
    where
        F: Fn(&Vec<(Constant, CodeOffset)>, &FunctionTarget) -> bool,
    {
        predicate(&expr_key.collect_operands().1, function_target)
    }

    /// Sanitize the expr, after expanding the args, with a given predicate
    #[allow(dead_code)]
    fn sanitize_exprs<F>(expr_key: &ExprKey, target: &FunctionTarget, predicate: F) -> bool
    where
        F: Fn(&Vec<ExpOp>, &FunctionTarget) -> bool,
    {
        // collect the experssions involved in the `expr` in a depth-first order
        fn dfs(expr: &ExprKey, visited: &mut Vec<ExpOp>) {
            visited.push(expr.op.clone());
            for arg in expr.args.iter() {
                if let ExpArg::Expr(arg_expr) = arg {
                    dfs(arg_expr.as_ref(), visited);
                }
            }
        }

        let mut expr_seq_dfs = Vec::new();
        dfs(expr_key, &mut expr_seq_dfs);

        predicate(&expr_seq_dfs, target)
    }
}
