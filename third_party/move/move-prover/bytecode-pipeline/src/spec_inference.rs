// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

use crate::{
    data_invariant_instrumentation::INVARIANT_FAILS_MESSAGE as DATA_INVARIANT_FAILS_MESSAGE,
    global_invariant_instrumentation::GLOBAL_INVARIANT_FAILS_MESSAGE,
    loop_analysis::{LoopInvariantEvidence, LoopsWithoutInvariants},
    options::ProverOptions,
    spec_instrumentation::{ABORTS_CODE_NOT_COVERED, ABORTS_IF_FAILS_MESSAGE, ABORT_NOT_COVERED},
    verification_analysis,
};
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::{Bytecode as MoveBytecode, CodeOffset};
use move_core_types::function::ClosureMask;
use move_model::{
    ast::{
        Condition, ConditionKind, Exp, ExpData, MemoryLabel, MemoryRange, Operation as AstOp,
        Pattern, PropertyValue, QuantKind, RewriteResult, TempIndex, Value,
    },
    exp_generator::{ExpGenerator, RangeCheckKind},
    exp_rewriter::{strip_all_olds, ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    exp_simplifier::{flatten_conjunction_owned, is_complementary, ExpSimplifier},
    memory_labels::{all_labels_in_exp, MemoryLabelInfo},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, QualifiedId, SpecFunId, StructEnv,
        StructId,
    },
    pragmas::{
        ABORTS_IF_IS_PARTIAL_PRAGMA, CONDITION_INFERRED_PROP, CONDITION_INFERRED_SATHARD,
        CONDITION_INFERRED_VACUOUS, INFERENCE_PRAGMA, OPAQUE_PRAGMA, VERIFY_PRAGMA,
    },
    sourcifier::Sourcifier,
    spec_derivation,
    symbol::Symbol,
    ty::{PrimitiveType, Type, BOOL_TYPE, NUM_TYPE},
    well_known,
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
use num::{BigInt, ToPrimitive, Zero};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    fmt,
    hash::{Hash, Hasher},
};

/// Prefix for inferred intermediate state labels in displayed specs.
const INFERRED_LABEL_PREFIX: &str = "S";

#[derive(Clone, Debug)]
struct LoopEvidenceSeed {
    offset: CodeOffset,
    head_index: usize,
    carried: Vec<(TempIndex, String)>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LoopHeadEvidence {
    pub facts: Vec<String>,
    pub omitted_facts: usize,
    pub incomplete: bool,
}

struct SpecInferenceRun {
    annotation: Option<WPAnnotation>,
    entry_state: Option<WPState>,
    incomplete: bool,
}

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
    /// Whether this state originated from a normal return (Ret instruction).
    /// This is `false` for abort-only or unreachable (Stop) paths. Used to
    /// distinguish functions with no return values (empty ensures but normal return)
    /// from abort-only states (empty ensures, no normal return).
    pub is_normal_return: bool,
    /// Predecessor block ID this state originated from (for edge tracking during joins).
    /// Used to identify which branch edge a state came from.
    pub origin_block: Option<BlockId>,
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
    /// Tracks globals modified via `update<R>` mutation builtins (from WriteBack(GlobalRoot)).
    /// Unlike `captured_globals`, these temps use in-place substitution in WriteBack(Reference)
    /// rather than the separate ensures path. Used by `has_global_mutations()` to create
    /// intermediate labels for sequential writes.
    pub update_globals: BTreeSet<TempIndex>,
    /// Tracks globals directly modified by MoveFrom/MoveTo (which bypass the borrow+writeback path).
    /// Each entry is a `global<R>(addr)` expression (no label) used to emit `modifies` clauses.
    pub direct_modifies: Vec<Exp>,
    /// Subset of `direct_modifies` produced by a mutation in this function's
    /// own body, rather than propagated from a callee frame.
    pub body_modifies: Vec<Exp>,
    /// Whether abort conditions were dropped because they crossed a memory
    /// havoc (loop-modified global memory): cumulative abort effects cannot be
    /// inferred exactly there. The resulting aborts specification is emitted
    /// as partial.
    pub aborts_partial: bool,
    /// Whether a behavioral summary came from a callee explicitly excluded
    /// from verification. Such contracts are useful hints but cannot support
    /// independently trusted caller conditions.
    pub solver_hard: bool,
}

impl WPState {
    /// Create a new WPState with the given post-state label
    fn new(post: MemoryLabel) -> Self {
        Self {
            ensures: vec![],
            aborts: vec![],
            is_normal_return: false,
            origin_block: None,
            post,
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            update_globals: BTreeSet::new(),
            direct_modifies: vec![],
            body_modifies: vec![],
            aborts_partial: false,
            solver_hard: false,
        }
    }

    /// Create a state with a single aborts condition
    fn with_aborts(exp: Exp, post: MemoryLabel) -> Self {
        Self {
            ensures: vec![],
            aborts: vec![exp],
            is_normal_return: false,
            origin_block: None,
            post,
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            update_globals: BTreeSet::new(),
            direct_modifies: vec![],
            body_modifies: vec![],
            aborts_partial: false,
            solver_hard: false,
        }
    }

    /// Transform conditions (e.g., for substitution)
    fn map(&self, mut f: impl FnMut(&Exp) -> Exp) -> Self {
        Self {
            ensures: self.ensures.iter().map(&mut f).collect(),
            aborts: self.aborts.iter().map(&mut f).collect(),
            is_normal_return: self.is_normal_return,
            origin_block: self.origin_block,
            post: self.post,
            captured_mut_params: self.captured_mut_params.clone(),
            captured_globals: self.captured_globals.clone(),
            update_globals: self.update_globals.clone(),
            direct_modifies: self.direct_modifies.iter().map(&mut f).collect(),
            body_modifies: self.body_modifies.iter().map(&mut f).collect(),
            aborts_partial: self.aborts_partial,
            solver_hard: self.solver_hard,
        }
    }

    /// Check if this state is empty (no conditions)
    fn is_empty(&self) -> bool {
        self.ensures.is_empty() && self.aborts.is_empty()
    }

    /// Clear origin tracking (used after joins to avoid stale tracking)
    fn clear_origin(&mut self) {
        self.origin_block = None;
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

    /// Record a mutation performed by bytecode in the current function.
    fn add_body_modifies(&mut self, exp: Exp) {
        push_if_new(&mut self.body_modifies, exp.clone());
        self.add_direct_modifies(exp);
    }

    /// Whether any global mutation has been captured in backward analysis so far.
    /// When true, `state.post` may have been updated to an intermediate label
    /// by a WriteBack, so operations that create abort conditions should not
    /// blindly use `state.post` as the existence-check label.
    fn has_global_mutations(&self) -> bool {
        !self.captured_globals.is_empty() || !self.update_globals.is_empty()
    }
}

impl AbstractDomain for WPState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let old_ensures_len = self.ensures.len();
        let old_aborts_len = self.aborts.len();
        let old_captured_len = self.captured_mut_params.len();

        // Abort-only states come from user-written `abort` statements or `Stop` in
        // loop bodies. In these cases the abort conditions are already captured
        // analytically in the transfer function. Skip abort-only states to avoid:
        // - ensures intersection removing ensures from the normal return path
        // - `aborts: true` creating spurious path-conditional aborts at Branch joins
        //
        // Note: We use `is_normal_return` instead of `ensures.is_empty()` because
        // functions with no return values have empty ensures on normal return paths.
        // Abort handler blocks (from `on_abort goto`) are neutralized before
        // analysis and never reach this point.
        let self_is_abort_only = !self.is_normal_return;
        let other_is_abort_only = !other.is_normal_return;

        self.aborts_partial = self.aborts_partial || other.aborts_partial;
        let old_solver_hard = self.solver_hard;
        self.solver_hard = self.solver_hard || other.solver_hard;

        if self_is_abort_only && !other_is_abort_only {
            // Current is abort-only; adopt incoming state wholesale
            self.ensures = other.ensures.clone();
            self.aborts = other.aborts.clone();
            self.is_normal_return = true;
        } else if !other_is_abort_only {
            // Both have ensures (both return normally): standard join
            self.ensures
                .retain(|exp| ensures_contains(&other.ensures, exp));
            for exp in &other.aborts {
                if !ensures_contains(&self.aborts, exp) {
                    self.aborts.push(exp.clone());
                }
            }
        } else if self_is_abort_only {
            // Both abort-only: union abort conditions from both paths
            for exp in &other.aborts {
                if !ensures_contains(&self.aborts, exp) {
                    self.aborts.push(exp.clone());
                }
            }
        }
        // else: other is abort-only but self has normal ensures — skip
        // (abort conditions are already captured at the abort site)

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

        // For update_globals: same union semantics.
        let old_update_globals_len = self.update_globals.len();
        for idx in &other.update_globals {
            self.update_globals.insert(*idx);
        }

        // For direct_modifies: union semantics (modification from ANY path counts)
        let old_direct_modifies_len = self.direct_modifies.len();
        for exp in &other.direct_modifies {
            push_if_new(&mut self.direct_modifies, exp.clone());
        }
        let old_body_modifies_len = self.body_modifies.len();
        for exp in &other.body_modifies {
            push_if_new(&mut self.body_modifies, exp.clone());
        }

        if self.ensures.len() != old_ensures_len
            || self.aborts.len() != old_aborts_len
            || self.captured_mut_params.len() != old_captured_len
            || self.captured_globals.len() != old_captured_globals_len
            || self.update_globals.len() != old_update_globals_len
            || self.direct_modifies.len() != old_direct_modifies_len
            || self.body_modifies.len() != old_body_modifies_len
            || self.solver_hard != old_solver_hard
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

/// Functions for which inference added an explicit global frame.
///
/// Frame targets are stored separately from conditions in the model. Keeping
/// this run-local marker lets source writers emit a frame-only inferred spec
/// without mistaking a handwritten `modifies` clause for inference output.
#[derive(Clone, Debug, Default)]
pub struct InferredFrameTargets(pub BTreeSet<QualifiedId<FunId>>);

/// Functions for which this invocation added inferred conditions.  Keeping a
/// run-local set prevents file output from appending conditions that were
/// merely loaded from a previous inference run.
#[derive(Clone, Debug, Default)]
pub struct InferredConditionTargets(pub BTreeSet<QualifiedId<FunId>>);

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

/// Merge complementary branch postconditions into one compact condition.
///
/// `P ==> L == A` together with `!P ==> L == B` is exactly
/// `L == if (P) A else B`. The same rule applies below a shared path prefix:
/// `C && P` / `C && !P` first merge under `C`, then the fixed-point loop can
/// merge `C` with its complement. Identical consequents collapse without an
/// `if`. The transformation is deliberately limited to complementary guards,
/// so it never guesses whether two paths are exhaustive.
fn combine_complementary_ensures<'env>(
    generator: &mut impl ExpGenerator<'env>,
    ensures: &[Exp],
) -> Vec<Exp> {
    fn as_implies(exp: &Exp) -> Option<(&Exp, &Exp)> {
        match exp.as_ref() {
            ExpData::Call(_, AstOp::Implies, args) if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }

    fn equality_parts(exp: &Exp) -> Option<(&Exp, &Exp)> {
        match exp.as_ref() {
            ExpData::Call(_, AstOp::Eq, args) if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }

    /// View a Boolean result/path proposition as an equality, so compact
    /// propositional forms produced by the expression simplifier can still be
    /// merged with an equality from the other branch. For example, `result`
    /// is treated as `result == true` and `!result` as `result == false`.
    fn equality_or_boolean_path<'env>(
        generator: &mut impl ExpGenerator<'env>,
        exp: &Exp,
    ) -> Option<(Exp, Exp)> {
        if let Some((lhs, rhs)) = equality_parts(exp) {
            return Some((lhs.clone(), rhs.clone()));
        }
        let (target, value) = match exp.as_ref() {
            ExpData::Call(_, AstOp::Not, args)
                if args.len() == 1 && is_procedure_level_path(&args[0]) =>
            {
                (&args[0], false)
            },
            _ if is_procedure_level_path(exp) => (exp, true),
            _ => return None,
        };
        if !generator
            .global_env()
            .get_node_type(target.node_id())
            .is_bool()
        {
            return None;
        }
        Some((target.clone(), generator.mk_bool_const(value)))
    }

    fn common_equality_target<'a>(
        a: (&'a Exp, &'a Exp),
        b: (&'a Exp, &'a Exp),
    ) -> Option<(&'a Exp, &'a Exp, &'a Exp)> {
        for (a_target, a_value) in [(a.0, a.1), (a.1, a.0)] {
            for (b_target, b_value) in [(b.0, b.1), (b.1, b.0)] {
                if a_target.structural_eq(b_target) {
                    return Some((a_target, a_value, b_value));
                }
            }
        }
        None
    }

    fn flatten_guard(exp: &Exp, out: &mut Vec<Exp>) {
        if let ExpData::Call(_, AstOp::And, args) = exp.as_ref() {
            if args.len() == 2 {
                flatten_guard(&args[0], out);
                flatten_guard(&args[1], out);
                return;
            }
        }
        out.push(exp.clone());
    }

    /// Return the shared guard and the branch condition from `left` when the
    /// two guards differ by exactly one complementary conjunct.
    fn complementary_guard_parts<'env>(
        generator: &mut impl ExpGenerator<'env>,
        left: &Exp,
        right: &Exp,
    ) -> Option<(Option<Exp>, Exp)> {
        if is_complementary(left, right) {
            return Some((None, left.clone()));
        }
        let mut left_parts = Vec::new();
        let mut right_parts = Vec::new();
        flatten_guard(left, &mut left_parts);
        flatten_guard(right, &mut right_parts);
        let mut common = Vec::new();
        let mut left_only = Vec::new();
        for part in left_parts {
            if let Some(index) = right_parts.iter().position(|r| r.structural_eq(&part)) {
                common.push(part);
                right_parts.remove(index);
            } else {
                left_only.push(part);
            }
        }
        if left_only.len() != 1
            || right_parts.len() != 1
            || !is_complementary(&left_only[0], &right_parts[0])
        {
            return None;
        }
        let shared = common
            .into_iter()
            .reduce(|lhs, rhs| generator.mk_bool_call(AstOp::And, vec![lhs, rhs]));
        Some((shared, left_only.remove(0)))
    }

    fn under_shared_guard<'env>(
        generator: &mut impl ExpGenerator<'env>,
        shared: Option<Exp>,
        body: Exp,
    ) -> Exp {
        match shared {
            Some(guard) => generator.mk_bool_call(AstOp::Implies, vec![guard, body]),
            None => body,
        }
    }

    let mut result = ensures.to_vec();
    loop {
        let mut replacement = None;
        'pairs: for i in 0..result.len() {
            let Some((guard_i, body_i)) = as_implies(&result[i]) else {
                continue;
            };
            for (j, item_j) in result.iter().enumerate().skip(i + 1) {
                let Some((guard_j, body_j)) = as_implies(item_j) else {
                    continue;
                };
                let Some((shared_guard, branch_guard)) =
                    complementary_guard_parts(generator, guard_i, guard_j)
                else {
                    continue;
                };
                if body_i.structural_eq(body_j) {
                    let merged = under_shared_guard(generator, shared_guard, body_i.clone());
                    let mut simplifier = ExpSimplifier::new(generator);
                    replacement = Some((i, j, simplifier.simplify(merged)));
                    break 'pairs;
                }
                let (Some(eq_i), Some(eq_j)) = (
                    equality_or_boolean_path(generator, body_i),
                    equality_or_boolean_path(generator, body_j),
                ) else {
                    continue;
                };
                let Some((target, on_i, on_j)) =
                    common_equality_target((&eq_i.0, &eq_i.1), (&eq_j.0, &eq_j.1))
                else {
                    continue;
                };
                let value_ty = generator.global_env().get_node_type(on_i.node_id());
                let ite_id = generator.new_node(value_ty, None);
                let ite =
                    ExpData::IfElse(ite_id, branch_guard, on_i.clone(), on_j.clone()).into_exp();
                let equality = generator.mk_bool_call(AstOp::Eq, vec![target.clone(), ite]);
                let equality = under_shared_guard(generator, shared_guard, equality);
                let mut simplifier = ExpSimplifier::new(generator);
                replacement = Some((i, j, simplifier.simplify(equality)));
                break 'pairs;
            }
        }
        let Some((i, j, merged)) = replacement else {
            return deduplicate_exps(result);
        };
        result.remove(j);
        result.remove(i);
        result.push(merged);
    }
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
                    if body_i.as_ref().structural_eq(body_j) && is_complementary(cond_i, cond_j) {
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

/// Normalize endpoint overflow checks on reference parameters after quantified
/// abort reasoning is complete. References are erased in emitted specs, but
/// their AST nodes retain `&T`; looking through that wrapper earlier would
/// perturb loop/quantifier simplification. At this final stage,
/// `r > MAX_T - 1` can safely become the clearer `r == MAX_T` while quantified
/// subexpressions are deliberately left untouched.
fn normalize_reference_endpoint_abort<'env>(
    generator: &mut impl ExpGenerator<'env>,
    exp: &Exp,
) -> Exp {
    if matches!(exp.as_ref(), ExpData::Quant(..)) {
        return exp.clone();
    }
    if let ExpData::Call(_, op, args) = exp.as_ref() {
        if args.len() == 2 && matches!(op, AstOp::And | AstOp::Or | AstOp::Implies) {
            let lhs = normalize_reference_endpoint_abort(generator, &args[0]);
            let rhs = normalize_reference_endpoint_abort(generator, &args[1]);
            return generator.mk_bool_call(op.clone(), vec![lhs, rhs]);
        }
        if args.len() == 1 && matches!(op, AstOp::Not) {
            let inner = normalize_reference_endpoint_abort(generator, &args[0]);
            return generator.mk_bool_call(op.clone(), vec![inner]);
        }
        let (value, constant) = match op {
            AstOp::Gt if args.len() == 2 => (&args[0], &args[1]),
            AstOp::Lt if args.len() == 2 => (&args[1], &args[0]),
            _ => return exp.clone(),
        };
        let ExpData::Value(_, Value::Number(constant)) = constant.as_ref() else {
            return exp.clone();
        };
        let ty = generator.global_env().get_node_type(value.node_id());
        let Type::Reference(_, inner) = ty else {
            return exp.clone();
        };
        let Type::Primitive(primitive) = inner.as_ref() else {
            return exp.clone();
        };
        let Some(max) = primitive.get_max_value() else {
            return exp.clone();
        };
        if constant.clone() + 1 != max {
            return exp.clone();
        }
        let max = generator.mk_num_const(max);
        return generator.mk_bool_call(AstOp::Eq, vec![value.clone(), max]);
    }
    exp.clone()
}

/// Information about a Branch instruction for path-conditional joining
struct BranchInfo {
    /// The condition temporary
    cond_temp: TempIndex,
    /// Block ID of the true branch target
    true_target_block: BlockId,
    /// Block ID of the false branch target
    false_target_block: BlockId,
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
        let inferred_sym = fun_env
            .module_env
            .env
            .symbol_pool()
            .make(CONDITION_INFERRED_PROP);
        if fun_env
            .get_spec()
            .conditions
            .iter()
            .any(|condition| condition.properties.contains_key(&inferred_sym))
        {
            return data;
        }

        let annotation = run_spec_inference_on_data(fun_env, &data, self.annotate, false);
        if let Some(annotation) = annotation {
            data.annotations.set::<WPAnnotation>(annotation, true);
        }
        report_uninvariant_loops(fun_env, &data);
        drop_vacuous_conditions(fun_env);
        data
    }

    fn dump_result(
        &self,
        f: &mut fmt::Formatter,
        env: &GlobalEnv,
        _targets: &FunctionTargetsHolder,
    ) -> fmt::Result {
        writeln!(f, "\n\n==== spec-inference results ====\n")?;

        let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);

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

                // A frame-only inferred contract has no condition carrying the
                // ordinary marker, so consult the run-local frame marker too.
                let has_inferred = spec
                    .conditions
                    .iter()
                    .any(|c| c.properties.contains_key(&inferred_sym))
                    || env
                        .get_extension::<InferredFrameTargets>()
                        .is_some_and(|frames| frames.0.contains(&fun.get_qualified_id()));

                if has_inferred {
                    // Print the entire function (signature + body + spec)
                    sourcifier.print_fun(fun.get_qualified_id(), fun.get_def());
                }
            }
        }

        write!(f, "{}", sourcifier.result())?;
        Ok(())
    }

    fn finalize(&self, env: &GlobalEnv, _targets: &mut FunctionTargetsHolder) {
        // A direct call to a transparent function executes that function's
        // body; it does not obtain its return value from the behavioral
        // `result_of` Skolem used for opaque/function-value calls.  When WP
        // had no exact value model and fell back to `result_of`, the resulting
        // candidate is therefore useful as a refinement hint but cannot be
        // independently verified yet.  Processing order is not a call-graph
        // order, so classify these dependencies here, after every inferred
        // callee has had a chance to become opaque.
        mark_transparent_result_dependencies_solver_hard(env);

        // Function-target processing order is not a call-graph order. A caller
        // can therefore consume an apparently complete `aborts_of<callee>`
        // before inference later marks that callee's abort summary partial.
        // Propagate the final partialness through inferred abort dependencies
        // to a fixpoint before the enriched source is written.
        propagate_inferred_partial_aborts(env);

        // These specifications are attached after the compiler's spec
        // rewriter has cached each function's memory summary. Inferred
        // behavioral predicates can introduce transitive memory reads (for
        // example `aborts_of<account::exists_at>`), and stale summaries make
        // the backend emit a Boogie function which refers to an undeclared
        // global memory variable. Refresh all inferred targets to a fixpoint
        // after processing, when their final conditions are available.
        refresh_inferred_spec_memory_usage(env, |_| true);
    }
}

/// Explain loop abstraction behind an imprecise inferred contract.
///
/// Loop analysis has already turned loops into a DAG before WP runs.  When a
/// loop lacks an invariant, the inserted havoc forces WP to quantify mutated
/// state. This can produce solver-hard clauses, or, more seriously, vacuous
/// clauses when the havocked state remains unconstrained. The location
/// annotation comes from that earlier transformation and preserves whether the
/// loop came from inline expansion.
/// Remove inferred conditions that constrain nothing.
///
/// A vacuous condition carries no information and is unsound to build on, so it
/// is never handed back; the loop that produced it is reported instead.
fn drop_vacuous_conditions(fun_env: &FunctionEnv) {
    let pool = fun_env.module_env.env.symbol_pool();
    let inferred_sym = pool.make(CONDITION_INFERRED_PROP);
    let vacuous_sym = pool.make(CONDITION_INFERRED_VACUOUS);
    let mut spec = fun_env.get_mut_spec();
    spec.conditions.retain(|condition| {
        !matches!(
            condition.properties.get(&inferred_sym),
            Some(PropertyValue::Symbol(value)) if *value == vacuous_sym
        )
    });
}

fn report_uninvariant_loops(fun_env: &FunctionEnv, data: &FunctionData) {
    let pool = fun_env.module_env.env.symbol_pool();
    let inferred_sym = pool.make(CONDITION_INFERRED_PROP);
    let vacuous_sym = pool.make(CONDITION_INFERRED_VACUOUS);
    let sathard_sym = pool.make(CONDITION_INFERRED_SATHARD);
    let (has_vacuous, has_sathard) = fun_env.get_spec().conditions.iter().fold(
        (false, false),
        |(has_vacuous, has_sathard), condition| match condition.properties.get(&inferred_sym) {
            Some(PropertyValue::Symbol(value)) if *value == vacuous_sym => (true, has_sathard),
            Some(PropertyValue::Symbol(value)) if *value == sathard_sym => (has_vacuous, true),
            _ => (has_vacuous, has_sathard),
        },
    );
    if !has_vacuous && !has_sathard {
        return;
    }
    let uninvariant = data
        .annotations
        .get::<LoopsWithoutInvariants>()
        .map(|loops| loops.0.as_slice())
        .unwrap_or_default();
    if uninvariant.is_empty() {
        // Weakest preconditions are exact without loops, so loop havoc is the
        // only source of a `vacuous` condition. `sathard` has others, such as a
        // top-level quantifier or an untrusted `result_of` carrier.
        if has_vacuous {
            fun_env.module_env.env.diag(
                Severity::Warning,
                &fun_env.get_loc(),
                "bug: inference produced a `vacuous` condition for a function with no \
                 uninvariant loop. Weakest preconditions are exact without loops, so this \
                 is a defect in the inference pass rather than a missing loop invariant.",
            );
        }
        return;
    }
    for loop_info in uninvariant {
        let (severity, message) = if has_vacuous {
            (
                Severity::Warning,
                "WP inferred `vacuous` conditions after this loop without an invariant. \
                 The loop havoc left part of the inferred condition unconstrained. \
                 Add a loop invariant before relying on the inferred specification.",
            )
        } else if loop_info.is_inlined {
            (
                Severity::Warning,
                "WP inferred `sathard` conditions after this loop without an invariant. \
                 The loop came from inline expansion: if it is an inline higher-order \
                 iterator, express its accumulator with a `folds_of` loop invariant. \
                 If fold handling is not applicable, rewrite the iterator call as a \
                 source `while` loop and provide ordinary loop invariants.",
            )
        } else {
            (
                Severity::Warning,
                "WP inferred `sathard` conditions after this loop without an invariant. \
             Add ordinary loop invariants, or, when the iteration is naturally a \
             fold, consider an inline higher-order iterator with a `folds_of` \
             loop invariant.",
            )
        };
        let evidence = data
            .annotations
            .get::<LoopInvariantEvidence>()
            .and_then(|all| {
                all.0
                    .iter()
                    .find(|evidence| evidence.loop_id == loop_info.loop_id)
            });
        if let Some(evidence) = evidence {
            let mut notes = vec![format!(
                "loop-invariant evidence (bounded to {} completed back-edge traversal(s); diagnostic only)",
                evidence.depth
            )];
            if !evidence.carried_names.is_empty() {
                notes.push(format!(
                    "source-visible loop-carried state: {}",
                    evidence.carried_names.join(", ")
                ));
            }
            if let Some(reason) = &evidence.unavailable {
                notes.push(format!("bounded WP evidence unavailable: {}", reason));
            } else {
                let status = if evidence.partial_notes.is_empty() {
                    "exact within the displayed bound"
                } else {
                    "partial"
                };
                notes.push(format!("bounded WP status: {}", status));
                let mut observations =
                    String::from("bounded loop-head facts (for paths reaching each head):");
                for head in &evidence.heads {
                    if head.facts.is_empty() {
                        observations.push_str(&format!(
                            "\n  head[{}]: no source-level fact retained",
                            head.index
                        ));
                    } else {
                        for (fact_index, fact) in head.facts.iter().enumerate() {
                            if fact_index == 0 {
                                observations
                                    .push_str(&format!("\n  head[{}]: {}", head.index, fact));
                            } else {
                                observations.push_str(&format!("\n           {}", fact));
                            }
                        }
                    }
                }
                notes.push(observations);
                notes.extend(
                    evidence
                        .partial_notes
                        .iter()
                        .map(|note| format!("partial evidence: {}", note)),
                );
            }
            notes.push(
                "seek a predicate which includes the entry facts and is preserved by one \
                 back-edge; bounded observations are not an invariant or a proof"
                    .to_string(),
            );
            fun_env
                .module_env
                .env
                .diag_with_notes(severity, &loop_info.loc, message, notes);
        } else {
            fun_env
                .module_env
                .env
                .diag(severity, &loop_info.loc, message);
        }
    }
}

fn mark_transparent_result_dependencies_solver_hard(env: &GlobalEnv) {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let sathard_sym = env.symbol_pool().make(CONDITION_INFERRED_SATHARD);
    for module in env.get_modules() {
        for fun in module.get_functions() {
            let indices = {
                let spec = fun.get_spec();
                spec.conditions
                    .iter()
                    .enumerate()
                    .filter(|(_, condition)| {
                        condition.properties.contains_key(&inferred_sym)
                            && condition_depends_on_transparent_result(env, &condition.exp)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>()
            };
            if indices.is_empty() {
                continue;
            }
            let mut spec = fun.get_mut_spec();
            for index in indices {
                spec.conditions[index]
                    .properties
                    .insert(inferred_sym, PropertyValue::Symbol(sathard_sym));
            }
        }
    }
}

fn condition_depends_on_transparent_result(env: &GlobalEnv, exp: &Exp) -> bool {
    let mut found = false;
    exp.visit_pre_order(&mut |node| {
        let ExpData::Call(_, AstOp::Behavior(move_model::ast::BehaviorKind::ResultOf, _), args) =
            node
        else {
            return true;
        };
        if let Some(fun_exp) = args.first()
            && let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = fun_exp.as_ref()
            && !env
                .get_function((*module_id).qualified(*fun_id))
                .is_opaque()
        {
            found = true;
        }
        !found
    });
    found
}

fn propagate_inferred_partial_aborts(env: &GlobalEnv) {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let partial_sym = env.symbol_pool().make(ABORTS_IF_IS_PARTIAL_PRAGMA);
    loop {
        let mut changed = false;
        for module in env.get_modules() {
            for fun in module.get_functions() {
                let retained = {
                    let spec = fun.get_spec();
                    spec.conditions
                        .iter()
                        .filter(|condition| {
                            !inferred_abort_depends_on_partial_callee(env, condition, inferred_sym)
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                };
                let old_len = fun.get_spec().conditions.len();
                if retained.len() != old_len {
                    changed = true;
                    let mut spec = fun.get_mut_spec();
                    spec.conditions = retained;
                    spec.properties
                        .entry(partial_sym)
                        .or_insert(PropertyValue::Value(Value::Bool(true)));
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn inferred_abort_depends_on_partial_callee(
    env: &GlobalEnv,
    condition: &Condition,
    inferred_sym: Symbol,
) -> bool {
    if !matches!(condition.kind, ConditionKind::AbortsIf)
        || !condition.properties.contains_key(&inferred_sym)
    {
        return false;
    }
    let mut found = false;
    condition.exp.visit_pre_order(&mut |node| {
        if let ExpData::Call(_, AstOp::Behavior(move_model::ast::BehaviorKind::AbortsOf, _), args) =
            node
            && let Some(fun_exp) = args.first()
            && let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = fun_exp.as_ref()
            && env
                .get_function((*module_id).qualified(*fun_id))
                .is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false)
        {
            found = true;
        }
        !found
    });
    found
}

/// Behavioral predicates are currently left uninterpreted when an abort
/// contract reflects over a generic type parameter. Such a predicate cannot
/// serve as a sufficient `aborts_if` condition in an inferred caller.
fn function_abort_spec_uses_generic_type_reflection(fun: &FunctionEnv<'_>) -> bool {
    let env = fun.module_env.env;
    fun.get_spec().conditions.iter().any(|condition| {
        matches!(
            condition.kind,
            ConditionKind::AbortsIf | ConditionKind::AbortsWith
        ) && condition
            .exp
            .called_spec_funs(env)
            .iter()
            .any(|called| env.spec_fun_uses_generic_type_reflection(called))
    })
}

// =================================================================================================
// LambdaSpecInferenceProcessor

/// Auto-infers specs for lambda-lifted functions which have no user-written spec
/// conditions, in verify mode. Lambdas passed to *non-inline* higher-order
/// functions are lifted into function values; without an inferred spec,
/// behavioral predicates over such a lambda degrade to trivial values at call
/// sites (`bp_ensures_of = true`, `bp_aborts_of = false`, `result_of`
/// unconstrained). Inferring `ensures result == …` and `aborts_if …` from the
/// body gives callers real information. (Lambdas passed to inline functions
/// are not lifted; behavioral predicates over them are inlined at the
/// expansion site by the inliner instead.)
///
/// Operates on the Baseline variant (before `SpecInstrumentationProcessor` would
/// clear it for opaque callees). Best-effort: if the analyzer cannot summarize the
/// body, the spec is left empty and behavioral predicates degrade as before.
pub struct LambdaSpecInferenceProcessor();

impl LambdaSpecInferenceProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self())
    }
}

impl FunctionTargetProcessor for LambdaSpecInferenceProcessor {
    fn name(&self) -> String {
        "lambda_spec_inference".to_string()
    }

    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() || fun_env.is_intrinsic() {
            return data;
        }
        if data.code.is_empty() {
            return data;
        }
        if !is_lambda_lifted_name(fun_env) {
            return data;
        }
        if fun_env.get_spec().has_conditions() {
            return data;
        }
        if !fun_env.module_env.env.is_verify_mode() {
            return data;
        }
        // Only infer for lambdas that are actually verified. The inferred spec is
        // marked opaque and trusted at call sites through `ensures_of`/`aborts_of`,
        // so the body must be checked against it; a lambda excluded from
        // verification (narrow scope, `pragma verify = false`, `--verify-exclude`)
        // must not contribute a trusted-but-unproven spec.
        if !verification_analysis::get_info(&FunctionTarget::new(fun_env, &data)).verified {
            return data;
        }
        if !needs_inference(fun_env) {
            return data;
        }
        let _ = run_spec_inference_on_data(
            fun_env, &data, /*annotate=*/ false, /*silent=*/ true,
        );
        data
    }

    fn finalize(&self, env: &GlobalEnv, _targets: &mut FunctionTargetsHolder) {
        // The lambdas' spec memory summaries were computed by the compiler's
        // spec rewriter when they had no specs; the conditions attached above
        // can reference global memory (directly or through behavioral
        // predicates over callees), and the behavioral evaluators derive
        // their memory parameters from the summaries — stale-empty summaries
        // would make the evaluators reference memory globals, which Boogie
        // rejects inside functions. Recomputed as a fixpoint here: a lambda
        // can reference another lambda (a nested lambda has no static call
        // edge, so per-function processing order guarantees nothing), and
        // the recomputation reads the target's summaries. Unions only grow,
        // so this terminates within the lambda nesting depth.
        refresh_inferred_spec_memory_usage(env, is_lambda_lifted_name);
    }
}

/// Recompute cached memory summaries for inferred specifications until
/// behavioral-predicate dependencies reach a fixpoint. Candidates are fixed
/// across rounds because `set_spec_memory_usage` changes no conditions.
fn refresh_inferred_spec_memory_usage(env: &GlobalEnv, include: impl Fn(&FunctionEnv) -> bool) {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let candidates: Vec<FunctionEnv> = env
        .get_modules()
        .flat_map(|module_env| module_env.into_functions())
        .filter(|fun_env| {
            include(fun_env)
                && fun_env
                    .get_spec()
                    .any(|c| c.properties.contains_key(&inferred_sym))
        })
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for fun_env in &candidates {
            let (used, generic_used, old, generic_old, uses_old) =
                fun_env.compute_spec_memory_usage();
            let stale = *fun_env.get_spec_used_memory() != used
                || *fun_env.get_spec_generic_used_memory() != generic_used
                || *fun_env.get_spec_old_memory() != old
                || *fun_env.get_spec_generic_old_memory() != generic_old
                || fun_env.spec_uses_old() != uses_old;
            if stale {
                fun_env.set_spec_memory_usage(used, generic_used, old, generic_old, uses_old);
                changed = true;
            }
        }
    }
}

fn is_lambda_lifted_name(fun_env: &FunctionEnv) -> bool {
    crate::lifted_lambda::is_lifted_lambda(fun_env)
}

// =================================================================================================
// Helper Functions

/// Checks if a function needs spec inference
fn needs_inference(fun_env: &FunctionEnv) -> bool {
    if let Some(mode) = fun_env.get_symbol_pragma(INFERENCE_PRAGMA) {
        let pool = fun_env.module_env.env.symbol_pool();
        pool.string(mode).as_str() != "none"
    } else {
        true
    }
}

/// Runs spec inference on a function's bytecode and writes inferred conditions into
/// the function's spec via [`update_spec`]. The caller is responsible for the higher-
/// level gates (e.g. native/intrinsic/empty-code checks, variant selection, and
/// [`needs_inference`]). Returns the WP annotation when `annotate` is true so callers
/// can install it on `FunctionData.annotations`.
///
/// When `silent_on_failure` is true, the WP-loss diagnostics (which indicate a bug in
/// inference for the normal path) are suppressed; this is appropriate for best-effort
/// uses where the function's spec is allowed to stay empty if the analyzer cannot
/// summarize the body.
fn run_spec_inference_on_data(
    fun_env: &FunctionEnv,
    data: &FunctionData,
    annotate: bool,
    silent_on_failure: bool,
) -> Option<WPAnnotation> {
    run_spec_inference_analysis(
        fun_env,
        data,
        annotate,
        silent_on_failure,
        /*update_model=*/ true,
        None,
    )
    .annotation
}

/// Run the existing WP engine from a synthetic bounded loop-head cut point.
/// Ordinary exits are neutral in this mode, so each returned condition is a
/// relation between function-entry values and a path which reaches this head.
/// The analysis is isolated: it never updates the function's specification.
pub(crate) fn infer_loop_head_evidence(
    fun_env: &FunctionEnv,
    data: &FunctionData,
    cutpoint_offset: CodeOffset,
    head_index: usize,
    carried: &[(TempIndex, String)],
) -> LoopHeadEvidence {
    const MAX_FACTS_PER_HEAD: usize = 8;

    let seed = LoopEvidenceSeed {
        offset: cutpoint_offset,
        head_index,
        carried: carried.to_vec(),
    };
    let run = run_spec_inference_analysis(
        fun_env,
        data,
        /*annotate=*/ false,
        /*silent_on_failure=*/ true,
        /*update_model=*/ false,
        Some(seed),
    );
    let Some(state) = run.entry_state else {
        return LoopHeadEvidence {
            incomplete: true,
            ..LoopHeadEvidence::default()
        };
    };

    let marker_prefix = format!("__loop_head_{}_", head_index);
    let mut facts = vec![];
    let mut omitted_facts = 0;
    for exp in state.ensures {
        let sourcifier = Sourcifier::new(fun_env.module_env.env, true);
        sourcifier.print_exp_for_fun_spec(fun_env, &exp);
        let mut rendered = sourcifier.result().trim().to_string();
        for (_, name) in carried {
            rendered = rendered.replace(
                &format!("{}{}", marker_prefix, name),
                &format!("head[{}].{}", head_index, name),
            );
        }
        // `$` denotes a compiler/WP temporary. Such a condition may be useful
        // internally but is not actionable source-level evidence.
        if rendered.contains('$') || rendered.contains(&marker_prefix) {
            omitted_facts += 1;
            continue;
        }
        facts.push(rendered);
    }
    facts.sort();
    facts.dedup();
    if facts.len() > MAX_FACTS_PER_HEAD {
        omitted_facts += facts.len() - MAX_FACTS_PER_HEAD;
        facts.truncate(MAX_FACTS_PER_HEAD);
    }
    LoopHeadEvidence {
        facts,
        omitted_facts,
        incomplete: run.incomplete,
    }
}

fn run_spec_inference_analysis(
    fun_env: &FunctionEnv,
    data: &FunctionData,
    annotate: bool,
    silent_on_failure: bool,
    update_model: bool,
    evidence_seed: Option<LoopEvidenceSeed>,
) -> SpecInferenceRun {
    let has_uninvariant_loop = data
        .annotations
        .get::<LoopsWithoutInvariants>()
        .is_some_and(|loops| !loops.0.is_empty());
    let mut analyzer = SpecInferenceAnalyzer::new_with_evidence_seed(fun_env, data, evidence_seed);
    let (wp_map, has_skipped_blocks) = analyzer.analyze();

    if has_skipped_blocks {
        if !silent_on_failure {
            fun_env.module_env.env.diag(
                Severity::Bug,
                &fun_env.get_loc(),
                "unexpected loss of weakest precondition: \
                 loops in backward CFG prevented complete analysis",
            );
        }
        drop(analyzer);
        return SpecInferenceRun {
            annotation: annotate.then_some(WPAnnotation(wp_map)),
            entry_state: None,
            incomplete: true,
        };
    }

    let mut normalized_entry = None;

    // By construction, for well-typed code the WP at entry should only reference
    // parameters. If this invariant is violated, it indicates a bug in spec inference.
    let entry_state = wp_map.get(&0).or_else(|| wp_map.get(&1));
    if let Some(state) = entry_state {
        let entry_post_label = state.post;
        let mut state = state.clone();

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
        if analyzer.evidence_seed.is_none() {
            for idx in 0..num_params {
                let ty = analyzer.get_local_type(idx);
                if ty.is_mutable_reference() && !state.captured_mut_params.contains(&idx) {
                    let param_exp = analyzer.mk_temporary(idx);
                    let old_param = analyzer.mk_old(param_exp.clone());
                    state.add_ensures(analyzer.mk_eq(param_exp, old_param));
                }
            }
        }

        // Resolve borrow temps from captured globals.
        // On exit paths, the BorrowGlobal handler may not have been reached
        // (it only appears on the loop body path), leaving unresolved
        // Temporary(idx) or Freeze(Temporary(idx)) references to borrow temps.
        // Substitute them with the corresponding global<R>[@entry](addr),
        // using the entry label so MemoryLabelInfo::normalize can later
        // wrap it in old() for ensures context.
        let captured_globals: Vec<TempIndex> = state.captured_globals.iter().copied().collect();
        for &temp in &captured_globals {
            if let Some((mid, sid, targs, addr_temp)) =
                analyzer.borrow_global_info.get(&temp).cloned()
            {
                let struct_env = analyzer.get_struct(mid, sid);
                let addr_exp = analyzer.mk_temporary(addr_temp);
                let global_exp = analyzer.mk_global_with_label(
                    &struct_env,
                    &targs,
                    addr_exp,
                    Some(entry_post_label),
                );
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
                    let e = analyzer.substitute_exp_with_exp(&e, &old_temp_exp, &old_global_exp);
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

        // Collect modifies targets before normalization (labels still present).
        for ensures in &state.ensures {
            collect_modifies_targets(ensures, &mut state.direct_modifies);
        }

        // Final label normalization: classify all labels and convert.
        // - pre-labels in ensures → old(Global(None))
        // - pre-labels in aborts → Global(None) (already pre-state context)
        // - post-labels → stripped to None (implicit post-state)
        // - intermediate labels preserved
        {
            let all_conds: Vec<&Exp> = state.ensures.iter().chain(state.aborts.iter()).collect();
            let label_info = MemoryLabelInfo::from_conditions(&all_conds, Some(entry_post_label));
            let env = analyzer.global_env();
            state.ensures = state
                .ensures
                .iter()
                .map(|e| label_info.normalize(env, e, true))
                .collect();
            state.aborts = state
                .aborts
                .iter()
                .map(|e| label_info.normalize(env, e, false))
                .collect();
        }

        // Eliminate WP-internal `WriteOf` carriers before simplification
        // so any tautologies it produces are folded away.
        analyzer.eliminate_write_of(&mut state);

        // Call arguments may become constants only after backward substitution
        // (for example, string::utf8(b"") initially receives a temporary).
        // Reduce now-known non-aborting behavioral predicates before boolean
        // simplification so they cannot survive inside path conditions.
        state = state.map(|exp| analyzer.reduce_known_non_aborting_behaviors(exp));

        // Simplify conditions: constant folding, arithmetic/boolean
        // identities, and assumption-based redundancy elimination.
        // Multiple passes allow multi-step simplification chains to complete
        // (e.g., normalize → pinch → one-point rule).
        state = simplify_state(&mut analyzer, &state);
        state = simplify_state(&mut analyzer, &state);
        state = simplify_state(&mut analyzer, &state);

        // A full `update<R>(addr, update_field(...))` relative to an
        // intermediate opaque-call state asserts equality for every field of
        // `R`. The emitted source can only characterize that intermediate
        // state through the callee's `ensures_of`, which may intentionally
        // leave unrelated fields unspecified. Preserve the observable leaf
        // update instead; it is a sound consequence which does not require an
        // inaccessible program-point snapshot.
        analyzer.weaken_intermediate_field_updates(&mut state);

        // A loop or an imprecise alias join can leave internal locals in the
        // entry predicate even after ordinary backward substitution. Such
        // temporaries are not names available in a source-level spec. Close
        // them using the same nondeterministic semantics as Havoc: universal
        // quantification for normal-return obligations and existential
        // quantification for possible aborts. These clauses are marked
        // solver-hard by `update_spec`, so the interactive workflow can refine
        // them, but inference must never emit invalid source or abort the whole
        // package transformation.
        analyzer.close_non_parameter_temporaries(&mut state);
        normalized_entry = Some(state.clone());

        if !state.is_empty() {
            // An uninvariant loop's havoc produces path summaries whose
            // clauses are mutually dependent approximations. Individual
            // clauses must not be advertised as independently easy/valid
            // merely because only a sibling clause contains the explicit
            // quantifier. A loop that does have invariants is different: its
            // havoc is constrained by those invariants and must not taint
            // every inferred condition as solver-hard.
            if update_model {
                // An invariant-less loop havocs its carried state, so what WP
                // derives past it is not justified -- unreliable, not merely
                // expensive. That is `vacuous`, not `sathard`.
                let havoc_unreliable = has_uninvariant_loop && !analyzer.havoc_targets.is_empty();
                update_spec(
                    fun_env,
                    &state,
                    &mut analyzer,
                    havoc_unreliable,
                    state.solver_hard,
                );
                // Extract repeated state-neutral sub-expressions into let bindings.
                cse_inferred_conditions(fun_env);
                // Emit modifies clauses only for opaque specs (they need
                // explicit modifies to declare which globals may change).
                if !ProverOptions::get(analyzer.global_env()).no_inference_opaque {
                    emit_modifies(fun_env, &state);
                }
                // Check for inferred conditions referencing non-parameter temporaries
                check_bad_temps(fun_env);
            }
        } else if !silent_on_failure {
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
    } else if !silent_on_failure {
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

    drop(analyzer);
    SpecInferenceRun {
        annotation: annotate.then_some(WPAnnotation(wp_map)),
        entry_state: normalized_entry,
        incomplete: false,
    }
}

fn update_spec<'env>(
    fun_env: &FunctionEnv,
    state: &WPState,
    generator: &mut impl ExpGenerator<'env>,
    havoc_unreliable: bool,
    solver_hard_summary: bool,
) {
    let env = fun_env.module_env.env;
    let pool = env.symbol_pool();
    let inferred_sym = pool.make(CONDITION_INFERRED_PROP);
    let vacuous_sym = pool.make(CONDITION_INFERRED_VACUOUS);
    let sathard_sym = pool.make(CONDITION_INFERRED_SATHARD);
    let loc = fun_env.get_loc();
    // Every reason the emitted abort clauses became a lower bound rather than
    // an exact characterization. Reported to the caller at the end.
    let mut partial_abort_reasons: Vec<&'static str> = vec![];

    // Read the inference pragma to decide what to emit.
    let infer_ensures;
    let infer_aborts;
    if let Some(mode) = fun_env.get_symbol_pragma(INFERENCE_PRAGMA) {
        let mode_str = pool.string(mode);
        infer_ensures = mode_str.as_str() != "only_aborts" && mode_str.as_str() != "none";
        infer_aborts = mode_str.as_str() != "only_ensures" && mode_str.as_str() != "none";
    } else {
        infer_ensures = true;
        infer_aborts = true;
    }

    let mut spec = fun_env.get_mut_spec();

    let mk_cond = |kind: ConditionKind, exp: &Exp| {
        // `vacuous` marks a condition the derivation cannot justify, so it is
        // dropped rather than published; `sathard` marks one which holds but
        // is expensive for the solver, so it is kept and flagged.
        let is_vacuous = has_unconstrained_quant_var(exp) || havoc_unreliable;
        let is_sathard = !is_vacuous
            && (solver_hard_summary
                || has_top_level_quantifier(exp)
                || has_untrusted_transparent_result_of(env, exp));
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

    // A post label only needs a source-level name when another expression reads
    // it as a pre-state.  Otherwise it denotes the ambient function post-state
    // and must be omitted; emitting it would define an unreferenced label, which
    // the source checker correctly rejects.  Also collapse `S..S` to pre-only
    // notation: defining a label in terms of itself creates a cycle.
    let mut referenced_pre_labels = BTreeSet::new();
    for exp in state.ensures.iter().chain(state.aborts.iter()) {
        exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, op, _) = e {
                match op {
                    AstOp::Global(Some(label)) | AstOp::Exists(Some(label)) => {
                        referenced_pre_labels.insert(*label);
                    },
                    AstOp::SpecPublish(range)
                    | AstOp::SpecRemove(range)
                    | AstOp::SpecUpdate(range)
                    | AstOp::SpecFunction(_, _, range)
                    | AstOp::Behavior(_, range) => {
                        if let Some(label) = range.pre {
                            referenced_pre_labels.insert(label);
                        }
                    },
                    _ => {},
                }
            }
            true
        });
    }
    struct OrphanPostStripper<'a> {
        referenced_pre_labels: &'a BTreeSet<MemoryLabel>,
    }
    impl ExpRewriterFunctions for OrphanPostStripper<'_> {
        fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
            let strip = |range: &MemoryRange| {
                let post = range.post.filter(|label| {
                    Some(*label) != range.pre && self.referenced_pre_labels.contains(label)
                });
                (post != range.post).then_some(MemoryRange {
                    pre: range.pre,
                    post,
                })
            };
            match oper {
                AstOp::SpecPublish(range) => strip(range)
                    .map(|r| ExpData::Call(id, AstOp::SpecPublish(r), args.to_vec()).into_exp()),
                AstOp::SpecRemove(range) => strip(range)
                    .map(|r| ExpData::Call(id, AstOp::SpecRemove(r), args.to_vec()).into_exp()),
                AstOp::SpecUpdate(range) => strip(range)
                    .map(|r| ExpData::Call(id, AstOp::SpecUpdate(r), args.to_vec()).into_exp()),
                AstOp::SpecFunction(mid, fid, range) => strip(range).map(|r| {
                    ExpData::Call(id, AstOp::SpecFunction(*mid, *fid, r), args.to_vec()).into_exp()
                }),
                AstOp::Behavior(kind, range) => strip(range).map(|r| {
                    ExpData::Call(id, AstOp::Behavior(*kind, r), args.to_vec()).into_exp()
                }),
                _ => None,
            }
        }
    }
    let mut orphan_stripper = OrphanPostStripper {
        referenced_pre_labels: &referenced_pre_labels,
    };
    let normalized_ensures: Vec<Exp> = state
        .ensures
        .iter()
        .map(|e| orphan_stripper.rewrite_exp(e.clone()))
        .collect();
    let normalized_aborts: Vec<Exp> = state
        .aborts
        .iter()
        .map(|e| orphan_stripper.rewrite_exp(e.clone()))
        .collect();

    // Strip undefined state labels: any label not defined by a two-state operation
    // (mutation builtin, behavioral predicate, or spec function with range.post)
    // references the function's entry/pre-state and should be None.
    // Collect defined labels: labels that appear as range.post in a defining operation
    let mut defined_labels = BTreeSet::new();
    for exp in normalized_ensures.iter().chain(normalized_aborts.iter()) {
        exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, op, _) = e {
                let post = match op {
                    AstOp::SpecPublish(r)
                    | AstOp::SpecRemove(r)
                    | AstOp::SpecUpdate(r)
                    | AstOp::SpecFunction(_, _, r) => r.post,
                    AstOp::Behavior(
                        move_model::ast::BehaviorKind::EnsuresOf
                        | move_model::ast::BehaviorKind::ResultOf,
                        r,
                    ) => r.post,
                    _ => None,
                };
                if let Some(label) = post {
                    defined_labels.insert(label);
                }
            }
            true
        });
    }

    // Rewrite: replace undefined labels with None
    struct UndefinedLabelStripper<'a> {
        defined: &'a BTreeSet<MemoryLabel>,
    }
    impl UndefinedLabelStripper<'_> {
        fn strip_range(&self, range: &MemoryRange) -> Option<MemoryRange> {
            let new_pre = match range.pre {
                Some(l) if !self.defined.contains(&l) => None,
                other => other,
            };
            let new_post = match range.post {
                Some(l) if !self.defined.contains(&l) => None,
                other => other,
            };
            if new_pre != range.pre || new_post != range.post {
                Some(MemoryRange {
                    pre: new_pre,
                    post: new_post,
                })
            } else {
                None
            }
        }
    }
    impl ExpRewriterFunctions for UndefinedLabelStripper<'_> {
        fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
            match oper {
                AstOp::Global(Some(l)) if !self.defined.contains(l) => {
                    Some(ExpData::Call(id, AstOp::Global(None), args.to_vec()).into_exp())
                },
                AstOp::Exists(Some(l)) if !self.defined.contains(l) => {
                    Some(ExpData::Call(id, AstOp::Exists(None), args.to_vec()).into_exp())
                },
                AstOp::SpecPublish(r) => self
                    .strip_range(r)
                    .map(|nr| ExpData::Call(id, AstOp::SpecPublish(nr), args.to_vec()).into_exp()),
                AstOp::SpecRemove(r) => self
                    .strip_range(r)
                    .map(|nr| ExpData::Call(id, AstOp::SpecRemove(nr), args.to_vec()).into_exp()),
                AstOp::SpecUpdate(r) => self
                    .strip_range(r)
                    .map(|nr| ExpData::Call(id, AstOp::SpecUpdate(nr), args.to_vec()).into_exp()),
                AstOp::Behavior(kind, r) => self.strip_range(r).map(|nr| {
                    ExpData::Call(id, AstOp::Behavior(*kind, nr), args.to_vec()).into_exp()
                }),
                AstOp::SpecFunction(mid, fid, r) => self.strip_range(r).map(|nr| {
                    ExpData::Call(id, AstOp::SpecFunction(*mid, *fid, nr), args.to_vec()).into_exp()
                }),
                _ => None,
            }
        }
    }

    let mut stripper = UndefinedLabelStripper {
        defined: &defined_labels,
    };
    let stripped_ensures: Vec<Exp> = normalized_ensures
        .iter()
        .map(|e| stripper.rewrite_exp(e.clone()))
        .collect();
    let stripped_aborts: Vec<Exp> = normalized_aborts
        .iter()
        .map(|e| stripper.rewrite_exp(e.clone()))
        .collect();

    // Add each ensures condition separately, filtering out trivial `true` conditions
    if infer_ensures {
        let ensures_conds: Vec<_> = stripped_ensures
            .iter()
            .filter(|e| !is_trivial_true(e))
            .collect();
        spec.conditions.extend(
            ensures_conds
                .iter()
                .map(|e| mk_cond(ConditionKind::Ensures, e)),
        );
    }

    // Add each aborts condition separately. A trivial `true` is retained only
    // for abort-only functions, where it is the exact behavior.
    // Strip `old()` wrappers since aborts_if is implicitly evaluated in pre-state,
    // then re-simplify to catch tautologies introduced by stripping (e.g., `r == r`
    // from `Old(r) == Old(r)`).
    if infer_aborts {
        // The clauses are read as a disjunction, so each may be simplified
        // where every earlier one is false. That is the same short-circuit
        // structure the body had: WP conjoins each obligation with the guard
        // that reached it, which restates the negation of the earlier clauses
        // verbatim. `A || (!A && B)` is `A || B`, so dropping the restatement
        // changes nothing and removes the repetition that makes an inferred
        // abort specification unreadable.
        // Simplest first. A disjunction may be reordered freely, and the pass
        // below can only discharge a guard that restates a clause it has
        // already emitted — WP emits the guarded clause first, so in source
        // order there is nothing yet to discharge against.
        let mut ordered_aborts: Vec<Exp> = stripped_aborts.iter().map(strip_all_olds).collect();
        ordered_aborts.sort_by_cached_key(|e| {
            let mut size = 0usize;
            e.visit_pre_order(&mut |_| {
                size += 1;
                true
            });
            size
        });
        let mut normalized_abort_conds: Vec<Exp> = vec![];
        for exp in ordered_aborts {
            let mut s = ExpSimplifier::new(generator);
            for earlier in &normalized_abort_conds {
                if is_trivial_false(earlier) {
                    continue;
                }
                s.assume_negated(earlier.clone());
            }
            // Each abort obligation is conjoined with the path guard that
            // reached it, and the guard often refutes the obligation
            // outright. `simplify_conjunction` lets the guard discharge it;
            // plain `simplify` is bottom-up and never relates the two.
            normalized_abort_conds.push(s.simplify_conjunction(exp));
        }
        // `true` is the exact abort condition for a function with no normal
        // return. For a function which can also return, however, it only says
        // that WP could not characterize the aborting paths. Dropping that
        // clause is sound only if the emitted contract is explicitly partial.
        let dropped_uninformative_abort =
            state.is_normal_return && normalized_abort_conds.iter().any(is_trivial_true);
        let aborts_conds: Vec<_> = normalized_abort_conds
            .iter()
            .filter(|e| !is_trivial_false(e) && (!is_trivial_true(e) || !state.is_normal_return))
            .collect();
        let has_flagged_abort = aborts_conds.iter().any(|exp| {
            has_unconstrained_quant_var(exp)
                || havoc_unreliable
                || solver_hard_summary
                || has_top_level_quantifier(exp)
                || has_untrusted_transparent_result_of(env, exp)
        });
        // A loop invariant can retain `!aborts_of<dynamic_closure>(...)` in an
        // ensures summary after the loop transfer has lost the call's own
        // `aborts_partial` bit. If simplification then proves the only remaining
        // (for example, out-of-range) abort clause false, emitting an exact
        // `aborts_if false` would hide the still-uncharacterized closure abort.
        let has_unaccounted_behavioral_abort = aborts_conds.is_empty()
            && stripped_ensures.iter().any(|exp| {
                exp.as_ref().any(&mut |e| {
                    matches!(
                        e,
                        ExpData::Call(
                            _,
                            AstOp::Behavior(move_model::ast::BehaviorKind::AbortsOf, _),
                            _
                        )
                    )
                })
            });
        if state.aborts_partial
            || has_flagged_abort
            || dropped_uninformative_abort
            || has_unaccounted_behavioral_abort
        {
            if state.aborts_partial {
                partial_abort_reasons
                    .push("an abort condition did not survive a memory-havocking loop");
            }
            if has_flagged_abort {
                partial_abort_reasons
                    .push("an emitted abort condition is flagged `vacuous` or `sathard`");
            }
            if dropped_uninformative_abort {
                partial_abort_reasons.push("an abort condition carried no usable information");
            }
            if has_unaccounted_behavioral_abort {
                partial_abort_reasons.push("a callee's `aborts_of` behavior is not accounted for");
            }
            // Abort conditions crossing a memory-havocking loop were dropped,
            // or at least one emitted condition is explicitly marked unusable
            // by the refinement workflow. Once those flagged clauses are
            // removed, the remainder is only a lower bound, not an exact abort
            // characterization. Advertise that fact in the source up front so
            // the enriched package remains sound after deterministic cleanup.
            spec.properties.insert(
                pool.make(ABORTS_IF_IS_PARTIAL_PRAGMA),
                PropertyValue::Value(Value::Bool(true)),
            );
            spec.conditions.extend(
                aborts_conds
                    .iter()
                    .map(|e| mk_cond(ConditionKind::AbortsIf, e)),
            );
        } else if aborts_conds.is_empty() {
            // No abort conditions: emit `aborts_if false` to indicate the function
            // never aborts. Without this, an opaque function with no aborts_if would
            // have unspecified abort behavior when called.
            let false_exp = generator.mk_bool_const(false);
            spec.conditions
                .push(mk_cond(ConditionKind::AbortsIf, &false_exp));
        } else {
            spec.conditions.extend(
                aborts_conds
                    .iter()
                    .map(|e| mk_cond(ConditionKind::AbortsIf, e)),
            );
        }
    }

    // Stackless bytecode temporaries which are not function parameters have no
    // source-level name in a function specification. Likewise, source syntax
    // for behavioral predicates requires a function name between `<...>` and
    // cannot represent a compiler-lifted captured closure. Complex control
    // flow, especially higher-order calls in module-wide inference, can leave
    // either shape in otherwise useful inferred conditions. Emitting either
    // would make the generated contract fail to compile. Weaken it by dropping
    // only those unrepresentable inferred conditions.
    // If an abort condition is dropped, advertise that the remaining abort
    // clauses are a lower bound so callers do not assume abort completeness.
    let num_params = fun_env.get_parameter_count();
    let mut dropped_abort_condition = false;
    spec.conditions.retain(|condition| {
        if !condition.properties.contains_key(&inferred_sym) {
            return true;
        }
        let references_only_params = exp_only_references_params(&condition.exp, num_params);
        let unsourcifiable_behavior = has_unsourcifiable_behavior_target(&condition.exp);
        let unsourcifiable_ghost = has_unsourcifiable_ghost_memory(env, &condition.exp);
        if references_only_params && !unsourcifiable_behavior && !unsourcifiable_ghost {
            return true;
        }
        dropped_abort_condition |= matches!(condition.kind, ConditionKind::AbortsIf);
        false
    });
    if dropped_abort_condition {
        partial_abort_reasons.push("an abort condition had no representable source-level spelling");
        spec.properties.insert(
            pool.make(ABORTS_IF_IS_PARTIAL_PRAGMA),
            PropertyValue::Value(Value::Bool(true)),
        );
    }

    // A generated companion spec must not introduce a module dependency which
    // the implementation did not already have.  In particular, ambient
    // assumptions propagated from a caller can mention that caller's resource,
    // creating a reverse edge and a module cycle when sourcified.  Drop those
    // conditions as another sound weakening boundary.
    let mut dropped_dependency_abort = false;
    spec.conditions.retain(|condition| {
        if !condition.properties.contains_key(&inferred_sym) {
            return true;
        }
        let used_modules = expression_module_usage(env, &condition.exp);
        if used_modules
            .iter()
            .all(|module| fun_env.module_env.is_transitive_dependency(*module))
        {
            true
        } else {
            dropped_dependency_abort |= matches!(condition.kind, ConditionKind::AbortsIf);
            false
        }
    });
    if dropped_dependency_abort {
        partial_abort_reasons
            .push("an abort condition would have introduced a new module dependency");
        spec.properties.insert(
            pool.make(ABORTS_IF_IS_PARTIAL_PRAGMA),
            PropertyValue::Value(Value::Bool(true)),
        );
    }

    // Simplification and abort pre-state normalization can remove the only
    // condition which consumed an intermediate post label.  Recompute usage
    // from the exact inferred conditions that will be emitted and strip any
    // newly orphaned definitions.
    let mut emitted_pre_labels = BTreeSet::new();
    for condition in &spec.conditions {
        if !condition.properties.contains_key(&inferred_sym) {
            continue;
        }
        condition.exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, op, _) = e {
                match op {
                    AstOp::Global(Some(label)) | AstOp::Exists(Some(label)) => {
                        emitted_pre_labels.insert(*label);
                    },
                    AstOp::SpecPublish(range)
                    | AstOp::SpecRemove(range)
                    | AstOp::SpecUpdate(range)
                    | AstOp::SpecFunction(_, _, range)
                    | AstOp::Behavior(_, range) => {
                        if let Some(label) = range.pre {
                            emitted_pre_labels.insert(label);
                        }
                    },
                    _ => {},
                }
            }
            true
        });
    }
    let mut emitted_orphan_stripper = OrphanPostStripper {
        referenced_pre_labels: &emitted_pre_labels,
    };
    for condition in &mut spec.conditions {
        if condition.properties.contains_key(&inferred_sym) {
            condition.exp = emitted_orphan_stripper.rewrite_exp(condition.exp.clone());
        }
    }

    // A source-level state label is a definition: `post` names the state
    // produced by an operation and `pre` may consume another definition.  WP
    // over nested opaque/higher-order calls can occasionally produce a cycle
    // in this graph.  There is no valid source interpretation for such a
    // cycle, so discard the participating inferred conditions rather than
    // emitting a spec which fails during instrumentation.
    let mut condition_label_sets = vec![];
    let mut all_defined_labels = BTreeSet::new();
    let mut direct_range_dependencies = vec![];
    for condition in &spec.conditions {
        if !condition.properties.contains_key(&inferred_sym) {
            continue;
        }
        let defined = condition.exp.as_ref().all_defined_labels();
        let used = all_labels_in_exp(&condition.exp);
        condition.exp.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, op, _) = e {
                let range = match op {
                    AstOp::SpecPublish(range)
                    | AstOp::SpecRemove(range)
                    | AstOp::SpecUpdate(range)
                    | AstOp::SpecFunction(_, _, range)
                    | AstOp::Behavior(_, range) => Some(range),
                    _ => None,
                };
                if let Some(MemoryRange {
                    pre: Some(pre),
                    post: Some(post),
                }) = range
                {
                    direct_range_dependencies.push((*post, *pre));
                }
            }
            true
        });
        all_defined_labels.extend(defined.iter().copied());
        condition_label_sets.push((defined, used));
    }
    let mut label_dependencies: BTreeMap<MemoryLabel, BTreeSet<MemoryLabel>> = BTreeMap::new();
    for (defined, used) in condition_label_sets {
        // Definitions nested in the same expression are inherently ordered by
        // the expression tree and are treated as co-definitions by the
        // instrumentation pass.  Only labels defined by another condition are
        // graph dependencies.
        let dependencies: BTreeSet<_> = used
            .difference(&defined)
            .filter(|label| all_defined_labels.contains(label))
            .copied()
            .collect();
        for label in defined {
            label_dependencies
                .entry(label)
                .or_default()
                .extend(dependencies.iter().copied());
        }
    }
    // A single condition can contain multiple range operations whose order is
    // not interchangeable (for example an antecedent S1..S3 and a consequent
    // S3..S1). The condition-level set difference above treats their labels as
    // co-definitions, so retain each operation's direct post -> pre edge too.
    for (post, pre) in direct_range_dependencies {
        if all_defined_labels.contains(&pre) {
            label_dependencies.entry(post).or_default().insert(pre);
        }
    }
    let mut cyclic_labels = BTreeSet::new();
    for start in label_dependencies.keys().copied() {
        let mut pending: Vec<_> = label_dependencies
            .get(&start)
            .into_iter()
            .flat_map(|next| next.iter().copied())
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(label) = pending.pop() {
            if label == start {
                cyclic_labels.insert(start);
                break;
            }
            if seen.insert(label) {
                pending.extend(
                    label_dependencies
                        .get(&label)
                        .into_iter()
                        .flat_map(|next| next.iter().copied()),
                );
            }
        }
    }
    if !cyclic_labels.is_empty() {
        let mut dropped_cyclic_abort = false;
        spec.conditions.retain(|condition| {
            if !condition.properties.contains_key(&inferred_sym) {
                return true;
            }
            let mut uses_cycle = false;
            condition.exp.visit_pre_order(&mut |e| {
                if let ExpData::Call(_, op, _) = e {
                    let range = match op {
                        AstOp::SpecPublish(range)
                        | AstOp::SpecRemove(range)
                        | AstOp::SpecUpdate(range)
                        | AstOp::SpecFunction(_, _, range)
                        | AstOp::Behavior(_, range) => Some(range),
                        _ => None,
                    };
                    uses_cycle |= match op {
                        AstOp::Global(Some(label)) | AstOp::Exists(Some(label)) => {
                            cyclic_labels.contains(label)
                        },
                        _ => range.is_some_and(|range| {
                            range
                                .pre
                                .is_some_and(|label| cyclic_labels.contains(&label))
                                || range
                                    .post
                                    .is_some_and(|label| cyclic_labels.contains(&label))
                        }),
                    };
                }
                !uses_cycle
            });
            if uses_cycle {
                dropped_cyclic_abort |= matches!(condition.kind, ConditionKind::AbortsIf);
                false
            } else {
                true
            }
        });
        if dropped_cyclic_abort {
            partial_abort_reasons
                .push("an abort condition formed a cycle through its own spec functions");
            spec.properties.insert(
                pool.make(ABORTS_IF_IS_PARTIAL_PRAGMA),
                PropertyValue::Value(Value::Bool(true)),
            );
        }

        // Removing a cyclic consumer can orphan an otherwise acyclic label.
        // Normalize the exact remaining condition set once more.
        let mut remaining_pre_labels = BTreeSet::new();
        for condition in &spec.conditions {
            if condition.properties.contains_key(&inferred_sym) {
                condition.exp.visit_pre_order(&mut |e| {
                    if let ExpData::Call(_, op, _) = e {
                        match op {
                            AstOp::Global(Some(label)) | AstOp::Exists(Some(label)) => {
                                remaining_pre_labels.insert(*label);
                            },
                            AstOp::SpecPublish(range)
                            | AstOp::SpecRemove(range)
                            | AstOp::SpecUpdate(range)
                            | AstOp::SpecFunction(_, _, range)
                            | AstOp::Behavior(_, range) => {
                                if let Some(label) = range.pre {
                                    remaining_pre_labels.insert(label);
                                }
                            },
                            _ => {},
                        }
                    }
                    true
                });
            }
        }
        let mut final_orphan_stripper = OrphanPostStripper {
            referenced_pre_labels: &remaining_pre_labels,
        };
        for condition in &mut spec.conditions {
            if condition.properties.contains_key(&inferred_sym) {
                condition.exp = final_orphan_stripper.rewrite_exp(condition.exp.clone());
            }
        }
        let mut remaining_defined = BTreeSet::new();
        for condition in &spec.conditions {
            if condition.properties.contains_key(&inferred_sym) {
                condition.exp.visit_pre_order(&mut |e| {
                    if let ExpData::Call(_, op, _) = e {
                        let post = match op {
                            AstOp::SpecPublish(range)
                            | AstOp::SpecRemove(range)
                            | AstOp::SpecUpdate(range)
                            | AstOp::SpecFunction(_, _, range)
                            | AstOp::Behavior(_, range) => range.post,
                            _ => None,
                        };
                        if let Some(label) = post {
                            remaining_defined.insert(label);
                        }
                    }
                    true
                });
            }
        }
        let mut final_undefined_stripper = UndefinedLabelStripper {
            defined: &remaining_defined,
        };
        for condition in &mut spec.conditions {
            if condition.properties.contains_key(&inferred_sym) {
                condition.exp = final_undefined_stripper.rewrite_exp(condition.exp.clone());
            }
        }
    }

    // Add `pragma opaque` so inferred specs are treated as opaque specifications.
    if !ProverOptions::get(env).no_inference_opaque {
        let opaque_sym = pool.make(OPAQUE_PRAGMA);
        spec.properties
            .insert(opaque_sym, PropertyValue::Value(Value::Bool(true)));
    }

    // Inferred expressions are installed after the compiler's ordinary spec
    // usage pass. Register every companion spec function they introduce,
    // including companions reached through behavioral-predicate callees, so
    // monomorphization and the Boogie backend emit their declarations.
    let mut expressions: Vec<Exp> = spec
        .conditions
        .iter()
        .filter(|condition| condition.properties.contains_key(&inferred_sym))
        .map(|condition| condition.exp.clone())
        .collect();
    drop(spec);
    // Record the target even when every candidate condition was discarded.
    // In that case `opaque` plus `aborts_if_is_partial` is still essential
    // source-level output: without it a strict enclosing module silently turns
    // a sound, deliberately incomplete inference result into an invalid exact
    // no-abort contract.
    let mut inferred_targets = env
        .get_extension::<InferredConditionTargets>()
        .map(|targets| (*targets).clone())
        .unwrap_or_default();
    inferred_targets.0.insert(fun_env.get_qualified_id());
    env.set_extension(inferred_targets);
    let mut pending_functions = vec![];
    let mut visited_functions = BTreeSet::new();
    while let Some(expression) = expressions.pop() {
        for callee in expression.as_ref().called_spec_funs(env) {
            env.add_used_spec_fun_transitive(callee.to_qualified_id());
        }
        expression.visit_pre_order(&mut |subexpression| {
            if let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = subexpression {
                pending_functions.push(module_id.qualified(*fun_id));
            }
            true
        });
        while let Some(function) = pending_functions.pop() {
            if !visited_functions.insert(function) || function == fun_env.get_qualified_id() {
                continue;
            }
            let callee = env.get_function(function);
            expressions.extend(
                callee
                    .get_spec()
                    .conditions
                    .iter()
                    .map(|condition| condition.exp.clone()),
            );
        }
    }

    report_partial_aborts(fun_env, &partial_abort_reasons);
}

/// Report that the emitted abort clauses are a lower bound, not an exact
/// characterization.
///
/// `aborts_if_is_partial` keeps the generated contract sound: without it an
/// opaque function whose dropped clauses left no `aborts_if` behind would claim
/// exact no-abort behavior. It is still an incomplete result, so name it and
/// the reasons for it rather than leaving the pragma to be discovered in the
/// generated source.
fn report_partial_aborts(fun_env: &FunctionEnv, reasons: &[&'static str]) {
    if reasons.is_empty() {
        return;
    }
    let mut message = format!(
        "WP could not characterize the aborts of `{}` exactly, so its emitted \
         `aborts_if` clauses are a lower bound and the specification carries \
         `aborts_if_is_partial`. Complete the abort behavior and remove that pragma \
         before relying on the contract. Reasons:",
        fun_env.get_full_name_str()
    );
    let mut seen = BTreeSet::new();
    for reason in reasons {
        if seen.insert(*reason) {
            message.push_str("\n  = ");
            message.push_str(reason);
        }
    }
    fun_env
        .module_env
        .env
        .diag(Severity::Warning, &fun_env.get_loc(), &message);
}

/// Collect every module mentioned by an expression, including modules carried
/// only by node types or instantiations. `ExpData::module_usage` covers named
/// operations but not type-driven operations such as `global<R>` and
/// `exists<R>`; missing those can let a generated companion spec introduce a
/// reverse module dependency and fail with a dependency cycle.
fn expression_module_usage(env: &GlobalEnv, exp: &Exp) -> BTreeSet<ModuleId> {
    let mut usage = BTreeSet::new();
    exp.as_ref().module_usage(&mut usage);
    exp.visit_pre_order(&mut |node| {
        let id = node.node_id();
        for ty in std::iter::once(env.get_node_type(id))
            .chain(env.get_node_instantiation_opt(id).into_iter().flatten())
        {
            ty.visit(&mut |nested| match nested {
                Type::Struct(module_id, _, _) | Type::ResourceDomain(module_id, _, _) => {
                    usage.insert(*module_id);
                },
                _ => {},
            });
        }
        true
    });
    usage
}

/// Whether a behavioral predicate contains a closure value which cannot be
/// preserved reliably through source emission. A captured target would
/// sourcify as an inline lambda inside `requires_of<...>` (or a sibling
/// predicate), which the grammar does not accept. Other target expressions,
/// including function-valued field selections, are valid. A closure passed as
/// another behavioral argument reparses to a fresh lifted function identity,
/// so the original predicate cannot be used as a verified contract either.
fn has_unsourcifiable_behavior_target(exp: &Exp) -> bool {
    fn contains_closure_argument(exp: &Exp) -> bool {
        let mut found = false;
        exp.visit_pre_order(&mut |node| match node {
            // A nested behavioral predicate's target is syntax, not a value
            // passed to the enclosing call. Inspect only its actual arguments.
            ExpData::Call(_, AstOp::Behavior(..), args) => {
                found = args.iter().skip(1).any(contains_closure_argument);
                false
            },
            ExpData::Call(_, AstOp::Closure(..), _) | ExpData::Lambda(..) => {
                found = true;
                false
            },
            _ => !found,
        });
        found
    }

    let mut found = false;
    exp.visit_pre_order(&mut |node| {
        let ExpData::Call(_, AstOp::Behavior(..), args) = node else {
            return true;
        };
        let Some(target) = args.first() else {
            return true;
        };
        let target_requires_inline_lambda = match target.as_ref() {
            ExpData::Call(_, AstOp::Closure(_, _, mask), captures) => {
                !captures.is_empty() || mask.captured_count() != 0
            },
            ExpData::Lambda(..) => true,
            _ => false,
        };
        let has_closure_argument = args.iter().skip(1).any(contains_closure_argument);
        found = target_requires_inline_lambda || has_closure_argument;
        !found
    });
    found
}

/// Specification variables are represented internally as synthetic
/// `Ghost$...` resources.  Their publish/remove/update operations have no
/// source-level resource syntax, so inferred clauses containing them must be
/// weakened away rather than emitting invalid identifiers.
fn has_unsourcifiable_ghost_memory(env: &GlobalEnv, exp: &Exp) -> bool {
    let mut found = false;
    exp.visit_pre_order(&mut |node| {
        let ExpData::Call(
            id,
            AstOp::Global(_)
            | AstOp::Exists(_)
            | AstOp::SpecPublish(_)
            | AstOp::SpecRemove(_)
            | AstOp::SpecUpdate(_),
            _,
        ) = node
        else {
            return true;
        };
        found = env
            .get_node_instantiation_opt(*id)
            .and_then(|inst| inst.first().cloned())
            .is_some_and(|ty| match ty {
                Type::Struct(module_id, struct_id, _) => env
                    .get_module(module_id)
                    .into_struct(struct_id)
                    .is_ghost_memory(),
                _ => false,
            });
        !found
    });
    found
}

// =================================================================================================
// Common Sub-Expression Elimination for Inferred Specs

/// Computes a structural hash of an expression, ignoring NodeIds.
fn structural_hash(exp: &ExpData) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    structural_hash_impl(exp, &mut hasher);
    hasher.finish()
}

fn structural_hash_impl(exp: &ExpData, hasher: &mut impl Hasher) {
    std::mem::discriminant(exp).hash(hasher);
    match exp {
        ExpData::Value(_, v) => v.hash(hasher),
        ExpData::LocalVar(_, s) => s.hash(hasher),
        ExpData::Temporary(_, t) => t.hash(hasher),
        ExpData::Call(_, op, args) => {
            op.hash(hasher);
            for a in args {
                structural_hash_impl(a.as_ref(), hasher);
            }
        },
        ExpData::Invoke(_, f, args) => {
            structural_hash_impl(f.as_ref(), hasher);
            for a in args {
                structural_hash_impl(a.as_ref(), hasher);
            }
        },
        _ => {
            // For complex expressions (lambda, quant, etc.) just use discriminant.
            // They won't be CSE candidates anyway.
        },
    }
}

/// Returns the number of nodes in an expression tree.
fn exp_node_count(exp: &ExpData) -> usize {
    let mut count = 0;
    exp.visit_pre_order(&mut |_| {
        count += 1;
        true
    });
    count
}

/// Returns true if the expression is too trivial to extract.
/// Only extract function calls and pack operations — not field accesses, variant tests,
/// or other small operations that are more readable inline.
fn is_cse_candidate(env: &GlobalEnv, exp: &ExpData) -> bool {
    let ExpData::Call(_, oper, args) = exp else {
        return false;
    };
    let worth_naming = match oper {
        // A call names itself, so one occurrence is already readable.
        AstOp::MoveFunction(..) | AstOp::SpecFunction(..) | AstOp::Pack(..) => true,
        // WP restates the path guard in every obligation it reached, so the
        // same conjunction appears in clause after clause. That repetition is
        // what makes an inferred abort specification long, and naming it once
        // is what a reader would do.
        AstOp::And | AstOp::Or => true,
        // Arithmetic is deliberately excluded even though it repeats just as
        // often. A `let` is evaluated where it is bound, not where it is used,
        // so hoisting `x / d` above the guard `d != 0` that made it defined
        // changes the specification's meaning — the binding is then evaluated
        // on inputs the guard excluded. Booleans are total and carry no such
        // obligation.
        _ => false,
    };
    if !worth_naming {
        return false;
    }
    // A single operand carries no structure worth a name of its own.
    if args.len() < 2 && !matches!(oper, AstOp::MoveFunction(..) | AstOp::SpecFunction(..)) {
        return false;
    }
    // Don't hoist subexpressions that contain quantifier-bound (free) local
    // variables.  Such variables are only in scope inside the quantifier body;
    // extracting the expression into a top-level `let` binding makes them
    // undeclared and produces a compilation error (e.g., `undeclared x`).
    exp.free_vars().is_empty() && !env.get_node_type(exp.node_id()).is_tuple()
}

/// Generates a readable name for a CSE binding based on expression structure.
fn cse_name_for(env: &GlobalEnv, exp: &ExpData) -> String {
    let name = match exp {
        ExpData::Call(_, AstOp::MoveFunction(mid, fid), _)
        | ExpData::Call(_, AstOp::Closure(mid, fid, _), _) => {
            let fun_env = env.get_module(*mid).into_function(*fid);
            fun_env.get_name().display(env.symbol_pool()).to_string()
        },
        ExpData::Call(_, AstOp::SpecFunction(mid, fid, _), _) => {
            let module = env.get_module(*mid);
            let spec_fun = module.get_spec_fun(*fid);
            spec_fun.name.display(env.symbol_pool()).to_string()
        },
        ExpData::Call(_, AstOp::Select(_, _, field_id), _) => {
            field_id.symbol().display(env.symbol_pool()).to_string()
        },
        _ => "cse".to_string(),
    };
    // Strip internal `$` prefix used for auto-generated spec function names.
    let name = name.strip_prefix('$').unwrap_or(&name);
    // Always append a trailing underscore to avoid collision with the
    // imported function/field name; further collisions are resolved by
    // appending more underscores at the call site.
    format!("{}_", name)
}

/// Performs common sub-expression elimination on inferred spec conditions.
/// Extracts repeated state-neutral sub-expressions into `let` bindings.
fn cse_inferred_conditions(fun_env: &FunctionEnv) {
    let env = fun_env.module_env.env;
    let pool = env.symbol_pool();
    let loc = fun_env.get_loc();
    let inferred_sym = pool.make(CONDITION_INFERRED_PROP);

    // Collect inferred condition indices and clone their expressions (to release the borrow).
    let (inferred_indices, inferred_exps) = {
        let spec = fun_env.get_spec();
        let indices: Vec<usize> = spec
            .conditions
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                matches!(c.kind, ConditionKind::Ensures | ConditionKind::AbortsIf)
                    && c.properties.contains_key(&inferred_sym)
            })
            .map(|(i, _)| i)
            .collect();
        let exps: Vec<Exp> = indices
            .iter()
            .map(|&i| spec.conditions[i].exp.clone())
            .collect();
        (indices, exps)
    };

    if inferred_indices.is_empty() {
        return;
    }

    // Phase 1: Collect candidate sub-expressions with occurrence counts.
    // Group by structural hash, then confirm with structural_eq.
    let mut hash_buckets: HashMap<u64, Vec<(Exp, usize)>> = HashMap::new();

    for exp in &inferred_exps {
        exp.as_ref().visit_pre_order(&mut |e: &ExpData| {
            if !is_cse_candidate(env, e) || !e.is_state_neutral() {
                return true; // keep visiting children but don't count this node
            }
            let h = structural_hash(e);
            let exp_ref = ExpData::into_exp(e.clone());
            let bucket = hash_buckets.entry(h).or_default();
            if let Some(entry) = bucket
                .iter_mut()
                .find(|(existing, _)| existing.as_ref().structural_eq(&exp_ref))
            {
                entry.1 += 1;
            } else {
                bucket.push((exp_ref, 1));
            }
            true
        });
    }

    // Phase 2: Select candidates. Must appear >= 2 times, have at least 2 nodes
    // (not a trivial select), and enough total weight to justify a let binding.
    let mut candidates: Vec<(Exp, usize)> = hash_buckets
        .into_values()
        .flatten()
        .filter(|(_, count)| *count >= 3)
        .collect();

    if candidates.is_empty() {
        return;
    }

    // Sort by node count ascending (extract smallest/deepest first so outer expressions
    // can reference the let variable after inner ones are replaced).
    candidates.sort_by_cached_key(|(exp, _)| exp_node_count(exp.as_ref()));

    // Phase 3: Create LetPre conditions and rewrite.
    let mut let_conditions: Vec<Condition> = Vec::new();
    let mut used_names: BTreeSet<String> = BTreeSet::new();

    // Collect names that must not be shadowed: parameters, local function names,
    // and imported member names (from `use Module::member;` declarations).
    for param in fun_env.get_parameters() {
        used_names.insert(param.0.display(pool).to_string());
    }
    for func in fun_env.module_env.get_functions() {
        used_names.insert(func.get_name().display(pool).to_string());
    }
    for use_decl in fun_env.module_env.get_use_decls() {
        // Module-level alias (e.g., `use M as Alias;`)
        if let Some(alias) = use_decl.alias {
            used_names.insert(alias.display(pool).to_string());
        }
        // Member-level imports (e.g., `use M::f;` or `use M::f as g;`)
        for (_, member_name, alias) in &use_decl.members {
            let effective_name = alias.unwrap_or(*member_name);
            used_names.insert(effective_name.display(pool).to_string());
        }
    }

    // Process each candidate: create a let binding and rewrite all conditions
    for (candidate_exp, _count) in candidates.iter() {
        // Generate a unique name. The base name ends with `_`; resolve
        // collisions by appending additional underscores.
        let mut name = cse_name_for(env, candidate_exp.as_ref());
        while used_names.contains(&name) {
            name.push('_');
        }
        used_names.insert(name.clone());
        let name_sym = pool.make(&name);

        // Create the replacement expression: LocalVar with the let-binding name
        let cand_type = env.get_node_type(candidate_exp.as_ref().node_id());
        let var_node = env.new_node(loc.clone(), cand_type);
        let var_exp: Exp = ExpData::LocalVar(var_node, name_sym).into_exp();

        // Rewrite all inferred conditions, replacing structural matches
        {
            let mut spec = fun_env.get_mut_spec();
            for &idx in &inferred_indices {
                let old_exp = spec.conditions[idx].exp.clone();
                let new_exp = rewrite_cse(&old_exp, candidate_exp, &var_exp);
                spec.conditions[idx].exp = new_exp;
            }
        }

        // Create the LetPre condition (must have inferred property so it survives filtering)
        let_conditions.push(Condition {
            loc: loc.clone(),
            kind: ConditionKind::LetPre(name_sym, loc.clone()),
            properties: BTreeMap::from([(inferred_sym, PropertyValue::Value(Value::Bool(true)))]),
            exp: candidate_exp.clone(),
            additional_exps: vec![],
        });
    }

    // Phase 4: Also rewrite let-binding expressions (inner lets may reference outer candidates).
    // Since we processed smallest first, earlier let bindings might contain expressions
    // that later bindings extracted. Rewrite them.
    for i in 0..let_conditions.len() {
        for j in (i + 1)..let_conditions.len() {
            let candidate = let_conditions[i].exp.clone();
            let ConditionKind::LetPre(name_sym, _) = let_conditions[i].kind else {
                continue;
            };
            let cand_type = env.get_node_type(candidate.as_ref().node_id());
            let var_node = env.new_node(loc.clone(), cand_type);
            let var_exp: Exp = ExpData::LocalVar(var_node, name_sym).into_exp();
            let old_exp = let_conditions[j].exp.clone();
            let_conditions[j].exp = rewrite_cse(&old_exp, &candidate, &var_exp);
        }
    }

    // Drop bindings nothing refers to. A candidate is matched structurally
    // against the expressions collected before any rewriting, so extracting a
    // nested one changes the shape of the candidate that contained it and its
    // own match is then lost. The binding is still created, and left in place
    // it makes the specification longer rather than shorter — the opposite of
    // what the pass is for.
    {
        let spec = fun_env.get_spec();
        let mut live: Vec<Condition> = vec![];
        for let_cond in let_conditions.into_iter() {
            let ConditionKind::LetPre(name_sym, _) = let_cond.kind else {
                live.push(let_cond);
                continue;
            };
            let used_by_condition = inferred_indices
                .iter()
                .any(|&i| exp_uses_local(&spec.conditions[i].exp, name_sym));
            let used_by_binding = live.iter().any(|c| exp_uses_local(&c.exp, name_sym));
            if used_by_condition || used_by_binding {
                live.push(let_cond);
            }
        }
        drop(spec);
        let_conditions = live;
    }

    // Insert let conditions at the front of the spec (before inferred conditions).
    if !let_conditions.is_empty() {
        let mut spec = fun_env.get_mut_spec();
        // Find insertion point: after any non-inferred conditions, before inferred ones.
        let insert_pos = inferred_indices
            .first()
            .copied()
            .unwrap_or(spec.conditions.len());
        for (i, let_cond) in let_conditions.into_iter().enumerate() {
            spec.conditions.insert(insert_pos + i, let_cond);
        }
    }
}

/// Whether `exp` refers to the local variable `name`.
fn exp_uses_local(exp: &Exp, name: Symbol) -> bool {
    let mut found = false;
    exp.as_ref().visit_pre_order(&mut |e: &ExpData| {
        if matches!(e, ExpData::LocalVar(_, sym) if *sym == name) {
            found = true;
            return false;
        }
        true
    });
    found
}

/// Rewrites an expression, replacing all structural matches of `target` with `replacement`.
fn rewrite_cse(exp: &Exp, target: &Exp, replacement: &Exp) -> Exp {
    struct CseRewriter<'a> {
        target: &'a Exp,
        replacement: &'a Exp,
    }
    impl ExpRewriterFunctions for CseRewriter<'_> {
        fn rewrite_exp(&mut self, exp: Exp) -> Exp {
            if exp.as_ref().structural_eq(self.target) {
                return self.replacement.clone();
            }
            self.rewrite_exp_descent(exp)
        }
    }
    let mut rewriter = CseRewriter {
        target,
        replacement,
    };
    rewriter.rewrite_exp(exp.clone())
}

/// Checks that all inferred conditions only reference parameter temporaries.
/// If any condition references a non-parameter temporary, emits a Bug diagnostic.
fn check_bad_temps(fun_env: &FunctionEnv) {
    let env = fun_env.module_env.env;
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let num_params = fun_env.get_parameter_count();
    let spec = fun_env.get_spec();
    let offenders = spec
        .conditions
        .iter()
        .filter(|c| {
            c.properties.contains_key(&inferred_sym)
                && !exp_only_references_params(&c.exp, num_params)
        })
        .map(|c| format!("  {:?}: {}", c.kind, c.exp.as_ref().display(env)))
        .collect::<Vec<_>>()
        .join("\n");
    if !offenders.is_empty() {
        env.diag(
            Severity::Bug,
            &fun_env.get_loc(),
            &format!("inferred spec references non-parameter temporaries:\n{offenders}"),
        );
    }
}

/// Emit modifies clauses for globals modified in the inferred spec.
/// Scans ensures conditions for `global<R>(addr)` (post-state, no label) on the LHS
/// of equality patterns and emits a modifies clause for each unique one.
fn emit_modifies(fun_env: &FunctionEnv, state: &WPState) {
    let mut modifies_targets: Vec<Exp> = Vec::new();
    let num_params = fun_env.get_parameter_count();

    // From borrow_global_mut WriteBack path: scan ensures for global<R>(addr) on LHS of Eq
    if state.has_global_mutations() {
        for ensures in &state.ensures {
            collect_modifies_targets(ensures, &mut modifies_targets);
        }
    }

    // From MoveFrom/MoveTo direct path
    for target in &state.direct_modifies {
        let stripped = strip_labels_in_exp(target);
        push_if_new(&mut modifies_targets, stripped);
    }

    // Do not duplicate user-provided frame targets. Inference can run on a
    // partially specified function, and the sourcified result must remain a
    // valid ordinary Move specification.
    if let Some(frame) = fun_env.get_frame_spec() {
        modifies_targets.retain(|target| {
            !frame
                .modifies_targets
                .iter()
                .any(|existing| existing.structural_eq(target))
        });
    }

    // A frame expression is evaluated at function entry and can only mention
    // parameters.  Loop indices and callee-local aliases sometimes survive in
    // a path-specific global address even after the corresponding ensures was
    // simplified away.  Such a target cannot be represented as a valid Move
    // `modifies` clause; emitting it produces undeclared `_tN`/local names.
    modifies_targets.retain(|target| {
        exp_only_references_params(target, num_params)
            && !target
                .as_ref()
                .any(&mut |e| matches!(e, ExpData::LocalVar(..)))
    });

    // A body-local mutation is independently sufficient for its frame target.
    // Only propagated targets need the conservative callee check below: a
    // callee may modify R outside the caller's enumerated target when another
    // ordinary direct callee lacks a precise R frame.
    let body_modifies: Vec<_> = state
        .body_modifies
        .iter()
        .map(strip_labels_in_exp)
        .collect();
    let env = fun_env.module_env.env;
    let callees: Vec<_> = fun_env
        .get_called_functions()
        .into_iter()
        .flatten()
        .map(|qid| env.get_function(*qid))
        .filter(|callee| !callee.is_native() && !callee.is_intrinsic() && !callee.is_struct_api())
        .collect();
    modifies_targets.retain(|target| {
        let target_type = env.get_node_type(target.node_id());
        let Type::Struct(module_id, struct_id, _) = target_type.skip_reference() else {
            return false;
        };
        if env
            .get_module(*module_id)
            .into_struct(*struct_id)
            .is_ghost_memory()
        {
            return false;
        }
        if body_modifies
            .iter()
            .any(|body_target| body_target.structural_eq(target))
        {
            return true;
        }
        let resource = module_id.qualified(*struct_id);
        callees
            .iter()
            .all(|callee| callee.get_modify_targets().contains_key(&resource))
    });

    if !modifies_targets.is_empty() {
        fun_env.add_modifies_targets(modifies_targets);
        let env = fun_env.module_env.env;
        let mut inferred_frames = env
            .get_extension::<InferredFrameTargets>()
            .map(|frames| (*frames).clone())
            .unwrap_or_default();
        inferred_frames.0.insert(fun_env.get_qualified_id());
        env.set_extension(inferred_frames);
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

/// Check if an expression contains a state-anchor operation
/// (`SaveStateAnchor` marker or `WithStateAnchor` wrapper). Such props must
/// not be consumed as inference facts: the marker is a positional no-op, and
/// an anchored condition refers to an intermediate program point's state.
fn contains_state_anchor(exp: &Exp) -> bool {
    let mut found = false;
    exp.visit_pre_order(&mut |e| {
        if matches!(
            e,
            ExpData::Call(
                _,
                AstOp::SaveStateAnchor(..)
                    | AstOp::WithStateAnchor(..)
                    | AstOp::FoldsCaptureAnchor(..)
                    | AstOp::InlineCallSummary,
                _
            )
        ) {
            found = true;
        }
        !found
    });
    found
}

/// Check whether an expression contains the verifier-internal frame permission
/// predicate. Instrumentation can combine `CanModify` with other facts, so
/// checking only the root operation lets it leak into sourcified contracts.
fn contains_can_modify(exp: &Exp) -> bool {
    let mut found = false;
    exp.visit_pre_order(&mut |e| {
        if matches!(e, ExpData::Call(_, AstOp::CanModify, _)) {
            found = true;
        }
        !found
    });
    found
}

/// Evaluate a fully constant byte-vector expression and check UTF-8 validity.
/// Returns false for both invalid UTF-8 and non-constant expressions; callers
/// use this only to prove that `string::utf8` cannot abort.
fn constant_valid_utf8(exp: &Exp) -> bool {
    fn byte(value: &Value) -> Option<u8> {
        match value {
            Value::Number(number) => number.to_u8(),
            _ => None,
        }
    }

    fn bytes(exp: &Exp) -> Option<Vec<u8>> {
        match exp.as_ref() {
            ExpData::Call(_, AstOp::EmptyVec, elements) if elements.is_empty() => Some(vec![]),
            ExpData::Call(_, AstOp::Vector, elements) => elements
                .iter()
                .map(|element| match element.as_ref() {
                    ExpData::Value(_, value) => byte(value),
                    _ => None,
                })
                .collect(),
            ExpData::Value(_, Value::ByteArray(values)) => Some(values.clone()),
            ExpData::Value(_, Value::Vector(values)) => values.iter().map(byte).collect(),
            ExpData::Call(_, AstOp::Old | AstOp::Freeze(_) | AstOp::Copy | AstOp::Move, args)
                if args.len() == 1 =>
            {
                bytes(&args[0])
            },
            _ => None,
        }
    }

    bytes(exp).is_some_and(|values| std::str::from_utf8(&values).is_ok())
}

/// Check if an expression is a verification-infrastructure assumption that
/// should be skipped during inference. Matches:
/// - Direct `WellFormed(x)`
/// - Quantifiers over `ResourceDomain`: these are implied properties of data
///   invariants (injected by WellFormed/DataInvariant instrumentation) and do
///   not need to surface in inference results.
fn is_well_formed_prop(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Call(_, AstOp::WellFormed, _) => true,
        ExpData::Quant(_, QuantKind::Forall, ranges, _, _, _) => ranges
            .iter()
            .any(|(_, range)| matches!(range.as_ref(), ExpData::Call(_, AstOp::ResourceDomain, _))),
        _ => false,
    }
}

/// Returns true if `exp` is statically `false` on the normal‑return path
/// because of a verification‑only abort marker (`AbortFlag()` itself, or
/// `aborts_of<f>(args)` — both are `false` whenever the caller is on the
/// normal‑return path). When the cond is false on that path,
/// `cond ==> Q` simplifies to `true`, so wrapping `Q` with such a cond
/// adds only a vacuous antecedent. The corresponding `aborts_if` clauses
/// already capture the same fact via `post = aborts ∨ ensures`.
///
/// Detection is structural: the cond is a marker by itself, or a
/// conjunction with at least one marker as a (possibly nested) conjunct.
/// A marker buried in a *disjunction* (e.g. `P || AbortFlag()`) does
/// **not** make the cond false — `P || false == P` — so we must keep the
/// wrapping in that case.
fn cond_is_false_on_normal_return(exp: &Exp) -> bool {
    use move_model::ast::BehaviorKind;
    fn is_marker(e: &ExpData) -> bool {
        matches!(
            e,
            ExpData::Call(_, AstOp::AbortFlag, _)
                | ExpData::Call(_, AstOp::Behavior(BehaviorKind::AbortsOf, _), _)
        )
    }
    fn check(e: &ExpData) -> bool {
        if is_marker(e) {
            return true;
        }
        if let ExpData::Call(_, AstOp::And, args) = e {
            return args.iter().any(|a| check(a.as_ref()));
        }
        false
    }
    check(exp.as_ref())
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

/// Match `lhs == result_of<f>(args)` (single-return) or the multi-return
/// destructure `lhs == { let (..._t_i...) = result_of<f>(args); _t_i }`.
/// Returns `(lhs, output_idx, fun_exp, args, range)`.
fn extract_result_of_clause(exp: &Exp) -> Option<(Exp, usize, Exp, Vec<Exp>, MemoryRange)> {
    use move_model::ast::BehaviorKind;
    let ExpData::Call(_, AstOp::Eq, eq_args) = exp.as_ref() else {
        return None;
    };
    if eq_args.len() != 2 {
        return None;
    }
    let lhs = eq_args[0].clone();
    let rhs = &eq_args[1];

    if let ExpData::Call(_, AstOp::Behavior(BehaviorKind::ResultOf, range), bp_args) = rhs.as_ref()
    {
        if bp_args.is_empty() {
            return None;
        }
        let fun_exp = bp_args[0].clone();
        let args = bp_args[1..].to_vec();
        return Some((lhs, 0, fun_exp, args, range.clone()));
    }

    if let ExpData::Block(_, pat, Some(binding), body) = rhs.as_ref() {
        let Pattern::Tuple(_, pat_elems) = pat else {
            return None;
        };
        let ExpData::Call(_, AstOp::Behavior(BehaviorKind::ResultOf, range), bp_args) =
            binding.as_ref()
        else {
            return None;
        };
        let ExpData::LocalVar(_, body_sym) = body.as_ref() else {
            return None;
        };
        let body_sym = *body_sym;
        for (i, pe) in pat_elems.iter().enumerate() {
            if let Pattern::Var(_, sym) = pe {
                if *sym == body_sym {
                    if bp_args.is_empty() {
                        return None;
                    }
                    let fun_exp = bp_args[0].clone();
                    let args = bp_args[1..].to_vec();
                    return Some((lhs, i, fun_exp, args, range.clone()));
                }
            }
        }
    }

    None
}

/// Match `ensures_of<f>(args, …)` at the clause top, possibly under nested
/// `c ==> …` guards (from nested branches); return
/// `(fun_exp, args_after_fun, range, guards)` with guards outermost-first.
fn extract_top_ensures_of_clause(exp: &Exp) -> Option<(Exp, Vec<Exp>, MemoryRange, Vec<Exp>)> {
    use move_model::ast::BehaviorKind;
    let mut guards: Vec<Exp> = Vec::new();
    let mut target = exp;
    while let ExpData::Call(_, AstOp::Implies, impl_args) = target.as_ref() {
        if impl_args.len() != 2 {
            break;
        }
        guards.push(impl_args[0].clone());
        target = &impl_args[1];
    }
    let ExpData::Call(_, AstOp::Behavior(BehaviorKind::EnsuresOf, range), bp_args) =
        target.as_ref()
    else {
        return None;
    };
    if bp_args.is_empty() {
        return None;
    }
    let fun_exp = bp_args[0].clone();
    let args = bp_args[1..].to_vec();
    Some((fun_exp, args, range.clone(), guards))
}

/// Structural equality of the (function, args) pair identifying a call site.
fn calls_match(fun1: &Exp, args1: &[Exp], fun2: &Exp, args2: &[Exp]) -> bool {
    if !fun1.structural_eq(fun2) {
        return false;
    }
    if args1.len() != args2.len() {
        return false;
    }
    args1
        .iter()
        .zip(args2.iter())
        .all(|(a, b)| a.structural_eq(b))
}

/// Collect `(write_of_call, L_path)` from each `Eq(L, R)` where `R` is
/// `write_of(...)` or `update_field(B, f, write_of(...))` (possibly nested).
/// Each `update_field` layer extends `L_path` with a `Select` step, so the
/// path names the procedure-level `&mut` post-state even when WP
/// propagated the body-borrow source through `let` bindings (e.g.
/// `Eq(result, update_field(p, x, write_of(...)))` → `write_of ↦ result.x`).
fn collect_write_of_bindings(env: &GlobalEnv, exps: &[Exp]) -> Vec<(Exp, Exp)> {
    let mut result: Vec<(Exp, Exp)> = Vec::new();
    for exp in exps {
        exp.visit_pre_order(&mut |sub| {
            if let ExpData::Call(_, AstOp::Eq, args) = sub {
                if args.len() == 2 && is_procedure_level_path(&args[0]) {
                    decompose_write_of_binding(env, &args[0], &args[1], &mut result);
                }
            }
            true
        });
    }
    result
}

/// True if `exp` is a `Temporary`, the procedure result, or a `Select` /
/// `SelectVariants` chain rooted at one. Guards binding collection: a
/// quantifier-bound `LocalVar` LHS would leak out of scope after
/// substitution.
fn is_procedure_level_path(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Temporary(..) => true,
        ExpData::Call(_, AstOp::Result(..), _) => true,
        ExpData::Call(_, AstOp::Select(..), args) if args.len() == 1 => {
            is_procedure_level_path(&args[0])
        },
        ExpData::Call(_, AstOp::SelectVariants(..), args) if args.len() == 1 => {
            is_procedure_level_path(&args[0])
        },
        _ => false,
    }
}

/// Peel `update_field` wrappers off `rhs`, extending `l` by a `Select`
/// step per layer; on reaching `write_of`, record `(write_of, l)`.
/// Both the *value* arm (this update's RHS) and the *base* arm (the
/// remaining struct, which may itself be a nested `update_field` chain
/// for sibling fields) are searched.
fn decompose_write_of_binding(env: &GlobalEnv, l: &Exp, rhs: &Exp, out: &mut Vec<(Exp, Exp)>) {
    use move_model::ast::BehaviorKind;
    match rhs.as_ref() {
        ExpData::Call(_, AstOp::Behavior(BehaviorKind::WriteOf(_), _), _) => {
            out.push((rhs.clone(), l.clone()));
        },
        ExpData::Call(_, AstOp::UpdateField(mid, sid, fid), uf_args) if uf_args.len() == 2 => {
            let base = &uf_args[0];
            let value = &uf_args[1];
            let value_ty = env.get_node_type(value.node_id());
            let l_loc = env.get_node_loc(l.node_id());
            let new_l_id = env.new_node(l_loc, value_ty);
            let new_l = ExpData::Call(new_l_id, AstOp::Select(*mid, *sid, *fid), vec![l.clone()])
                .into_exp();
            decompose_write_of_binding(env, &new_l, value, out);
            // Sibling-field updates live in the base arm.
            decompose_write_of_binding(env, l, base, out);
        },
        _ => {},
    }
}

/// Replace each `write_of<f, j>(args)` with the bound `lhs` from `bindings`;
/// when no binding matches (body-borrow case), fall back to
/// `strip_all_olds(args[mut_param_pos(j)])`.
fn substitute_write_of_with_natural(env: &GlobalEnv, exp: &Exp, bindings: &[(Exp, Exp)]) -> Exp {
    use move_model::ast::BehaviorKind;
    struct Sub<'a> {
        env: &'a GlobalEnv,
        bindings: &'a [(Exp, Exp)],
    }
    impl ExpRewriterFunctions for Sub<'_> {
        fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
            let AstOp::Behavior(BehaviorKind::WriteOf(j), _) = oper else {
                return None;
            };
            // Match against the original (pre-recursion) form: bindings come from the same clause-set.
            let original = ExpData::Call(id, oper.clone(), args.to_vec()).into_exp();
            for (wo, lhs) in self.bindings {
                if wo.as_ref().structural_eq(&original) {
                    return Some(lhs.clone());
                }
            }
            let new_args: Vec<Exp> = args.iter().map(|a| self.rewrite_exp(a.clone())).collect();
            if new_args.is_empty() {
                return None;
            }
            let fun_exp = &new_args[0];
            let fun_type = self.env.get_node_type(fun_exp.node_id());
            let Type::Fun(arg_ty, _, _) = fun_type else {
                return None;
            };
            let flat = arg_ty.flatten();
            let mut_pos = flat
                .iter()
                .enumerate()
                .filter(|(_, t)| t.is_mutable_reference())
                .nth(*j)
                .map(|(pos, _)| pos)?;
            let mut_arg = new_args.get(1 + mut_pos)?;
            Some(strip_all_olds(mut_arg))
        }
    }
    Sub { env, bindings }.rewrite_exp(exp.clone())
}

/// True for `true`, `Eq(x, x)`, `Implies(_, true_body)`, conjunctions of
/// these, and `Forall x. true_body`. Used to drop tautologies left behind
/// by `write_of → lhs` substitution.
fn is_trivially_true(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Value(_, Value::Bool(true)) => true,
        ExpData::Call(_, AstOp::Eq, args) if args.len() == 2 => args[0].structural_eq(&args[1]),
        ExpData::Call(_, AstOp::Implies, args) if args.len() == 2 => is_trivially_true(&args[1]),
        ExpData::Call(_, AstOp::And, args) => args.iter().all(is_trivially_true),
        ExpData::Quant(_, QuantKind::Forall, _, _, _, body) => is_trivially_true(body),
        _ => false,
    }
}

/// Strip all memory labels from Global, Exists, Behavior, and SpecFunction operations.
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
                AstOp::Behavior(kind, range) if !range.is_default() => Some(
                    ExpData::Call(
                        id,
                        AstOp::Behavior(*kind, MemoryRange::default()),
                        args.to_vec(),
                    )
                    .into_exp(),
                ),
                AstOp::SpecFunction(mid, fid, range) if !range.is_default() => Some(
                    ExpData::Call(
                        id,
                        AstOp::SpecFunction(*mid, *fid, MemoryRange::default()),
                        args.to_vec(),
                    )
                    .into_exp(),
                ),
                AstOp::SpecPublish(range) if !range.is_default() => Some(
                    ExpData::Call(
                        id,
                        AstOp::SpecPublish(MemoryRange::default()),
                        args.to_vec(),
                    )
                    .into_exp(),
                ),
                AstOp::SpecRemove(range) if !range.is_default() => Some(
                    ExpData::Call(id, AstOp::SpecRemove(MemoryRange::default()), args.to_vec())
                        .into_exp(),
                ),
                AstOp::SpecUpdate(range) if !range.is_default() => Some(
                    ExpData::Call(id, AstOp::SpecUpdate(MemoryRange::default()), args.to_vec())
                        .into_exp(),
                ),
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

/// Whether an inferred condition relies on a `result_of` carrier for a
/// transparent Move function. The carrier is the concrete result witness for
/// an opaque call summarized by the behavioral-predicate backend. For a
/// transparent call the prover executes the body instead, so an independently
/// generated carrier is not related to that runtime result and cannot justify
/// a caller postcondition such as `result == result_of<f>(args)`.
///
/// Such clauses are retained for inspection but marked `sathard`, allowing the
/// deterministic compatibility refinement to remove only the unusable clause.
fn has_untrusted_transparent_result_of(env: &GlobalEnv, exp: &Exp) -> bool {
    exp.as_ref().any(&mut |node| {
        let ExpData::Call(_, AstOp::Behavior(move_model::ast::BehaviorKind::ResultOf, _), args) =
            node
        else {
            return false;
        };
        let Some(target) = args.first() else {
            return true;
        };
        let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = target.as_ref() else {
            return true;
        };
        let callee = env.get_function((*module_id).qualified(*fun_id));
        !callee.is_opaque() || callee.is_pragma_false(VERIFY_PRAGMA)
    })
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
        flatten_conjunction_owned(body)
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

    // Flatten top-level conjunctions into separate ensures for cleaner output.
    let simplified_ensures: Vec<Exp> = simplified_ensures
        .into_iter()
        .flat_map(|e| flatten_conjunction_owned(&e))
        .collect();

    // Deduplicate ensures using structural equality.
    // Simplification can produce structural duplicates from conditions that were
    // syntactically different before simplification.
    let mut simplified_ensures = deduplicate_exps(simplified_ensures);

    // Drop the ensures simplifier so we can reborrow the generator.
    drop(simplifier);

    // Replace complementary path clauses with one unconditional or
    // conditional-value postcondition before the next simplification pass.
    simplified_ensures = combine_complementary_ensures(generator, &simplified_ensures);

    // Remove foralls that are provably inconsistent: if instantiating at two distinct
    // witness values yields consequents that contradict each other, the forall is false
    // (for parameter values where both witnesses satisfy the antecedent).
    // E.g., `forall x: P(x) ==> n == x + 1` at x=0 gives n==1, at x=1 gives n==2.
    // These are contradictory, so the forall is false when n >= 2.
    // A false forall ensures is uninformative and should be removed.
    {
        let mut simplifier = ExpSimplifier::new(generator);
        simplified_ensures.retain(|e| !simplifier.is_forall_provably_false(e));
    }

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

    let simplified_aborts: Vec<_> = simplified_aborts
        .iter()
        .map(|exp| normalize_reference_endpoint_abort(generator, exp))
        .collect();

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
    let body_modifies = deduplicate_exps(state.body_modifies.clone());

    WPState {
        ensures: simplified_ensures,
        aborts: simplified_aborts,
        is_normal_return: state.is_normal_return,
        origin_block: state.origin_block,
        post: state.post,
        captured_mut_params: state.captured_mut_params.clone(),
        captured_globals: state.captured_globals.clone(),
        update_globals: state.update_globals.clone(),
        direct_modifies,
        body_modifies,
        aborts_partial: state.aborts_partial,
        solver_hard: state.solver_hard,
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
            // Also avoid names that appear free in range expressions: a range like
            // `a..x` must not have its upper bound `x` shadowed by the bound variable.
            for (_, range_exp) in ranges {
                for sym in range_exp.as_ref().free_vars() {
                    if !bound_syms.contains(&sym) {
                        used_names.insert(sym.display(pool).to_string());
                    }
                }
            }
            // Also avoid names bound by inner quantifiers. Without this, the outer
            // rename can pick a name already used as an inner binder (e.g. `x`), and
            // then substituting the outer variable into the inner body causes
            // `x: u64` references to be shadowed by the inner `x: BitVector` binder,
            // producing type errors like `forall x: BitVector: x >= amount`.
            body.as_ref().visit_pre_order(&mut |e| {
                if let ExpData::Quant(_, _, inner_ranges, _, _, _) = e {
                    for (pat, _) in inner_ranges {
                        if let Pattern::Var(_, sym) = pat {
                            if !bound_syms.contains(sym) {
                                used_names.insert(sym.display(pool).to_string());
                            }
                        }
                    }
                }
                true
            });
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
    /// Label for the function entry state (before any mutations).
    at_entry_label: MemoryLabel,
    /// Label for the function exit state (post-return). Created during
    /// construction; never reassigned. Uses `Cell` for `.get()` from `&self`.
    at_end_label: Cell<MemoryLabel>,
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
    /// Pre-assigned labels from forward state boundary analysis.
    /// Maps each code offset to (state_before, state_after).
    /// For state-changing instructions: state_before ≠ state_after.
    /// For non-state-changing: state_before == state_after.
    forward_label_map: RefCell<BTreeMap<CodeOffset, (MemoryLabel, MemoryLabel)>>,
    /// Counter for generating sequential label names (S1, S2, ...).
    label_counter: Cell<usize>,
    /// Optional synthetic exit used only by bounded loop-invariant evidence.
    evidence_seed: Option<LoopEvidenceSeed>,
}

// =================================================================================================
// Forward State Boundary Analysis

/// State for the forward label pre-assignment analysis.
/// Tracks the current memory label at each program point.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StateBoundaryState(MemoryLabel);

impl AbstractDomain for StateBoundaryState {
    fn join(&mut self, other: &Self) -> JoinResult {
        if self.0 == other.0 {
            JoinResult::Unchanged
        } else {
            // At CFG merge points, pick the smaller label as canonical representative.
            // This is safe because the backward WP's path_aware_join handles the
            // semantic merge at branches — this forward pass only provides label hints.
            // Invariant: the backward WP must use path_aware_join (not plain join)
            // for correctness when multiple forward labels converge.
            let merged = std::cmp::min(self.0, other.0);
            if self.0 != merged {
                self.0 = merged;
                JoinResult::Changed
            } else {
                JoinResult::Unchanged
            }
        }
    }
}

/// Forward analysis that pre-assigns memory labels at state-changing instructions.
/// Runs before the backward WP to eliminate retroactive label rewriting.
struct StateBoundaryAnalysis<'a, 'env> {
    analyzer: &'a SpecInferenceAnalyzer<'env>,
}

impl TransferFunctions for StateBoundaryAnalysis<'_, '_> {
    type State = StateBoundaryState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut StateBoundaryState, instr: &Bytecode, offset: CodeOffset) {
        if self.is_state_changing(instr) {
            state.0 = self.analyzer.mk_label_at(offset);
        }
    }
}

impl DataflowAnalysis for StateBoundaryAnalysis<'_, '_> {}

impl StateBoundaryAnalysis<'_, '_> {
    /// Determine whether an instruction creates a state boundary (new memory label).
    fn is_state_changing(&self, instr: &Bytecode) -> bool {
        match instr {
            Bytecode::Call(_, dests, op, srcs, _) => match op {
                Operation::WriteBack(BorrowNode::GlobalRoot(_), _) => true,
                Operation::MoveTo(_, _, _) | Operation::MoveFrom(_, _, _) => true,
                Operation::HavocGlobal(_, _, _) => true,
                Operation::Function(module_id, fun_id, type_inst) => {
                    // Mirror the backward WP's cascade: a function call is only
                    // non-state-changing if it qualifies for one of the direct-
                    // substitution branches — pure spec call or native spec exp.
                    // Either path applies the call's semantics by substitution
                    // and emits no behavioral predicate, so the memory label
                    // must not be advanced. Otherwise a vector::length (or
                    // similar native) before a behavioral call would create a
                    // spurious label boundary even though the underlying
                    // memory state is unchanged.
                    !(dests.len() == 1
                        && (self
                            .analyzer
                            .try_as_pure_spec_call(*module_id, *fun_id, type_inst)
                            .is_some()
                            || self
                                .analyzer
                                .try_as_native_spec_exp(*module_id, *fun_id, type_inst, srcs)
                                .is_some()))
                },
                Operation::Invoke => true,
                _ => false,
            },
            _ => false,
        }
    }
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
                if self.evidence_seed.is_some() {
                    // A cut-point query is interested only in paths reaching its
                    // synthetic exit, not paths reaching an ordinary function exit.
                    *state = WPState::new(state.post);
                } else {
                    // Base case for backward analysis: compute the ensures conditions
                    // Creates one ensures per return value that references a parameter
                    *state = self.mk_return_ensures(vals);
                }
            },
            Bytecode::Abort(_, _, _) => {
                if self.evidence_seed.is_some() {
                    *state = WPState::new(state.post);
                } else {
                    // Abort sets the aborts condition to true
                    *state =
                        WPState::with_aborts(self.mk_bool_const(true), self.at_end_label.get());
                }
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
                let const_exp = self.constant_to_exp(constant, self.get_local_type(*dest));
                *state = self.substitute_exp_state(state, *dest, &const_exp);
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
                        // Vector bytecode-instruction natives (and
                        // singleton/contains) get their WP applied directly
                        // via substitution, so no BP over a vector op appears
                        // in the inferred spec. Checked before the pure path
                        // so the rewrite wins for both pure (e.g., `borrow`)
                        // and mutating (e.g., `swap`) vector callees.
                        if self.try_wp_vector_intrinsic_call(
                            state, offset, *module_id, *fun_id, type_inst, srcs, dests,
                        ) || self.try_wp_map_intrinsic_call(
                            state, offset, *module_id, *fun_id, type_inst, srcs, dests,
                        ) {
                            self.add_direct_call_modifies(
                                state, *module_id, *fun_id, type_inst, srcs,
                            );
                        } else if dests.len() == 1
                            && let Some((spec_fun_id, result_type)) =
                                self.try_as_pure_spec_call(*module_id, *fun_id, type_inst)
                        {
                            // Pure callee with no `&mut` params: substitute the
                            // result with a SpecFunction call and emit aborts_of.
                            // WP[dest := f(args)](Q) = Q[dest ↦ spec_f(args)]
                            let args: Vec<Exp> =
                                srcs.iter().map(|s| self.mk_temporary(*s)).collect();
                            // Use mk_call_with_inst to record the type instantiation so
                            // the sourcifier emits explicit type arguments (e.g.,
                            // `spec_new<K, V>()` instead of `spec_new()`). This is
                            // required for zero-argument spec functions where type args
                            // cannot be inferred from the argument list.
                            let spec_call = self.mk_call_with_inst(
                                &result_type,
                                type_inst.to_vec(),
                                AstOp::SpecFunction(
                                    *module_id,
                                    spec_fun_id,
                                    MemoryRange::default(),
                                ),
                                args.clone(),
                            );
                            *state = self.substitute_exp_state(state, dests[0], &spec_call);
                            self.global_env()
                                .add_used_spec_fun_transitive(module_id.qualified(spec_fun_id));
                            let (fun_exp, _) = self.mk_closure(
                                *module_id,
                                *fun_id,
                                type_inst,
                                ClosureMask::empty(),
                                vec![],
                            );
                            if self.callee_is_known_non_aborting(&fun_exp, &args) {
                                // Nothing to add.
                            } else if self.callee_has_trusted_abort_summary(&fun_exp) {
                                let aborts = self.mk_aborts_of(fun_exp, args);
                                state.add_aborts(aborts);
                            } else {
                                state.aborts_partial = true;
                            }
                        } else if dests.len() == 1
                            && let Some(spec_exp) =
                                self.try_as_native_spec_exp(*module_id, *fun_id, type_inst, srcs)
                        {
                            // WP[dest := native_f(args)](Q) = Q[dest ↦ builtin_spec_op(args)]
                            // Used for native std::vector functions with direct spec-language
                            // equivalents (empty→[], length→len, borrow→index).  Avoids
                            // creating an anonymous lambda that gets compiled to an
                            // uninterpreted behavioral spec function.
                            *state = self.substitute_exp_state(state, dests[0], &spec_exp);
                            // Native vector functions cannot abort; no aborts_of needed.
                        } else if dests.len() == 1
                            && let Some(result_exp) = self
                                .try_as_functional_result_exp(*module_id, *fun_id, type_inst, srcs)
                        {
                            *state = self.substitute_exp_state(state, dests[0], &result_exp);
                            let args: Vec<Exp> =
                                srcs.iter().map(|src| self.mk_temporary(*src)).collect();
                            let (fun_exp, _) = self.mk_closure(
                                *module_id,
                                *fun_id,
                                type_inst,
                                ClosureMask::empty(),
                                vec![],
                            );
                            if self.callee_is_known_non_aborting(&fun_exp, &args) {
                                // Nothing to add.
                            } else if self.callee_has_trusted_abort_summary(&fun_exp) {
                                state.add_aborts(self.mk_aborts_of(fun_exp, args));
                            } else {
                                state.aborts_partial = true;
                            }
                            self.add_direct_call_modifies(
                                state, *module_id, *fun_id, type_inst, srcs,
                            );
                        } else {
                            // WP[dest := f(args)](Q) = Q[dest ↦ result_of<f>(args)]
                            let (fun_exp, result_type) = self.mk_closure(
                                *module_id,
                                *fun_id,
                                type_inst,
                                ClosureMask::empty(),
                                vec![],
                            );
                            let args = self.mk_behavioral_call_args(state, srcs);
                            let mut_ref_srcs: Vec<(usize, TempIndex)> = srcs
                                .iter()
                                .enumerate()
                                .filter(|&(_, &idx)| {
                                    self.get_local_type(idx).is_mutable_reference()
                                })
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
                            self.add_direct_call_modifies(
                                state, *module_id, *fun_id, type_inst, srcs,
                            );
                        }
                    },

                    // WP[dest := closure<f>(captured_args)](Q) = Q[dest ↦ |provided| f(captured, provided)]
                    Operation::Closure(module_id, fun_id, type_inst, mask) => {
                        if dests.len() == 1 {
                            let captured_args: Vec<Exp> =
                                srcs.iter().map(|&s| self.mk_temporary(s)).collect();
                            let (closure_exp, _) = self.mk_closure(
                                *module_id,
                                *fun_id,
                                type_inst,
                                *mask,
                                captured_args,
                            );
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
                        let args = self.mk_behavioral_call_args(state, actual_args);
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
                                if state.is_normal_return {
                                    let param_exp = self.mk_temporary(ref_idx);
                                    state.add_ensures(self.mk_eq(param_exp, val_exp));
                                }
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
                        // Add abort condition: !exists<R>(@label)(addr)
                        // Use the forward-analysis label at this offset.
                        let abort_label = self.forward_label_at(offset);
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp,
                            Some(abort_label),
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
                        // Add abort condition: !exists<R>(@label)(addr)
                        // Use the forward-analysis label at this offset.
                        let abort_label = self.forward_label_at(offset);
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp,
                            Some(abort_label),
                        );
                        let not_exists = self.mk_not(exists_exp);
                        state.add_aborts(not_exists);
                    },

                    // WP[dest := move_from<R>(addr)](Q) =
                    //   Q[dest ↦ R[addr]] ∧ exists<R>(addr) ∧ ensures(!exists<R>(addr))
                    Operation::MoveFrom(module_id, struct_id, type_args) => {
                        let dest = dests[0];
                        let pre_label = self.forward_label_at(offset);
                        let post_label = state.post;
                        let addr_exp = self.mk_temporary(srcs[0]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // Return value comes from global state
                        let global_exp = self.mk_global_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(pre_label),
                        );
                        *state = self.substitute_exp_state(state, dest, &global_exp);
                        // Add abort condition: !exists<R>(@addr)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(pre_label),
                        );
                        let not_exists = self.mk_not(exists_exp);
                        state.add_aborts(not_exists.clone());
                        // Post-state ensures: remove<R>(addr) defines the transition.
                        let range = MemoryRange {
                            pre: Some(pre_label),
                            post: if post_label == self.at_end_label.get() {
                                None
                            } else {
                                Some(post_label)
                            },
                        };
                        state.add_ensures(self.mk_spec_remove(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            range,
                        ));
                        // Track as direct modifies target
                        let modifies_target = self.mk_global(&struct_env, type_args, addr_exp);
                        state.add_body_modifies(modifies_target);
                        state.post = pre_label;
                    },

                    // WP[move_to<R>(signer, val)](Q) =
                    //   Q ∧ !exists<R>(addr) ∧ ensures(exists<R>(addr) ∧ R[addr] == val)
                    Operation::MoveTo(module_id, struct_id, type_args) => {
                        let pre_label = self.forward_label_at(offset);
                        let post_label = state.post;
                        // srcs[0] = signer/address, srcs[1] = resource value
                        let addr_exp = self.signer_to_address(self.mk_temporary(srcs[0]));
                        let val_exp = self.mk_temporary(srcs[1]);
                        let struct_env = self.get_struct(*module_id, *struct_id);
                        // Add abort condition: exists<R>(@addr) (resource already there)
                        let exists_exp = self.mk_exists_with_label(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            Some(pre_label),
                        );
                        state.add_aborts(exists_exp.clone());
                        // Post-state ensures: publish<R>(addr, val) defines the transition.
                        let range = MemoryRange {
                            pre: Some(pre_label),
                            post: if post_label == self.at_end_label.get() {
                                None
                            } else {
                                Some(post_label)
                            },
                        };
                        state.add_ensures(self.mk_spec_publish(
                            &struct_env,
                            type_args,
                            addr_exp.clone(),
                            val_exp,
                            range,
                        ));
                        // Track as direct modifies target
                        let modifies_target = self.mk_global(&struct_env, type_args, addr_exp);
                        state.add_body_modifies(modifies_target);
                        state.post = pre_label;
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
                                            if state.is_normal_return {
                                                state.add_ensures(self.mk_eq(old_exp, new_exp));
                                            }
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
                                    self.borrow_global_info.get(&ref_temp).cloned()
                                {
                                    // Emit update<R>(addr, ref_temp) with pre-label from
                                    // forward analysis and post-label from backward state.
                                    let struct_env = self.get_struct(mid, sid);
                                    let addr_exp = self.mk_temporary(addr_temp);
                                    let val_exp = self.mk_temporary(ref_temp);

                                    let pre_label = self.forward_label_at(offset);
                                    let post_label = state.post;
                                    let range = MemoryRange {
                                        pre: Some(pre_label),
                                        post: if post_label == self.at_end_label.get() {
                                            None
                                        } else {
                                            Some(post_label)
                                        },
                                    };
                                    state.add_ensures(self.mk_spec_update(
                                        &struct_env,
                                        &targs,
                                        addr_exp.clone(),
                                        val_exp,
                                        range,
                                    ));

                                    // Track as direct modifies target
                                    let modifies_target =
                                        self.mk_global(&struct_env, &targs, addr_exp);
                                    state.add_body_modifies(modifies_target);
                                    state.post = pre_label;
                                }
                                // Track in update_globals (not captured_globals) so
                                // has_global_mutations() returns true for sequential
                                // writes, but is_global_or_mut_param() returns false
                                // so WriteBack(Reference) substitutes in-place.
                                state.update_globals.insert(ref_temp);
                            },
                            BorrowNode::ReturnPlaceholder(_) => {
                                // This doesn't appear in bytecode instructions, skip
                            },
                        }
                    },

                    // Havoc: wp(x := *, Q) = forall x. Q
                    // Wrap conditions referencing dest in a universal quantifier.
                    // NOTE: For loop-modified variables, the existential quantification
                    // of abort conditions may over-approximate. E.g., a loop that
                    // increments a counter n times produces `aborts_if 0 < n` instead
                    // of the precise `aborts_if counter + n > MAX_U64`, because the
                    // quantifier abstracts away cumulative effects. This is sound
                    // (over-approximates aborts) but imprecise.
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
                        // Flatten an expression into its top-level conjuncts.
                        // `a && b && c` → `[a, b, c]`.
                        fn flat_conjuncts(e: &Exp) -> Vec<Exp> {
                            match e.as_ref() {
                                ExpData::Call(_, AstOp::And, args) if args.len() == 2 => {
                                    let mut v = flat_conjuncts(&args[0]);
                                    v.extend(flat_conjuncts(&args[1]));
                                    v
                                },
                                _ => vec![e.clone()],
                            }
                        }
                        let wrap = |this: &Self, e: &Exp, quant_kind: QuantKind| -> Exp {
                            if !e.as_ref().any(
                                &mut |ed| matches!(ed, ExpData::Temporary(_, idx) if *idx == dest),
                            ) {
                                return e.clone();
                            }
                            // For `forall` (ensures), distribute the quantifier over top-level
                            // conjuncts: `forall x: (A(x) && B)` = `(forall x: A(x)) && B`.
                            // This prevents unrelated conjuncts like `i >= amount` from being
                            // pulled inside `forall x: T: ...` just because some other conjunct
                            // mentions `self`/`dest`.
                            //
                            // For `exists` (aborts) we must NOT split: `exists x: A(x) && B(x)`
                            // is NOT equivalent to `(exists x: A(x)) && (exists x: B(x))` —
                            // splitting loses the correlation and over-approximates aborts.
                            if quant_kind == QuantKind::Forall {
                                let conjuncts = flat_conjuncts(e);
                                if conjuncts.len() > 1 {
                                    let wrapped: Vec<Exp> = conjuncts
                                        .iter()
                                        .map(|conjunct| {
                                            if !conjunct.as_ref().any(&mut |ed| {
                                                matches!(ed, ExpData::Temporary(_, idx) if *idx == dest)
                                            }) {
                                                return conjunct.clone();
                                            }
                                            let body = if is_ref {
                                                this.substitute_temp_outside_old(
                                                    conjunct, dest, &local_exp,
                                                )
                                            } else {
                                                this.substitute_temp_with_exp(
                                                    conjunct, dest, &local_exp,
                                                )
                                            };
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
                                        })
                                        .collect();
                                    return wrapped
                                        .into_iter()
                                        .reduce(|a, b| this.mk_and(a, b))
                                        .unwrap_or_else(|| e.clone());
                                }
                            }
                            // Default path: wrap the whole expression (used for Exists, or
                            // when there's only a single conjunct).
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

                    // Opaque calls: when inference is active, opaque calls appear as
                    // Operation::Function (see spec_instrumentation), so these are no-ops.
                    Operation::OpaqueCallBegin(_, _, _) | Operation::OpaqueCallEnd(_, _, _) => {},

                    // WP[...](Q) = Q  (verification IL; no effect on inference)
                    Operation::IsParent(_, _)
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
                        if self
                            .evidence_seed
                            .as_ref()
                            .is_some_and(|seed| seed.offset == offset)
                        {
                            *state = self.mk_loop_evidence_seed();
                        } else {
                            *state = WPState::new(state.post);
                        }
                    },
                    // Memory havoc at loop headers: nothing is known about the
                    // post-havoc memory. Abort conditions crossing the havoc
                    // are dropped (cumulative loop aborts are inexact, so the
                    // aborts spec becomes partial); `state.post` is advanced
                    // so ensures accumulated backward for pre-loop code bind
                    // strictly before the loop, never to the end-state.
                    Operation::HavocGlobal(_, _, _) => {
                        state.aborts.clear();
                        state.aborts_partial = true;
                        state.post = self.forward_label_at(offset);
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
            Bytecode::Prop(id, kind, exp) => {
                match kind {
                    PropKind::Assume | PropKind::Assert => {
                        // Skip props carrying state-anchor operations. A
                        // `SaveStateAnchor` marker is a positional no-op (it
                        // directs where spec instrumentation snapshots state),
                        // not a logical fact; and a condition under a
                        // `WithStateAnchor` wrapper refers to the state of an
                        // intermediate program point, which the WP state model
                        // cannot represent. Such props arise in lambda bodies
                        // into which the inliner expanded an anchored inline
                        // call (e.g. a lambda wrapping an inline function's
                        // once-applied function parameter). Dropping a fact
                        // only weakens antecedents; the inferred spec is still
                        // verified afterwards.
                        if contains_state_anchor(exp) {
                            return;
                        }
                        // Treat WellFormed assumptions as no-ops for inference:
                        // they wrap every ensures in implications like
                        // `WellFormed(a) ==> WellFormed(b) ==> result == a + b`
                        // which are unhelpful for inferred specs.
                        // Also skip quantified forms from WellFormedInstrumentation:
                        // `forall x in ResourceDomain<T>: WellFormed(x)`
                        if matches!(kind, PropKind::Assume) && is_well_formed_prop(exp) {
                            return;
                        }
                        // `CanModify` is a verifier-internal frame-permission
                        // assumption emitted from a `modifies` clause. Though
                        // it has source syntax for round-tripping inferred
                        // expressions, it is not program behavior and must not
                        // become an antecedent of inferred conditions.
                        if contains_can_modify(exp) {
                            return;
                        }
                        // Skip data/global invariant asserts: they are verification
                        // conditions proven separately by the Boogie backend. Including
                        // them as WP antecedents wraps every inferred ensures in an
                        // invariant precondition, producing unhelpful conditional specs
                        // for any function that constructs or packs a struct.
                        if matches!(kind, PropKind::Assert) {
                            let vc_msg = self.target.get_vc_info(*id).map(|s| s.as_str());
                            if matches!(
                                vc_msg,
                                Some(s) if s == DATA_INVARIANT_FAILS_MESSAGE
                                    || s == GLOBAL_INVARIANT_FAILS_MESSAGE
                                    // These obligations are generated from the
                                    // pre-inference abort specification. Feeding
                                    // them back into WP is circular; in strict
                                    // mode an empty spec contributes `assert
                                    // false`, erasing an abort-only path.
                                    || s == ABORTS_IF_FAILS_MESSAGE
                                    || s == ABORT_NOT_COVERED
                                    || s == ABORTS_CODE_NOT_COVERED
                            ) {
                                return;
                            }
                        }

                        // Handle Identical (from spec let bindings) as substitution.
                        // SpecInstrumentationProcessor emits `let x = e` as
                        // `Prop(Assume, Identical($tN, e))` where $tN is a spec-only
                        // temp with no corresponding bytecode Assign. Wrapping the
                        // state with `implies(Identical($tN, e), ...)` would embed
                        // an unresolvable temp. Instead, inline the definition.
                        if let ExpData::Call(_, AstOp::Identical, args) = exp.as_ref() {
                            if args.len() == 2 {
                                if let ExpData::Temporary(_, idx) = args[0].as_ref() {
                                    *state = self.substitute_exp_state(state, *idx, &args[1]);
                                    return;
                                }
                            }
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
                        // ensures: standard WP (P ==> Q), but skip abort‑related
                        // antecedents. The total Move post‑condition is
                        // `aborts ∨ ensures`, so when ensures is checked the
                        // function did not abort — making `AbortFlag()` and
                        // `aborts_of(…)`‑bearing conditions tautologies on this
                        // path. Wrapping ensures with such conditions only clutters
                        // the inferred spec; the corresponding `aborts_if` clauses
                        // already capture the same information.
                        if !cond_is_false_on_normal_return(&cond) {
                            state.ensures = state
                                .ensures
                                .iter()
                                .map(|e| self.mk_implies(cond.clone(), e.clone()))
                                .collect();
                        }
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
                            Some(block_id),
                        );
                    },
                    None => {
                        // First state arriving at this block.
                        // Record the predecessor block ID so path_aware_join can
                        // determine which branch side the already-stored state
                        // came from when the second side arrives.
                        let mut initial_post = post.clone();
                        if branch_info.is_some() {
                            initial_post.origin_block = Some(block_id);
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
    fn close_non_parameter_temporaries(&self, state: &mut WPState) {
        let parameter_count = self.target.get_parameter_count();
        let close = |expression: &Exp, kind: QuantKind| {
            let mut temporaries: Vec<TempIndex> = expression
                .as_ref()
                .used_temporaries()
                .into_iter()
                .filter(|index| *index >= parameter_count)
                .collect();
            temporaries.sort_unstable();
            temporaries.dedup();
            temporaries
                .into_iter()
                .rev()
                .fold(expression.clone(), |body, index| {
                    let raw_type = self.get_local_type(index);
                    let quantified_type = raw_type.skip_reference().clone();
                    let symbol = self.mk_symbol(&format!("$local{}", index));
                    let local = self.mk_local_by_sym(symbol, quantified_type.clone());
                    let body = self.substitute_temp_with_exp(&body, index, &local);
                    let range = self.mk_type_domain(quantified_type.clone());
                    let pattern = self.mk_decl(symbol, quantified_type);
                    let id = self.new_node(BOOL_TYPE.clone(), None);
                    ExpData::Quant(id, kind, vec![(pattern, range)], vec![], None, body).into_exp()
                })
        };
        state.ensures = state
            .ensures
            .iter()
            .map(|expression| close(expression, QuantKind::Forall))
            .collect();
        state.aborts = state
            .aborts
            .iter()
            .map(|expression| close(expression, QuantKind::Exists))
            .collect();
    }

    /// Get the struct environment for a given module and struct id.
    fn get_struct(&self, module_id: ModuleId, struct_id: StructId) -> StructEnv<'env> {
        self.global_env().get_struct(QualifiedId {
            module_id,
            id: struct_id,
        })
    }

    fn new_with_evidence_seed(
        fun_env: &'env FunctionEnv<'env>,
        data: &'env FunctionData,
        evidence_seed: Option<LoopEvidenceSeed>,
    ) -> Self {
        let target = FunctionTarget::new(fun_env, data);
        let env = fun_env.module_env.env;

        // Create the entry label representing the function's initial state.
        let at_entry_label = MemoryLabel::new(env.new_global_id().as_usize());
        let at_entry_sym = env.symbol_pool().make("at_entry");
        env.set_memory_label_name(at_entry_label, at_entry_sym);

        // Create the "at_end" label representing the final state (post-return).
        let at_end_label = MemoryLabel::new(env.new_global_id().as_usize());
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
            at_entry_label,
            at_end_label: Cell::new(at_end_label),
            offset_labels: RefCell::new(BTreeMap::new()),
            borrow_global_info,
            havoc_targets,
            forward_label_map: RefCell::new(BTreeMap::new()),
            label_counter: Cell::new(0),
            evidence_seed,
        }
    }

    fn mk_loop_evidence_seed(&self) -> WPState {
        let seed = self
            .evidence_seed
            .as_ref()
            .expect("loop evidence seed available");
        let ensures = seed
            .carried
            .iter()
            .map(|(temp, name)| {
                let symbol = self.mk_symbol(&format!("__loop_head_{}_{}", seed.head_index, name));
                let head_value = self.mk_local_by_sym(symbol, self.get_local_type(*temp));
                self.mk_eq(head_value, self.mk_temporary(*temp))
            })
            .collect();
        WPState {
            ensures,
            aborts: vec![],
            is_normal_return: true,
            origin_block: None,
            post: self.at_end_label.get(),
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            update_globals: BTreeSet::new(),
            direct_modifies: vec![],
            body_modifies: vec![],
            aborts_partial: false,
            solver_hard: false,
        }
    }

    /// Create or retrieve a memory label for a specific code offset.
    /// Returns the same label for the same offset (for fixpoint stability).
    fn mk_label_at(&self, offset: CodeOffset) -> MemoryLabel {
        let mut cache = self.offset_labels.borrow_mut();
        *cache.entry(offset).or_insert_with(|| {
            let env = self.global_env();
            let label = MemoryLabel::new(env.new_global_id().as_usize());
            let seq = self.label_counter.get() + 1;
            self.label_counter.set(seq);
            let name = format!("{}{}", INFERRED_LABEL_PREFIX, seq);
            let sym = env.symbol_pool().make(&name);
            env.set_memory_label_name(label, sym);
            label
        })
    }

    /// Run forward state boundary analysis to pre-assign memory labels.
    /// Must be called before the backward WP analysis.
    ///
    /// The forward analysis assigns a fresh label at each state-changing instruction.
    /// The result maps each bytecode offset to (state_before, state_after).
    /// `at_end_label` is NOT derived from the forward analysis (CFG joins can
    /// lose branch-specific information); it's created in the constructor.
    fn run_forward_label_analysis(&self, bytecode: &[Bytecode]) {
        let fwd_cfg = StacklessControlFlowGraph::new_forward(bytecode);
        let initial = StateBoundaryState(self.at_entry_label);
        let analysis = StateBoundaryAnalysis { analyzer: self };
        let state_map = analysis.analyze_function(initial, bytecode, &fwd_cfg);
        let label_map =
            analysis.state_per_instruction(state_map, bytecode, &fwd_cfg, |before, after| {
                // Map the forward exit state to at_end_label at Ret/Abort.
                // The forward analysis may produce different exit labels on
                // different paths (due to CFG join imprecision), but the
                // backward WP uses at_end_label uniformly for the exit state.
                let after_label = if after.0 != before.0 {
                    after.0 // state-changing: keep the forward-assigned label
                } else {
                    before.0 // non-state-changing: same as before
                };
                (before.0, after_label)
            });
        *self.forward_label_map.borrow_mut() = label_map;
    }

    /// Look up the forward-analysis memory label at a given code offset.
    /// Returns the state BEFORE the instruction (the ambient state).
    fn forward_label_at(&self, offset: CodeOffset) -> MemoryLabel {
        self.forward_label_map
            .borrow()
            .get(&offset)
            .map(|(pre, _)| *pre)
            .unwrap_or(self.at_entry_label)
    }

    /// Shared WP logic for function calls (direct) and closure invocations.
    /// Substitutes `dest_i ↦ result_of<f>(args)[i]` and
    /// `&mut src_j ↦ write_of<f, j>(args)`. For caller-`&mut`-param
    /// chaining, `write_of` flows through `substitute_old_param_in_state`.
    /// For void callees, emits an `ensures_of<f>(args)` state-chain anchor.
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
        if let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = fun_exp.as_ref()
            && self
                .global_env()
                .get_function((*module_id).qualified(*fun_id))
                .is_pragma_false(VERIFY_PRAGMA)
        {
            state.solver_hard = true;
        }
        let pre_label = self.forward_label_at(offset);
        let call_post = state.post;

        // `aborts_of` has no post-state — aborts don't produce state.
        let behavior_pre = Some(pre_label);
        // Source-level state ranges use an omitted post label for the ambient
        // function post-state.  A repeated `S..S` range is both redundant and
        // rejected as a cyclic label definition on reparse.  The same label can
        // arise when a call is observationally state-preserving.
        let behavior_post = if call_post == self.at_end_label.get() || call_post == pre_label {
            None
        } else {
            Some(call_post)
        };
        let aborts_pre = Some(pre_label);
        let aborts_post: Option<MemoryLabel> = None;

        let num_declared_results = result_type.clone().flatten().len();

        let mk_write_of_for = |me: &Self, j: usize| -> Exp {
            let mut_ref_value_type = me
                .get_local_type(mut_ref_srcs[j].1)
                .skip_reference()
                .clone();
            me.mk_write_of_with_state(
                fun_exp.clone(),
                args.clone(),
                &mut_ref_value_type,
                j,
                behavior_pre,
                behavior_post,
            )
        };

        // Collect all substitutions and apply simultaneously: sequential
        // substitution would re-substitute inside already-substituted args.
        let mut all_subs: Vec<(TempIndex, Exp)> = Vec::new();

        for (i, &dest) in dests.iter().enumerate() {
            let result_exp = self.mk_result_of_at_with_state(
                fun_exp.clone(),
                args.clone(),
                result_type,
                i,
                num_declared_results,
                behavior_pre,
                behavior_post,
            );
            all_subs.push((dest, result_exp));
        }

        // Skip already-captured caller-`&mut`-params: their `old(param)`
        // gets handled by `substitute_old_param_in_state` below; adding a
        // direct substitution here would cause double-application.
        for (j, &(_, idx)) in mut_ref_srcs.iter().enumerate() {
            if self.is_mut_ref_param(idx) && state.captured_mut_params.contains(&idx) {
                continue;
            }
            all_subs.push((idx, mk_write_of_for(self, j)));
        }

        // If no dest is referenced downstream, the result is discarded —
        // emit `ensures_of` as a state-chain anchor instead.
        let any_dest_referenced = dests.iter().any(|&d| {
            state.ensures.iter().chain(state.aborts.iter()).any(|e| {
                let mut found = false;
                e.visit_pre_order(&mut |sub| {
                    if let ExpData::Temporary(_, idx) = sub {
                        if *idx == d {
                            found = true;
                        }
                    }
                    !found
                });
                found
            })
        });

        *state = self.substitute_multiple_temps_in_state(state, &all_subs);

        // For caller-`&mut`-params: on first encounter, add the binding
        // `Eq(param, write_of(...))` and mark captured. On subsequent
        // calls, substitute `old(param)` with the chained write_of.
        for (j, &(_, idx)) in mut_ref_srcs.iter().enumerate() {
            if self.is_mut_ref_param(idx) {
                if !state.captured_mut_params.contains(&idx) {
                    if state.is_normal_return {
                        let param_exp = self.mk_temporary(idx);
                        let write_exp = mk_write_of_for(self, j);
                        state.add_ensures(self.mk_eq(param_exp, write_exp));
                    }
                    state.captured_mut_params.insert(idx);
                } else {
                    let write_exp = mk_write_of_for(self, j);
                    *state = self.substitute_old_param_in_state(state, idx, &write_exp);
                }
            }
        }

        // For void callees: `ensures_of<f>(args)` is the only postcondition.
        // For non-void with discarded result: anchor with `ensures_of<f>(args,
        // result_of<f>(args)...)` so downstream predicates can reference
        // the intermediate state.
        if num_declared_results == 0 {
            let ensures_of = self.mk_ensures_of_with_state(
                fun_exp.clone(),
                args.clone(),
                behavior_pre,
                behavior_post,
            );
            state.add_ensures(ensures_of);
        } else if !any_dest_referenced {
            let mut ensures_args = args.clone();
            for i in 0..num_declared_results {
                ensures_args.push(self.mk_result_of_at_with_state(
                    fun_exp.clone(),
                    args.clone(),
                    result_type,
                    i,
                    num_declared_results,
                    behavior_pre,
                    behavior_post,
                ));
            }
            let ensures_of = self.mk_ensures_of_with_state(
                fun_exp.clone(),
                ensures_args,
                behavior_pre,
                behavior_post,
            );
            state.add_ensures(ensures_of);
        }

        if self.callee_is_known_non_aborting(&fun_exp, &args) {
            // Nothing to add.
        } else if self.callee_has_trusted_abort_summary(&fun_exp) {
            let aborts = self.mk_aborts_of_with_state(fun_exp, args, aborts_pre, aborts_post);
            state.add_aborts(aborts);
        } else {
            state.aborts_partial = true;
        }

        // Update post-state for predecessor: they see this call's pre-state
        state.post = pre_label;
    }

    /// Return whether a call is provably unable to abort from information
    /// available while constructing its WP. This is deliberately narrow: an
    /// absent or partial abort specification never qualifies.
    fn callee_is_known_non_aborting(&self, fun_exp: &Exp, args: &[Exp]) -> bool {
        let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = fun_exp.as_ref() else {
            return false;
        };
        let env = self.global_env();
        let callee = env.get_function((*module_id).qualified(*fun_id));
        if callee.is_well_known(well_known::TYPE_NAME_MOVE)
            || callee.is_well_known(well_known::TYPE_INFO_MOVE)
            || callee.is_well_known(well_known::TYPE_NAME_GET_MOVE)
        {
            return true;
        }
        let module_name = callee
            .module_env
            .get_name()
            .name()
            .display(env.symbol_pool())
            .to_string();
        let function_name = callee.get_name().display(env.symbol_pool()).to_string();
        if module_name == "simple_map" && function_name == "contains_key" {
            return true;
        }
        if callee.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false) {
            return false;
        }
        let spec = callee.get_spec();
        let aborts_if: Vec<_> = spec
            .conditions
            .iter()
            .filter(|condition| matches!(condition.kind, ConditionKind::AbortsIf))
            .collect();
        let has_aborts_with = spec
            .conditions
            .iter()
            .any(|condition| matches!(condition.kind, ConditionKind::AbortsWith));
        if !aborts_if.is_empty()
            && !has_aborts_with
            && aborts_if
                .iter()
                .all(|condition| is_trivial_false(&condition.exp))
        {
            return true;
        }
        drop(spec);

        // Function-target processing order does not guarantee that a callee's
        // inferred `aborts_if false` has been installed before its caller is
        // analyzed. Recognize only verifier-safe, straight-line value
        // constructors here; every operation capable of calling, branching,
        // arithmetic failure, or accessing global state stays conservative.
        if callee.get_bytecode().is_some_and(|code| {
            !code.is_empty()
                && code.iter().all(|instruction| {
                    matches!(
                        instruction,
                        MoveBytecode::Pop
                            | MoveBytecode::Ret
                            | MoveBytecode::LdU8(_)
                            | MoveBytecode::LdU16(_)
                            | MoveBytecode::LdU32(_)
                            | MoveBytecode::LdU64(_)
                            | MoveBytecode::LdU128(_)
                            | MoveBytecode::LdU256(_)
                            | MoveBytecode::LdI8(_)
                            | MoveBytecode::LdI16(_)
                            | MoveBytecode::LdI32(_)
                            | MoveBytecode::LdI64(_)
                            | MoveBytecode::LdI128(_)
                            | MoveBytecode::LdI256(_)
                            | MoveBytecode::LdConst(_)
                            | MoveBytecode::LdTrue
                            | MoveBytecode::LdFalse
                            | MoveBytecode::CopyLoc(_)
                            | MoveBytecode::MoveLoc(_)
                            | MoveBytecode::StLoc(_)
                            | MoveBytecode::Pack(_)
                            | MoveBytecode::PackGeneric(_)
                            | MoveBytecode::PackVariant(_)
                            | MoveBytecode::PackVariantGeneric(_)
                    )
                })
        }) {
            return true;
        }

        // A Move byte-string literal is lowered through string::utf8. The
        // native UTF-8 predicate is intentionally opaque to the prover, but a
        // fully constant byte vector can be checked exactly here. Without
        // this reduction, a valid non-empty literal becomes an unprovable
        // `aborts_of<string::utf8>(vector[...])` in sourcified WP output.
        let module = env.get_module(*module_id);
        module.get_name().addr() == &env.get_stdlib_address()
            && module.get_name().name() == env.symbol_pool().make(well_known::STRING_MODULE)
            && function_name == well_known::UTF8_FUNCTION_NAME
            && matches!(args, [arg] if constant_valid_utf8(arg))
    }

    /// Whether `aborts_of<callee>(..)` is an exact summary of the callee's
    /// aborts, so it can be emitted as an abort condition of the caller.
    ///
    /// Stated `aborts_if` clauses give that guarantee
    /// ([`spec_derivation::spec_aborts_are_exact`]). `pragma opaque`
    /// additionally stands in for a callee whose own contract this run is
    /// still inferring and has not installed yet -- inference adds the
    /// pragma to every target it processes, and a caller analyzed before
    /// its callee would otherwise lose the callee's abort behavior.
    fn callee_has_trusted_abort_summary(&self, fun_exp: &Exp) -> bool {
        let ExpData::Call(_, AstOp::Closure(module_id, fun_id, _), _) = fun_exp.as_ref() else {
            return false;
        };
        let callee_qid = (*module_id).qualified(*fun_id);
        let callee = self.global_env().get_function(callee_qid);
        // `spec_aborts_are_exact` already excludes `aborts_if_is_partial`;
        // the opaque fallback must exclude it too.
        (spec_derivation::spec_aborts_are_exact(self.global_env(), callee_qid)
            || (callee.is_pragma_true(OPAQUE_PRAGMA, || false)
                && !callee.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false)))
            && !function_abort_spec_uses_generic_type_reflection(&callee)
    }

    fn reduce_known_non_aborting_behaviors(&self, exp: &Exp) -> Exp {
        struct Reducer<'a, 'env> {
            analyzer: &'a SpecInferenceAnalyzer<'env>,
        }

        impl ExpRewriterFunctions for Reducer<'_, '_> {
            fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                if matches!(
                    oper,
                    AstOp::Behavior(move_model::ast::BehaviorKind::AbortsOf, _)
                ) && let Some((fun_exp, call_args)) = args.split_first()
                    && self
                        .analyzer
                        .callee_is_known_non_aborting(fun_exp, call_args)
                {
                    return Some(ExpData::Value(id, Value::Bool(false)).into_exp());
                }
                None
            }
        }

        Reducer { analyzer: self }.rewrite_exp(exp.clone())
    }

    /// WP for a `std::vector` bytecode-instruction native (and `singleton`/
    /// `contains`) — applies the call's spec semantics directly via
    /// substitution, so no behavioral predicate over the vector intrinsic
    /// ever appears in inferred specs.
    ///
    /// Returns `true` when the callee is a recognized vector intrinsic and
    /// the WP rewrite was applied; `false` otherwise (caller falls back to
    /// the generic BP-emission path).
    fn try_wp_vector_intrinsic_call(
        &self,
        state: &mut WPState,
        offset: CodeOffset,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
        srcs: &[TempIndex],
        dests: &[TempIndex],
    ) -> bool {
        let args = self.mk_behavioral_call_args(state, srcs);
        let mut_ref_srcs: Vec<(usize, TempIndex)> = srcs
            .iter()
            .enumerate()
            .filter(|&(_, &idx)| self.get_local_type(idx).is_mutable_reference())
            .map(|(i, &idx)| (i, idx))
            .collect();

        // Tag outputs with the dest temp types (and `&mut` src local types
        // stripped of references) so the simplifier's type-bound reasoning
        // matches the dests — e.g. `vector::length` returns `u64`, not the
        // operator `Len`'s natural `Num` type.
        let mut output_types: Vec<Type> = dests
            .iter()
            .map(|&d| self.get_local_type(d).clone())
            .collect();
        for (_, idx) in &mut_ref_srcs {
            output_types.push(self.get_local_type(*idx).skip_reference().clone());
        }

        let wp = match move_model::well_known::vector_intrinsic_wp(
            self.global_env(),
            self,
            module_id.qualified(fun_id),
            type_inst,
            &args,
            &output_types,
        ) {
            Some(wp) => wp,
            None => return false,
        };

        let num_explicit = dests.len();

        let mut all_subs: Vec<(TempIndex, Exp)> = Vec::new();
        for (i, &dest) in dests.iter().enumerate() {
            all_subs.push((dest, wp.outputs[i].clone()));
        }
        for (j, (_, idx)) in mut_ref_srcs.iter().enumerate() {
            all_subs.push((*idx, wp.outputs[num_explicit + j].clone()));
        }
        *state = self.substitute_multiple_temps_in_state(state, &all_subs);

        // Captured-param tracking for `&mut` params, mirroring
        // `wp_function_call` but using the vector intrinsic's concrete
        // post-state expression in place of `result_of<f>(args)[…]`.
        for (j, (_, idx)) in mut_ref_srcs.iter().enumerate() {
            if self.is_mut_ref_param(*idx) {
                let post_exp = wp.outputs[num_explicit + j].clone();
                if !state.captured_mut_params.contains(idx) {
                    if state.is_normal_return {
                        let param_exp = self.mk_temporary(*idx);
                        state.add_ensures(self.mk_eq(param_exp, post_exp));
                    }
                    state.captured_mut_params.insert(*idx);
                } else {
                    *state = self.substitute_old_param_in_state(state, *idx, &post_exp);
                }
            }
        }

        state.add_aborts(wp.aborts);
        state.post = self.forward_label_at(offset);
        true
    }

    /// Apply the exact value-level WP for intrinsic map mutators. This is the
    /// bytecode counterpart of `spec_derivation`'s map-intrinsic path; without
    /// it, multi-result calls such as `simple_map::remove` become
    /// unconstrained `result_of` carriers which cannot be related back to the
    /// actual call during verification.
    fn try_wp_map_intrinsic_call(
        &self,
        state: &mut WPState,
        offset: CodeOffset,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
        srcs: &[TempIndex],
        dests: &[TempIndex],
    ) -> bool {
        let args = self.mk_behavioral_call_args(state, srcs);
        let mut_ref_srcs: Vec<TempIndex> = srcs
            .iter()
            .copied()
            .filter(|idx| self.get_local_type(*idx).is_mutable_reference())
            .collect();
        let wp = match move_model::well_known::map_intrinsic_wp(
            self.global_env(),
            self,
            module_id.qualified(fun_id),
            type_inst,
            &args,
        ) {
            Some(wp) if wp.outputs.len() == dests.len() + mut_ref_srcs.len() => wp,
            _ => return false,
        };

        let num_explicit = dests.len();
        let mut substitutions = Vec::with_capacity(wp.outputs.len());
        substitutions.extend(
            dests
                .iter()
                .enumerate()
                .map(|(i, dest)| (*dest, wp.outputs[i].clone())),
        );
        substitutions.extend(
            mut_ref_srcs
                .iter()
                .enumerate()
                .map(|(i, src)| (*src, wp.outputs[num_explicit + i].clone())),
        );
        *state = self.substitute_multiple_temps_in_state(state, &substitutions);

        for (i, src) in mut_ref_srcs.iter().enumerate() {
            if self.is_mut_ref_param(*src) {
                let post = wp.outputs[num_explicit + i].clone();
                if !state.captured_mut_params.contains(src) {
                    if state.is_normal_return {
                        state.add_ensures(self.mk_eq(self.mk_temporary(*src), post));
                    }
                    state.captured_mut_params.insert(*src);
                } else {
                    *state = self.substitute_old_param_in_state(state, *src, &post);
                }
            }
        }

        state.add_aborts(wp.aborts);
        state.post = self.forward_label_at(offset);
        true
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
    ///
    /// Identical to reading it: the derivation models references
    /// transparently, so a borrow and a read of the same place produce the
    /// same symbolic value.
    #[allow(clippy::too_many_arguments)]
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
        self.wp_get_field(
            state,
            dest,
            src,
            module_id,
            struct_id,
            variants,
            type_args,
            field_offset,
        )
    }

    /// Check if a temporary is a `&mut` parameter.
    fn is_mut_ref_param(&self, idx: TempIndex) -> bool {
        idx < self.target.get_parameter_count()
            && self.target.get_local_type(idx).is_mutable_reference()
    }

    /// Build behavioral predicate arguments for a call site.
    ///
    /// `&mut` source temps are wrapped in `old(...)` so the inferred
    /// expression captures the *pre-call* value. The wrapping is essential for
    /// the WP substitution machinery: without it, substituting a `&mut` temp
    /// during state propagation would replace its occurrences inside its own
    /// `result_of` post-state expression, producing nested-loop garbage like
    /// `result_of<f>(result_of<f>(...))`. With `old(...)`, the substitution
    /// only fires for bare temps in the state (post-state references), while
    /// pre-state references are preserved by `substitute_old_param_in_state`.
    fn mk_behavioral_call_args(&self, _state: &WPState, srcs: &[TempIndex]) -> Vec<Exp> {
        srcs.iter()
            .map(|&idx| {
                let temp = self.mk_temporary(idx);
                if self.is_mut_ref_param(idx) {
                    self.mk_old(temp)
                } else {
                    temp
                }
            })
            .collect()
    }

    /// For native `std::vector` functions that have a direct spec-language equivalent
    /// (built-in operations like `[]`, `len`, index), return the equivalent spec
    /// expression substituting for the call result.  This avoids wrapping the function
    /// reference in an anonymous lambda that gets compiled to an uninterpreted
    /// behavioral spec function.
    ///
    /// Returns `Some(exp)` where `exp` is the spec expression for the result, or `None`
    /// if the function has no direct spec equivalent.
    fn try_as_native_spec_exp(
        &self,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
        srcs: &[TempIndex],
    ) -> Option<Exp> {
        let env = self.global_env();
        let module = env.get_module(module_id);
        // Only handle std::vector native functions.
        if module.get_name().addr() != &env.get_stdlib_address() {
            return None;
        }
        if module.get_name().name() != env.symbol_pool().make(well_known::VECTOR_MODULE) {
            return None;
        }
        let fun_env = env.get_function(module_id.qualified(fun_id));
        let fun_name = fun_env.get_name().display(env.symbol_pool()).to_string();

        match fun_name.as_str() {
            // vector::empty<T>() → [] (empty vector literal)
            "empty" => {
                // Monomorphized code always provides T; bail to the behavioral
                // predicate path rather than fabricating a wrong-typed literal.
                let elem_type = type_inst.first().cloned()?;
                let result_type = Type::Vector(Box::new(elem_type.clone()));
                Some(self.mk_call_with_inst(&result_type, vec![elem_type], AstOp::Vector, vec![]))
            },
            // vector::length<T>(v) → len(v)
            "length" if !srcs.is_empty() => {
                let v = self.mk_temporary(srcs[0]);
                Some(self.mk_call(&NUM_TYPE, AstOp::Len, vec![v]))
            },
            // vector::borrow<T>(v, i) → v[i]
            "borrow" if srcs.len() >= 2 => {
                // Monomorphized code always provides T; bail to the behavioral
                // predicate path rather than fabricating a wrong-typed index.
                let elem_type = type_inst.first().cloned()?;
                let v = self.mk_temporary(srcs[0]);
                let i = self.mk_temporary(srcs[1]);
                Some(self.mk_call_with_inst(
                    &elem_type,
                    vec![elem_type.clone()],
                    AstOp::Index,
                    vec![v, i],
                ))
            },
            _ => None,
        }
    }

    /// Instantiate a caller-visible functional postcondition `result == E`
    /// as a direct value expression for a single-result callee. This keeps the
    /// inferred value tied to the actual call; a generic `result_of` carrier
    /// is insufficient for transparent callees because their executions are
    /// not summarized by the carrier's Skolem axiom.
    fn try_as_functional_result_exp(
        &self,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
        srcs: &[TempIndex],
    ) -> Option<Exp> {
        let env = self.global_env();
        let callee_qid = module_id.qualified(fun_id);
        let callee = env.get_function(callee_qid);
        if callee.get_return_count() != 1
            || callee
                .get_parameters()
                .iter()
                .any(|parameter| parameter.1.is_mutable_reference())
        {
            return None;
        }
        let value = spec_derivation::functional_result_spec_values(env, callee_qid).remove(&0)?;
        let mut replacer = |_id: NodeId, target: RewriteTarget| match target {
            RewriteTarget::Temporary(index) => srcs.get(index).map(|src| self.mk_temporary(*src)),
            RewriteTarget::LocalVar(_) => None,
        };
        let value = ExpRewriter::new(env, &mut replacer)
            .set_type_args(type_inst)
            .rewrite_exp(value);
        for called in value.called_spec_funs(env) {
            env.add_used_spec_fun_transitive(called.to_qualified_id());
        }
        Some(value)
    }

    /// The pure-spec-call test shared with the source-level derivation; see
    /// [`spec_derivation::try_as_pure_spec_call`].
    fn try_as_pure_spec_call(
        &self,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
    ) -> Option<(SpecFunId, Type)> {
        spec_derivation::try_as_pure_spec_call(self.global_env(), module_id, fun_id, type_inst)
    }

    fn add_direct_call_modifies(
        &self,
        state: &mut WPState,
        module_id: ModuleId,
        fun_id: FunId,
        type_inst: &[Type],
        srcs: &[TempIndex],
    ) {
        let callee = self.global_env().get_function(QualifiedId {
            module_id,
            id: fun_id,
        });
        let modifies: Vec<Exp> = callee
            .get_frame_spec()
            .map(|fs| fs.modifies_targets.clone())
            .unwrap_or_default();
        let env = self.global_env();
        for target in modifies.iter() {
            // Instantiate the callee's type parameters with the call-site types.
            let mut target = ExpData::rewrite_node_id(target.clone(), &mut |id| {
                ExpData::instantiate_node(env, id, type_inst)
            });

            // Frame targets can refer to `let` bindings declared in the
            // callee's spec block. Resolve those bindings before moving the
            // target into the caller, where their local names are out of scope.
            for condition in callee.get_spec().conditions.iter().rev() {
                if let ConditionKind::LetPre(symbol, _) = condition.kind {
                    let replacement = ExpData::rewrite_node_id(condition.exp.clone(), &mut |id| {
                        ExpData::instantiate_node(env, id, type_inst)
                    });
                    let mut replacer = |_id: NodeId, rewrite_target: RewriteTarget| {
                        matches!(rewrite_target, RewriteTarget::LocalVar(found) if found == symbol)
                            .then(|| replacement.clone())
                    };
                    target = ExpRewriter::new(env, &mut replacer).rewrite_exp(target);
                }
            }

            // Source-level frame specs use named LocalVars for parameters,
            // whereas derived specs can use Temporary indices. Substitute both
            // representations with the actual call-site arguments.
            let parameter_symbols: BTreeMap<Symbol, Exp> = callee
                .get_parameters_ref()
                .iter()
                .zip(srcs)
                .map(|(parameter, src)| (parameter.0, self.mk_temporary(*src)))
                .collect();
            let mut replacer = |_id: NodeId, rewrite_target: RewriteTarget| match rewrite_target {
                RewriteTarget::LocalVar(symbol) => parameter_symbols.get(&symbol).cloned(),
                RewriteTarget::Temporary(index) => {
                    srcs.get(index).map(|src| self.mk_temporary(*src))
                },
            };
            target = ExpRewriter::new(env, &mut replacer).rewrite_exp(target);
            state.add_direct_modifies(strip_labels_in_exp(&target));
        }
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

    /// Replace resource-wide update predicates based on a non-entry state
    /// with an equality for the deepest updated field. This keeps useful
    /// post-state information while avoiding equality claims about fields an
    /// opaque callee's contract leaves unspecified.
    fn weaken_intermediate_field_updates(&self, state: &mut WPState) {
        struct Rewriter<'a, 'env> {
            analyzer: &'a SpecInferenceAnalyzer<'env>,
        }

        impl ExpRewriterFunctions for Rewriter<'_, '_> {
            fn rewrite_call(&mut self, id: NodeId, oper: &AstOp, args: &[Exp]) -> Option<Exp> {
                let AstOp::SpecUpdate(range) = oper else {
                    return None;
                };
                if args.len() != 2
                    || range
                        .pre
                        .is_none_or(|label| label == self.analyzer.at_entry_label)
                {
                    return None;
                }
                let instantiation = self.analyzer.global_env().get_node_instantiation(id);
                let Some(Type::Struct(mid, sid, type_args)) = instantiation.first() else {
                    return None;
                };
                let struct_env = self.analyzer.get_struct(*mid, *sid);
                let post_value = self.analyzer.mk_global_with_label(
                    &struct_env,
                    type_args,
                    args[0].clone(),
                    range.post,
                );
                self.analyzer.field_update_relation(post_value, &args[1])
            }
        }

        let mut rewriter = Rewriter { analyzer: self };
        *state = state.map(|exp| rewriter.rewrite_exp(exp.clone()));
    }

    fn field_update_relation(&self, post_value: Exp, updated_value: &Exp) -> Option<Exp> {
        let ExpData::Call(
            update_id,
            AstOp::UpdateField(module_id, struct_id, field_id),
            update_args,
        ) = updated_value.as_ref()
        else {
            return None;
        };
        if update_args.len() != 2 {
            return None;
        }
        let instantiation = self.global_env().get_node_instantiation(*update_id);
        let Some(Type::Struct(_, _, type_args)) = instantiation.first() else {
            return None;
        };
        let struct_env = self.get_struct(*module_id, *struct_id);
        let field_env = struct_env.get_field(*field_id);
        let post_field = self.mk_field_select(&field_env, type_args, post_value);
        if matches!(
            update_args[1].as_ref(),
            ExpData::Call(_, AstOp::UpdateField(..), _)
        ) {
            self.field_update_relation(post_field, &update_args[1])
        } else {
            Some(self.mk_eq(post_field, update_args[1].clone()))
        }
    }

    /// Rewrite the WP-internal `WriteOf(j)` carrier into user-facing
    /// behavioral predicates. WP emits two shapes:
    ///   (a) `Eq(lhs, write_of(...))` (possibly wrapped in
    ///       `Implies(WellFormed, ...)`) for each caller-`&mut` captured on
    ///       a normal return — `lhs` is the procedure-level post-state name.
    ///   (b) `write_of(...)` nested inside other expressions (body-borrow
    ///       `&mut` sources).
    ///
    /// Phases:
    ///   1. Collect `(write_of, lhs_path)` bindings (sees through
    ///      `update_field` layers).
    ///   2. For each call site with a binding for every `&mut` slot, emit
    ///      `ensures_of<f>(args, ..dests, ..post_state_slots)` in canonical
    ///      form. Sites missing a binding are skipped (the substitution
    ///      below is still sound but loses the `&mut` post-state binding).
    ///   3. Substitute remaining `write_of`s with the bound `lhs` (or
    ///      `strip_olds(args[mut_pos(j)])` as a fallback) and drop
    ///      tautologies.
    fn eliminate_write_of(&self, state: &mut WPState) {
        let env = self.global_env();

        let bindings = collect_write_of_bindings(env, &state.ensures);

        let mut sites: Vec<(Exp, Vec<Exp>, MemoryRange)> = Vec::new();
        for (write_of_call, _lhs) in &bindings {
            let ExpData::Call(_, AstOp::Behavior(_, range), bp_args) = write_of_call.as_ref()
            else {
                continue;
            };
            if bp_args.is_empty() {
                continue;
            }
            let fun_exp = bp_args[0].clone();
            let args = &bp_args[1..];
            let args_natural: Vec<Exp> = args.iter().map(strip_all_olds).collect();
            // Sites are identified by (function, args, RANGE): the same call
            // with identical arguments in different branches can carry
            // different memory snapshots, and merging them would assign one
            // branch's pre/post state to the other.
            if !sites
                .iter()
                .any(|(f, a, r)| calls_match(f, a, &fun_exp, &args_natural) && r == range)
            {
                sites.push((fun_exp, args_natural, range.clone()));
            }
        }

        let mut to_remove: BTreeSet<usize> = BTreeSet::new();
        let mut to_add: Vec<Exp> = Vec::new();

        for (fun_exp, args_natural, range) in &sites {
            let num_inputs = args_natural.len();
            let fun_type = env.get_node_type(fun_exp.node_id());
            let Type::Fun(arg_ty_box, result_ty, _) = fun_type else {
                continue;
            };
            let arg_types: Vec<Type> = arg_ty_box.flatten();
            let result_type: Type = (*result_ty).clone();
            let declared_result_types: Vec<Type> = result_type.clone().flatten();
            let num_declared_results = declared_result_types.len();
            let is_void = num_declared_results == 0;

            // One binding-LHS per `&mut` slot — otherwise we can't fill
            // the canonical's post-state slots.
            let mut post_state_slots: Vec<Option<Exp>> = Vec::new();
            for (k, t) in arg_types.iter().enumerate() {
                if !t.is_mutable_reference() {
                    continue;
                }
                let mut_ref_idx = arg_types[..k]
                    .iter()
                    .filter(|ty| ty.is_mutable_reference())
                    .count();
                let lhs = bindings.iter().find_map(|(wo_call, lhs)| {
                    use move_model::ast::BehaviorKind;
                    let ExpData::Call(
                        _,
                        AstOp::Behavior(BehaviorKind::WriteOf(j), wo_range),
                        bp_args,
                    ) = wo_call.as_ref()
                    else {
                        return None;
                    };
                    if *j != mut_ref_idx || wo_range != range {
                        return None;
                    }
                    if bp_args.is_empty() {
                        return None;
                    }
                    let wo_fun = &bp_args[0];
                    let wo_args_natural: Vec<Exp> =
                        bp_args[1..].iter().map(strip_all_olds).collect();
                    if !calls_match(fun_exp, args_natural, wo_fun, &wo_args_natural) {
                        return None;
                    }
                    Some(lhs.clone())
                });
                post_state_slots.push(lhs);
            }
            if post_state_slots.iter().any(Option::is_none) {
                continue;
            }
            let post_state_slots: Vec<Exp> =
                post_state_slots.into_iter().map(Option::unwrap).collect();

            // Walk clauses once, collecting both (a) explicit `dest`s from
            // sibling `result_of` clauses and (b) any void state-anchor
            // candidates for this site. Removal is committed only below,
            // *after* we have decided to build a replacement canonical —
            // never leave an anchor removed without a replacement.
            let mut dests_by_idx: BTreeMap<usize, Exp> = BTreeMap::new();
            let mut anchors_for_site: Vec<(usize, Vec<Exp>)> = Vec::new();
            for (idx, clause) in state.ensures.iter().enumerate() {
                if let Some((dest, output_idx, fun2, args2, range2)) =
                    extract_result_of_clause(clause)
                {
                    let args2_natural: Vec<Exp> = args2.iter().map(strip_all_olds).collect();
                    if calls_match(fun_exp, args_natural, &fun2, &args2_natural) && range2 == *range
                    {
                        dests_by_idx.insert(output_idx, dest);
                    }
                } else if let Some((fun2, args2, range2, guard2)) =
                    extract_top_ensures_of_clause(clause)
                {
                    let args2_natural: Vec<Exp> = args2.iter().map(strip_all_olds).collect();
                    // Anchors come in two shapes: void/state anchors carry
                    // the inputs only; discarded-result anchors additionally
                    // carry synthesized `result_of` projections. Both must be
                    // replaced by the full-arity canonical (the evaluator
                    // encoding expects trailing post-state slots), so match
                    // on the input prefix.
                    let shape_ok = args2.len() == num_inputs
                        || args2.len() == num_inputs + num_declared_results;
                    if shape_ok
                        && calls_match(fun_exp, args_natural, &fun2, &args2_natural[..num_inputs])
                        && range2 == *range
                    {
                        anchors_for_site.push((idx, guard2));
                    }
                }
            }

            // Build a canonical only when something anchors this site: a
            // captured `dest`, or an anchor clause — including the
            // discarded-result shape, which cannot be left in place (its
            // arity lacks the post-state slots).
            let has_result_of = !dests_by_idx.is_empty();
            if !is_void && !has_result_of && anchors_for_site.is_empty() {
                continue;
            }

            // Fill every declared result slot — uncaptured ones become
            // synthesized `result_of<f>(...)` projections so the canonical
            // keeps the full declared-result arity.
            let mut dests: Vec<Exp> = Vec::with_capacity(num_declared_results);
            for i in 0..num_declared_results {
                if let Some(d) = dests_by_idx.remove(&i) {
                    dests.push(d);
                } else {
                    dests.push(self.mk_result_of_at_with_state(
                        fun_exp.clone(),
                        args_natural.clone(),
                        &result_type,
                        i,
                        num_declared_results,
                        range.pre,
                        range.post,
                    ));
                }
            }

            let mut canonical_args: Vec<Exp> =
                Vec::with_capacity(1 + args_natural.len() + dests.len() + post_state_slots.len());
            canonical_args.push(fun_exp.clone());
            canonical_args.extend(args_natural.iter().cloned());
            canonical_args.extend(dests);
            canonical_args.extend(post_state_slots);
            let bool_ty = Type::Primitive(PrimitiveType::Bool);
            let new_id = self.new_node(bool_ty.clone(), None);
            let canonical = ExpData::Call(
                new_id,
                AstOp::Behavior(move_model::ast::BehaviorKind::EnsuresOf, range.clone()),
                canonical_args,
            )
            .into_exp();
            // Keep guarded anchors guarded: an unguarded canonical would claim
            // the callee's ensures on paths that never make the call.
            let mut emitted_unguarded = false;
            for (idx, guards) in &anchors_for_site {
                if guards.is_empty() {
                    if !emitted_unguarded {
                        to_add.push(canonical.clone());
                        emitted_unguarded = true;
                    }
                } else {
                    let mut wrapped = canonical.clone();
                    for g in guards.iter().rev() {
                        let imp_id = self.new_node(bool_ty.clone(), None);
                        wrapped = ExpData::Call(imp_id, AstOp::Implies, vec![g.clone(), wrapped])
                            .into_exp();
                    }
                    to_add.push(wrapped);
                }
                to_remove.insert(*idx);
            }
            if anchors_for_site.is_empty() {
                to_add.push(canonical);
            }
        }

        let mut new_ensures: Vec<Exp> = Vec::new();
        for (idx, clause) in std::mem::take(&mut state.ensures).into_iter().enumerate() {
            if !to_remove.contains(&idx) {
                new_ensures.push(clause);
            }
        }
        new_ensures.extend(to_add);
        state.ensures = new_ensures;

        state.ensures = state
            .ensures
            .iter()
            .map(|e| substitute_write_of_with_natural(env, e, &bindings))
            .collect();
        state.aborts = state
            .aborts
            .iter()
            .map(|e| substitute_write_of_with_natural(env, e, &bindings))
            .collect();
        state.direct_modifies = state
            .direct_modifies
            .iter()
            .map(|e| substitute_write_of_with_natural(env, e, &bindings))
            .collect();

        state.ensures.retain(|e| !is_trivially_true(e));
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
                    AstOp::Behavior(kind, range) => {
                        let new_pre = range.pre.and_then(|l| (self.label_map)(l));
                        let new_post = range.post.and_then(|l| (self.label_map)(l));
                        if new_pre.is_some() || new_post.is_some() {
                            let new_range = MemoryRange {
                                pre: new_pre.unwrap_or(range.pre),
                                post: new_post.unwrap_or(range.post),
                            };
                            Some(
                                ExpData::Call(id, AstOp::Behavior(*kind, new_range), args.to_vec())
                                    .into_exp(),
                            )
                        } else {
                            None
                        }
                    },
                    AstOp::SpecFunction(mid, fid, range) => {
                        let new_pre = range.pre.and_then(|l| (self.label_map)(l));
                        let new_post = range.post.and_then(|l| (self.label_map)(l));
                        if new_pre.is_some() || new_post.is_some() {
                            let new_range = MemoryRange {
                                pre: new_pre.unwrap_or(range.pre),
                                post: new_post.unwrap_or(range.post),
                            };
                            Some(
                                ExpData::Call(
                                    id,
                                    AstOp::SpecFunction(*mid, *fid, new_range),
                                    args.to_vec(),
                                )
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
        let body_modifies = state
            .body_modifies
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
            is_normal_return: state.is_normal_return,
            origin_block: state.origin_block,
            post: new_post,
            captured_mut_params: state.captured_mut_params.clone(),
            captured_globals: state.captured_globals.clone(),
            update_globals: state.update_globals.clone(),
            direct_modifies,
            body_modifies,
            aborts_partial: state.aborts_partial,
            solver_hard: state.solver_hard,
        }
    }

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
        // A compiler-generated abort handler can also be the destination of an
        // explicit `abort` path in the source. Calls have their AbortAction
        // stripped below, so preserve a shared handler block whenever ordinary
        // control flow still targets it; otherwise the explicit abort would be
        // erased from the WP analysis together with the call's handler.
        let explicit_targets: BTreeSet<Label> = bytecode
            .iter()
            .flat_map(|bc| match bc {
                Bytecode::Jump(_, label) => vec![*label],
                Bytecode::Branch(_, true_label, false_label, _) => {
                    vec![*true_label, *false_label]
                },
                _ => vec![],
            })
            .collect();
        let handler_only_labels: BTreeSet<Label> = abort_handler_labels
            .difference(&explicit_targets)
            .copied()
            .collect();

        // Phase 2: Build modified bytecode
        let mut abort_handler_label = None;
        bytecode
            .iter()
            .map(|bc| {
                if let Bytecode::Label(_, label) = bc {
                    abort_handler_label = if handler_only_labels.contains(label) {
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

        // Run forward state boundary analysis to pre-assign memory labels
        // at state-changing instructions. This determines the label chain
        // and sets at_end_label.
        self.run_forward_label_analysis(bytecode);

        // Build backward CFG for analysis (backward from exit to entry)
        // Use from_all_blocks=false so DUMMY_EXIT only connects to actual exit blocks (Ret/Abort),
        // not all blocks. This is needed for path-conditional join to work correctly.
        let cfg = StacklessControlFlowGraph::new_backward(bytecode, false);

        // Initial state: post points to the "at_end" label (the final state)
        let initial_state = WPState::new(self.at_end_label.get());

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
            is_normal_return: true,
            origin_block: None,
            post: self.at_end_label.get(),
            captured_mut_params: BTreeSet::new(),
            captured_globals: BTreeSet::new(),
            update_globals: BTreeSet::new(),
            direct_modifies: vec![],
            body_modifies: vec![],
            aborts_partial: false,
            solver_hard: false,
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
        if !state.has_global_mutations() {
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
        if !state.has_global_mutations() {
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
                // Modulo aborts on: b == 0, or for signed: a == MIN && b == -1
                let zero = self.mk_num_const(BigInt::zero());
                let mod_zero = self.mk_eq(b.clone(), zero);

                if ty.is_signed_int() {
                    let min = self.mk_num_min(prim_ty)?;
                    let neg_one = self.mk_num_const(BigInt::from(-1));
                    let a_eq_min = self.mk_eq(a, min);
                    let b_eq_neg1 = self.mk_eq(b, neg_one);
                    let min_mod_neg1 = self.mk_and(a_eq_min, b_eq_neg1);
                    Some(self.mk_or(mod_zero, min_mod_neg1))
                } else {
                    Some(mod_zero)
                }
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

    /// Convert any stackless bytecode constant to the corresponding model value.
    /// Keeping the destination's concrete type is important for addresses and
    /// vectors; dropping those loads leaves internal temporaries in inferred specs.
    fn constant_to_exp(&self, constant: &Constant, ty: Type) -> Exp {
        let value = constant.to_model_value();
        let node_id = self.new_node(ty, None);
        ExpData::Value(node_id, value).into_exp()
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
                true_target_block: cfg.enclosing_block(true_offset),
                false_target_block: cfg.enclosing_block(false_offset),
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
        incoming_pred_block: Option<BlockId>,
    ) -> JoinResult {
        // Unify post labels: when one branch modified state (intermediate label)
        // and the other didn't (at_end_label), prefer the intermediate label.
        // This ensures operations before the branch correctly reference the
        // pre-modification state (e.g., `exists<R>(addr)` captured into a local
        // before a conditional `move_from` references the pre-move state).
        let incoming_post = incoming.post;
        let current_post = current.post;

        let unified_post =
            if incoming_post != current_post && current_post == self.at_end_label.get() {
                // Current has the default label; incoming modified state.
                // Adopt incoming's intermediate label.
                incoming_post
            } else {
                current_post
            };

        // Rewrite current state's labels if we're adopting the incoming's post.
        if current_post != unified_post {
            *current = self.substitute_labels_in_state(current, &|label| {
                if label == current_post {
                    Some(Some(unified_post))
                } else {
                    None
                }
            });
            current.post = unified_post;
        }

        // Rewrite incoming state's labels to match the unified post.
        let incoming = &self.substitute_labels_in_state(incoming, &|label| {
            if label == incoming_post && incoming_post != unified_post {
                Some(Some(unified_post))
            } else {
                None
            }
        });

        // If no branch info, fall back to standard join
        let Some(branch) = branch_info else {
            return current.join(incoming);
        };

        // Classify a predecessor block ID as the true or false branch target.
        // Compare predecessor block IDs against the branch target block IDs.
        let classify_block = |block: BlockId| -> Option<bool> {
            if block == branch.true_target_block {
                Some(true)
            } else if block == branch.false_target_block {
                Some(false)
            } else {
                None
            }
        };

        // Determine which branch side each state came from.
        // `current.origin_block` was set when the first state arrived at this block.
        // `incoming_pred_block` is the predecessor block ID for the second state.
        let current_is_true = current.origin_block.and_then(&classify_block);
        let incoming_is_true = incoming_pred_block.and_then(&classify_block);

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

        // Union direct_modifies (modification from ANY path counts)
        let old_direct_modifies_len = current.direct_modifies.len();
        for exp in &incoming.direct_modifies {
            push_if_new(&mut current.direct_modifies, exp.clone());
        }
        let modifies_changed = current.direct_modifies.len() != old_direct_modifies_len;
        let old_body_modifies_len = current.body_modifies.len();
        for exp in &incoming.body_modifies {
            push_if_new(&mut current.body_modifies, exp.clone());
        }
        let body_modifies_changed = current.body_modifies.len() != old_body_modifies_len;

        // Propagate normal-return status: if either side is a normal return, the result is too
        if incoming.is_normal_return && !current.is_normal_return {
            current.is_normal_return = true;
        }

        // Incompleteness of either branch is incompleteness of the join, as in
        // `AbstractDomain::join`. Dropping these here would let a branch whose
        // callee has unknown aborts merge into an exact `aborts_if false`.
        current.aborts_partial |= incoming.aborts_partial;
        current.solver_hard |= incoming.solver_hard;

        // Clear origin after merge since we've combined paths
        current.clear_origin();

        if ensures_changed
            || aborts_changed
            || captured_changed
            || captured_globals_changed
            || modifies_changed
            || body_modifies_changed
        {
            JoinResult::Changed
        } else {
            JoinResult::Unchanged
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nested_can_modify() {
        let env = GlobalEnv::new();
        let can_modify_id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        let can_modify = ExpData::Call(can_modify_id, AstOp::CanModify, vec![]).into_exp();
        let not_id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        let nested = ExpData::Call(not_id, AstOp::Not, vec![can_modify]).into_exp();

        assert!(contains_can_modify(&nested));
    }

    #[test]
    fn ordinary_expression_does_not_contain_can_modify() {
        let env = GlobalEnv::new();
        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        let value = ExpData::Value(id, Value::Bool(true)).into_exp();

        assert!(!contains_can_modify(&value));
    }

    #[test]
    fn recognizes_only_valid_constant_utf8_vectors() {
        let env = GlobalEnv::new();
        let vector = |values: &[u8]| {
            let elements = values
                .iter()
                .map(|value| {
                    let id = env.new_node(Loc::default(), Type::Primitive(PrimitiveType::U8));
                    ExpData::Value(id, Value::Number(BigInt::from(*value))).into_exp()
                })
                .collect();
            let id = env.new_node(
                Loc::default(),
                Type::Vector(Box::new(Type::Primitive(PrimitiveType::U8))),
            );
            ExpData::Call(id, AstOp::Vector, elements).into_exp()
        };

        assert!(constant_valid_utf8(&vector(b"Aptos Coin")));
        assert!(constant_valid_utf8(&vector(&[])));
        assert!(!constant_valid_utf8(&vector(&[0xFF])));
        let non_constant = ExpData::Temporary(
            env.new_node(
                Loc::default(),
                Type::Vector(Box::new(Type::Primitive(PrimitiveType::U8))),
            ),
            0,
        )
        .into_exp();
        assert!(!constant_valid_utf8(&non_constant));
    }
}
