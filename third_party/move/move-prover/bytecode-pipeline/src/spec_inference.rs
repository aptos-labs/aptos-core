// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Spec Inference via Weakest Precondition Analysis
//!
//! This module automatically infers formal specifications (`ensures`, `aborts_if`,
//! `modifies`) for Move functions that have empty spec blocks. The result is a
//! fully precise specification that can then be verified by the Boogie backend or
//! displayed to the user as documentation.
//!
//! # Approach
//!
//! The core idea is *backward symbolic execution*: starting from the function's
//! return points, we propagate a symbolic state backward through the bytecode,
//! building up the weakest precondition (WP) that describes what the function
//! does.
//!
//! At each return instruction, the initial WP state records `result_i == $t`
//! for every return value temporary `$t`. As the analysis walks backward through
//! assignments and operations, it substitutes temporaries with their defining
//! expressions until only function parameters remain. The final state at the
//! entry point becomes the inferred spec.
//!
//! For **function calls**, the WP uses *behavioral predicates* (`result_of`,
//! `ensures_of`, `aborts_of`) rather than inlining the callee's spec. This
//! keeps the inferred conditions modular: the caller's spec says "the result
//! is whatever `f` returns" without committing to `f`'s implementation.
//!
//! For **abort conditions**, each operation that can abort (arithmetic overflow,
//! missing resource, wrong variant, etc.) adds its abort predicate to the WP
//! state. These propagate backward alongside the ensures conditions and appear
//! as `aborts_if` clauses in the final spec.
//!
//! # WP State
//!
//! The [`WPState`] at each program point carries:
//!
//! - **ensures** — conditions that hold on normal return (eventually become
//!   `ensures` clauses).
//! - **aborts** — conditions under which the function aborts (become
//!   `aborts_if` clauses).
//! - **post** — a memory label identifying the post-state for state-chaining
//!   across multiple calls.
//! - **captured_mut_params** — set of `&mut` parameter indices that have been
//!   written to (tracks which params need `ensures p == ...`).
//! - **captured_globals** — set of temps representing `borrow_global_mut` results
//!   that have been written back (tracks which globals need `modifies` clauses).
//! - **direct_modifies** — explicit modifies targets from `MoveTo`/`MoveFrom`.
//!
//! # Post-Processing Pipeline
//!
//! After the backward fixpoint converges at the function entry, several
//! post-processing steps clean up the raw WP:
//!
//! 1. **Label stripping** — removes memory labels that correspond to the
//!    function's implicit entry/exit states (labels are only meaningful for
//!    intermediate call boundaries).
//! 2. **Orphaned pre-label removal** — drops behavioral predicate labels
//!    whose post-label isn't defined by any predicate.
//! 3. **IsParent resolution** — replaces `is_parent` temps with path conditions
//!    computed via dominator-tree analysis.
//! 4. **Unmodified &mut params** — adds `ensures p == old(p)` for `&mut`
//!    parameters that were never written to.
//! 5. **Captured global resolution** — substitutes borrow temps in the WP
//!    with `global<R>(addr)` expressions and strips labels inside `old()`.
//! 6. **Simplification** — constant folding, boolean/arithmetic identities,
//!    tautology removal.
//! 7. **Spec update** — attaches the simplified conditions to the function's
//!    spec as `[inferred]` properties, and emits `modifies` clauses.
//!
//! # Control Flow
//!
//! The analysis uses a topological-order traversal of the backward CFG (from
//! exit to entry). At branch join points, a *path-conditional join* merges
//! the two sides under the branch condition (`if c then Q_true else Q_false`),
//! preserving path sensitivity.
//!
//! Loops are handled by the pipeline stages that run before this processor:
//! `LoopAnalysisProcessor` unrolls loops and inserts `Havoc` instructions for
//! modified variables. The WP for `Havoc(x)` universally quantifies `x` in the
//! ensures and existentially quantifies it in the aborts, effectively abstracting
//! over all possible loop iterations.
//!
//! Per-function control is available via pragmas inside an empty spec block:
//!
//! ```move
//! fun my_fun() { ... }
//! spec my_fun {
//!     pragma inference = "only_ensures"; // skip aborts_if inference
//!     // pragma inference = "only_aborts"; // skip ensures inference
//! }
//! ```
//!
//! See `move-prover/src/inference.rs` for command-line usage.

use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{
        BehaviorState, Condition, ConditionKind, Exp, ExpData, MemoryLabel, Operation as AstOp,
        Pattern, PropertyValue, QuantKind, RewriteResult, TempIndex, Value,
    },
    exp_generator::{ExpGenerator, RangeCheckKind},
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    exp_simplifier::ExpSimplifier,
    model::{FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, QualifiedId, StructEnv, StructId},
    pragmas::INFERENCE_PRAGMA,
    sourcifier::Sourcifier,
    symbol::Symbol,
    ty::{PrimitiveType, Type, BOOL_TYPE, NUM_TYPE},
};
use move_stackless_bytecode::{
    dataflow_analysis::{BlockState, DataflowAnalysis, StateMap, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    graph::{DomRelation, Graph},
    stackless_bytecode::{
        AbortAction, BorrowEdge, BorrowNode, Bytecode, Constant, Label, Operation, PropKind,
    },
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use num::{bigint::Sign, BigInt, Zero};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

/// Property name used to mark inferred spec conditions
const INFERRED_PROPERTY: &str = "inferred";

/// Symbol value for inferred conditions that are vacuously strong (unconstrained quant vars)
const VACUOUS_VALUE: &str = "vacuous";

/// Symbol value for inferred conditions with quantifiers hard for SAT/SMT solvers
const SATHARD_VALUE: &str = "sathard";

// =================================================================================================
// WP State and Annotation

/// State at a program point during WP analysis.
/// For backward analysis, state flows from successors to predecessors.
/// Also used as the annotation type for bytecode dumps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WPState {
    /// The ensures conditions - what must be true for normal return
    pub ensures: Vec<Exp>,
    /// The aborts conditions - under what conditions the function can abort
    pub aborts: Vec<Exp>,
    /// Code offset this state originated from (for edge tracking during joins).
    /// Used to identify which branch edge a state came from.
    pub origin_offset: Option<CodeOffset>,
    /// Post-state label: memory state after operations at this point.
    /// In backward analysis, this represents the state that successor operations see.
    pub post: MemoryLabel,
    /// Tracks which `&mut` parameters have already had their final value captured.
    /// In backward analysis, the first write encountered (last in execution) captures the
    /// final value. Subsequent writes encountered (earlier in execution) are skipped.
    pub captured_mut_params: BTreeSet<TempIndex>,
    /// Tracks temps originating from `borrow_global_mut` whose final value has been captured.
    /// These follow the same lifecycle as `&mut` parameters: they get written to and written
    /// back via `WriteBack(GlobalRoot)`. The BorrowGlobal handler (processed last in backward
    /// order) resolves the temp to `global<R>(addr)`.
    pub captured_globals: BTreeSet<TempIndex>,
    /// Tracks globals directly modified by MoveFrom/MoveTo (which bypass the borrow+writeback path).
    /// Each entry is a `global<R>(addr)` expression (no label) used to emit `modifies` clauses.
    pub direct_modifies: Vec<Exp>,
}

impl WPState {
    /// Create a new WPState with the given post-state label
    fn new(post: MemoryLabel) -> Self {
        Self {
            ensures: vec![],
            aborts: vec![],
            origin_offset: None,
            post,
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            direct_modifies: vec![],
        }
    }

    /// Create a state with a single aborts condition
    fn with_aborts(exp: Exp, post: MemoryLabel) -> Self {
        Self {
            ensures: vec![],
            aborts: vec![exp],
            origin_offset: None,
            post,
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            direct_modifies: vec![],
        }
    }

    /// Transform conditions (e.g., for substitution)
    fn map(&self, mut f: impl FnMut(&Exp) -> Exp) -> Self {
        Self {
            ensures: self.ensures.iter().map(&mut f).collect(),
            aborts: self.aborts.iter().map(&mut f).collect(),
            origin_offset: self.origin_offset,
            post: self.post,
            captured_mut_params: self.captured_mut_params.clone(),
            captured_globals: self.captured_globals.clone(),
            direct_modifies: self.direct_modifies.iter().map(&mut f).collect(),
        }
    }

    /// Check if this state is empty (no conditions)
    fn is_empty(&self) -> bool {
        self.ensures.is_empty() && self.aborts.is_empty()
    }

    /// Clear origin tracking (used after joins to avoid stale tracking)
    fn clear_origin(&mut self) {
        self.origin_offset = None;
    }

    /// Add an ensures condition if a structurally equivalent one doesn't already exist
    fn add_ensures(&mut self, exp: Exp) {
        push_if_new(&mut self.ensures, exp);
    }

    /// Add an aborts condition if a structurally equivalent one doesn't already exist
    fn add_aborts(&mut self, exp: Exp) {
        push_if_new(&mut self.aborts, exp);
    }

    /// Add a direct modifies target if a structurally equivalent one doesn't already exist.
    /// Used for MoveFrom/MoveTo which modify globals without the borrow+writeback path.
    fn add_direct_modifies(&mut self, exp: Exp) {
        push_if_new(&mut self.direct_modifies, exp);
    }
}

/// Strip `old()` wrappers from an expression.
/// This is used for aborts conditions which are implicitly evaluated in pre-state.
fn strip_old(exp: &Exp) -> Exp {
    struct OldStripper;

    impl ExpRewriterFunctions for OldStripper {
        fn rewrite_call(&mut self, _id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
            if matches!(oper, AstOp::Old) && args.len() == 1 {
                // Unwrap old(e) to just e (recursively rewritten)
                Some(self.rewrite_exp(args[0].clone()))
            } else {
                None
            }
        }
    }

    OldStripper.rewrite_exp(exp.clone())
}

impl AbstractDomain for WPState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let old_ensures_len = self.ensures.len();
        let old_aborts_len = self.aborts.len();
        let old_captured_len = self.captured_mut_params.len();

        // Abort-only states (empty ensures) come from user-written `abort` statements
        // or `Stop` in loop bodies. In these cases the abort conditions are already
        // captured analytically in the transfer function. Skip abort-only states to
        // avoid:
        // - ensures intersection removing ensures from the normal return path
        // - `aborts: true` creating spurious path-conditional aborts at Branch joins
        //
        // Note: Abort handler blocks (from `on_abort goto`) are neutralized before
        // analysis and never reach this point.
        let self_is_abort_only = self.ensures.is_empty();
        let other_is_abort_only = other.ensures.is_empty();

        if self_is_abort_only && !other_is_abort_only {
            // Current is abort-only; adopt incoming state wholesale
            self.ensures = other.ensures.clone();
            self.aborts = other.aborts.clone();
        } else if !other_is_abort_only {
            // Both have ensures (both return normally): standard join
            self.ensures
                .retain(|exp| ensures_contains(&other.ensures, exp));
            for exp in &other.aborts {
                if !ensures_contains(&self.aborts, exp) {
                    self.aborts.push(exp.clone());
                }
            }
        }
        // If other is abort-only, skip it entirely (keep self as-is)

        // For captured_mut_params: use union semantics (if captured on any path, it's captured)
        // This is correct because in backward analysis, if a param was written to on any path,
        // we've already captured its final value and shouldn't add another ensures for it.
        for idx in &other.captured_mut_params {
            self.captured_mut_params.insert(*idx);
        }

        // For captured_globals: same union semantics as captured_mut_params.
        let old_captured_globals_len = self.captured_globals.len();
        for idx in &other.captured_globals {
            self.captured_globals.insert(*idx);
        }

        // For direct_modifies: union semantics (modification from ANY path counts)
        let old_direct_modifies_len = self.direct_modifies.len();
        for exp in &other.direct_modifies {
            push_if_new(&mut self.direct_modifies, exp.clone());
        }

        if self.ensures.len() != old_ensures_len
            || self.aborts.len() != old_aborts_len
            || self.captured_mut_params.len() != old_captured_len
            || self.captured_globals.len() != old_captured_globals_len
            || self.direct_modifies.len() != old_direct_modifies_len
        {
            JoinResult::Changed
        } else {
            JoinResult::Unchanged
        }
    }
}

/// Annotation which can be attached to function data containing WP analysis results.
#[derive(Default, Clone)]
pub struct WPAnnotation(pub BTreeMap<CodeOffset, WPState>);

impl WPAnnotation {
    /// Get the WP state at a specific code offset.
    pub fn get_wp_at(&self, code_offset: CodeOffset) -> Option<&WPState> {
        self.0.get(&code_offset)
    }
}

/// Format a WP annotation for display in bytecode dumps.
pub fn format_wp_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    if let Some(WPAnnotation(map)) = target.get_annotations().get::<WPAnnotation>() {
        if let Some(state) = map.get(&code_offset) {
            let env = target.global_env();
            let mut parts = vec![];

            if !state.ensures.is_empty() {
                let ensures_str = state
                    .ensures
                    .iter()
                    .map(|e| format!("{}", e.as_ref().display(env)))
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("ensures: {}", ensures_str));
            }

            if !state.aborts.is_empty() {
                let aborts_str = state
                    .aborts
                    .iter()
                    .map(|e| format!("{}", e.as_ref().display(env)))
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("aborts: {}", aborts_str));
            }

            if !parts.is_empty() {
                return Some(format!("wp: {{ {} }}", parts.join("; ")));
            }
        }
    }
    None
}

// =================================================================================================
// Branch Info for Path-Conditional Joining

/// Push an expression to a list if a structurally equivalent one doesn't already exist.
fn push_if_new(list: &mut Vec<Exp>, exp: Exp) {
    if !list.iter().any(|e| e.structural_eq(&exp)) {
        list.push(exp);
    }
}

/// Deduplicate a list of expressions by structural equality.
fn deduplicate_exps(exps: Vec<Exp>) -> Vec<Exp> {
    let mut deduped = Vec::new();
    for e in exps {
        push_if_new(&mut deduped, e);
    }
    deduped
}

/// Check if a list of Exps contains one structurally equivalent to the target.
fn ensures_contains(list: &[Exp], target: &Exp) -> bool {
    list.iter().any(|e| e.as_ref().structural_eq(target))
}

/// Combine complementary path-conditional aborts in a disjunctive list.
/// If both `P && Q` and `!P && Q` appear, replace them with `Q`,
/// since `(P && Q) || (!P && Q)` ≡ `Q`.
fn combine_complementary_aborts(aborts: &[Exp]) -> Vec<Exp> {
    /// If `exp` is `And(lhs, rhs)`, return `(lhs, rhs)`.
    fn as_and(exp: &Exp) -> Option<(&Exp, &Exp)> {
        match exp.as_ref() {
            ExpData::Call(_, AstOp::And, args) if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }
    /// Check if `a` is the negation of `b` (structurally: `Not(b)` ≡ `a` or `Not(a)` ≡ `b`).
    fn is_negation(a: &Exp, b: &Exp) -> bool {
        match a.as_ref() {
            ExpData::Call(_, AstOp::Not, args) if args.len() == 1 => {
                args[0].as_ref().structural_eq(b)
            },
            _ => match b.as_ref() {
                ExpData::Call(_, AstOp::Not, args) if args.len() == 1 => {
                    args[0].as_ref().structural_eq(a)
                },
                _ => false,
            },
        }
    }

    let mut result: Vec<Exp> = Vec::new();
    let mut consumed: Vec<bool> = vec![false; aborts.len()];
    for i in 0..aborts.len() {
        if consumed[i] {
            continue;
        }
        if let Some((cond_i, body_i)) = as_and(&aborts[i]) {
            // Look for a complement: `!cond_i && body_i`
            let mut found = false;
            for j in (i + 1)..aborts.len() {
                if consumed[j] {
                    continue;
                }
                if let Some((cond_j, body_j)) = as_and(&aborts[j]) {
                    if body_i.as_ref().structural_eq(body_j) && is_negation(cond_i, cond_j) {
                        // Found complement pair — emit just the body
                        result.push(body_i.clone());
                        consumed[i] = true;
                        consumed[j] = true;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                result.push(aborts[i].clone());
            }
        } else {
            result.push(aborts[i].clone());
        }
    }
    result
}

/// Information about a Branch instruction for path-conditional joining
struct BranchInfo {
    /// The condition temporary
    cond_temp: TempIndex,
    /// Code offset of the true branch target
    true_target_offset: CodeOffset,
    /// Code offset of the false branch target
    false_target_offset: CodeOffset,
}

// =================================================================================================
// Spec Inference Processor

/// A processor that infers specifications for functions with empty spec blocks.
pub struct SpecInferenceProcessor {
    /// Whether to store the WPAnnotation in the function data for dump output.
    annotate: bool,
}

impl SpecInferenceProcessor {
    pub fn new(annotate: bool) -> Box<Self> {
        Box::new(Self { annotate })
    }
}

impl FunctionTargetProcessor for SpecInferenceProcessor {
    fn name(&self) -> String {
        "spec_inference".to_string()
    }

    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        // Skip native/intrinsic functions
        if fun_env.is_native() || fun_env.is_intrinsic() {
            return data;
        }

        // Skip variants with empty code (e.g., baseline cleared by SpecInstrumentation)
        if data.code.is_empty() {
            return data;
        }

        // Only run inference on Verification variants (which come out of
        // SpecInstrumentation with fully instrumented code). The Baseline
        // variant may have been cleared. The spec is shared across variants,
        // so we must run exactly once per function.
        if !data.variant.is_verified() {
            return data;
        }

        // Check if spec block is empty and needs inference
        if !needs_inference(fun_env) {
            return data;
        }

        // Run the spec inference analysis
        let mut analyzer = SpecInferenceAnalyzer::new(fun_env, &data);
        let (wp_map, has_skipped_blocks) = analyzer.analyze();

        // If the backward analysis skipped blocks due to cycles (unprocessed loops),
        // the WP is incomplete. Report this and skip spec inference.
        if has_skipped_blocks {
            fun_env.module_env.env.diag(
                Severity::Bug,
                &fun_env.get_loc(),
                "unexpected loss of weakest precondition: \
                 loops in backward CFG prevented complete analysis",
            );
        } else {
            // Get WP at entry point and update spec
            // By construction, for well-typed code the WP at entry should only reference parameters.
            // If this invariant is violated, it indicates a bug in spec inference.
            let entry_state = wp_map.get(&0).or_else(|| wp_map.get(&1));
            if let Some(state) = entry_state {
                // Remove implicit labels (at_start and at_end) before updating spec.
                // These labels represent the initial and final states which are implicit
                // in the ensures/aborts_if semantics.
                // The entry state's `post` label is the function's pre-state: it may differ
                // from `offset_labels[0]` when the first label-creating instruction isn't at
                // offset 0 (e.g., a borrow_field precedes the first call).
                let entry_post_label = state.post;
                let at_start_label = analyzer.offset_labels.borrow().get(&0).copied();
                let at_end_label = analyzer.at_end_label;
                let mutation_labels = analyzer.mutation_labels.borrow().clone();
                let mut state = analyzer.substitute_labels_in_state(state, &|label| {
                    if mutation_labels.contains(&label) {
                        None // Keep intermediate mutation labels
                    } else if Some(label) == at_start_label
                        || label == at_end_label
                        || label == entry_post_label
                    {
                        Some(None) // Remove the label
                    } else {
                        None // Keep unchanged
                    }
                });

                // Strip orphaned pre-labels on behavioral predicates: if a pre-label
                // references a post-label that no behavioral predicate defines,
                // remove it. This happens when a call's result is unused (no result_of)
                // so no behavioral predicate carries the post-label definition.
                state = analyzer.strip_orphaned_behavior_pre_labels(state);

                // Resolve is_parent temporaries: substitute them with their path conditions
                // computed via dominator tree analysis.
                let bytecode = analyzer.target.get_bytecode();
                let is_parent_subs = analyzer.compute_is_parent_substitutions(bytecode);
                if !is_parent_subs.is_empty() {
                    state = analyzer.resolve_is_parent_in_state(&state, &is_parent_subs);
                }

                // For &mut params that were never written on any path, add `ensures param == old(param)`.
                // This captures the fact that unmodified reference parameters retain their original value.
                let num_params = fun_env.get_parameter_count();
                for idx in 0..num_params {
                    let ty = analyzer.get_local_type(idx);
                    if ty.is_mutable_reference() && !state.captured_mut_params.contains(&idx) {
                        let param_exp = analyzer.mk_temporary(idx);
                        let old_param = analyzer.mk_old(param_exp.clone());
                        state.add_ensures(analyzer.mk_eq(param_exp, old_param));
                    }
                }

                // Resolve borrow temps from captured globals.
                // On exit paths, the BorrowGlobal handler may not have been reached
                // (it only appears on the loop body path), leaving unresolved
                // Temporary(idx) or Freeze(Temporary(idx)) references to borrow temps.
                // Substitute them with the corresponding global<R>(addr).
                let captured_globals: Vec<TempIndex> =
                    state.captured_globals.iter().copied().collect();
                for &temp in &captured_globals {
                    if let Some((mid, sid, targs, addr_temp)) =
                        analyzer.borrow_global_info.get(&temp).cloned()
                    {
                        let struct_env = analyzer.get_struct(mid, sid);
                        let addr_exp = analyzer.mk_temporary(addr_temp);
                        let global_exp = analyzer.mk_global(&struct_env, &targs, addr_exp);
                        // Replace patterns referencing the borrow temp with global<R>(addr):
                        // - Freeze(Temporary(temp)) → global<R>(addr)
                        // - bare Temporary(temp) → global<R>(addr)
                        // - Old(Temporary(temp)) → Old(global<R>(addr))
                        let temp_exp = analyzer.mk_temporary(temp);
                        let global_node_type = analyzer.global_env().get_node_type(
                            if let ExpData::Call(id, ..) = global_exp.as_ref() {
                                *id
                            } else {
                                unreachable!()
                            },
                        );
                        let freeze_id = analyzer.new_node(global_node_type, None);
                        let freeze_exp =
                            ExpData::Call(freeze_id, AstOp::Freeze(false), vec![temp_exp.clone()])
                                .into_exp();
                        let old_temp_exp = analyzer.mk_old(temp_exp.clone());
                        let old_global_exp = analyzer.mk_old(global_exp.clone());
                        state = state.map(|e| {
                            let e = analyzer.substitute_exp_with_exp(e, &freeze_exp, &global_exp);
                            let e = analyzer.substitute_exp_with_exp(
                                &e,
                                &old_temp_exp,
                                &old_global_exp,
                            );
                            // Use substitute_temp_with_exp for bare Temporary — substitute_exp_with_exp
                            // only matches Call patterns and would miss Temporary nodes.
                            analyzer.substitute_temp_with_exp(&e, temp, &global_exp)
                        });
                    }
                }

                // Strip memory labels inside old() wrappers.
                // BorrowGlobal substitution inserts the state.post label everywhere,
                // including inside old(). Labels inside old() are semantically wrong
                // (old() already refers to function entry state).
                state = analyzer.strip_labels_inside_old(&state);

                // Simplify conditions: constant folding, arithmetic/boolean
                // identities, and assumption-based redundancy elimination.
                state = simplify_state(&mut analyzer, &state);

                if !state.is_empty() {
                    update_spec(fun_env, &state, &mut analyzer);
                    // Emit modifies clauses for all captured globals
                    emit_modifies(fun_env, &state);
                } else {
                    // Entry state is empty but there may be non-empty WP states at intermediate
                    // offsets, indicating that weakest preconditions were lost during joins.
                    let has_non_empty_wp = wp_map.values().any(|s| !s.is_empty());
                    if has_non_empty_wp {
                        fun_env.module_env.env.diag(
                            Severity::Bug,
                            &fun_env.get_loc(),
                            "unexpected loss of weakest precondition: \
                             intermediate WP states exist but did not propagate to entry",
                        );
                    }
                }
            } else {
                // No entry state at all but there may be non-empty WP states at other offsets.
                let has_non_empty_wp = wp_map.values().any(|s| !s.is_empty());
                if has_non_empty_wp {
                    fun_env.module_env.env.diag(
                        Severity::Bug,
                        &fun_env.get_loc(),
                        "unexpected loss of weakest precondition: \
                         intermediate WP states exist but did not propagate to entry",
                    );
                }
            }
        }

        // Store the WP annotation if requested (for test/debug dump output)
        drop(analyzer);
        if self.annotate {
            data.annotations
                .set::<WPAnnotation>(WPAnnotation(wp_map), true);
        }

        data
    }

    fn dump_result(
        &self,
        f: &mut fmt::Formatter,
        env: &GlobalEnv,
        _targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        writeln!(f, "\n\n==== spec-inference results ====\n")?;

        let inferred_sym = env.symbol_pool().make(INFERRED_PROPERTY);

        // Use Sourcifier to print functions with their inferred specs
        let sourcifier = Sourcifier::new(env, true);

        for module in env.get_modules() {
            if !module.is_target() {
                continue;
            }

            for fun in module.get_functions() {
                if fun.is_native() || fun.is_intrinsic() {
                    continue;
                }

                let spec = fun.get_spec();

                // Check if any conditions were inferred (marked with "inferred" property)
                let has_inferred = spec
                    .conditions
                    .iter()
                    .any(|c| c.properties.contains_key(&inferred_sym));

                if has_inferred {
                    // Print the entire function (signature + body + spec)
                    sourcifier.print_fun(fun.get_qualified_id(), fun.get_def());

                    // Check for conditions referencing non-parameter temporaries
                    let num_params = fun.get_parameter_count();
                    let has_bad_temps = spec.conditions.iter().any(|c| {
                        c.properties.contains_key(&inferred_sym)
                            && !exp_only_references_params(&c.exp, num_params)
                    });
                    if has_bad_temps {
                        env.diag(
                            Severity::Bug,
                            &fun.get_loc(),
                            "inferred spec references non-parameter temporaries",
                        );
                    }
                }
            }
        }

        write!(f, "{}", sourcifier.result())?;
        Ok(())
    }
}

// =================================================================================================
// Helper Functions

/// Checks if a function needs spec inference
fn needs_inference(fun_env: &FunctionEnv) -> bool {
    !fun_env.is_opaque()
}

/// Updates the function spec with inferred conditions from WPState
fn update_spec<'env>(
    fun_env: &FunctionEnv,
    state: &WPState,
    generator: &mut impl ExpGenerator<'env>,
) {
    let env = fun_env.module_env.env;
    let pool = env.symbol_pool();
    let inferred_sym = pool.make(INFERRED_PROPERTY);
    let vacuous_sym = pool.make(VACUOUS_VALUE);
    let sathard_sym = pool.make(SATHARD_VALUE);
    let loc = fun_env.get_loc();

    // Read the inference pragma to decide what to emit.
    let infer_ensures;
    let infer_aborts;
    if let Some(mode) = fun_env.get_symbol_pragma(INFERENCE_PRAGMA) {
        let mode_str = pool.string(mode);
        infer_ensures = mode_str.as_str() != "only_aborts";
        infer_aborts = mode_str.as_str() != "only_ensures";
    } else {
        infer_ensures = true;
        infer_aborts = true;
    }

    let mut spec = fun_env.get_mut_spec();

    let mk_cond = |kind: ConditionKind, exp: &Exp| {
        let is_vacuous = has_unconstrained_quant_var(exp);
        let is_sathard = !is_vacuous && has_top_level_quantifier(exp);
        let inferred_value = if is_vacuous {
            PropertyValue::Symbol(vacuous_sym)
        } else if is_sathard {
            PropertyValue::Symbol(sathard_sym)
        } else {
            PropertyValue::Value(Value::Bool(true))
        };
        let properties = BTreeMap::from([(inferred_sym, inferred_value)]);
        Condition {
            loc: loc.clone(),
            kind,
            properties,
            exp: exp.clone(),
            additional_exps: vec![],
        }
    };

    // Add each ensures condition separately, filtering out trivial `true` conditions.
    if infer_ensures {
        let ensures_conds: Vec<_> = state
            .ensures
            .iter()
            .filter(|e| !is_trivial_true(e))
            .collect();
        spec.conditions.extend(
            ensures_conds
                .iter()
                .map(|e| mk_cond(ConditionKind::Ensures, e)),
        );
    }

    // Add each aborts condition separately, filtering out trivial `true` conditions
    // (which would incorrectly claim the function always aborts).
    // Strip `old()` wrappers since aborts_if is implicitly evaluated in pre-state,
    // then re-simplify to catch tautologies introduced by stripping (e.g., `r == r`
    // from `Old(r) == Old(r)`).
    if infer_aborts {
        let aborts_conds: Vec<_> = state
            .aborts
            .iter()
            .filter(|e| !is_trivial_true(e) && !is_trivial_false(e))
            .map(strip_old)
            .map(|e| {
                let mut s = ExpSimplifier::new(generator);
                s.simplify(e)
            })
            .filter(|e| !is_trivial_true(e) && !is_trivial_false(e))
            .collect();
        spec.conditions.extend(
            aborts_conds
                .iter()
                .map(|e| mk_cond(ConditionKind::AbortsIf, e)),
        );
    }
}

/// Emit modifies clauses for globals modified in the inferred spec.
/// Scans ensures conditions for `global<R>(addr)` (post-state, no label) on the LHS
/// of equality patterns and emits a modifies clause for each unique one.
fn emit_modifies(fun_env: &FunctionEnv, state: &WPState) {
    let env = fun_env.module_env.env;
    let inferred_sym = env.symbol_pool().make(INFERRED_PROPERTY);
    let loc = fun_env.get_loc();
    let properties = BTreeMap::from([(inferred_sym, PropertyValue::Value(Value::Bool(true)))]);

    let mut modifies_targets: Vec<Exp> = Vec::new();

    // From borrow_global_mut WriteBack path: scan ensures for global<R>(addr) on LHS of Eq
    if !state.captured_globals.is_empty() {
        for ensures in &state.ensures {
            collect_modifies_targets(ensures, &mut modifies_targets);
        }
    }

    // From MoveFrom/MoveTo direct path
    for target in &state.direct_modifies {
        let stripped = strip_labels_in_exp(target);
        push_if_new(&mut modifies_targets, stripped);
    }

    if !modifies_targets.is_empty() {
        let mut spec = fun_env.get_mut_spec();
        for target in modifies_targets {
            spec.conditions.push(Condition {
                loc: loc.clone(),
                kind: ConditionKind::Modifies,
                properties: properties.clone(),
                exp: target,
                additional_exps: vec![],
            });
        }
    }
}

/// Collect `global<R>(addr)` expressions from an ensures condition that represent
/// modified globals (post-state or intermediate labeled state). Handles patterns:
/// - `Eq(global<R>(addr), ...)`  — direct ensures
/// - `Eq(global[@label]<R>(addr), ...)` — intermediate state ensures
/// - `Implies(cond, ...)` — path-conditional ensures (recurse into body)
fn collect_modifies_targets(exp: &Exp, targets: &mut Vec<Exp>) {
    match exp.as_ref() {
        ExpData::Call(_, AstOp::Implies, args) if args.len() == 2 => {
            collect_modifies_targets(&args[1], targets);
        },
        ExpData::Call(_, AstOp::Eq, args) if args.len() == 2 => {
            // Check if the LHS is a Global (post-state or intermediate labeled state)
            if let ExpData::Call(_, AstOp::Global(_), _) = args[0].as_ref() {
                // Strip labels to get the canonical modifies target: global<R>(addr)
                let stripped = strip_labels_in_exp(&args[0]);
                push_if_new(targets, stripped);
            }
        },
        _ => {},
    }
}

/// Check if an expression is a trivial boolean `true` literal
fn is_trivial_true(exp: &Exp) -> bool {
    matches!(exp.as_ref(), ExpData::Value(_, Value::Bool(true)))
}

/// An entity determined by an ensures clause: either a temporary or a global expression.
#[derive(Debug)]
enum DeterminedEntity {
    Temp(TempIndex),
    Global(Exp),
}

/// If `exp` is `Eq(entity, _)` or `Eq(_, entity)` where entity is a `Temporary(idx)` or
/// `Call(Global(_), _)`, return the determined entity. Used to detect ensures that fully
/// determine a result variable or global.
fn ensures_determines_entity(exp: &Exp) -> Option<DeterminedEntity> {
    if let ExpData::Call(_, AstOp::Eq, args) = exp.as_ref() {
        if args.len() == 2 {
            for arg in args {
                match arg.as_ref() {
                    ExpData::Temporary(_, idx) => return Some(DeterminedEntity::Temp(*idx)),
                    ExpData::Call(_, AstOp::Global(_), _) => {
                        return Some(DeterminedEntity::Global(arg.clone()))
                    },
                    _ => {},
                }
            }
        }
    }
    None
}

/// Check if two determined entities match.
fn entities_match(a: &DeterminedEntity, b: &DeterminedEntity) -> bool {
    match (a, b) {
        (DeterminedEntity::Temp(i), DeterminedEntity::Temp(j)) => i == j,
        (DeterminedEntity::Global(e1), DeterminedEntity::Global(e2)) => e1.structural_eq(e2),
        _ => false,
    }
}

/// Check if an expression is a trivial boolean `false` literal
fn is_trivial_false(exp: &Exp) -> bool {
    matches!(exp.as_ref(), ExpData::Value(_, Value::Bool(false)))
}

/// Strip all memory labels from Global and Exists operations in an expression.
/// `Global(Some(label))` → `Global(None)`, `Exists(Some(label))` → `Exists(None)`.
fn strip_labels_in_exp(exp: &Exp) -> Exp {
    struct LabelStripper;

    impl ExpRewriterFunctions for LabelStripper {
        fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
            match oper {
                AstOp::Global(Some(_)) => {
                    Some(ExpData::Call(id, AstOp::Global(None), args.to_vec()).into_exp())
                },
                AstOp::Exists(Some(_)) => {
                    Some(ExpData::Call(id, AstOp::Exists(None), args.to_vec()).into_exp())
                },
                _ => None,
            }
        }
    }

    LabelStripper.rewrite_exp(exp.clone())
}

/// Check if a top-level quantifier has any quantified variable that is unconstrained.
/// A variable is unconstrained if it does not co-occur with any non-quantified free variable
/// (i.e., a function parameter or outer variable) in a constraint context.
///
/// For `forall`: constraint contexts are antecedents of the implication chain.
/// For `exists`: constraint contexts are conjuncts of the body.
///
/// Examples:
/// - `forall x: x <= 0 ==> result == x` — antecedent `x <= 0` has no non-quant vars → unconstrained
/// - `forall x: x <= n ==> ...` — antecedent `x <= n` has non-quant `n` → constrained
/// - `exists x: !in_range(0..MAX, x - 1)` — no non-quant vars → unconstrained
/// - `exists x: x <= n && !in_range(...)` — conjunct `x <= n` has non-quant `n` → constrained
fn has_unconstrained_quant_var(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Quant(_, QuantKind::Forall, ranges, _, _, body) => {
            let quant_syms: BTreeSet<Symbol> = ranges
                .iter()
                .filter_map(|(pat, _)| {
                    if let Pattern::Var(_, sym) = pat {
                        Some(*sym)
                    } else {
                        None
                    }
                })
                .collect();
            quant_syms
                .iter()
                .any(|sym| !sym_constrained_with_external(body, *sym, &quant_syms, true))
        },
        ExpData::Quant(_, QuantKind::Exists, ranges, _, _, body) => {
            let quant_syms: BTreeSet<Symbol> = ranges
                .iter()
                .filter_map(|(pat, _)| {
                    if let Pattern::Var(_, sym) = pat {
                        Some(*sym)
                    } else {
                        None
                    }
                })
                .collect();
            quant_syms
                .iter()
                .any(|sym| !sym_constrained_with_external(body, *sym, &quant_syms, false))
        },
        _ => false,
    }
}

/// Checks if the expression has a top-level quantifier (Forall or Exists).
/// Such expressions are hard for SAT/SMT solvers when they appear in certain
/// spec positions (exists in aborts_if, forall in ensures).
fn has_top_level_quantifier(exp: &Exp) -> bool {
    matches!(exp.as_ref(), ExpData::Quant(..))
}

/// Check if a quantified variable `sym` is constrained by co-occurring with at least one
/// non-quantified variable in a constraint context.
///
/// For forall (`is_forall=true`): constraint contexts are antecedents of the implication chain.
/// For exists (`is_forall=false`): constraint contexts are conjuncts of the body.
///
/// A non-quantified variable is either a `Temporary` (function parameter/local) or a `LocalVar`
/// whose symbol is not in `quant_syms`.
fn sym_constrained_with_external(
    body: &Exp,
    sym: Symbol,
    quant_syms: &BTreeSet<Symbol>,
    is_forall: bool,
) -> bool {
    let contexts = if is_forall {
        collect_antecedents(body)
    } else {
        move_model::exp_simplifier::flatten_conjunction_owned(body)
    };
    // The variable is constrained if at least one context contains both `sym`
    // and a non-quantified variable (either a Temporary or a non-quant LocalVar).
    contexts.iter().any(|ctx| {
        let has_sym = ctx
            .as_ref()
            .any(&mut |ed| matches!(ed, ExpData::LocalVar(_, s) if *s == sym));
        let has_external = ctx.as_ref().any(&mut |ed| {
            matches!(ed, ExpData::Temporary(..))
                || matches!(ed, ExpData::LocalVar(_, s) if !quant_syms.contains(s))
        });
        has_sym && has_external
    })
}

/// Collect all antecedents from a nested implication chain `a ==> b ==> c ==> ...`.
fn collect_antecedents(body: &Exp) -> Vec<Exp> {
    let mut result = Vec::new();
    let mut current = body;
    loop {
        match current.as_ref() {
            ExpData::Call(_, AstOp::Implies, args) if args.len() == 2 => {
                result.push(args[0].clone());
                current = &args[1];
            },
            ExpData::Quant(_, QuantKind::Forall, _, _, _, inner_body) => {
                current = inner_body;
            },
            _ => break,
        }
    }
    result
}

/// Simplify a WPState using the ExpSimplifier.
///
/// For ensures: use preceding ensures as assumptions for later ones.
/// If ensures[i] becomes `true` under assumptions from ensures[0..i-1], it's redundant.
///
/// For aborts: simplify independently (no cross-assumptions — they represent distinct paths).
fn simplify_state<'env>(generator: &mut impl ExpGenerator<'env>, state: &WPState) -> WPState {
    let mut simplifier = ExpSimplifier::new(generator);

    // Simplify ensures: process non-quantified ensures first so that direct equalities
    // (e.g., `r == old(r) * pow2(n)`) are assumed before quantified ensures, enabling
    // substitution-based simplification.
    // If ensures[i] becomes `true` under assumptions from preceding ones, it's redundant.
    let (non_quant, quant): (Vec<_>, Vec<_>) = state
        .ensures
        .iter()
        .partition(|e| !matches!(e.as_ref(), ExpData::Quant(..)));
    let mut simplified_ensures = Vec::new();
    for exp in non_quant.iter().chain(quant.iter()) {
        let simplified = simplifier.simplify((*exp).clone());
        if !is_trivial_true(&simplified) {
            simplifier.assume(simplified.clone());
            simplified_ensures.push(simplified);
        }
    }
    // Remove quantified ensures that constrain an entity (result variable or global)
    // already fully determined by a non-quantified ensures (e.g., `r == expr` or
    // `global<T>(addr) == expr`).
    let determined_entities: Vec<DeterminedEntity> = simplified_ensures
        .iter()
        .filter(|e| !matches!(e.as_ref(), ExpData::Quant(..)))
        .filter_map(ensures_determines_entity)
        .collect();
    if !determined_entities.is_empty() {
        simplified_ensures.retain(|e| {
            if let ExpData::Quant(_, QuantKind::Forall, _, _, _, body) = e.as_ref() {
                // Check if the consequent of the implication constrains a determined entity
                let consequent = match body.as_ref() {
                    ExpData::Call(_, AstOp::Implies, args) if args.len() == 2 => &args[1],
                    _ => body,
                };
                if let Some(entity) = ensures_determines_entity(consequent) {
                    return !determined_entities
                        .iter()
                        .any(|d| entities_match(d, &entity));
                }
            }
            true
        });
    }

    // Eliminate foralls that are provably false via counterexample.
    // Try instantiating all quantified variables with 0 (minimum of unsigned type domains).
    // If the body evaluates to `false`, the forall is exactly `false`.
    // A false ensures carries no information; a false aborts_if means "never aborts
    // for this reason" — both should be removed.
    simplified_ensures.retain(|e| !simplifier.is_forall_provably_false(e));

    // Deduplicate ensures using structural equality.
    // Simplification can produce structural duplicates from conditions that were
    // syntactically different before simplification.
    let simplified_ensures = deduplicate_exps(simplified_ensures);

    // Drop the ensures simplifier so we can reborrow the generator.
    drop(simplifier);

    // Simplify aborts independently (no cross-assumptions — they represent distinct paths).
    let simplified_aborts: Vec<Exp> = state
        .aborts
        .iter()
        .map(|exp| {
            let mut s = ExpSimplifier::new(generator);
            s.simplify(exp.clone())
        })
        .filter(|exp| !is_trivial_false(exp))
        .collect();

    // Deduplicate aborts using structural equality before subsumption checking.
    // Substitution (e.g., Assign) can produce structural duplicates that the subsumption
    // check would incorrectly eliminate (each duplicate subsumes the other, removing both).
    let simplified_aborts = deduplicate_exps(simplified_aborts);

    // Combine complementary path-conditional aborts: if both `P && Q` and `!P && Q`
    // appear, replace them with just `Q` (since (P && Q) || (!P && Q) ≡ Q).
    let simplified_aborts = combine_complementary_aborts(&simplified_aborts);

    // Remove aborts conditions subsumed by other conditions.
    // In a disjunctive context, if b ==> a (a subsumes b), then b is redundant.
    let simplified_aborts = {
        let simplifier = ExpSimplifier::new(generator);
        let mut result = Vec::new();
        for (i, a) in simplified_aborts.iter().enumerate() {
            let subsumed = simplified_aborts
                .iter()
                .enumerate()
                .any(|(j, b)| i != j && simplifier.subsumes(b, a));
            if !subsumed {
                result.push(a.clone());
            }
        }
        result
    };

    // Rename quantified variables to nice names (x, y, z, x1, x2, ...),
    // avoiding clashes with function parameter names.
    let env = generator.global_env();
    let pool = env.symbol_pool();
    let fun_env = generator.function_env();
    let reserved: BTreeSet<String> = fun_env
        .get_parameters()
        .iter()
        .map(|p| p.0.display(pool).to_string())
        .filter(|name| !name.starts_with('$'))
        .collect();
    let simplified_ensures = simplified_ensures
        .iter()
        .map(|e| rename_quant_vars_in_exp(env, &reserved, e))
        .collect();
    let simplified_aborts = simplified_aborts
        .iter()
        .map(|e| rename_quant_vars_in_exp(env, &reserved, e))
        .collect();

    // Deduplicate direct_modifies using structural equality.
    let direct_modifies = deduplicate_exps(state.direct_modifies.clone());

    WPState {
        ensures: simplified_ensures,
        aborts: simplified_aborts,
        origin_offset: state.origin_offset,
        post: state.post,
        captured_mut_params: state.captured_mut_params.clone(),
        captured_globals: state.captured_globals.clone(),
        direct_modifies,
    }
}

/// Rename quantified variables in a single expression.
/// Traverses the expression bottom-up. For each Forall quantifier, renames bound
/// variables to the first available nice name that doesn't conflict with free variables
/// or `reserved` names (function parameter/local names).
fn rename_quant_vars_in_exp(env: &GlobalEnv, reserved: &BTreeSet<String>, exp: &Exp) -> Exp {
    match exp.as_ref() {
        ExpData::Quant(
            id,
            kind @ (QuantKind::Forall | QuantKind::Exists),
            ranges,
            triggers,
            cond,
            body,
        ) => {
            // First recurse into the body
            let body = rename_quant_vars_in_exp(env, reserved, body);

            // Collect external free variables (free in body but not bound here)
            let bound_syms: BTreeSet<_> = ranges
                .iter()
                .filter_map(|(pat, _)| {
                    if let Pattern::Var(_, sym) = pat {
                        Some(*sym)
                    } else {
                        None
                    }
                })
                .collect();
            let pool = env.symbol_pool();
            let mut used_names: BTreeSet<String> = body
                .as_ref()
                .free_vars()
                .iter()
                .filter(|s| !bound_syms.contains(s))
                .map(|s| s.display(pool).to_string())
                .collect();
            // Also avoid reserved names (function parameters/locals)
            used_names.extend(reserved.iter().cloned());

            // Assign nice names to each bound variable
            let nice_names = ["x", "y", "z"];
            let mut renames: Vec<(Symbol, Symbol)> = vec![];
            for (pat, _) in ranges {
                if let Pattern::Var(_, old_sym) = pat {
                    let new_name =
                        if let Some(name) = nice_names.iter().find(|n| !used_names.contains(**n)) {
                            name.to_string()
                        } else {
                            let mut i = 1;
                            loop {
                                let candidate = format!("x{}", i);
                                if !used_names.contains(&candidate) {
                                    break candidate;
                                }
                                i += 1;
                            }
                        };
                    used_names.insert(new_name.clone());
                    let new_sym = pool.make(&new_name);
                    if new_sym != *old_sym {
                        renames.push((*old_sym, new_sym));
                    }
                }
            }

            if renames.is_empty() {
                return ExpData::Quant(
                    *id,
                    *kind,
                    ranges.clone(),
                    triggers.clone(),
                    cond.clone(),
                    body,
                )
                .into_exp();
            }

            // Apply renames to body
            let mut body = body;
            for (old_sym, new_sym) in &renames {
                let var_ty = ranges
                    .iter()
                    .find_map(|(pat, _)| {
                        if let Pattern::Var(nid, sym) = pat {
                            if *sym == *old_sym {
                                Some(env.get_node_type(*nid))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .unwrap_or(BOOL_TYPE.clone());
                let replacement_id = env.new_node(env.get_node_loc(*id), var_ty);
                let replacement = ExpData::LocalVar(replacement_id, *new_sym).into_exp();
                let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
                    if let RewriteTarget::LocalVar(s) = target {
                        if s == *old_sym {
                            return Some(replacement.clone());
                        }
                    }
                    None
                };
                body = ExpRewriter::new(env, &mut replacer).rewrite_exp(body);
            }

            // Apply renames to ranges
            let new_ranges: Vec<_> = ranges
                .iter()
                .map(|(pat, range)| {
                    if let Pattern::Var(nid, sym) = pat {
                        if let Some((_, new_sym)) = renames.iter().find(|(old, _)| *old == *sym) {
                            (Pattern::Var(*nid, *new_sym), range.clone())
                        } else {
                            (pat.clone(), range.clone())
                        }
                    } else {
                        (pat.clone(), range.clone())
                    }
                })
                .collect();

            ExpData::Quant(*id, *kind, new_ranges, triggers.clone(), cond.clone(), body).into_exp()
        },
        // Recurse into subexpressions
        ExpData::Call(id, op, args) => {
            let new_args: Vec<Exp> = args
                .iter()
                .map(|a| rename_quant_vars_in_exp(env, reserved, a))
                .collect();
            if args
                .iter()
                .zip(new_args.iter())
                .all(|(a, b)| ExpData::ptr_eq(a, b))
            {
                exp.clone()
            } else {
                ExpData::Call(*id, op.clone(), new_args).into_exp()
            }
        },
        _ => exp.clone(),
    }
}

/// Check if an expression only references temps that are parameters (index < num_params)
fn exp_only_references_params(exp: &Exp, num_params: usize) -> bool {
    !exp.as_ref()
        .any(&mut |e| matches!(e, ExpData::Temporary(_, idx) if *idx >= num_params))
}

// =================================================================================================
// Spec Inference Analyzer

/// The main analyzer that performs weakest precondition analysis.
struct SpecInferenceAnalyzer<'env> {
    fun_env: &'env FunctionEnv<'env>,
    target: FunctionTarget<'env>,
    /// Current location for expression creation
    current_loc: Loc,
    /// The "at_end" label representing the final state (post-return)
    at_end_label: MemoryLabel,
    /// Cache of labels per code offset (for fixpoint stability)
    offset_labels: RefCell<BTreeMap<CodeOffset, MemoryLabel>>,
    /// Pre-scanned mapping from `borrow_global_mut` dest temp to
    /// (module_id, struct_id, type_args, addr_temp). Used to "un-resolve"
    /// globals back to temps during unrolled loop WP chaining.
    borrow_global_info: BTreeMap<TempIndex, (ModuleId, StructId, Vec<Type>, TempIndex)>,
    /// Temps that are destinations of `Havoc` operations (loop-modified variables).
    /// Used to decide whether BorrowGlobal resolution should be deferred: only temps
    /// that will be havoc'd need deferral so the quantifier can bind them.
    havoc_targets: BTreeSet<TempIndex>,
    /// Labels created at `WriteBack(GlobalRoot)` for same-resource-different-addr mutations.
    /// These intermediate labels must be preserved during label stripping (they represent
    /// intermediate memory states between two writes to the same resource type at different
    /// addresses).
    mutation_labels: RefCell<BTreeSet<MemoryLabel>>,
}

// =================================================================================================
// ExpGenerator Implementation

impl<'env> ExpGenerator<'env> for SpecInferenceAnalyzer<'env> {
    fn function_env(&self) -> &FunctionEnv<'env> {
        self.fun_env
    }

    fn get_current_loc(&self) -> Loc {
        self.current_loc.clone()
    }

    fn set_loc(&mut self, _loc: Loc) {
        // Not needed for spec inference - panic if called
        panic!("set_loc not supported in SpecInferenceAnalyzer")
    }

    fn add_local(&mut self, _ty: Type) -> TempIndex {
        // Not needed for spec inference - panic if called
        panic!("add_local not supported in SpecInferenceAnalyzer")
    }

    fn get_local_type(&self, temp: TempIndex) -> Type {
        self.target.get_local_type(temp).clone()
    }
}

// =================================================================================================
// TransferFunctions Implementation

impl<'env> TransferFunctions for SpecInferenceAnalyzer<'env> {
    type State = WPState;

    const BACKWARD: bool = true;

    fn execute(&self, state: &mut WPState, instr: &Bytecode, offset: CodeOffset) {
        match instr {
            Bytecode::Ret(_, vals) => {
                // Base case for backward analysis: compute the ensures conditions
                // Creates one ensures per return value that references a parameter
                *state = self.mk_return_ensures(vals);
                // Track origin for path-conditional join handling
                state.origin_offset = Some(offset);
            },
            Bytecode::Abort(_, _, _) => {
                // Abort sets the aborts condition to true
                *state = WPState::with_aborts(self.mk_bool_const(true), self.at_end_label);
                state.origin_offset = Some(offset);
            },
            Bytecode::Assign(_, dest, src, _) => {
                // WP[x := e](Q) = Q[x ↦ e]
                // For captured &mut params, the read value is the pre-state.
                if self.is_mut_ref_param(*src) && state.captured_mut_params.contains(src) {
                    let old_exp = self.mk_old(self.mk_temporary(*src));
                    *state = self.substitute_exp_state(state, *dest, &old_exp);
                } else {
                    *state = self.substitute_state(state, *dest, *src);
                }
                // Preserve origin through substitution
            },
            Bytecode::Load(_, dest, constant) => {
                // WP[dest := const](Q) = Q[dest ↦ const]
                // Substitute dest with the constant expression in the state
                if let Some(const_exp) = self.constant_to_exp(constant) {
                    *state = self.substitute_exp_state(state, *dest, &const_exp);
                }
            },
            Bytecode::Call(_, dests, op, srcs, _abort_action) => {
                match op {
                    // ==================== Implemented Operations ====================

                    // Arithmetic operations (with overflow/underflow abort conditions)
                    Operation::Add
                    | Operation::Sub
                    | Operation::Mul
                    | Operation::Div
                    | Operation::Mod => {
                        // WP[dest := a op b](Q) = Q[dest ↦ a op b] ∧ abort_cond
                        let dest = dests[0];
                        let arith_exp = self.mk_arith_exp(op, srcs);

                        // Substitute dest with arithmetic expression in ensures
                        *state = self.substitute_exp_state(state, dest, &arith_exp);

                        // Add abort condition for overflow/underflow/div-by-zero
                        if let Some(abort_cond) = self.mk_arith_abort_cond(op, dest, srcs) {
                            state.add_aborts(abort_cond);
                        }
                    },

                    // Comparison operations (never abort)
                    Operation::Eq
                    | Operation::Neq
                    | Operation::Lt
                    | Operation::Le
                    | Operation::Gt
                    | Operation::Ge => {
                        // WP[dest := a cmp b](Q) = Q[dest ↦ a cmp b]
                        let dest = dests[0];
                        let cmp_exp = self.mk_cmp_exp(op, srcs);
                        *state = self.substitute_exp_state(state, dest, &cmp_exp);
                    },

                    // Direct function call
                    Operation::Function(module_id, fun_id, type_inst) => {
                        // WP[dest := f(args)](Q) = Q[dest ↦ result_of<f>(args)]
                        let (fun_exp, result_type) =
                            self.mk_closure(*module_id, *fun_id, type_inst);
                        let args: Vec<Exp> =
                            srcs.iter().map(|&idx| self.mk_temporary(idx)).collect();
                        let mut_ref_srcs: Vec<(usize, TempIndex)> = srcs
                            .iter()
                            .enumerate()
                            .filter(|&(_, &idx)| self.get_local_type(idx).is_mutable_reference())
                            .map(|(i, &idx)| (i, idx))
                            .collect();
                        self.wp_function_call(
                            state,
                            offset,
                            fun_exp,
                            args,
                            &result_type,
                            dests,
                            &mut_ref_srcs,
                        );
                    },

                    // WP[dest := closure<f>](Q) = Q[dest ↦ f]
                    Operation::Closure(module_id, fun_id, type_inst, _mask) => {
                        if dests.len() == 1 {
                            let (closure_exp, _) = self.mk_closure(*module_id, *fun_id, type_inst);
                            *state = self.substitute_exp_state(state, dests[0], &closure_exp);
                        }
                    },

                    // WP[dest := invoke(args, closure)](Q) — same as Function call
                    // but the callee is a closure expression rather than a static function.
                    Operation::Invoke => {
                        // srcs = [args..., closure] (closure is LAST)
                        if srcs.is_empty() {
                            return;
                        }
                        let closure_idx = srcs.len() - 1;
                        let fun_exp = self.mk_temporary(srcs[closure_idx]);
                        let fun_type = self.get_local_type(srcs[closure_idx]);
                        let result_type = if let Type::Fun(_, result, _) = &fun_type {
                            result.as_ref().clone()
                        } else {
                            return;
                        };
                        let actual_args = &srcs[..closure_idx];
                        let args: Vec<Exp> = actual_args
                            .iter()
                            .map(|&idx| self.mk_temporary(idx))
                            .collect();
                        let mut_ref_srcs: Vec<(usize, TempIndex)> = actual_args
                            .iter()
                            .enumerate()
                            .filter(|&(_, &idx)| self.get_local_type(idx).is_mutable_reference())
                            .map(|(i, &idx)| (i, idx))
                            .collect();
                        self.wp_function_call(
                            state,
                            offset,
                            fun_exp,
                            args,
                            &result_type,
                            dests,
                            &mut_ref_srcs,
                        );
                    },

                    // WP[dest := a lop b](Q) = Q[dest ↦ a lop b]  (never abort)
                    Operation::Or | Operation::And | Operation::Not => {
                        let dest = dests[0];
                        let logical_exp = self.mk_logical_exp(op, srcs);
                        *state = self.substitute_exp_state(state, dest, &logical_exp);
                    },

                    // WP[dest := a bop b](Q) = Q[dest ↦ a bop b]  (never abort)
                    Operation::BitOr | Operation::BitAnd | Operation::Xor => {
                        let dest = dests[0];
                        let bitwise_exp = self.mk_bitwise_exp(op, srcs);
                        *state = self.substitute_exp_state(state, dest, &bitwise_exp);
                    },
                    // WP[dest := a sop b](Q) = Q[dest ↦ a sop b] ∧ (b < bit_width)
                    Operation::Shl | Operation::Shr => {
                        let dest = dests[0];
                        let bitwise_exp = self.mk_bitwise_exp(op, srcs);
                        *state = self.substitute_exp_state(state, dest, &bitwise_exp);
                        // Add abort condition for shift amount >= bit width
                        if let Some(abort_cond) = self.mk_shift_abort_cond(dest, srcs) {
                            state.add_aborts(abort_cond);
                        }
                    },

                    // WP[dest := -src](Q) = Q[dest ↦ -src] ∧ (src != MIN)
                    Operation::Negate => {
                        let dest = dests[0];
                        let src = self.mk_temporary(srcs[0]);
                        let neg_exp = self.mk_negate(src);
                        *state = self.substitute_exp_state(state, dest, &neg_exp);
                        // Add abort condition for signed overflow
                        if let Some(abort_cond) = self.mk_negate_abort_cond(dest, srcs) {
                            state.add_aborts(abort_cond);
                        }
                    },

                    // WP[dest := src as T](Q) = Q[dest ↦ src as T] ∧ (src in T::range)
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
                    | Operation::CastI256 => {
                        let dest = dests[0];
                        let cast_exp = self.mk_cast_exp(op, srcs);
                        *state = self.substitute_exp_state(state, dest, &cast_exp);
                        // Add abort condition for out-of-range value
                        if let Some(abort_cond) = self.mk_cast_abort_cond(op, srcs) {
                            state.add_aborts(abort_cond);
                        }
                    },

                    // ==================== Struct & Variant Operations ====================

                    // WP[dest := pack S(fields)](Q) = Q[dest ↦ S{fields}]
                    Operation::Pack(module_id, struct_id, type_args) => {
                        self.wp_pack(
                            state, dests[0], srcs, *module_id, *struct_id, None, type_args,
                        );
                    },
                    // WP[dest := pack S::V(fields)](Q) = Q[dest ↦ S::V{fields}]
                    Operation::PackVariant(module_id, struct_id, variant, type_args) => {
                        self.wp_pack(
                            state,
                            dests[0],
                            srcs,
                            *module_id,
                            *struct_id,
                            Some(*variant),
                            type_args,
                        );
                    },

                    // WP[dests := unpack S(src)](Q) = Q[dest_i ↦ src.field_i]
                    Operation::Unpack(module_id, struct_id, type_args) => {
                        self.wp_unpack(
                            state, dests, srcs[0], *module_id, *struct_id, None, type_args,
                        );
                    },
                    // WP[dests := unpack S::V(src)](Q) = Q[dest_i ↦ src.field_i] ∧ (src is V)
                    Operation::UnpackVariant(module_id, struct_id, variant, type_args) => {
                        self.wp_unpack(
                            state,
                            dests,
                            srcs[0],
                            *module_id,
                            *struct_id,
                            Some(*variant),
                            type_args,
                        );
                    },

                    // WP[dest := src.field](Q) = Q[dest ↦ src.field]
                    Operation::GetField(module_id, struct_id, type_args, field_offset) => {
                        self.wp_get_field(
                            state,
                            dests[0],
                            srcs[0],
                            module_id,
                            struct_id,
                            &[],
                            type_args,
                            *field_offset,
                        );
                    },
                    // WP[dest := src.field](Q) = Q[dest ↦ src.field] ∧ (src is V)
                    Operation::GetVariantField(
                        module_id,
                        struct_id,
                        variants,
                        type_args,
                        field_offset,
                    ) => {
                        self.wp_get_field(
                            state,
                            dests[0],
                            srcs[0],
                            module_id,
                            struct_id,
                            variants,
                            type_args,
                            *field_offset,
                        );
                    },

                    // WP[dest := src is V](Q) = Q[dest ↦ src is V]
                    Operation::TestVariant(module_id, struct_id, variant, _type_args) => {
                        let dest = dests[0];
                        let src_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        let test_exp = self.mk_variant_test(&struct_env, *variant, src_exp);
                        *state = self.substitute_exp_state(state, dest, &test_exp);
                    },

                    // ==================== Reference Operations ====================
                    // TODO(#18762): References are treated as transparent aliases (borrow ≈
                    // copy, deref ≈ identity). This works for simple patterns (single
                    // borrow, linear use) but does not model the full reference semantics:
                    // WriteRef to a non-param ref doesn't propagate to the referent's
                    // other aliases, and nested borrow chains (borrow_field of
                    // borrow_global) lose their connection to the underlying global.

                    // BorrowLoc - create a reference to a local variable
                    // WP[dest := borrow_loc(src)](Q) = Q[dest => src]
                    Operation::BorrowLoc => {
                        let dest = dests[0];
                        let src = srcs[0];
                        *state = self.substitute_state(state, dest, src);
                    },

                    // BorrowField - create a reference to a struct field
                    // WP[dest := borrow_field<S>.field(src)](Q) = Q[dest => select S.field(src)]
                    Operation::BorrowField(module_id, struct_id, type_args, field_offset) => {
                        self.wp_borrow_field(
                            state,
                            dests[0],
                            srcs[0],
                            module_id,
                            struct_id,
                            &[],
                            type_args,
                            *field_offset,
                        );
                    },

                    // WP[dest := &src.field](Q) = Q[dest ↦ src.field] ∧ (src is V)
                    Operation::BorrowVariantField(
                        module_id,
                        struct_id,
                        variants,
                        type_args,
                        field_offset,
                    ) => {
                        self.wp_borrow_field(
                            state,
                            dests[0],
                            srcs[0],
                            module_id,
                            struct_id,
                            variants.as_slice(),
                            type_args,
                            *field_offset,
                        );
                    },

                    // ReadRef - dereference a reference
                    // WP[dest := *ref](Q) = Q[dest => ref]
                    // For &mut params that have been captured (written to), use old() to
                    // reference the initial value.
                    Operation::ReadRef => {
                        let dest = dests[0];
                        let src = srcs[0];

                        if self.is_global_or_mut_param(state, src)
                            && state.captured_mut_params.contains(&src)
                        {
                            // Reading from a &mut param that has been written to.
                            // For ensures: use Old() to represent the initial value.
                            // This is later corrected by prepare/restore_ensures_for_ref_havoc
                            // during loop havoc processing.
                            let old_exp = self.mk_old(self.mk_temporary(src));
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| self.substitute_temp_with_exp(e, dest, &old_exp))
                                .collect();
                            // For aborts: also use Old() so substitute_old_param_in_state
                            // correctly chains mutations across multiple writes.
                            state.aborts = state
                                .aborts
                                .iter()
                                .map(|e| self.substitute_temp_with_exp(e, dest, &old_exp))
                                .collect();
                        } else {
                            // Normal case: substitute dest with src
                            *state = self.substitute_state(state, dest, src);
                        }
                    },

                    // WriteRef - write through a reference: Q[x => v]
                    Operation::WriteRef => {
                        // srcs = [reference, value]
                        let ref_idx = srcs[0];
                        let val_exp = self.mk_temporary(srcs[1]);

                        if self.is_global_or_mut_param(state, ref_idx) {
                            // For &mut params: add ensures if not already captured.
                            // In backward analysis, the first write encountered (last in execution)
                            // is the final value. Subsequent writes (earlier) need to substitute
                            // old(param) with the written value.
                            if !state.captured_mut_params.contains(&ref_idx) {
                                let param_exp = self.mk_temporary(ref_idx);
                                state.add_ensures(self.mk_eq(param_exp, val_exp));
                                state.captured_mut_params.insert(ref_idx);
                            } else {
                                // Earlier write to already-captured param: substitute old(param)
                                // with the written value. This correctly chains multiple writes.
                                *state =
                                    self.substitute_old_param_in_state(state, ref_idx, &val_exp);
                            }
                        } else {
                            // Non-param refs: use substitution as before
                            *state = self.substitute_exp_state(state, ref_idx, &val_exp);
                        }
                    },

                    // FreezeRef - convert mutable reference to immutable
                    // WP[dest := freeze(src)](Q) = Q[dest => src]
                    // Freeze is just an alias — treat like BorrowLoc.
                    Operation::FreezeRef(_) => {
                        let dest = dests[0];
                        let src = srcs[0];
                        *state = self.substitute_state(state, dest, src);
                    },

                    // WP[dest := &R[addr]](Q) = Q[dest ↦ R[addr]] ∧ exists<R>(addr)
                    Operation::BorrowGlobal(module_id, struct_id, type_args) => {
                        let dest = dests[0];
                        let addr_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // If dest will be havoc'd (loop-modified variable), defer resolution
                        // to entry-state processing (Part 3). This keeps the borrow temp
                        // alive so the havoc step can quantify over it and link the user
                        // invariant to the ensures.
                        if !self.havoc_targets.contains(&dest) {
                            // Substitute dest with global<R>(addr) — like BorrowLoc substitutes
                            // dest with src. In backward analysis this resolves the temp that
                            // WriteBack(GlobalRoot) captured earlier.
                            let global_exp = self.mk_global_with_label(
                                &struct_env,
                                type_args,
                                addr_exp.clone(),
                                Some(state.post),
                            );
                            *state = self.substitute_exp_state(state, dest, &global_exp);
                        }
                        // Add abort condition: !exists<R>(@addr)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp,
                            Some(state.post),
                        );
                        let not_exists = self.mk_not(exists_exp);
                        state.add_aborts(not_exists);
                    },

                    // WP[dest := exists<R>(addr)](Q) = Q[dest ↦ exists<R>(addr)]
                    Operation::Exists(module_id, struct_id, type_args) => {
                        let dest = dests[0];
                        let addr_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp,
                            Some(state.post),
                        );
                        *state = self.substitute_exp_state(state, dest, &exists_exp);
                    },

                    // WP[dest := R[addr]](Q) = Q[dest ↦ R[addr]] ∧ exists<R>(addr)
                    Operation::GetGlobal(module_id, struct_id, type_args) => {
                        let dest = dests[0];
                        let addr_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // Return value is current state (resource is not removed)
                        let global_exp = self.mk_global_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(state.post),
                        );
                        // If this global is captured (modified in a loop), the GetGlobal
                        // reads the pre-loop value. Wrap in old() so backward analysis
                        // produces meaningful (non-tautological) invariants.
                        let global_exp = if self
                            .find_captured_global_for_resource(
                                state, *module_id, *struct_id, type_args, srcs[0],
                            )
                            .is_some()
                        {
                            self.mk_old(global_exp)
                        } else {
                            global_exp
                        };
                        *state = self.substitute_exp_state(state, dest, &global_exp);
                        // Add abort condition: !exists<R>(@addr)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp,
                            Some(state.post),
                        );
                        let not_exists = self.mk_not(exists_exp);
                        state.add_aborts(not_exists);
                    },

                    // WP[dest := move_from<R>(addr)](Q) =
                    //   Q[dest ↦ R[addr]] ∧ exists<R>(addr) ∧ ensures(!exists<R>(addr))
                    Operation::MoveFrom(module_id, struct_id, type_args) => {
                        let dest = dests[0];
                        let addr_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // Return value comes from global state
                        let global_exp = self.mk_global_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(state.post),
                        );
                        *state = self.substitute_exp_state(state, dest, &global_exp);
                        // Add abort condition: !exists<R>(@addr)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(state.post),
                        );
                        let not_exists = self.mk_not(exists_exp);
                        state.add_aborts(not_exists.clone());
                        // Post-state: resource no longer exists after removal
                        state.add_ensures(not_exists);
                        // Track as direct modifies target
                        let modifies_target = self.mk_global(&struct_env, type_args, addr_exp);
                        state.add_direct_modifies(modifies_target);
                    },

                    // WP[move_to<R>(signer, val)](Q) =
                    //   Q ∧ !exists<R>(addr) ∧ ensures(exists<R>(addr) ∧ R[addr] == val)
                    Operation::MoveTo(module_id, struct_id, type_args) => {
                        // srcs[0] = signer/address, srcs[1] = resource value
                        let addr_exp = self.signer_to_address(self.mk_temporary(srcs[0]));
                        let val_exp = self.mk_temporary(srcs[1]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // Add abort condition: exists<R>(@addr) (resource already there)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(state.post),
                        );
                        state.add_aborts(exists_exp.clone());
                        // Post-state ensures: resource now exists with the given value
                        let global_post = self.mk_global_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(state.post),
                        );
                        state.add_ensures(exists_exp);
                        state.add_ensures(self.mk_eq(global_post, val_exp));
                        // Track as direct modifies target
                        let modifies_target = self.mk_global(&struct_env, type_args, addr_exp);
                        state.add_direct_modifies(modifies_target);
                    },

                    // WP[dest := vector[e1, ..., en]](Q) = Q[dest ↦ vector[e1, ..., en]]
                    Operation::Vector => {
                        let dest = dests[0];
                        let elements: Vec<Exp> =
                            srcs.iter().map(|&idx| self.mk_temporary(idx)).collect();
                        let vec_type = self.get_local_type(dest);
                        let elem_type = if let Type::Vector(inner) = &vec_type {
                            (**inner).clone()
                        } else {
                            vec_type.clone()
                        };
                        let node_id = self.new_node(vec_type, Some(vec![elem_type]));
                        let vec_exp = ExpData::Call(node_id, AstOp::Vector, elements).into_exp();
                        *state = self.substitute_exp_state(state, dest, &vec_exp);
                    },

                    // WP[drop/release](Q) = Q  (no effect on spec state)
                    Operation::Drop | Operation::Release => {
                        // These don't affect the spec state
                    },

                    // WriteBack - write back through borrow edge
                    Operation::WriteBack(node, edge) => {
                        // srcs[0] is the value being written back
                        let val_exp = self.mk_temporary(srcs[0]);

                        match node {
                            BorrowNode::LocalRoot(dest) | BorrowNode::Reference(dest) => {
                                // WP[write_back[LocalRoot/Reference(x), e] := v](Q) = Q[x => trans[e](x, v)]
                                let old_exp = self.mk_temporary(*dest);
                                if let Some(new_exp) =
                                    self.mk_edge_transform(edge, old_exp.clone(), val_exp)
                                {
                                    if self.is_global_or_mut_param(state, *dest) {
                                        // Wrap bare references to the param in new_exp with
                                        // old() since they represent the pre-state value.
                                        let old_dest = self.mk_old(self.mk_temporary(*dest));
                                        let new_exp = self
                                            .substitute_temp_with_exp(&new_exp, *dest, &old_dest);
                                        if !state.captured_mut_params.contains(dest) {
                                            // First write_back (last in execution): add ensures.
                                            state.add_ensures(self.mk_eq(old_exp, new_exp));
                                            state.captured_mut_params.insert(*dest);
                                        } else {
                                            // Earlier write_back: substitute old(param) with
                                            // the transformed value in existing ensures.
                                            *state = self.substitute_old_param_in_state(
                                                state, *dest, &new_exp,
                                            );
                                        }
                                    } else {
                                        // Non-param refs: use substitution as before
                                        *state = self.substitute_exp_state(state, *dest, &new_exp);
                                    }
                                }
                            },
                            BorrowNode::GlobalRoot(_qid) => {
                                let ref_temp = srcs[0];
                                let needs_unresolve = state.captured_globals.contains(&ref_temp)
                                    || self.has_captured_same_global(state, ref_temp);
                                if needs_unresolve {
                                    // Second+ capture of the same global (either same temp
                                    // from a loop, or different temp borrowing the same
                                    // resource). "Un-resolve": substitute
                                    // global<R>(addr) → ref_temp in state, including inside
                                    // old(), so existing chaining via
                                    // substitute_old_param_in_state works.
                                    if let Some((mid, sid, targs, addr_temp)) =
                                        self.borrow_global_info.get(&ref_temp)
                                    {
                                        let struct_env =
                                            self.global_env().get_struct(QualifiedId {
                                                module_id: *mid,
                                                id: *sid,
                                            });
                                        let addr_exp = self.mk_temporary(*addr_temp);
                                        let global_exp = self.mk_global_with_label(
                                            &struct_env,
                                            targs,
                                            addr_exp,
                                            Some(state.post),
                                        );
                                        let ref_exp = self.mk_temporary(ref_temp);
                                        // Replace global<R>(addr) → ref_temp everywhere
                                        // (including inside old())
                                        *state = state.map(|e| {
                                            self.substitute_exp_with_exp(e, &global_exp, &ref_exp)
                                        });
                                    }
                                    // For the different-temp case, also insert ref_temp
                                    // into captured_mut_params so WriteBack(Reference(ref_temp))
                                    // triggers the "earlier write" chaining path.
                                    if !state.captured_globals.contains(&ref_temp) {
                                        state.captured_mut_params.insert(ref_temp);
                                    }
                                } else if let Some((mid, sid, targs, addr_temp)) =
                                    self.find_same_resource_different_addr(state, ref_temp)
                                {
                                    // Same resource type but different address (N=2 case).
                                    // Create intermediate label to chain the two writes.
                                    let mid_label = self.mk_label_at(offset);
                                    self.mutation_labels.borrow_mut().insert(mid_label);

                                    let struct_env = self.global_env().get_struct(QualifiedId {
                                        module_id: mid,
                                        id: sid,
                                    });
                                    let resource_type = Type::Struct(mid, sid, targs.clone());
                                    let addr_exp = self.mk_temporary(addr_temp);

                                    // Retroactive rewrite: old(global[@state.post]<R>(...))
                                    //                    → global[@mid_label]<R>(...)
                                    *state = self.rewrite_old_globals_to_label(
                                        state,
                                        Some(&resource_type),
                                        state.post,
                                        mid_label,
                                    );

                                    // Emit frame: forall a: a != addr ==>
                                    //   global[@mid]<R>(a) == old(global<R>(a))
                                    let frame = self.mk_intermediate_frame(
                                        mid_label,
                                        &struct_env,
                                        &targs,
                                        addr_exp,
                                    );
                                    state.add_ensures(frame);

                                    // Update state.post for predecessor instructions
                                    state.post = mid_label;
                                }
                                // When a function call precedes this WriteBack in backward
                                // order (= follows in program order), state.post is the
                                // call's pre_label. Preserve it so label stripping doesn't
                                // remove it (it would otherwise match entry_post_label).
                                if state.post != self.at_end_label {
                                    self.mutation_labels.borrow_mut().insert(state.post);
                                }
                                // Mark the source temp as a captured global. The BorrowGlobal
                                // handler (processed later in backward order) will resolve this
                                // temp to `global<R>(addr)`.
                                state.captured_globals.insert(ref_temp);
                            },
                            BorrowNode::ReturnPlaceholder(_) => {
                                // This doesn't appear in bytecode instructions, skip
                            },
                        }
                    },

                    // Havoc: wp(x := *, Q) = forall x. Q
                    // Wrap conditions referencing dest in a universal quantifier.
                    Operation::Havoc(_) => {
                        let dest = dests[0];
                        let raw_ty = self.get_local_type(dest);
                        let is_ref = raw_ty.is_reference();
                        // For references, quantify over the base type (the value behind the ref)
                        let ty = if is_ref {
                            raw_ty.skip_reference().clone()
                        } else {
                            raw_ty
                        };
                        let sym = self.mk_symbol(&format!("$q{}", dest));
                        let local_exp = self.mk_local_by_sym(sym, ty.clone());
                        let wrap = |this: &Self, e: &Exp, quant_kind: QuantKind| -> Exp {
                            if !e.as_ref().any(
                                &mut |ed| matches!(ed, ExpData::Temporary(_, idx) if *idx == dest),
                            ) {
                                return e.clone();
                            }
                            // Replace Temporary(dest) with LocalVar(sym) in body.
                            // For references, skip replacement inside old() — pre-state
                            // values of reference parameters are fixed and should not
                            // be quantified over.
                            let body = if is_ref {
                                this.substitute_temp_outside_old(e, dest, &local_exp)
                            } else {
                                this.substitute_temp_with_exp(e, dest, &local_exp)
                            };
                            // Build quantifier with the (possibly stripped) type.
                            // For ensures: forall (all paths satisfy post).
                            // For aborts: exists (some path can abort).
                            let range = this.mk_type_domain(ty.clone());
                            let pat = this.mk_decl(sym, ty.clone());
                            let node_id = this.new_node(BOOL_TYPE.clone(), None);
                            ExpData::Quant(
                                node_id,
                                quant_kind,
                                vec![(pat, range)],
                                vec![],
                                None,
                                body,
                            )
                            .into_exp()
                        };
                        if is_ref
                            && (state.captured_mut_params.contains(&dest)
                                || state.captured_globals.contains(&dest))
                        {
                            // For captured &mut params in loops: prepare ensures entries
                            // to correctly distinguish function output from loop variable.
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| self.prepare_ensures_for_ref_havoc(e, dest))
                                .collect();
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| wrap(self, e, QuantKind::Forall))
                                .collect();
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| self.restore_ensures_after_ref_havoc(e, dest))
                                .collect();
                        } else {
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| wrap(self, e, QuantKind::Forall))
                                .collect();
                        }
                        // For captured &mut ref params: strip old() from aborts before
                        // existential quantification so the quantifier variable binds correctly.
                        if is_ref && state.captured_mut_params.contains(&dest) {
                            let bare_temp = self.mk_temporary(dest);
                            state.aborts = state
                                .aborts
                                .iter()
                                .map(|e| self.substitute_old_param(e, dest, &bare_temp))
                                .collect();
                        }
                        state.aborts = state
                            .aborts
                            .iter()
                            .map(|e| wrap(self, e, QuantKind::Exists))
                            .collect();
                    },

                    // WP[...](Q) = Q  (verification IL; no effect on inference)
                    Operation::OpaqueCallBegin(_, _, _)
                    | Operation::OpaqueCallEnd(_, _, _)
                    | Operation::IsParent(_, _)
                    | Operation::UnpackRef
                    | Operation::PackRef
                    | Operation::UnpackRefDeep
                    | Operation::PackRefDeep
                    | Operation::Uninit
                    | Operation::TraceLocal(_)
                    | Operation::TraceReturn(_)
                    | Operation::TraceAbort
                    | Operation::TraceExp(_, _)
                    | Operation::TraceGlobalMem(_)
                    | Operation::EmitEvent
                    | Operation::EventStoreDiverge => {
                        // Extended bytecodes: not applicable for spec inference
                    },
                    // WP[stop](Q) = true  (unreachable; no conditions propagate)
                    Operation::Stop => {
                        *state = WPState::new(state.post);
                        state.origin_offset = Some(offset);
                    },
                }
            },

            // ==================== Control Flow (handled by framework) ====================
            Bytecode::Label(_, _)
            | Bytecode::Jump(_, _)
            | Bytecode::Branch(_, _, _, _)
            | Bytecode::Nop(_)
            | Bytecode::SpecBlock(_, _) => {
                // Control flow is handled by the dataflow framework
            },

            // ==================== Extended Bytecodes (verification IL) ====================
            Bytecode::SaveMem(_, _, _) | Bytecode::SaveSpecVar(_, _, _) => {
                // Extended bytecodes: not applicable for spec inference
            },
            Bytecode::Prop(_, kind, exp) => {
                match kind {
                    PropKind::Assume | PropKind::Assert => {
                        // Treat WellFormed assumptions as no-ops for inference:
                        // they wrap every ensures in implications like
                        // `WellFormed(a) ==> WellFormed(b) ==> result == a + b`
                        // which are unhelpful for inferred specs.
                        if matches!(kind, PropKind::Assume)
                            && matches!(exp.as_ref(), ExpData::Call(_, AstOp::WellFormed, _))
                        {
                            return;
                        }

                        // Both assume and assert make P known true at this point.
                        // Assert is a proof obligation (verified separately), assume is
                        // a proof assumption. Neither causes runtime aborts.
                        // WP effect: Q becomes (P ==> Q) for all conditions.
                        //
                        // For Assert only (loop invariant base case, placed before havoc):
                        // the condition may contain Freeze(Temporary(idx)) for a captured
                        // &mut param. Since the param has been captured (written to later
                        // in execution), the raw $t_idx outside the forall refers to the
                        // post-state value. But at the assertion's program point (before
                        // the loop), the dereference gives the pre-state value. Replace
                        // Freeze($t) with Old($t) to correctly model this.
                        //
                        // Similarly, for captured globals in Assert, replace
                        // global<R>(addr) with Old(global<R>(addr)) so the base case
                        // becomes tautological (pre-loop value == pre-loop value).
                        //
                        // For Assume (induction hypothesis after havoc): replace
                        // global<R>(addr) with Freeze($t_borrow) so the havoc
                        // quantification links the invariant to the havocked temp.
                        // We do NOT apply Freeze replacement for &mut params here
                        // because the havoc correctly replaces $t with $q already.
                        let cond = if matches!(kind, PropKind::Assert) {
                            let cond = self.replace_freeze_of_captured_mut_params(exp, state);
                            self.replace_global_of_captured_globals_with_old(&cond, state)
                        } else {
                            self.replace_global_of_captured_globals_with_freeze(exp, state)
                        };
                        // ensures: standard WP (P ==> Q)
                        state.ensures = state
                            .ensures
                            .iter()
                            .map(|e| self.mk_implies(cond.clone(), e.clone()))
                            .collect();
                        // aborts: abort requires assumption to hold (P AND C)
                        state.aborts = state
                            .aborts
                            .iter()
                            .map(|e| self.mk_and(cond.clone(), e.clone()))
                            .collect();
                    },
                    PropKind::Modifies => {
                        // Not relevant for ensures/aborts inference
                    },
                }
            },
        }
    }
}

impl<'env> DataflowAnalysis for SpecInferenceAnalyzer<'env> {
    /// Custom analyze_function that implements branch-aware joins using topological ordering.
    ///
    /// Uses Kahn's algorithm to process blocks in topological order of the backward CFG.
    /// This ensures that when a block with multiple predecessors is processed, all
    /// predecessor states are ready. This is critical for is_parent branches where
    /// path-conditional joining must see both sides simultaneously rather than
    /// incrementally (which would cause fixpoint instability with multiple successive
    /// is_parent branches).
    ///
    /// For functions with actual loops (after LoopAnalysis converts them to DAGs),
    /// the backward CFG is still a DAG, so topological ordering always works.
    fn analyze_function(
        &self,
        initial_state: WPState,
        instrs: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
    ) -> StateMap<WPState> {
        // Build label-to-offset map for branch target resolution
        let label_offsets = Bytecode::label_offsets(instrs);

        // Collect all reachable blocks and compute in-degree (predecessor count)
        // for Kahn's topological sort algorithm.
        let mut in_degree: BTreeMap<BlockId, usize> = BTreeMap::new();
        let mut all_blocks: BTreeSet<BlockId> = BTreeSet::new();
        {
            let mut queue = VecDeque::new();
            queue.push_back(cfg.entry_block());
            while let Some(b) = queue.pop_front() {
                if !all_blocks.insert(b) {
                    continue;
                }
                in_degree.entry(b).or_insert(0);
                for s in cfg.successors(b) {
                    *in_degree.entry(*s).or_insert(0) += 1;
                    queue.push_back(*s);
                }
            }
        }

        // Initialize Kahn's algorithm: start with blocks that have no predecessors
        // (in-degree 0). In the backward CFG, this is typically DUMMY_EXIT.
        let mut ready_queue: VecDeque<BlockId> = VecDeque::new();
        for (&block, &deg) in &in_degree {
            if deg == 0 {
                ready_queue.push_back(block);
            }
        }

        let mut state_map: StateMap<WPState> = StateMap::new();
        state_map.insert(cfg.entry_block(), BlockState {
            pre: initial_state.clone(),
            post: initial_state.clone(),
        });

        while let Some(block_id) = ready_queue.pop_front() {
            // Process this block: execute its instructions on its pre-state
            let pre = state_map
                .get(&block_id)
                .map(|bs| bs.pre.clone())
                .unwrap_or_else(|| initial_state.clone());
            let post = self.execute_block(block_id, pre, instrs, cfg);

            // Compute predecessor block's last code offset for is_parent classification.
            let pred_last_offset = {
                let range = cfg.code_range(block_id);
                if range.is_empty() {
                    None
                } else {
                    Some((range.end - 1) as CodeOffset)
                }
            };

            // Propagate postcondition to successor blocks
            for next_block_id in cfg.successors(block_id) {
                let branch_info =
                    self.get_branch_info_for_block(*next_block_id, instrs, cfg, &label_offsets);

                match state_map.get_mut(next_block_id) {
                    Some(next_block_res) => {
                        // Join incoming state with existing state at this block
                        self.path_aware_join(
                            &mut next_block_res.pre,
                            &post,
                            branch_info,
                            pred_last_offset,
                        );
                    },
                    None => {
                        // First state arriving at this block.
                        // Record the predecessor offset so path_aware_join can
                        // determine which branch side the already-stored state
                        // came from when the second side arrives.
                        let mut initial_post = post.clone();
                        if branch_info.is_some() {
                            if let Some(offset) = pred_last_offset {
                                initial_post.origin_offset = Some(offset);
                            }
                        }
                        state_map.insert(*next_block_id, BlockState {
                            pre: initial_post,
                            post: initial_state.clone(),
                        });
                    },
                }

                // Decrement in-degree and add to ready queue when all predecessors done
                let deg = in_degree.get_mut(next_block_id).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    ready_queue.push_back(*next_block_id);
                }
            }

            // Store the post-state
            state_map.get_mut(&block_id).expect("basic block").post = post;
        }

        state_map
    }
}

// =================================================================================================
// Analyzer Methods

impl<'env> SpecInferenceAnalyzer<'env> {
    /// Get the struct environment for a given module and struct id.
    fn get_struct(&self, module_id: ModuleId, struct_id: StructId) -> StructEnv<'env> {
        self.global_env().get_struct(QualifiedId {
            module_id,
            id: struct_id,
        })
    }

    fn new(fun_env: &'env FunctionEnv<'env>, data: &'env FunctionData) -> Self {
        let target = FunctionTarget::new(fun_env, data);
        let env = fun_env.module_env.env;

        // Create the "at_end" label representing the final state
        let at_end_label = env.new_global_id();
        let at_end_sym = env.symbol_pool().make("at_end");
        env.set_memory_label_name(at_end_label, at_end_sym);

        // Pre-scan bytecodes to build mapping from borrow_global_mut dest temps
        // to their struct info and address temps (needed for global un-resolve in loops).
        let mut borrow_global_info = BTreeMap::new();
        let mut havoc_targets = BTreeSet::new();
        for bc in target.get_bytecode() {
            if let Bytecode::Call(_, dests, Operation::BorrowGlobal(mid, sid, targs), srcs, _) = bc
            {
                borrow_global_info.insert(dests[0], (*mid, *sid, targs.clone(), srcs[0]));
            }
            if let Bytecode::Call(_, dests, Operation::Havoc(_), _, _) = bc {
                havoc_targets.insert(dests[0]);
            }
        }

        Self {
            fun_env,
            target,
            current_loc: fun_env.get_loc(),
            at_end_label,
            offset_labels: RefCell::new(BTreeMap::new()),
            borrow_global_info,
            havoc_targets,
            mutation_labels: RefCell::new(BTreeSet::new()),
        }
    }

    /// Create or retrieve a memory label for a specific code offset.
    /// Returns the same label for the same offset (for fixpoint stability).
    fn mk_label_at(&self, offset: CodeOffset) -> MemoryLabel {
        let mut cache = self.offset_labels.borrow_mut();
        *cache.entry(offset).or_insert_with(|| {
            let env = self.global_env();
            let label = env.new_global_id();
            let name = env.symbol_pool().make(&format!("at_{}", offset));
            env.set_memory_label_name(label, name);
            label
        })
    }

    /// Create a fresh memory label with a given name (not cached by offset).
    /// Use this when `mk_label_at` would clash with an existing label at the same offset.
    fn mk_fresh_label(&self, name: &str) -> MemoryLabel {
        let env = self.global_env();
        let label = env.new_global_id();
        let sym = env.symbol_pool().make(name);
        env.set_memory_label_name(label, sym);
        label
    }

    /// Shared WP logic for function calls (direct) and closure invocations.
    ///
    /// Handles: pre-label creation, intermediate labels for captured globals,
    /// behavioral state setup, extended result type computation, simultaneous
    /// substitution of dests and &mut post-values, captured_mut_params tracking,
    /// ensures_of for void calls, aborts_of emission, and post-state update.
    fn wp_function_call(
        &self,
        state: &mut WPState,
        offset: CodeOffset,
        fun_exp: Exp,
        args: Vec<Exp>,
        result_type: &Type,
        dests: &[TempIndex],
        mut_ref_srcs: &[(usize, TempIndex)],
    ) {
        // Create a new pre-label for this call site
        let pre_label = self.mk_label_at(offset);

        // When captured globals exist (mutations processed earlier in
        // backward order = later in program order), the function call
        // creates a state transition boundary. Insert an intermediate
        // label so the mutations' old() references point to the post-call
        // state rather than the function entry state.
        let call_post = if !state.captured_globals.is_empty() {
            let mid_label = self.mk_fresh_label(&format!("post_call_{}", offset));
            self.mutation_labels.borrow_mut().insert(mid_label);
            // Rewrite existing old(global[@state.post]<R>(...)) and
            // old(global[None]<R>(...)) → global[@mid_label]<R>(...)
            // for all resource types (opaque call).
            *state = self.rewrite_old_globals_to_label(
                state, None, // all resource types
                state.post, mid_label,
            );
            mid_label
        } else {
            state.post
        };

        // Create state labels for the behavioral predicates:
        // - result_of uses pre_label as pre-state and call_post as post-state
        // - aborts_of uses only pre_label (no post-state: aborts don't produce state)
        // The post-processing step (substitute_labels_in_state) will strip
        // labels that correspond to the function's entry/exit states, and
        // a second pass strips orphaned pre-labels (those referencing
        // a post-label that no behavioral predicate defines).
        let behavior_state = BehaviorState::new(Some(pre_label), Some(call_post));
        let aborts_state = BehaviorState::new(Some(pre_label), None);

        // Compute extended result type: explicit results + &mut post-values
        let extended_result_type = if mut_ref_srcs.is_empty() {
            result_type.clone()
        } else {
            let mut all_outputs: Vec<Type> = result_type.clone().flatten();
            for (_, idx) in mut_ref_srcs {
                all_outputs.push(self.get_local_type(*idx).skip_reference().clone());
            }
            if all_outputs.len() == 1 {
                all_outputs.into_iter().next().unwrap()
            } else {
                Type::Tuple(all_outputs)
            }
        };
        let num_all_outputs = extended_result_type.clone().flatten().len();

        // Collect ALL substitutions (explicit dests + &mut post-values)
        // and apply them simultaneously to avoid double-nesting of result_of
        // expressions. Sequential substitution would replace &mut src temps
        // inside already-substituted result_of args, producing incorrect
        // nested result_of expressions.
        let num_explicit = dests.len();
        let mut all_subs: Vec<(TempIndex, Exp)> = Vec::new();

        // Explicit dests: dest_i ↦ result_of<f>(args)[i]
        for (i, &dest) in dests.iter().enumerate() {
            let result_exp = self.mk_result_of_at_with_state(
                fun_exp.clone(),
                args.clone(),
                &extended_result_type,
                i,
                num_all_outputs,
                behavior_state.clone(),
            );
            all_subs.push((dest, result_exp));
        }

        // &mut src post-values: src_j ↦ result_of<f>(args)[num_explicit + j]
        for (j, (_, idx)) in mut_ref_srcs.iter().enumerate() {
            let result_exp = self.mk_result_of_at_with_state(
                fun_exp.clone(),
                args.clone(),
                &extended_result_type,
                num_explicit + j,
                num_all_outputs,
                behavior_state.clone(),
            );
            all_subs.push((*idx, result_exp));
        }

        // Apply all substitutions simultaneously
        *state = self.substitute_multiple_temps_in_state(state, &all_subs);

        // For &mut srcs that are params of the current function:
        // add ensures and mark as captured, mirroring WriteRef handling.
        for (j, (_, idx)) in mut_ref_srcs.iter().enumerate() {
            if self.is_mut_ref_param(*idx) {
                if !state.captured_mut_params.contains(idx) {
                    // First capture (last write in execution order):
                    // add ensures param == result_of<f>(args)[num_explicit + j]
                    let param_exp = self.mk_temporary(*idx);
                    let result_exp = self.mk_result_of_at_with_state(
                        fun_exp.clone(),
                        args.clone(),
                        &extended_result_type,
                        num_explicit + j,
                        num_all_outputs,
                        behavior_state.clone(),
                    );
                    state.add_ensures(self.mk_eq(param_exp, result_exp));
                    state.captured_mut_params.insert(*idx);
                } else {
                    // Already captured: substitute old(param) with the
                    // call's post-value (chains through earlier writes).
                    let result_exp = self.mk_result_of_at_with_state(
                        fun_exp.clone(),
                        args.clone(),
                        &extended_result_type,
                        num_explicit + j,
                        num_all_outputs,
                        behavior_state.clone(),
                    );
                    *state = self.substitute_old_param_in_state(state, *idx, &result_exp);
                }
            }
        }

        // For void-returning calls with no &mut params, emit ensures_of
        // to capture the callee's post-conditions and define the post-label
        // for state chaining.
        if dests.is_empty() && mut_ref_srcs.is_empty() {
            let ensures_of = self.mk_ensures_of_with_state(
                fun_exp.clone(),
                args.clone(),
                behavior_state.clone(),
            );
            state.add_ensures(ensures_of);
        }

        let aborts = self.mk_aborts_of_with_state(fun_exp, args, aborts_state);
        state.add_aborts(aborts);

        // Update post-state for predecessor: they see this call's pre-state
        state.post = pre_label;
    }

    /// WP for Pack/PackVariant: Q[dest ↦ pack(fields)].
    fn wp_pack(
        &self,
        state: &mut WPState,
        dest: TempIndex,
        srcs: &[TempIndex],
        module_id: ModuleId,
        struct_id: StructId,
        variant: Option<Symbol>,
        type_args: &[Type],
    ) {
        let fields: Vec<Exp> = srcs.iter().map(|&idx| self.mk_temporary(idx)).collect();
        let pack_exp = if let Some(v) = variant {
            self.mk_pack_variant(module_id, struct_id, v, type_args, fields)
        } else {
            self.mk_pack(module_id, struct_id, type_args, fields)
        };
        *state = self.substitute_exp_state(state, dest, &pack_exp);
    }

    /// WP for Unpack/UnpackVariant: Q[dest_i ↦ select field_i(src)].
    /// For variants, adds an abort condition if the value is not the expected variant.
    fn wp_unpack(
        &self,
        state: &mut WPState,
        dests: &[TempIndex],
        src: TempIndex,
        module_id: ModuleId,
        struct_id: StructId,
        variant: Option<Symbol>,
        type_args: &[Type],
    ) {
        let src_exp = self.mk_temporary(src);
        let struct_env = self.get_struct(module_id, struct_id);
        for (i, &dest) in dests.iter().enumerate() {
            let field_env = struct_env.get_field_by_offset_optional_variant(variant, i);
            let select_exp = self.mk_field_select(&field_env, type_args, src_exp.clone());
            *state = self.substitute_exp_state(state, dest, &select_exp);
        }
        if let Some(v) = variant {
            let not_variant = self.mk_not(self.mk_variant_test(&struct_env, v, src_exp));
            state.add_aborts(not_variant);
        }
    }

    /// WP for GetField/GetVariantField: Q[dest ↦ select field(src)].
    /// For captured &mut params, wraps the source in old().
    /// For variants, adds an abort condition if the value is not one of the expected variants.
    fn wp_get_field(
        &self,
        state: &mut WPState,
        dest: TempIndex,
        src: TempIndex,
        module_id: &ModuleId,
        struct_id: &StructId,
        variants: &[Symbol],
        type_args: &[Type],
        field_offset: usize,
    ) {
        let src_exp = if self.is_global_or_mut_param(state, src)
            && state.captured_mut_params.contains(&src)
        {
            self.mk_old(self.mk_temporary(src))
        } else {
            self.mk_temporary(src)
        };
        let struct_env = self.get_struct(*module_id, *struct_id);
        let field_env = struct_env
            .get_field_by_offset_optional_variant(variants.first().copied(), field_offset);
        let select_exp = self.mk_field_select(&field_env, type_args, src_exp.clone());
        *state = self.substitute_exp_state(state, dest, &select_exp);
        if !variants.is_empty() {
            let not_variant = self.mk_not(self.mk_variant_tests(&struct_env, variants, src_exp));
            state.add_aborts(not_variant);
        }
    }

    /// WP for borrowing a (variant) field:
    /// Q[dest => select S.field(src)] + optional abort if wrong variant.
    fn wp_borrow_field(
        &self,
        state: &mut WPState,
        dest: TempIndex,
        src: TempIndex,
        module_id: &ModuleId,
        struct_id: &StructId,
        variants: &[Symbol],
        type_args: &[Type],
        field_offset: usize,
    ) {
        let src_exp = if self.is_global_or_mut_param(state, src)
            && state.captured_mut_params.contains(&src)
        {
            self.mk_old(self.mk_temporary(src))
        } else {
            self.mk_temporary(src)
        };
        let struct_env = self.get_struct(*module_id, *struct_id);
        let field_env = struct_env
            .get_field_by_offset_optional_variant(variants.first().copied(), field_offset);
        let select_exp = self.mk_field_select(&field_env, type_args, src_exp.clone());
        *state = self.substitute_exp_state(state, dest, &select_exp);
        if !variants.is_empty() {
            let not_variant = self.mk_not(self.mk_variant_tests(&struct_env, variants, src_exp));
            state.add_aborts(not_variant);
        }
    }

    /// Check if a temporary is a `&mut` parameter.
    fn is_mut_ref_param(&self, idx: TempIndex) -> bool {
        idx < self.target.get_parameter_count()
            && self.target.get_local_type(idx).is_mutable_reference()
    }

    /// Check if a temporary is a `&mut` parameter or a captured global reference.
    fn is_global_or_mut_param(&self, state: &WPState, temp: TempIndex) -> bool {
        self.is_mut_ref_param(temp) || state.captured_globals.contains(&temp)
    }

    /// Check whether any already-captured global temp maps to the same resource
    /// (same `mid`, `sid`, `targs`, `addr_temp`) as `ref_temp`.
    /// This detects when the same global is borrowed via different temps.
    fn has_captured_same_global(&self, state: &WPState, ref_temp: TempIndex) -> bool {
        let Some(info) = self.borrow_global_info.get(&ref_temp) else {
            return false;
        };
        state.captured_globals.iter().any(|&captured| {
            captured != ref_temp
                && self.borrow_global_info.get(&captured).is_some_and(|ci| {
                    ci.0 == info.0 && ci.1 == info.1 && ci.2 == info.2 && ci.3 == info.3
                })
        })
    }

    /// Check whether any already-captured global temp maps to the same resource
    /// type (same `mid`, `sid`, `targs`) but a DIFFERENT `addr_temp` as `ref_temp`.
    /// Returns Some((mid, sid, targs, addr_temp)) of ref_temp if found.
    /// Handles N>=2 different-addr writes to the same resource type.
    fn find_same_resource_different_addr(
        &self,
        state: &WPState,
        ref_temp: TempIndex,
    ) -> Option<(ModuleId, StructId, Vec<Type>, TempIndex)> {
        let info = self.borrow_global_info.get(&ref_temp)?;
        let has_match = state.captured_globals.iter().any(|&captured| {
            captured != ref_temp
                && self.borrow_global_info.get(&captured).is_some_and(|ci| {
                    ci.0 == info.0 && ci.1 == info.1 && ci.2 == info.2 && ci.3 != info.3
                    // Different addr_temp
                })
        });
        if has_match {
            Some((info.0, info.1, info.2.clone(), info.3))
        } else {
            None
        }
    }

    /// Find whether any captured global in the state matches the given resource
    /// (module_id, struct_id, type_args, addr_temp). Returns the borrow temp if found.
    fn find_captured_global_for_resource(
        &self,
        state: &WPState,
        module_id: ModuleId,
        struct_id: StructId,
        type_args: &[Type],
        addr_temp: TempIndex,
    ) -> Option<TempIndex> {
        state.captured_globals.iter().find_map(|&captured| {
            self.borrow_global_info
                .get(&captured)
                .and_then(|(mid, sid, targs, atemp)| {
                    if *mid == module_id
                        && *sid == struct_id
                        && targs == type_args
                        && *atemp == addr_temp
                    {
                        Some(captured)
                    } else {
                        None
                    }
                })
        })
    }

    /// Retroactively rewrite `old(global[@old_label]<R>(...))` and
    /// `old(global[None]<R>(...))` → `global[@new_label]<R>(...)` in a WPState.
    /// This makes already-processed writes read from the intermediate state
    /// instead of from the function entry state.
    /// When `resource_type` is `Some`, only globals of that type are rewritten;
    /// when `None`, all resource types are rewritten.
    fn rewrite_old_globals_to_label(
        &self,
        state: &WPState,
        resource_type: Option<&Type>,
        old_label: MemoryLabel,
        new_label: MemoryLabel,
    ) -> WPState {
        let env = self.global_env();
        state.map(|e| {
            struct OldGlobalRewriter<'a> {
                env: &'a GlobalEnv,
                resource_type: Option<&'a Type>,
                old_label: MemoryLabel,
                new_label: MemoryLabel,
            }

            impl OldGlobalRewriter<'_> {
                fn matches_resource_type(&self, inner_id: NodeId) -> bool {
                    match self.resource_type {
                        Some(rt) => self.env.get_node_type(inner_id) == *rt,
                        None => true,
                    }
                }
            }

            impl ExpRewriterFunctions for OldGlobalRewriter<'_> {
                fn rewrite_call(&mut self, _id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                    if matches!(oper, AstOp::Old) && args.len() == 1 {
                        match args[0].as_ref() {
                            // Match: old(global[@old_label]<R>(...))
                            ExpData::Call(inner_id, AstOp::Global(Some(label)), inner_args)
                                if *label == self.old_label
                                    && self.matches_resource_type(*inner_id) =>
                            {
                                let rewritten_args: Vec<Exp> = inner_args
                                    .iter()
                                    .map(|a| self.rewrite_exp(a.clone()))
                                    .collect();
                                return Some(
                                    ExpData::Call(
                                        *inner_id,
                                        AstOp::Global(Some(self.new_label)),
                                        rewritten_args,
                                    )
                                    .into_exp(),
                                );
                            },
                            // Match: old(global[None]<R>(...)) — bare old(global<R>(x))
                            ExpData::Call(inner_id, AstOp::Global(None), inner_args)
                                if self.matches_resource_type(*inner_id) =>
                            {
                                let rewritten_args: Vec<Exp> = inner_args
                                    .iter()
                                    .map(|a| self.rewrite_exp(a.clone()))
                                    .collect();
                                return Some(
                                    ExpData::Call(
                                        *inner_id,
                                        AstOp::Global(Some(self.new_label)),
                                        rewritten_args,
                                    )
                                    .into_exp(),
                                );
                            },
                            _ => {},
                        }
                    }
                    None
                }
            }

            OldGlobalRewriter {
                env,
                resource_type,
                old_label,
                new_label,
            }
            .rewrite_exp(e.clone())
        })
    }

    /// Build an intermediate frame condition:
    /// `forall a: address: a != addr_exp ==> global[@mid]<R>(a) == old(global<R>(a))`
    fn mk_intermediate_frame(
        &self,
        mid_label: MemoryLabel,
        struct_env: &StructEnv,
        type_args: &[Type],
        addr_exp: Exp,
    ) -> Exp {
        // Build the quantifier: forall a: address
        let sym = self.mk_symbol("$a");
        let addr_type = Type::Primitive(PrimitiveType::Address);
        let a_var = self.mk_local_by_sym(sym, addr_type.clone());

        // a != addr_exp
        let neq = self.mk_not(self.mk_eq(a_var.clone(), addr_exp));

        // global[@mid]<R>(a)
        let global_mid =
            self.mk_global_with_label(struct_env, type_args, a_var.clone(), Some(mid_label));

        // old(global<R>(a)) — Global(None) inside old() represents entry state
        let global_old = self.mk_old(self.mk_global(struct_env, type_args, a_var.clone()));

        // global[@mid]<R>(a) == old(global<R>(a))
        let eq = self.mk_eq(global_mid, global_old);

        // a != addr_exp ==> global[@mid]<R>(a) == old(global<R>(a))
        let body = self.mk_implies(neq, eq);

        // forall a: address: body
        let range = self.mk_type_domain(addr_type.clone());
        let pat = self.mk_decl(sym, addr_type);
        let node_id = self.new_node(BOOL_TYPE.clone(), None);
        ExpData::Quant(
            node_id,
            QuantKind::Forall,
            vec![(pat, range)],
            vec![],
            None,
            body,
        )
        .into_exp()
    }

    /// Strip memory labels inside `old()` wrappers in a WPState.
    /// Transforms: `old(global[@label]<R>(a))` → `old(global<R>(a))`
    /// This is needed because BorrowGlobal substitution inserts the state.post label
    /// into all occurrences of the temp (including those inside old()), but labels
    /// inside old() are semantically wrong (old() already refers to function entry state).
    fn strip_labels_inside_old(&self, state: &WPState) -> WPState {
        state.map(|e| {
            struct StripOldLabels;

            impl ExpRewriterFunctions for StripOldLabels {
                fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                    if matches!(oper, AstOp::Old) && args.len() == 1 {
                        // Rewrite the inner expression first, then strip labels
                        let inner = self.rewrite_exp(args[0].clone());
                        let stripped = strip_labels_in_exp(&inner);
                        if !stripped.structural_eq(&args[0]) {
                            Some(ExpData::Call(id, AstOp::Old, vec![stripped]).into_exp())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }

            StripOldLabels.rewrite_exp(e.clone())
        })
    }

    /// Substitute memory labels in an expression.
    /// The `label_map` function returns:
    /// - `None` to keep the label unchanged
    /// - `Some(None)` to remove the label (set to None)
    /// - `Some(Some(new_label))` to replace with a new label
    fn substitute_labels(
        &self,
        exp: &Exp,
        label_map: &impl Fn(MemoryLabel) -> Option<Option<MemoryLabel>>,
    ) -> Exp {
        struct LabelRewriter<'a, F> {
            label_map: &'a F,
        }

        impl<F: Fn(MemoryLabel) -> Option<Option<MemoryLabel>>> ExpRewriterFunctions
            for LabelRewriter<'_, F>
        {
            fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                match oper {
                    AstOp::Behavior(kind, state) => {
                        // Apply label_map to pre and post labels
                        let new_pre = state
                            .pre
                            .map(|l| (self.label_map)(l).unwrap_or(Some(l)))
                            .unwrap_or(None);
                        let new_post = state
                            .post
                            .map(|l| (self.label_map)(l).unwrap_or(Some(l)))
                            .unwrap_or(None);
                        if new_pre != state.pre || new_post != state.post {
                            let new_state = BehaviorState::new(new_pre, new_post);
                            Some(
                                ExpData::Call(id, AstOp::Behavior(*kind, new_state), args.to_vec())
                                    .into_exp(),
                            )
                        } else {
                            None
                        }
                    },
                    AstOp::Global(Some(label)) => (self.label_map)(*label).map(|new_opt| {
                        ExpData::Call(id, AstOp::Global(new_opt), args.to_vec()).into_exp()
                    }),
                    AstOp::Exists(Some(label)) => (self.label_map)(*label).map(|new_opt| {
                        ExpData::Call(id, AstOp::Exists(new_opt), args.to_vec()).into_exp()
                    }),
                    _ => None,
                }
            }
        }

        let mut rewriter = LabelRewriter { label_map };
        rewriter.rewrite_exp(exp.clone())
    }

    /// Substitute memory labels in a WPState.
    /// The `label_map` function returns:
    /// - `None` to keep the label unchanged
    /// - `Some(None)` to remove the label (set to None)
    /// - `Some(Some(new_label))` to replace with a new label
    fn substitute_labels_in_state(
        &self,
        state: &WPState,
        label_map: &impl Fn(MemoryLabel) -> Option<Option<MemoryLabel>>,
    ) -> WPState {
        let ensures = state
            .ensures
            .iter()
            .map(|e| self.substitute_labels(e, label_map))
            .collect();
        let aborts = state
            .aborts
            .iter()
            .map(|e| self.substitute_labels(e, label_map))
            .collect();
        let direct_modifies = state
            .direct_modifies
            .iter()
            .map(|e| self.substitute_labels(e, label_map))
            .collect();
        // For post label, we keep it as-is since it's always required
        let new_post = label_map(state.post)
            .and_then(|opt| opt)
            .unwrap_or(state.post);
        WPState {
            ensures,
            aborts,
            origin_offset: state.origin_offset,
            post: new_post,
            captured_mut_params: state.captured_mut_params.clone(),
            captured_globals: state.captured_globals.clone(),
            direct_modifies,
        }
    }

    /// Collect all post-labels defined by `Behavior` operations in a WPState.
    fn collect_behavior_post_labels(&self, state: &WPState) -> BTreeSet<MemoryLabel> {
        let mut labels = BTreeSet::new();
        for e in state.ensures.iter().chain(state.aborts.iter()) {
            e.as_ref().visit_pre_order(&mut |e| {
                if let ExpData::Call(_, AstOp::Behavior(_, bs), _) = e {
                    if let Some(post) = bs.post {
                        labels.insert(post);
                    }
                }
                true
            });
        }
        labels
    }

    /// Strip pre-labels on `Behavior` operations that reference a post-label
    /// not defined by any other `Behavior` in the same state. This handles
    /// the case where a call's result is unused (no `result_of` emitted), so
    /// a subsequent call's pre-label has no matching post-label definition.
    fn strip_orphaned_behavior_pre_labels(&self, state: WPState) -> WPState {
        let defined = self.collect_behavior_post_labels(&state);

        struct StripOrphans<'a> {
            defined: &'a BTreeSet<MemoryLabel>,
        }
        impl ExpRewriterFunctions for StripOrphans<'_> {
            fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                if let AstOp::Behavior(kind, bs) = oper {
                    if let Some(pre) = bs.pre {
                        if !self.defined.contains(&pre) {
                            let new_bs = BehaviorState::new(None, bs.post);
                            return Some(
                                ExpData::Call(id, AstOp::Behavior(*kind, new_bs), args.to_vec())
                                    .into_exp(),
                            );
                        }
                    }
                }
                None
            }
        }

        let mut rewriter = StripOrphans { defined: &defined };
        let ensures = state
            .ensures
            .iter()
            .map(|e| rewriter.rewrite_exp(e.clone()))
            .collect();
        let aborts = state
            .aborts
            .iter()
            .map(|e| rewriter.rewrite_exp(e.clone()))
            .collect();
        WPState {
            ensures,
            aborts,
            direct_modifies: state.direct_modifies,
            ..state
        }
    }

    /// Prepare bytecode for backward WP analysis by neutralizing abort handler blocks.
    ///
    /// Abort actions create edges from `Call` instructions to abort handler blocks.
    /// These are redundant for spec inference because abort conditions are computed
    /// analytically at each `Call` site (overflow checks, `!exists`, `aborts_of`, etc.).
    ///
    /// If left in the backward CFG, the `Abort` instruction's `aborts = [true]`
    /// propagates backward through these edges and creates spurious conditions.
    ///
    /// This function:
    /// 1. Collects abort handler labels (targets of `AbortAction` on `Call` instructions).
    /// 2. Strips `AbortAction` from all `Call` instructions (removing CFG edges).
    /// 3. Neutralizes handler blocks by replacing non-`Label` instructions with a
    ///    self-loop `Jump`. This makes them non-exit blocks, so the backward CFG
    ///    won't process them. User-written abort blocks are preserved.
    fn prepare_bytecode_for_analysis(bytecode: &[Bytecode]) -> Vec<Bytecode> {
        // Phase 1: Collect abort handler labels
        let abort_handler_labels: BTreeSet<Label> = bytecode
            .iter()
            .filter_map(|bc| match bc {
                Bytecode::Call(_, _, _, _, Some(AbortAction(label, _))) => Some(*label),
                _ => None,
            })
            .collect();

        // Phase 2: Build modified bytecode
        let mut abort_handler_label = None;
        bytecode
            .iter()
            .map(|bc| {
                if let Bytecode::Label(_, label) = bc {
                    abort_handler_label = if abort_handler_labels.contains(label) {
                        Some(*label)
                    } else {
                        None
                    };
                }
                if let Some(handler_label) = abort_handler_label {
                    match bc {
                        Bytecode::Label(..) => bc.clone(),
                        _ => {
                            if bc.is_always_branching() {
                                abort_handler_label = None;
                            }
                            Bytecode::Jump(bc.get_attr_id(), handler_label)
                        },
                    }
                } else {
                    match bc {
                        Bytecode::Call(id, dests, op, srcs, Some(_)) => {
                            Bytecode::Call(*id, dests.clone(), op.clone(), srcs.clone(), None)
                        },
                        other => other.clone(),
                    }
                }
            })
            .collect()
    }

    /// Main analysis entry point using the dataflow framework.
    /// Returns the WP state at each code offset and whether the analysis was
    /// incomplete (some blocks were skipped due to cycles in the backward CFG,
    /// which happens when loops are not unrolled).
    fn analyze(&self) -> (BTreeMap<CodeOffset, WPState>, bool) {
        let bytecode = self.target.get_bytecode();
        if bytecode.is_empty() {
            return (BTreeMap::new(), false);
        }

        let bytecode_for_analysis = Self::prepare_bytecode_for_analysis(bytecode);
        let bytecode = &bytecode_for_analysis;

        // Build backward CFG for analysis (backward from exit to entry)
        // Use from_all_blocks=false so DUMMY_EXIT only connects to actual exit blocks (Ret/Abort),
        // not all blocks. This is needed for path-conditional join to work correctly.
        let cfg = StacklessControlFlowGraph::new_backward(bytecode, false);

        // Initial state: post points to the "at_end" label (the final state)
        let initial_state = WPState::new(self.at_end_label);

        // Run dataflow analysis
        let state_map = self.analyze_function(initial_state, bytecode, &cfg);

        // Detect if the analysis was incomplete: if the state_map has fewer non-dummy
        // blocks than the backward-reachable blocks, some were skipped due to cycles
        // (unprocessed loops). We use reachable_blocks from the backward CFG entry
        // (DUMMY_EXIT) to exclude neutralized abort handler blocks and any blocks
        // that only lead to them — these are intentionally unreachable.
        let num_analyzed_blocks = state_map.keys().filter(|b| !cfg.is_dummy(**b)).count();
        let num_reachable_blocks = cfg
            .reachable_blocks(cfg.entry_block(), |_, _| true)
            .into_iter()
            .filter(|b| !cfg.is_dummy(*b))
            .count();
        let has_skipped_blocks = num_analyzed_blocks < num_reachable_blocks;

        // Get per-instruction state (for backward analysis, 'before' is what we need at entry)
        let wp_map =
            self.state_per_instruction(state_map, bytecode, &cfg, |before, _after| before.clone());
        (wp_map, has_skipped_blocks)
    }

    /// Substitute occurrences of dest with src in the state
    fn substitute_state(&self, state: &WPState, dest: TempIndex, src: TempIndex) -> WPState {
        state.map(|e| self.substitute_temp(e, dest, src))
    }

    // =================================================================================================
    // Expression Builders

    /// Create ensures conditions for return values.
    /// Returns a WPState with one ensures condition per return value (result_i == val_i).
    /// Creates conditions for ALL return values - backward analysis through assignments
    /// will substitute temporaries with their sources, and at the end we filter to keep
    /// only conditions that reference parameters.
    fn mk_return_ensures(&self, vals: &[TempIndex]) -> WPState {
        let result_type = self.fun_env.get_result_type();
        let types = result_type.flatten();

        // Build equality expressions for each return value
        let ensures: Vec<Exp> = vals
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let ty = if i < types.len() {
                    types[i].clone()
                } else {
                    Type::Primitive(PrimitiveType::Bool)
                };
                let result_exp = self.mk_result(i, &ty);
                let val_exp = self.mk_temporary(val);
                self.mk_eq(result_exp, val_exp)
            })
            .collect();

        WPState {
            ensures,
            aborts: vec![],
            origin_offset: None, // Will be set by execute()
            post: self.at_end_label,
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            direct_modifies: vec![],
        }
    }

    /// Substitute temp `dest` with temp `src` in expression
    fn substitute_temp(&self, exp: &Exp, dest: TempIndex, src: TempIndex) -> Exp {
        let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
            if let RewriteTarget::Temporary(idx) = target {
                if idx == dest {
                    return Some(self.mk_temporary(src));
                }
            }
            None
        };
        ExpRewriter::new(self.global_env(), &mut replacer).rewrite_exp(exp.clone())
    }

    /// Substitute temp `dest` with expression `replacement` in an expression
    fn substitute_temp_with_exp(&self, exp: &Exp, dest: TempIndex, replacement: &Exp) -> Exp {
        let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
            if let RewriteTarget::Temporary(idx) = target {
                if idx == dest {
                    return Some(replacement.clone());
                }
            }
            None
        };
        ExpRewriter::new(self.global_env(), &mut replacer).rewrite_exp(exp.clone())
    }

    /// Substitute temp `dest` with `replacement`, but skip occurrences inside `old()`.
    /// This is used for reference-typed havoc variables: the pre-state value (`old(r)`)
    /// should remain on the original parameter, not be replaced by the quantified variable.
    fn substitute_temp_outside_old(&self, exp: &Exp, dest: TempIndex, replacement: &Exp) -> Exp {
        ExpData::rewrite_exp_and_pattern(
            exp.clone(),
            &mut |e| match e.as_ref() {
                ExpData::Call(_, AstOp::Old, _) => {
                    // Don't descend into old() — pre-state values are fixed
                    RewriteResult::Rewritten(e)
                },
                // Freeze(Temporary(dest)) → replacement (strip Freeze).
                // The quantified variable has the base value type, so Freeze is
                // semantically unnecessary. Must match before Temporary so the
                // pre-order traversal doesn't descend into the inner node first.
                ExpData::Call(_, AstOp::Freeze(_), args)
                    if args.len() == 1
                        && matches!(args[0].as_ref(), ExpData::Temporary(_, idx) if *idx == dest) =>
                {
                    RewriteResult::Rewritten(replacement.clone())
                },
                ExpData::Temporary(_, idx) if *idx == dest => {
                    RewriteResult::Rewritten(replacement.clone())
                },
                _ => RewriteResult::Unchanged(e),
            },
            &mut |_, _| None,
        )
    }

    /// Replace `Freeze(false)(Temporary(idx))` with `Old(Temporary(idx))` for captured
    /// `&mut` params in an assertion condition.
    ///
    /// When a `&mut` param has been captured (written to later in execution), the raw
    /// `$t_idx` in the ensures context refers to the post-state value. But at the
    /// assertion's program point (e.g., loop header invariant base case), the dereference
    /// of the param gives its pre-state (or pre-loop) value. Wrapping in `Old()` ensures
    /// this is correctly modeled, and `substitute_old_param_in_state` from earlier writes
    /// (if any) will chain through correctly.
    fn replace_freeze_of_captured_mut_params(&self, exp: &Exp, state: &WPState) -> Exp {
        if state.captured_mut_params.is_empty() {
            return exp.clone();
        }
        ExpData::rewrite_exp_and_pattern(
            exp.clone(),
            &mut |e| match e.as_ref() {
                ExpData::Call(_, AstOp::Freeze(_), args)
                    if args.len() == 1
                        && matches!(
                            args[0].as_ref(),
                            ExpData::Temporary(_, idx)
                                if state.captured_mut_params.contains(idx)
                        ) =>
                {
                    // Replace Freeze($t_idx) with Old($t_idx)
                    RewriteResult::Rewritten(self.mk_old(args[0].clone()))
                },
                _ => RewriteResult::Unchanged(e),
            },
            &mut |_, _| None,
        )
    }

    /// Replace `global<R>(addr)` with `Old(global<R>(addr))` for captured globals in an
    /// Assert condition (loop invariant base case).
    ///
    /// Analogous to `replace_freeze_of_captured_mut_params` for `&mut` params: at the
    /// assertion's program point (before the loop), the global has its pre-loop value.
    /// Wrapping in `Old()` makes the base case tautological (e.g., `old(global).value ==
    /// old(global).value + 0`), which is correct.
    fn replace_global_of_captured_globals_with_old(&self, exp: &Exp, state: &WPState) -> Exp {
        if state.captured_globals.is_empty() {
            return exp.clone();
        }
        let env = self.global_env();
        ExpData::rewrite_exp_and_pattern(
            exp.clone(),
            &mut |e| match e.as_ref() {
                ExpData::Call(id, AstOp::Global(_label), args) if args.len() == 1 => {
                    // Extract struct info from node type
                    if let Type::Struct(mid, sid, targs) = env.get_node_type(*id) {
                        // Check if addr is a Temporary
                        if let ExpData::Temporary(_, addr_idx) = args[0].as_ref() {
                            if self
                                .find_captured_global_for_resource(
                                    state, mid, sid, &targs, *addr_idx,
                                )
                                .is_some()
                            {
                                return RewriteResult::Rewritten(self.mk_old(e));
                            }
                        }
                    }
                    RewriteResult::Unchanged(e)
                },
                _ => RewriteResult::Unchanged(e),
            },
            &mut |_, _| None,
        )
    }

    /// Replace `global<R>(addr)` with `Freeze(Temporary(borrow_temp))` for captured globals
    /// in an Assume condition (loop invariant induction hypothesis after havoc).
    ///
    /// After havoc, the borrow temp `$t` becomes `$q` (quantified variable). This links the
    /// invariant to the havocked variable, constraining it. Without this, the invariant uses
    /// `global<R>(addr)` which doesn't reference the borrow temp → unconstrained → vacuous.
    fn replace_global_of_captured_globals_with_freeze(&self, exp: &Exp, state: &WPState) -> Exp {
        if state.captured_globals.is_empty() {
            return exp.clone();
        }
        let env = self.global_env();
        ExpData::rewrite_exp_and_pattern(
            exp.clone(),
            &mut |e| match e.as_ref() {
                ExpData::Call(id, AstOp::Global(_label), args) if args.len() == 1 => {
                    // Extract struct info from node type
                    if let Type::Struct(mid, sid, targs) = env.get_node_type(*id) {
                        // Check if addr is a Temporary
                        if let ExpData::Temporary(_, addr_idx) = args[0].as_ref() {
                            if let Some(borrow_temp) = self.find_captured_global_for_resource(
                                state, mid, sid, &targs, *addr_idx,
                            ) {
                                // Replace global<R>(addr) with Freeze($t_borrow)
                                let temp_exp = self.mk_temporary(borrow_temp);
                                let freeze_id = self.new_node(env.get_node_type(*id), None);
                                return RewriteResult::Rewritten(
                                    ExpData::Call(freeze_id, AstOp::Freeze(false), vec![temp_exp])
                                        .into_exp(),
                                );
                            }
                        }
                    }
                    RewriteResult::Unchanged(e)
                },
                _ => RewriteResult::Unchanged(e),
            },
            &mut |_, _| None,
        )
    }

    /// Prepare an ensures expression for havoc of a captured `&mut` param.
    ///
    /// In a loop, the ensures from WriteRef has the form:
    ///   `Implies(conditions, Eq($t, expr))`
    /// where `expr` may contain `old($t)` from ReadRef (representing the read value).
    /// In straight-line code, `old($t)` correctly means the function pre-state. But in a
    /// loop after havoc, the read value is the *current iteration's* value, not `old($t)`.
    ///
    /// This function walks through the Implies chain to find `Eq($t, expr)` at the leaf:
    /// 1. In `expr`, replaces `old($t)` with bare `$t` (undo ReadRef's old-wrapping)
    /// 2. Wraps the Eq LHS `$t` in `old()` to protect it from havoc quantification
    ///
    /// After havoc quantification, `restore_ensures_after_ref_havoc` unwraps the LHS.
    fn prepare_ensures_for_ref_havoc(&self, exp: &Exp, idx: TempIndex) -> Exp {
        match exp.as_ref() {
            ExpData::Call(id, AstOp::Implies, args) if args.len() == 2 => {
                let new_body = self.prepare_ensures_for_ref_havoc(&args[1], idx);
                ExpData::Call(*id, AstOp::Implies, vec![args[0].clone(), new_body]).into_exp()
            },
            ExpData::Quant(id, QuantKind::Forall, ranges, triggers, cond, body) => {
                let new_body = self.prepare_ensures_for_ref_havoc(body, idx);
                ExpData::Quant(
                    *id,
                    QuantKind::Forall,
                    ranges.clone(),
                    triggers.clone(),
                    cond.clone(),
                    new_body,
                )
                .into_exp()
            },
            ExpData::Call(id, AstOp::Eq, args) if args.len() == 2 => {
                if matches!(args[0].as_ref(), ExpData::Temporary(_, i) if *i == idx) {
                    // Found Eq($t_idx, expr)
                    // Step 1: in expr, strip old($t_idx) → $t_idx
                    let temp_exp = self.mk_temporary(idx);
                    let new_rhs = ExpData::rewrite_exp_and_pattern(
                        args[1].clone(),
                        &mut |e| match e.as_ref() {
                            ExpData::Call(_, AstOp::Old, inner)
                                if inner.len() == 1
                                    && matches!(
                                        inner[0].as_ref(),
                                        ExpData::Temporary(_, i) if *i == idx
                                    ) =>
                            {
                                RewriteResult::Rewritten(temp_exp.clone())
                            },
                            _ => RewriteResult::Unchanged(e),
                        },
                        &mut |_, _| None,
                    );
                    // Step 2: wrap Eq LHS in old() to protect from quantification
                    let protected_lhs = self.mk_old(args[0].clone());
                    ExpData::Call(*id, AstOp::Eq, vec![protected_lhs, new_rhs]).into_exp()
                } else {
                    exp.clone()
                }
            },
            _ => exp.clone(),
        }
    }

    /// Restore ensures expression after havoc quantification of a captured `&mut` param.
    /// Unwraps `old()` from the protected Eq LHS (added by `prepare_ensures_for_ref_havoc`).
    #[allow(clippy::only_used_in_recursion)]
    fn restore_ensures_after_ref_havoc(&self, exp: &Exp, idx: TempIndex) -> Exp {
        match exp.as_ref() {
            ExpData::Call(id, AstOp::Implies, args) if args.len() == 2 => {
                let new_body = self.restore_ensures_after_ref_havoc(&args[1], idx);
                ExpData::Call(*id, AstOp::Implies, vec![args[0].clone(), new_body]).into_exp()
            },
            ExpData::Quant(id, QuantKind::Forall, ranges, triggers, cond, body) => {
                let new_body = self.restore_ensures_after_ref_havoc(body, idx);
                ExpData::Quant(
                    *id,
                    QuantKind::Forall,
                    ranges.clone(),
                    triggers.clone(),
                    cond.clone(),
                    new_body,
                )
                .into_exp()
            },
            ExpData::Call(id, AstOp::Eq, args) if args.len() == 2 => {
                // Check if LHS is old(Temporary(idx)) — the protected output
                if let ExpData::Call(_, AstOp::Old, inner) = args[0].as_ref() {
                    if inner.len() == 1
                        && matches!(inner[0].as_ref(), ExpData::Temporary(_, i) if *i == idx)
                    {
                        // Unwrap: old($t_idx) → $t_idx
                        return ExpData::Call(*id, AstOp::Eq, vec![
                            inner[0].clone(),
                            args[1].clone(),
                        ])
                        .into_exp();
                    }
                }
                exp.clone()
            },
            _ => exp.clone(),
        }
    }

    /// Substitute occurrences of dest with an expression in the state
    fn substitute_exp_state(&self, state: &WPState, dest: TempIndex, exp: &Exp) -> WPState {
        state.map(|e| self.substitute_temp_with_exp(e, dest, exp))
    }

    /// Simultaneously substitute multiple temporaries with expressions in the state.
    /// This is needed when a function call modifies multiple `&mut` args — sequential
    /// substitution would corrupt `result_of` expressions that reference the temps.
    fn substitute_multiple_temps_in_state(
        &self,
        state: &WPState,
        subs: &[(TempIndex, Exp)],
    ) -> WPState {
        state.map(|e| {
            let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
                if let RewriteTarget::Temporary(idx) = target {
                    for (dest, replacement) in subs {
                        if idx == *dest {
                            return Some(replacement.clone());
                        }
                    }
                }
                None
            };
            ExpRewriter::new(self.global_env(), &mut replacer).rewrite_exp(e.clone())
        })
    }

    /// Substitute `old($param_idx)` with `new_val` in an expression.
    /// This is used when encountering an earlier write to an already-captured `&mut` param.
    fn substitute_old_param(&self, exp: &Exp, param_idx: TempIndex, new_val: &Exp) -> Exp {
        struct OldParamRewriter<'a> {
            param_idx: TempIndex,
            new_val: &'a Exp,
        }

        impl ExpRewriterFunctions for OldParamRewriter<'_> {
            fn rewrite_call(&mut self, _id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                if matches!(oper, AstOp::Old) && args.len() == 1 {
                    if let ExpData::Temporary(_, idx) = args[0].as_ref() {
                        if *idx == self.param_idx {
                            return Some(self.new_val.clone());
                        }
                    }
                }
                None
            }
        }

        OldParamRewriter { param_idx, new_val }.rewrite_exp(exp.clone())
    }

    /// Substitute `old($param_idx)` with `new_val` in all expressions of a WPState.
    fn substitute_old_param_in_state(
        &self,
        state: &WPState,
        param_idx: TempIndex,
        new_val: &Exp,
    ) -> WPState {
        state.map(|e| self.substitute_old_param(e, param_idx, new_val))
    }

    /// Substitute all occurrences of `pattern` with `replacement` in an expression,
    /// using structural equality (ignoring NodeIds). This is used to "un-resolve"
    /// global expressions back to temporaries during unrolled loop WP chaining.
    fn substitute_exp_with_exp(&self, exp: &Exp, pattern: &Exp, replacement: &Exp) -> Exp {
        struct ExpSubstRewriter<'a> {
            pattern: &'a Exp,
            replacement: &'a Exp,
        }

        impl ExpRewriterFunctions for ExpSubstRewriter<'_> {
            fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                // Reconstruct the Call expression and check structural equality
                let candidate = ExpData::Call(id, oper.clone(), args.to_vec()).into_exp();
                if candidate.as_ref().structural_eq(self.pattern) {
                    Some(self.replacement.clone())
                } else {
                    None
                }
            }
        }

        ExpSubstRewriter {
            pattern,
            replacement,
        }
        .rewrite_exp(exp.clone())
    }

    // =================================================================================================
    // Arithmetic Operation Helpers

    /// Build the arithmetic expression for an operation (using ExpGenerator helpers).
    fn mk_arith_exp(&self, op: &Operation, srcs: &[TempIndex]) -> Exp {
        let a = self.mk_temporary(srcs[0]);
        let b = self.mk_temporary(srcs[1]);
        match op {
            Operation::Add => self.mk_num_add(a, b),
            Operation::Sub => self.mk_num_sub(a, b),
            Operation::Mul => self.mk_num_mul(a, b),
            Operation::Div => self.mk_num_div(a, b),
            Operation::Mod => self.mk_num_mod(a, b),
            _ => unreachable!(),
        }
    }

    /// Build abort condition for arithmetic operation.
    /// Returns None if type is not a bounded integer.
    fn mk_arith_abort_cond(
        &self,
        op: &Operation,
        dest: TempIndex,
        srcs: &[TempIndex],
    ) -> Option<Exp> {
        let ty = self.get_local_type(dest);
        let Type::Primitive(prim_ty) = &ty else {
            return None;
        };

        let a = self.mk_temporary(srcs[0]);
        let b = self.mk_temporary(srcs[1]);

        match op {
            Operation::Add | Operation::Sub | Operation::Mul => {
                // Compute result in spec (arbitrary precision num)
                let result = self.mk_arith_exp(op, srcs);
                // Determine which bound check is needed:
                // - Unsigned Add/Mul: operands ≥ 0, so result ≥ 0 → only overflow
                // - Unsigned Sub: result ≤ max operand ≤ MAX → only underflow
                // - Signed: both overflow and underflow possible
                let kind = if prim_ty.is_signed() {
                    RangeCheckKind::Both
                } else {
                    match op {
                        Operation::Add | Operation::Mul => RangeCheckKind::Overflow,
                        Operation::Sub => RangeCheckKind::Underflow,
                        _ => unreachable!(),
                    }
                };
                self.mk_range_check(prim_ty, kind, result)
            },
            Operation::Div => {
                // Division aborts on: b == 0, or for signed: a == MIN && b == -1
                let zero = self.mk_num_const(BigInt::zero());
                let div_zero = self.mk_eq(b.clone(), zero);

                if ty.is_signed_int() {
                    let min = self.mk_num_min(prim_ty)?;
                    let neg_one = self.mk_num_const(BigInt::from(-1));
                    let a_eq_min = self.mk_eq(a, min);
                    let b_eq_neg1 = self.mk_eq(b, neg_one);
                    let min_div_neg1 = self.mk_and(a_eq_min, b_eq_neg1);
                    Some(self.mk_or(div_zero, min_div_neg1))
                } else {
                    Some(div_zero)
                }
            },
            Operation::Mod => {
                // Modulo aborts on: b == 0
                let zero = self.mk_num_const(BigInt::zero());
                Some(self.mk_eq(b, zero))
            },
            _ => None,
        }
    }

    /// Build a comparison expression for an operation.
    fn mk_cmp_exp(&self, op: &Operation, srcs: &[TempIndex]) -> Exp {
        let a = self.mk_temporary(srcs[0]);
        let b = self.mk_temporary(srcs[1]);
        let ast_op = match op {
            Operation::Eq => AstOp::Eq,
            Operation::Neq => AstOp::Neq,
            Operation::Lt => AstOp::Lt,
            Operation::Le => AstOp::Le,
            Operation::Gt => AstOp::Gt,
            Operation::Ge => AstOp::Ge,
            _ => unreachable!(),
        };
        self.mk_bool_call(ast_op, vec![a, b])
    }

    /// Build a logical expression for an operation.
    fn mk_logical_exp(&self, op: &Operation, srcs: &[TempIndex]) -> Exp {
        match op {
            Operation::Not => {
                let a = self.mk_temporary(srcs[0]);
                self.mk_not(a)
            },
            Operation::And => {
                let a = self.mk_temporary(srcs[0]);
                let b = self.mk_temporary(srcs[1]);
                self.mk_and(a, b)
            },
            Operation::Or => {
                let a = self.mk_temporary(srcs[0]);
                let b = self.mk_temporary(srcs[1]);
                self.mk_or(a, b)
            },
            _ => unreachable!(),
        }
    }

    /// Build a bitwise expression for an operation.
    fn mk_bitwise_exp(&self, op: &Operation, srcs: &[TempIndex]) -> Exp {
        let a = self.mk_temporary(srcs[0]);
        let b = self.mk_temporary(srcs[1]);
        match op {
            Operation::BitOr => self.mk_bit_or(a, b),
            Operation::BitAnd => self.mk_bit_and(a, b),
            Operation::Xor => self.mk_xor(a, b),
            Operation::Shl => self.mk_shl(a, b),
            Operation::Shr => self.mk_shr(a, b),
            _ => unreachable!(),
        }
    }

    /// Build a cast expression for the given cast operation.
    fn mk_cast_exp(&self, op: &Operation, srcs: &[TempIndex]) -> Exp {
        let src = self.mk_temporary(srcs[0]);
        let target_ty = self.cast_op_to_type(op);
        self.mk_cast(target_ty, src)
    }

    /// Get the target type for a cast operation.
    fn cast_op_to_type(&self, op: &Operation) -> Type {
        let prim = match op {
            Operation::CastU8 => PrimitiveType::U8,
            Operation::CastU16 => PrimitiveType::U16,
            Operation::CastU32 => PrimitiveType::U32,
            Operation::CastU64 => PrimitiveType::U64,
            Operation::CastU128 => PrimitiveType::U128,
            Operation::CastU256 => PrimitiveType::U256,
            Operation::CastI8 => PrimitiveType::I8,
            Operation::CastI16 => PrimitiveType::I16,
            Operation::CastI32 => PrimitiveType::I32,
            Operation::CastI64 => PrimitiveType::I64,
            Operation::CastI128 => PrimitiveType::I128,
            Operation::CastI256 => PrimitiveType::I256,
            _ => unreachable!(),
        };
        Type::Primitive(prim)
    }

    /// Build abort condition for cast operation (value out of target type range).
    fn mk_cast_abort_cond(&self, op: &Operation, srcs: &[TempIndex]) -> Option<Exp> {
        let src = self.mk_temporary(srcs[0]);
        let src_ty = self.get_local_type(srcs[0]);
        let target_ty = self.cast_op_to_type(op);
        if let Type::Primitive(prim_ty) = &target_ty {
            // Unsigned source: value ≥ 0 always, so only overflow is possible
            let kind = if src_ty.is_signed_int() {
                RangeCheckKind::Both
            } else {
                RangeCheckKind::Overflow
            };
            self.mk_range_check(prim_ty, kind, src)
        } else {
            None
        }
    }

    /// Build abort condition for shift operations (shift amount >= bit width).
    fn mk_shift_abort_cond(&self, dest: TempIndex, srcs: &[TempIndex]) -> Option<Exp> {
        let ty = self.get_local_type(dest);
        let Type::Primitive(prim_ty) = &ty else {
            return None;
        };
        let bit_width = prim_ty.get_num_bits()?;
        let shift_amount = self.mk_temporary(srcs[1]);
        let max_shift = self.mk_num_const(BigInt::from(bit_width));
        // Abort if shift_amount >= bit_width
        Some(self.mk_bool_call(AstOp::Ge, vec![shift_amount, max_shift]))
    }

    /// Build abort condition for negation (overflow for signed min value).
    fn mk_negate_abort_cond(&self, dest: TempIndex, srcs: &[TempIndex]) -> Option<Exp> {
        let ty = self.get_local_type(dest);
        let Type::Primitive(prim_ty) = &ty else {
            return None;
        };
        // Only signed types can overflow on negation (at MIN value)
        if !prim_ty.is_signed() {
            return None;
        }
        let src = self.mk_temporary(srcs[0]);
        let min_val = self.mk_num_min(prim_ty)?;
        // Abort if src == MIN (negating MIN overflows)
        Some(self.mk_eq(src, min_val))
    }

    /// Convert a bytecode Constant to an Exp value.
    /// Returns None for constants that can't be easily represented (e.g., vectors).
    fn constant_to_exp(&self, constant: &Constant) -> Option<Exp> {
        let value = match constant {
            Constant::Bool(b) => Value::Bool(*b),
            Constant::U8(n) => Value::Number(BigInt::from(*n)),
            Constant::U16(n) => Value::Number(BigInt::from(*n)),
            Constant::U32(n) => Value::Number(BigInt::from(*n)),
            Constant::U64(n) => Value::Number(BigInt::from(*n)),
            Constant::U128(n) => Value::Number(BigInt::from(*n)),
            Constant::U256(n) => Value::Number(BigInt::from_bytes_le(Sign::Plus, &n.to_le_bytes())),
            Constant::I8(n) => Value::Number(BigInt::from(*n)),
            Constant::I16(n) => Value::Number(BigInt::from(*n)),
            Constant::I32(n) => Value::Number(BigInt::from(*n)),
            Constant::I64(n) => Value::Number(BigInt::from(*n)),
            Constant::I128(n) => Value::Number(BigInt::from(*n)),
            Constant::I256(n) => {
                // For signed 256-bit integers, we need to handle sign
                let sign = if n.is_negative() {
                    Sign::Minus
                } else {
                    Sign::Plus
                };
                // Use absolute value bytes
                let abs = n.abs();
                Value::Number(BigInt::from_bytes_le(sign, &abs.to_le_bytes()))
            },
            // Address and complex types - skip for now
            _ => return None,
        };
        let ty = match constant {
            Constant::Bool(_) => Type::Primitive(PrimitiveType::Bool),
            // All numeric constants get NUM_TYPE for spec expressions
            _ => NUM_TYPE.clone(),
        };
        let node_id = self.new_node(ty, None);
        Some(ExpData::Value(node_id, value).into_exp())
    }

    // =================================================================================================
    // Reference Operation Helpers

    /// Build the transformation expression for a BorrowEdge.
    /// `trans[e](old, new)` applies the edge's transformation to update `old` with `new`.
    /// Returns None for unsupported edge types.
    fn mk_edge_transform(&self, edge: &BorrowEdge, old_exp: Exp, new_exp: Exp) -> Option<Exp> {
        match edge {
            BorrowEdge::Direct => {
                // Direct: just return the new value
                Some(new_exp)
            },
            BorrowEdge::Field(qid, _variants, offset) => {
                // Field update: UpdateField(old, new)
                let struct_env = self.global_env().get_struct(qid.to_qualified_id());
                let field_env = struct_env.get_field_by_offset(*offset);
                let type_args = qid.inst.as_slice();
                Some(self.mk_field_update(&field_env, type_args, old_exp, new_exp))
            },
            // Other edge types not yet supported
            BorrowEdge::Index(_) | BorrowEdge::Invoke | BorrowEdge::Hyper(_) => None,
        }
    }

    // =================================================================================================
    // IsParent Path Condition Resolution

    /// Compute path conditions for borrow temps referenced by `is_parent` operations.
    ///
    /// Uses dominator tree analysis on the forward CFG to determine under which branch
    /// conditions each borrow was created. Returns a map from `is_parent` destination temps
    /// to the path condition expressions that should replace them.
    fn compute_is_parent_substitutions(&self, instrs: &[Bytecode]) -> BTreeMap<TempIndex, Exp> {
        let label_offsets = Bytecode::label_offsets(instrs);

        // 1. Collect is_parent instructions: map from is_parent dest temp ->
        //    (parent_temp, operand_temp)
        let mut is_parent_info: BTreeMap<TempIndex, (TempIndex, TempIndex)> = BTreeMap::new();
        for instr in instrs {
            if let Bytecode::Call(
                _,
                dests,
                Operation::IsParent(BorrowNode::Reference(node_temp), _),
                srcs,
                _,
            ) = instr
            {
                if let (Some(&dest), Some(&operand)) = (dests.first(), srcs.first()) {
                    is_parent_info.insert(dest, (*node_temp, operand));
                }
            }
        }

        if is_parent_info.is_empty() {
            return BTreeMap::new();
        }

        // 2. Build forward CFG
        let fwd_cfg = StacklessControlFlowGraph::new_forward(instrs);

        // Helper to find which block contains a given offset
        let find_block_for_offset = |offset: usize| -> Option<BlockId> {
            for block_id in fwd_cfg.blocks() {
                let range = fwd_cfg.code_range(block_id);
                if range.contains(&offset) {
                    return Some(block_id);
                }
            }
            None
        };

        // 3. For each is_parent, find the derivation point: where the operand_temp
        //    is assigned from the parent_temp. This is the definition that determines
        //    under which branch condition the is_parent result is true.
        //    We need this because parent_temp may be a parameter with no definition
        //    in the function body.
        let mut derivation_def_block: BTreeMap<TempIndex, BlockId> = BTreeMap::new();
        for (&is_parent_dest, &(parent_temp, operand_temp)) in &is_parent_info {
            for (offset, instr) in instrs.iter().enumerate() {
                let is_derivation = match instr {
                    // Direct assignment: operand := parent
                    Bytecode::Assign(_, dest, src, _)
                        if *dest == operand_temp && *src == parent_temp =>
                    {
                        true
                    },
                    // Borrow that creates operand from parent (e.g. BorrowField)
                    Bytecode::Call(_, dests, _, srcs, _)
                        if dests.first() == Some(&operand_temp)
                            && srcs.first() == Some(&parent_temp) =>
                    {
                        true
                    },
                    _ => false,
                };
                if is_derivation {
                    if let Some(block_id) = find_block_for_offset(offset) {
                        derivation_def_block.insert(is_parent_dest, block_id);
                    }
                    break;
                }
            }
            // If no derivation found (operand == parent, or unconditional), check
            // if parent_temp has a definition we can use as fallback
            if !derivation_def_block.contains_key(&is_parent_dest) {
                for (offset, instr) in instrs.iter().enumerate() {
                    let defines_parent = match instr {
                        Bytecode::Call(_, dests, Operation::BorrowLoc, _, _) => {
                            dests.first() == Some(&parent_temp)
                        },
                        Bytecode::Call(_, dests, Operation::BorrowField(..), _, _) => {
                            dests.first() == Some(&parent_temp)
                        },
                        Bytecode::Call(_, dests, Operation::BorrowGlobal(..), _, _) => {
                            dests.first() == Some(&parent_temp)
                        },
                        Bytecode::Assign(_, dest, _, _) if *dest == parent_temp => true,
                        _ => false,
                    };
                    if defines_parent {
                        if let Some(block_id) = find_block_for_offset(offset) {
                            derivation_def_block.insert(is_parent_dest, block_id);
                        }
                        break;
                    }
                }
            }
        }

        // 4. Build Graph<BlockId> for dominator computation
        let fwd_blocks = fwd_cfg.blocks();
        let mut edges = vec![];
        for &block_id in &fwd_blocks {
            for succ in fwd_cfg.successors(block_id) {
                edges.push((block_id, *succ));
            }
        }
        let graph = Graph::new(fwd_cfg.entry_block(), fwd_blocks, edges);
        let dom = DomRelation::new(&graph);

        // 5. Walk dominator tree for each is_parent dest to compute path conditions
        let mut result: BTreeMap<TempIndex, Exp> = BTreeMap::new();
        for (&is_parent_dest, &def_block) in &derivation_def_block {
            let mut conditions: Vec<Exp> = vec![];
            let mut block = def_block;

            loop {
                let Some(idom) = dom.immediate_dominator(block) else {
                    break; // reached entry
                };

                // Check if the dominator block ends with a Branch
                let idom_range = fwd_cfg.code_range(idom);
                if !idom_range.is_empty() {
                    let last_offset = idom_range.end - 1;
                    if let Some(Bytecode::Branch(_, true_label, false_label, cond_temp)) =
                        instrs.get(last_offset)
                    {
                        let true_offset = label_offsets.get(true_label).copied();
                        let false_offset = label_offsets.get(false_label).copied();

                        // Find which block the true/false labels start
                        let find_block_at_offset = |offset: CodeOffset| -> Option<BlockId> {
                            for &bid in &fwd_cfg.blocks() {
                                let range = fwd_cfg.code_range(bid);
                                if range.start == offset as usize {
                                    return Some(bid);
                                }
                            }
                            None
                        };

                        let true_block = true_offset.and_then(&find_block_at_offset);
                        let false_block = false_offset.and_then(&find_block_at_offset);

                        // Check which side our block is dominated by
                        if let Some(tb) = true_block {
                            if dom.is_dominated_by(block, tb) {
                                // derivation is on the true side of this branch
                                conditions.push(self.mk_temporary(*cond_temp));
                            }
                        }
                        if let Some(fb) = false_block {
                            if dom.is_dominated_by(block, fb) {
                                // derivation is on the false side of this branch
                                let cond_exp = self.mk_temporary(*cond_temp);
                                conditions.push(self.mk_not(cond_exp));
                            }
                        }
                        // If dominated by neither true nor false block specifically
                        // (e.g., both merge before this block), the branch doesn't constrain it.
                    }
                }

                block = idom;
            }

            // Build conjunction of all conditions. If none, the derivation is unconditional (true).
            let condition = if conditions.is_empty() {
                self.mk_bool_const(true)
            } else {
                conditions
                    .into_iter()
                    .reduce(|a, b| self.mk_and(a, b))
                    .unwrap()
            };
            result.insert(is_parent_dest, condition);
        }

        result
    }

    /// Substitute `is_parent` temporaries in a WPState with their resolved path conditions.
    fn resolve_is_parent_in_state(
        &mut self,
        state: &WPState,
        substitutions: &BTreeMap<TempIndex, Exp>,
    ) -> WPState {
        if substitutions.is_empty() {
            return state.clone();
        }
        // First pass: substitute temporaries (immutable borrow)
        let substituted = state.map(|exp| {
            let mut result = exp.clone();
            for (&temp, replacement) in substitutions {
                result = self.substitute_temp_with_exp(&result, temp, replacement);
            }
            result
        });
        // Second pass: simplify (mutable borrow for ExpSimplifier)
        substituted.map(|exp| ExpSimplifier::new(self).simplify(exp.clone()))
    }

    // =================================================================================================
    // Branch-Aware Join Helpers

    /// Get branch info if a block ends with a Branch instruction.
    /// In backward analysis, we need to know if we're joining states from different branches.
    fn get_branch_info_for_block(
        &self,
        block_id: BlockId,
        instrs: &[Bytecode],
        cfg: &StacklessControlFlowGraph,
        label_offsets: &BTreeMap<Label, CodeOffset>,
    ) -> Option<BranchInfo> {
        // In backward CFG, the block we're joining INTO contains the Branch instruction
        // Get the last instruction of this block using code_range
        let range = cfg.code_range(block_id);
        if range.is_empty() {
            return None;
        }
        let last_offset = range.end - 1;
        if let Some(Bytecode::Branch(_, true_label, false_label, cond_temp)) =
            instrs.get(last_offset)
        {
            let true_offset = *label_offsets.get(true_label)?;
            let false_offset = *label_offsets.get(false_label)?;

            return Some(BranchInfo {
                cond_temp: *cond_temp,
                true_target_offset: true_offset,
                false_target_offset: false_offset,
            });
        }
        None
    }

    /// Perform path-aware join of WP states.
    /// When joining at a Branch instruction:
    /// - Common ensures (same in both branches) remain unconditional
    /// - Non-common ensures become conditional: `cond ==> ensures` for true branch,
    ///   `!cond ==> ensures` for false branch
    fn path_aware_join(
        &self,
        current: &mut WPState,
        incoming: &WPState,
        branch_info: Option<BranchInfo>,
        incoming_pred_offset: Option<CodeOffset>,
    ) -> JoinResult {
        // Substitute labels in incoming state to match current state's post label.
        // This ensures that expressions from both branches use the same memory labels
        // for proper comparison and joining.
        let incoming_post = incoming.post;
        let current_post = current.post;

        let incoming = &self.substitute_labels_in_state(incoming, &|label| {
            if label == incoming_post {
                Some(Some(current_post)) // Replace with current_post
            } else {
                None // Keep unchanged
            }
        });

        // If no branch info, fall back to standard join
        let Some(branch) = branch_info else {
            return current.join(incoming);
        };

        // Determine which state came from which branch by checking which target
        // is closer to (but <= ) the offset
        let classify_offset = |offset: CodeOffset| -> Option<bool> {
            if branch.true_target_offset <= offset && branch.false_target_offset <= offset {
                // Both targets are at or before this offset - pick the closer one
                if branch.true_target_offset > branch.false_target_offset {
                    Some(true) // true branch is closer (higher offset)
                } else if branch.false_target_offset > branch.true_target_offset {
                    Some(false) // false branch is closer
                } else {
                    None // Same offset, can't determine
                }
            } else if branch.true_target_offset <= offset {
                Some(true) // Only true branch is at or before
            } else if branch.false_target_offset <= offset {
                Some(false) // Only false branch is at or before
            } else {
                None // Neither applies
            }
        };

        // Determine which branch side each state came from.
        // `current.origin_offset` was set when the first state arrived at this block.
        // `incoming_pred_offset` is the predecessor block's last offset for the second state.
        let current_is_true = current.origin_offset.and_then(&classify_offset);
        let incoming_is_true = incoming_pred_offset.and_then(&classify_offset);

        match (current_is_true, incoming_is_true) {
            (Some(c), Some(i)) if c != i => {
                self.do_path_conditional_join(current, incoming, &branch, c)
            },
            _ => {
                // Can't determine sides; fall back to standard join
                current.join(incoming)
            },
        }
    }

    /// Perform the actual path-conditional join given branch info and which side current is on.
    fn do_path_conditional_join(
        &self,
        current: &mut WPState,
        incoming: &WPState,
        branch: &BranchInfo,
        current_is_true: bool,
    ) -> JoinResult {
        // Build path condition expression
        let cond_exp = self.mk_temporary(branch.cond_temp);
        let not_cond_exp = self.mk_not(cond_exp.clone());

        // Assign path conditions
        let (current_cond, incoming_cond) = if current_is_true {
            (cond_exp, not_cond_exp)
        } else {
            (not_cond_exp, cond_exp)
        };

        // Find common ensures (present in both) - these stay unconditional.
        // Use structural equality (ignoring NodeIds) for stable fixpoint convergence.
        let common_ensures: Vec<Exp> = current
            .ensures
            .iter()
            .filter(|e| ensures_contains(&incoming.ensures, e))
            .cloned()
            .collect();

        // Find ensures unique to current state
        let current_only: Vec<Exp> = current
            .ensures
            .iter()
            .filter(|e| !ensures_contains(&incoming.ensures, e))
            .cloned()
            .collect();

        // Find ensures unique to incoming state
        let incoming_only: Vec<Exp> = incoming
            .ensures
            .iter()
            .filter(|e| !ensures_contains(&current.ensures, e))
            .cloned()
            .collect();

        // Build new ensures: common + path-conditional
        let mut new_ensures = common_ensures;

        // Add path-conditional ensures for current-only
        for e in current_only {
            new_ensures.push(self.mk_implies(current_cond.clone(), e));
        }

        // Add path-conditional ensures for incoming-only
        for e in incoming_only {
            new_ensures.push(self.mk_implies(incoming_cond.clone(), e));
        }

        // Check if ensures changed (using structural equality for stable fixpoint convergence)
        let ensures_changed = current.ensures.len() != new_ensures.len()
            || current
                .ensures
                .iter()
                .zip(new_ensures.iter())
                .any(|(a, b)| !a.as_ref().structural_eq(b));

        // Update current state
        current.ensures = new_ensures;

        // Join aborts with path-conditional semantics: common aborts stay unconditional,
        // branch-specific aborts are wrapped with their path condition.
        let common_aborts: Vec<Exp> = current
            .aborts
            .iter()
            .filter(|e| ensures_contains(&incoming.aborts, e))
            .cloned()
            .collect();
        let current_only_aborts: Vec<Exp> = current
            .aborts
            .iter()
            .filter(|e| !ensures_contains(&incoming.aborts, e))
            .cloned()
            .collect();
        let incoming_only_aborts: Vec<Exp> = incoming
            .aborts
            .iter()
            .filter(|e| !ensures_contains(&current.aborts, e))
            .cloned()
            .collect();
        let mut new_aborts = common_aborts;
        for e in current_only_aborts {
            new_aborts.push(self.mk_and(current_cond.clone(), e));
        }
        for e in incoming_only_aborts {
            new_aborts.push(self.mk_and(incoming_cond.clone(), e));
        }
        let aborts_changed = current.aborts.len() != new_aborts.len()
            || current
                .aborts
                .iter()
                .zip(new_aborts.iter())
                .any(|(a, b)| !a.as_ref().structural_eq(b));
        current.aborts = new_aborts;

        // Handle captured_mut_params with path conditions:
        // If one path captured a param and the other didn't, add conditional ensures
        // for the path that didn't capture (param == old(param) on that path).
        let old_captured_len = current.captured_mut_params.len();
        for &idx in &current.captured_mut_params.clone() {
            if !incoming.captured_mut_params.contains(&idx) {
                // Current path captured, incoming didn't -> incoming path leaves param unchanged
                // Add: incoming_cond ==> param == old(param)
                let param_exp = self.mk_temporary(idx);
                let old_param = self.mk_old(param_exp.clone());
                let unchanged = self.mk_eq(param_exp, old_param);
                current
                    .ensures
                    .push(self.mk_implies(incoming_cond.clone(), unchanged));
            }
        }
        for &idx in &incoming.captured_mut_params {
            if !current.captured_mut_params.contains(&idx) {
                // Incoming path captured, current didn't -> current path leaves param unchanged
                let param_exp = self.mk_temporary(idx);
                let old_param = self.mk_old(param_exp.clone());
                let unchanged = self.mk_eq(param_exp, old_param);
                current
                    .ensures
                    .push(self.mk_implies(current_cond.clone(), unchanged));
            }
            // Also add to current's captured set (union semantics for tracking)
            current.captured_mut_params.insert(idx);
        }
        let captured_changed = current.captured_mut_params.len() != old_captured_len;

        // Handle captured_globals with the same logic as captured_mut_params.
        let old_captured_globals_len = current.captured_globals.len();
        for &idx in &current.captured_globals.clone() {
            if !incoming.captured_globals.contains(&idx) {
                // Current path captured global, incoming didn't -> global unchanged on incoming path
                let temp_exp = self.mk_temporary(idx);
                let old_temp = self.mk_old(temp_exp.clone());
                let unchanged = self.mk_eq(temp_exp, old_temp);
                current
                    .ensures
                    .push(self.mk_implies(incoming_cond.clone(), unchanged));
            }
        }
        for &idx in &incoming.captured_globals {
            if !current.captured_globals.contains(&idx) {
                // Incoming path captured global, current didn't -> global unchanged on current path
                let temp_exp = self.mk_temporary(idx);
                let old_temp = self.mk_old(temp_exp.clone());
                let unchanged = self.mk_eq(temp_exp, old_temp);
                current
                    .ensures
                    .push(self.mk_implies(current_cond.clone(), unchanged));
            }
            current.captured_globals.insert(idx);
        }
        let captured_globals_changed = current.captured_globals.len() != old_captured_globals_len;

        // Clear origin after merge since we've combined paths
        current.clear_origin();

        if ensures_changed || aborts_changed || captured_changed || captured_globals_changed {
            JoinResult::Changed
        } else {
            JoinResult::Unchanged
        }
    }
}
