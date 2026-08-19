// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Source-level weakest-precondition analysis over AST expressions.
//!
//! Derives a specification (aborts, ensures, exact result values) for an
//! expression body by forward symbolic execution, mirroring the semantics of
//! the bytecode-level spec inference in the prover
//! (`move-prover/bytecode-pipeline/src/spec_inference.rs`): the same abort
//! side conditions for primitive operations, the same modular call summaries
//! via behavioral predicates, and the same output conventions.
//!
//! The main use is deriving specs for lambdas without attached spec blocks
//! when behavioral predicates over them are inlined at inline-function
//! expansion sites. Derived conditions therefore follow the lambda-spec
//! conventions consumed there: parameters are referenced as `LocalVar(sym)`,
//! the pre-state of a `&mut` parameter as `old(LocalVar(sym))` and its
//! post-state as plain `LocalVar(sym)`, results as `Operation::Result(i)`,
//! and free variables of the enclosing scope are left untouched.
//!
//! The analysis is exact: it returns `None` whenever the body is outside the
//! fragment it can describe precisely (loops, escaping lambda values, writes
//! through references of the enclosing scope, and constructs listed in the
//! evaluator). It never over- or under-approximates.

use crate::{
    ast::{
        Exp, ExpData, MatchArm, MemoryLabel, MemoryRange, Operation, Pattern, QuantKind, Value,
        VisitorPosition,
    },
    exp_generator::{ExpGenerator, RangeCheckKind},
    exp_rewriter::{strip_all_olds, ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    exp_simplifier::ExpSimplifier,
    memory_labels::{self, MemoryLabelInfo},
    model::{
        FunId, GlobalEnv, ModuleId, NodeId, Parameter, QualifiedId, QualifiedInstId, SpecFunId,
        StructId,
    },
    pureness_checker::{FunctionPurenessChecker, FunctionPurenessCheckerMode},
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type, BOOL_TYPE},
    well_known::{self, IntrinsicWp},
};
use move_core_types::function::ClosureMask;
use num::BigInt;
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap, BTreeSet},
    rc::Rc,
};

/// A specification derived from an expression body.
#[derive(Debug, Default)]
pub struct DerivedSpec {
    /// Conjunctive precondition obligations. Currently always empty; the
    /// field is part of the interface for future use (e.g. propagating
    /// callee `requires_of` obligations).
    pub requires: Vec<Exp>,
    /// Disjunctive abort conditions, phrased over the pre-state (all
    /// `old(..)` wrappers stripped).
    pub aborts: Vec<Exp>,
    /// Conjunctive conditions holding at normal return.
    pub ensures: Vec<Exp>,
    /// The exact symbolic result values, one per declared result; `None`
    /// if the body has no normally returning path.
    pub results: Option<Vec<Exp>>,
    /// Modified global memory, as unlabeled `global<R>(addr)` target
    /// expressions with addresses phrased over the entry state (pre-state
    /// reads `old(..)`-wrapped); a path-insensitive union over all
    /// execution paths. `None` if a modified cell's address cannot be
    /// expressed in entry-state terms (it depends on an intermediate
    /// memory state or an unresolved intermediate value).
    pub modifies: Option<Vec<Exp>>,
    /// The exact final value of each `&mut` parameter and capture at
    /// normal return (the per-return values folded into a conditional over
    /// the return guards), in parameter order, phrased like the `ensures`
    /// conventions (pre-state as `old(..)`). `None` if the body has no
    /// normally returning path or a final value cannot be expressed in
    /// entry-state terms (it depends on an unresolved intermediate value);
    /// the corresponding `ensures` conditions remain exact in that case
    /// via existential closure.
    pub mut_param_values: Option<Vec<(Symbol, Exp)>>,
    /// The deferred forwarded applications (see the
    /// `deferred_fun_param_temps` parameter of
    /// [`derive_spec_with_captures`]): per application of a deferred
    /// function-typed parameter, the parameter expression (in the scope of
    /// the enclosing function, left untouched), the exact argument values
    /// (phrased like the `ensures` conventions), and the path condition
    /// guarding the application (`None` when unconditional). Consumers
    /// which reason about the analyzed body's memory footprint
    /// (`DerivedSpec::modifies`) must account for these applications
    /// separately — the footprint excludes their (deferred) effects.
    pub deferred_applications: Vec<(Exp, Vec<Exp>, Option<Exp>)>,
}

/// Derives a specification for `body`. `params` are the analyzed
/// expression's parameters in order, all named (the caller invents fresh
/// symbols for wildcards); `var_types` is the typing environment covering
/// the parameters and free local variables. Returns `None` if the body is
/// outside the derivable fragment.
pub fn derive_spec<'env, G: ExpGenerator<'env>>(
    builder: &mut G,
    params: &[(Symbol, Type)],
    var_types: &BTreeMap<Symbol, Type>,
    result_type: &Type,
    body: &Exp,
) -> Option<DerivedSpec> {
    derive_spec_with_captures(
        builder,
        params,
        &[],
        var_types,
        result_type,
        body,
        &BTreeMap::new(),
    )
}

/// Like [`derive_spec`], but additionally treats `captures` — mutated free
/// variables of the enclosing scope, discovered e.g. by
/// [`collect_mutated_free_vars`] — as implicit `&mut` parameters: a
/// by-value capture `c` starts at its pre-state `old(c)`, reads and writes
/// follow value semantics, and its final value is exposed as the
/// post-state condition `c == <value>` (and in
/// [`DerivedSpec::mut_param_values`]); a capture of `&mut` reference type
/// enters as a literal `&mut` parameter. `var_types` must cover the
/// captures. With empty `captures` and `deferred_fun_param_temps` this is
/// exactly [`derive_spec`].
///
/// `deferred_fun_param_temps` maps temporary indices of function-typed
/// parameters of the *enclosing* function to their symbols (the body may
/// reference a parameter in either form); applications of these in the
/// body are deferred: their behavioral summaries are emitted label-free (as if
/// memory-free) and recorded in [`DerivedSpec::deferred_applications`].
/// This is the transitivity seam for forwarding lambdas: the summaries'
/// behavioral predicates target the enclosing function's parameter and are
/// re-resolved — with the state policies of that context — when the
/// enclosing function is itself expanded, or translate directly for a
/// genuine function-value parameter (whose backend encoding is equally
/// state-independent). Named callees are unaffected: their state effects
/// keep introducing labels.
pub fn derive_spec_with_captures<'env, G: ExpGenerator<'env>>(
    builder: &mut G,
    params: &[(Symbol, Type)],
    captures: &[(Symbol, Type)],
    var_types: &BTreeMap<Symbol, Type>,
    result_type: &Type,
    body: &Exp,
    deferred_fun_param_temps: &BTreeMap<usize, Symbol>,
) -> Option<DerivedSpec> {
    let mut deriver = make_deriver(builder, params, captures, var_types);
    deriver.deferred_fun_param_temps = deferred_fun_param_temps.clone();
    let terminal = deriver.eval(body).ok()?;
    if let Some(val) = terminal {
        deriver.record_return(val).ok()?;
    }
    deriver.assemble(result_type)
}

/// Constructs a fresh deriver over the given parameters and captures, with
/// the store initialized per the conventions of
/// [`derive_spec_with_captures`].
fn make_deriver<'a, 'env, G: ExpGenerator<'env>>(
    builder: &'a mut G,
    params: &[(Symbol, Type)],
    captures: &[(Symbol, Type)],
    var_types: &BTreeMap<Symbol, Type>,
) -> Deriver<'a, G> {
    let env = builder.global_env();
    let entry_label = MemoryLabel::new(env.new_global_id().as_usize());
    let mut param_infos = vec![];
    let mut store = BTreeMap::new();
    for (sym, ty) in params {
        let kind = if ty.is_mutable_reference() {
            ParamKind::MutRef
        } else {
            ParamKind::Value
        };
        param_infos.push(ParamInfo { sym: *sym, kind });
        let init = if kind == ParamKind::MutRef {
            // The current value of a `&mut` parameter starts at its
            // pre-state; writes replace it and the final value becomes
            // the post-state condition `p == <value>`.
            let var = builder.mk_local_by_sym(*sym, ty.skip_reference().clone());
            SymVal::Value(builder.mk_old(var))
        } else {
            SymVal::Value(builder.mk_local_by_sym(*sym, ty.clone()))
        };
        store.insert(*sym, init);
    }
    for (sym, ty) in captures {
        let kind = if ty.is_mutable_reference() {
            ParamKind::MutRef
        } else {
            ParamKind::ValueCapture
        };
        param_infos.push(ParamInfo { sym: *sym, kind });
        // Either way the capture's current value starts at its pre-state.
        let var = builder.mk_local_by_sym(*sym, ty.skip_reference().clone());
        store.insert(*sym, SymVal::Value(builder.mk_old(var)));
    }
    Deriver {
        builder,
        params: param_infos,
        var_types: var_types.clone(),
        path: vec![],
        frame: Frame {
            store,
            diverged: false,
            label: LabelState::Concrete(entry_label),
        },
        aborts: vec![],
        effects: vec![],
        modifies: vec![],
        modifies_exact: true,
        returns: vec![],
        call_records: vec![],
        entry_label,
        fresh_counter: 0,
        depth: 0,
        deferred_fun_param_temps: BTreeMap::new(),
    }
}

/// Collects the free variables of `body` — relative to `bound` and the
/// binders within the body — that the body possibly mutates: variables
/// assigned by `Assign`, roots of `Mutate` targets, roots of `&mut`
/// borrows (including `&mut c` passed to a callee), and `&mut`-typed free
/// variables passed directly to a `&mut` parameter of a callee or of an
/// applied function value (writes through captured references).
///
/// This is a syntactic pre-pass for capture discovery, feeding
/// [`derive_spec_with_captures`]. The set can miss mutations the analysis
/// cannot name — roots at `Temporary` (parameters of the enclosing
/// function) or mutations reachable only through local aliases. This is
/// safe: the derivation itself fails on any mutation of a variable outside
/// its store (see the bail sites in `bind_pattern_rec` and `eval_borrow`,
/// and the place checks for `Mutate` and `&mut` call arguments) rather
/// than dropping the effect.
pub fn collect_mutated_free_vars(
    env: &GlobalEnv,
    body: &Exp,
    bound: &BTreeSet<Symbol>,
) -> BTreeSet<Symbol> {
    collect_mutated_free_vars_and_temps(env, body, bound).0
}

/// Like [`collect_mutated_free_vars`], also returns mutated temporary roots.
pub fn collect_mutated_free_vars_and_temps(
    env: &GlobalEnv,
    body: &Exp,
    bound: &BTreeSet<Symbol>,
) -> (BTreeSet<Symbol>, BTreeSet<usize>) {
    // Shadowing depth per symbol, seeded with the initially bound symbols.
    let mut shadow: BTreeMap<Symbol, usize> = bound.iter().map(|sym| (*sym, 1)).collect();
    let mut locals = BTreeSet::new();
    let mut temps = BTreeSet::new();
    enum Root {
        Local(Symbol),
        Temporary(usize),
    }
    fn adjust_pat(pat: &Pattern, entering: bool, shadow: &mut BTreeMap<Symbol, usize>) {
        for (_, sym) in pat.vars() {
            let counter = shadow.entry(sym).or_insert(0);
            if entering {
                *counter += 1;
            } else {
                *counter = counter.saturating_sub(1);
            }
        }
    }
    fn is_free(sym: Symbol, shadow: &BTreeMap<Symbol, usize>) -> bool {
        shadow.get(&sym).copied().unwrap_or(0) == 0
    }
    // The root variable of a place-denoting expression, if free.
    fn free_root(exp: &Exp, shadow: &BTreeMap<Symbol, usize>) -> Option<Root> {
        match exp.as_ref() {
            ExpData::LocalVar(_, sym) if is_free(*sym, shadow) => Some(Root::Local(*sym)),
            ExpData::Temporary(_, idx) => Some(Root::Temporary(*idx)),
            ExpData::Call(
                _,
                Operation::Select(..)
                | Operation::SelectVariants(..)
                | Operation::Deref
                | Operation::Borrow(_),
                args,
            ) => free_root(&args[0], shadow),
            _ => None,
        }
    }
    let mut visitor = |pos: VisitorPosition, e: &ExpData| {
        use ExpData::*;
        use VisitorPosition::*;
        match (e, pos) {
            (Lambda(_, pat, ..), Pre) | (Block(_, pat, _, _), BeforeBody) => {
                adjust_pat(pat, true, &mut shadow);
            },
            (Lambda(_, pat, ..), Post) | (Block(_, pat, _, _), Post) => {
                adjust_pat(pat, false, &mut shadow);
            },
            (Match(_, _, arms), BeforeMatchBody(idx)) => {
                adjust_pat(&arms[idx].pattern, true, &mut shadow);
            },
            (Match(_, _, arms), AfterMatchBody(idx)) => {
                adjust_pat(&arms[idx].pattern, false, &mut shadow);
            },
            (Quant(_, _, ranges, ..), Pre) => {
                for (pat, _) in ranges {
                    adjust_pat(pat, true, &mut shadow);
                }
            },
            (Quant(_, _, ranges, ..), Post) => {
                for (pat, _) in ranges {
                    adjust_pat(pat, false, &mut shadow);
                }
            },
            (Assign(_, pat, _), Pre) => {
                for (_, sym) in pat.vars() {
                    if is_free(sym, &shadow) {
                        locals.insert(sym);
                    }
                }
            },
            (Mutate(_, lhs, _), Pre) => match free_root(lhs, &shadow) {
                Some(Root::Local(sym)) => {
                    locals.insert(sym);
                },
                Some(Root::Temporary(idx)) => {
                    temps.insert(idx);
                },
                None => {},
            },
            (Call(_, Operation::Borrow(ReferenceKind::Mutable), args), Pre) => {
                match free_root(&args[0], &shadow) {
                    Some(Root::Local(sym)) => {
                        locals.insert(sym);
                    },
                    Some(Root::Temporary(idx)) => {
                        temps.insert(idx);
                    },
                    None => {},
                }
            },
            (Call(_, Operation::MoveFunction(mid, fid), args), Pre) => {
                // A `&mut`-typed free variable passed directly to a `&mut`
                // parameter is written through the callee.
                let callee = env.get_function(mid.qualified(*fid));
                for (param, arg) in callee.get_parameters().iter().zip(args) {
                    if param.1.is_mutable_reference() {
                        match free_root(arg, &shadow) {
                            Some(Root::Local(sym)) => {
                                locals.insert(sym);
                            },
                            Some(Root::Temporary(idx)) => {
                                temps.insert(idx);
                            },
                            None => {},
                        }
                    }
                }
            },
            (Invoke(_, target, args), Pre) => {
                // Likewise for applications of function values.
                if let Type::Fun(param_ty, ..) = env.get_node_type(target.as_ref().node_id()) {
                    for (ty, arg) in param_ty.flatten().iter().zip(args) {
                        if ty.is_mutable_reference() {
                            match free_root(arg, &shadow) {
                                Some(Root::Local(sym)) => {
                                    locals.insert(sym);
                                },
                                Some(Root::Temporary(idx)) => {
                                    temps.insert(idx);
                                },
                                None => {},
                            }
                        }
                    }
                }
            },
            _ => {},
        }
        true
    };
    body.visit_positions(&mut visitor);
    (locals, temps)
}

/// Classifies whether the given expressions — e.g. a derived
/// specification's final capture values and abort conditions — are pure
/// and single-state: free of global memory reads (`global`/`exists`,
/// labeled or not), of memory effect operations, and of behavioral
/// predicates or spec function calls bound to specific memory states
/// (non-default ranges) or reading memory. `old(..)` wrappers are
/// permitted: over the expressions this classifies they refer to
/// parameter or capture pre-values, not to memory.
pub fn exps_are_pure_single_state<'a>(
    env: &GlobalEnv,
    exps: impl IntoIterator<Item = &'a Exp>,
) -> bool {
    exps_are_pure_single_state_impl(env, exps, false)
}

/// The stricter single-state classification required for material evaluated
/// once per `folds_of` iteration. Behavioral predicates are rejected even
/// with a default memory range: their evaluator can receive the target
/// function's current memory, whose per-iteration state is not expressible
/// by the fold recursion.
pub fn exps_are_pure_single_state_for_folds<'a>(
    env: &GlobalEnv,
    exps: impl IntoIterator<Item = &'a Exp>,
) -> bool {
    exps_are_pure_single_state_impl(env, exps, true)
}

fn exps_are_pure_single_state_impl<'a>(
    env: &GlobalEnv,
    exps: impl IntoIterator<Item = &'a Exp>,
    reject_behavior: bool,
) -> bool {
    let mut visited_spec_funs = BTreeSet::new();
    for exp in exps {
        if !exp_is_pure_single_state_impl(env, exp, reject_behavior, &mut visited_spec_funs) {
            return false;
        }
    }
    true
}

fn exp_is_pure_single_state_impl(
    env: &GlobalEnv,
    exp: &Exp,
    reject_behavior: bool,
    visited_spec_funs: &mut BTreeSet<QualifiedId<SpecFunId>>,
) -> bool {
    let mut pure = true;
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, oper, args) = e {
            match oper {
                Operation::Global(_)
                | Operation::Exists(_)
                | Operation::SpecPublish(_)
                | Operation::SpecRemove(_)
                | Operation::SpecUpdate(_) => pure = false,
                Operation::Behavior(_, range) => {
                    let reads_memory = match args.first().map(|arg| arg.as_ref()) {
                        Some(ExpData::Call(_, Operation::Closure(mid, fid, _), _)) => {
                            !fun_has_no_memory_effects(env, mid.qualified(*fid))
                        },
                        // An unresolved function value can be stateful; the
                        // fold's per-iteration evaluation state cannot be
                        // represented safely.
                        _ => true,
                    };
                    if !range.is_default() || (reject_behavior && reads_memory) {
                        pure = false;
                    }
                },
                Operation::SpecFunction(mid, fid, range) => {
                    let qid = mid.qualified(*fid);
                    let decl = env.get_spec_fun(qid);
                    if !range.is_default() || !decl.used_memory.is_empty() {
                        pure = false;
                    } else if reject_behavior && visited_spec_funs.insert(qid) {
                        // `used_memory` does not account for behavioral
                        // predicates. For fold material, inspect reachable
                        // spec-function bodies so a default-range behavior
                        // cannot be hidden behind a memory-empty wrapper.
                        if let Some(body) = &decl.body {
                            pure =
                                exp_is_pure_single_state_impl(env, body, true, visited_spec_funs);
                        }
                    }
                },
                _ => {},
            }
        }
        pure
    });
    pure
}

/// Whether calling the given function provably has *no* global-memory
/// effects: it neither reads nor writes global state on any execution path
/// (memory-pure, possibly abort-only), and its attached specification is
/// equally memory-free. For such a callee a behavioral call summary is
/// single-state: memory cannot change across the call, so the summarizer
/// keeps its exact modifies footprint and introduces no state labels.
///
/// The analysis is a conservative transitive scan of the function bodies:
/// any construct it cannot account for — natives outside a small
/// memory-free whitelist, applications of function values that may enter
/// from outside the scanned code, bodiless functions — makes it answer
/// `false`. Function values are accounted for as follows: the root must
/// not take function-typed parameters; lambda literals are scanned
/// structurally; closure targets are scanned as if called; and inside an
/// *inline* function, applying its own function-typed parameter is
/// permitted provided every scanned call site binds such parameters to a
/// lambda literal, a closure, or a forwarded inline function parameter —
/// so every function value an allowed application can take originates in
/// scanned material.
pub fn fun_has_no_memory_effects(env: &GlobalEnv, fun_id: QualifiedId<FunId>) -> bool {
    let fun = env.get_function(fun_id);
    // Function values entering through function-typed parameters have
    // unknown effects.
    if fun
        .get_parameters()
        .iter()
        .any(|p| matches!(p.1.skip_reference(), Type::Fun(..)))
    {
        return false;
    }
    // The summary's behavioral predicates resolve against the callee's
    // attached specification, so its conditions must be memory-free too.
    {
        let spec = fun.get_spec();
        if spec
            .frame_spec
            .as_ref()
            .is_some_and(|fs| fs.modifies_all || !fs.modifies_targets.is_empty())
            || !spec.update_map.is_empty()
        {
            return false;
        }
        let mut spec_fun_visited = BTreeSet::new();
        for cond in &spec.conditions {
            for exp in std::iter::once(&cond.exp).chain(&cond.additional_exps) {
                // Exact frame conjuncts `X == old(X)` (syntactically
                // identical two-state reads, e.g.
                // `supply<CoinType> == old(supply<CoinType>)` on
                // `coin::merge`) assert the *absence* of a memory effect;
                // they are allowed in an otherwise memory-free spec. Any
                // other memory mention disqualifies.
                for conjunct in exp_conjuncts(exp) {
                    if is_exact_frame_conjunct(&conjunct) {
                        continue;
                    }
                    if !exp_is_memory_free(
                        env,
                        &conjunct,
                        &[],
                        false,
                        &mut BTreeSet::new(),
                        &mut spec_fun_visited,
                    ) {
                        return false;
                    }
                }
            }
        }
    }
    // An omitted opaque frame is conservatively implemented from the body's
    // memory effects, so opacity alone cannot establish memory freedom.
    fun_body_is_memory_free(env, fun_id, &mut BTreeSet::new(), &mut BTreeSet::new())
}

/// Whether an expression and its callees are free of global-memory effects.
pub fn exp_has_no_memory_effects(env: &GlobalEnv, exp: &Exp) -> bool {
    exp_is_memory_free(
        env,
        exp,
        &[],
        false,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
    )
}

/// The conjuncts of an `And` tree, in order; a non-conjunction is its own
/// single conjunct.
fn exp_conjuncts(exp: &Exp) -> Vec<Exp> {
    fn collect(exp: &Exp, out: &mut Vec<Exp>) {
        if let ExpData::Call(_, Operation::And, args) = exp.as_ref() {
            for arg in args {
                collect(arg, out);
            }
        } else {
            out.push(exp.clone());
        }
    }
    let mut out = vec![];
    collect(exp, &mut out);
    out
}

/// Whether the expression is an exact frame conjunct `X == old(X)` (or
/// `old(X) == X`): both sides structurally identical modulo the single
/// top-level `old(..)` wrapper. Such a conjunct states that the enclosing
/// call leaves `X` unchanged; the reads it contains do not constitute a
/// memory effect or state dependency of the call's summary.
fn is_exact_frame_conjunct(exp: &Exp) -> bool {
    let ExpData::Call(_, Operation::Eq, args) = exp.as_ref() else {
        return false;
    };
    if args.len() != 2 {
        return false;
    }
    fn old_operand(e: &Exp) -> Option<&Exp> {
        match e.as_ref() {
            ExpData::Call(_, Operation::Old, xs) => xs.first(),
            _ => None,
        }
    }
    match (old_operand(&args[0]), old_operand(&args[1])) {
        (None, Some(x)) => args[0].as_ref().structural_eq(x),
        (Some(x), None) => x.as_ref().structural_eq(&args[1]),
        _ => false,
    }
}

/// The transitive body part of [`fun_has_no_memory_effects`]. Functions in
/// `visited` (including the in-progress ones, making recursion cycles
/// harmless) are not scanned again.
fn fun_body_is_memory_free(
    env: &GlobalEnv,
    fun_id: QualifiedId<FunId>,
    visited: &mut BTreeSet<QualifiedId<FunId>>,
    spec_fun_visited: &mut BTreeSet<QualifiedId<SpecFunId>>,
) -> bool {
    if !visited.insert(fun_id) {
        return true;
    }
    let fun = env.get_function(fun_id);
    if fun.is_native() {
        return well_known::is_memory_free_native(&fun);
    }
    let Some(def) = fun.get_def() else {
        return false;
    };
    let params = fun.get_parameters();
    exp_is_memory_free(
        env,
        def,
        &params,
        fun.is_inline(),
        visited,
        spec_fun_visited,
    )
}

/// Whether `exp` — a function body or a specification condition — is free
/// of global-memory operations, scanning callees transitively. See
/// [`fun_has_no_memory_effects`] for the treatment of function values;
/// `enclosing_params`/`enclosing_is_inline` describe the function owning
/// the expression. An inline function's body references its parameters as
/// `LocalVar` by symbol, so the own-function-parameter checks track
/// shadowing binders.
fn exp_is_memory_free(
    env: &GlobalEnv,
    exp: &Exp,
    enclosing_params: &[Parameter],
    enclosing_is_inline: bool,
    visited: &mut BTreeSet<QualifiedId<FunId>>,
    spec_fun_visited: &mut BTreeSet<QualifiedId<SpecFunId>>,
) -> bool {
    // Shadowing depth per symbol of the enclosing function's
    // function-typed parameters.
    let fun_param_syms: BTreeSet<Symbol> = if enclosing_is_inline {
        enclosing_params
            .iter()
            .filter(|p| matches!(p.1.skip_reference(), Type::Fun(..)))
            .map(|p| p.0)
            .collect()
    } else {
        BTreeSet::new()
    };
    let mut shadow: BTreeMap<Symbol, usize> = BTreeMap::new();
    fn adjust_pat(
        pat: &Pattern,
        entering: bool,
        tracked: &BTreeSet<Symbol>,
        shadow: &mut BTreeMap<Symbol, usize>,
    ) {
        for (_, sym) in pat.vars() {
            if tracked.contains(&sym) {
                let counter = shadow.entry(sym).or_insert(0);
                if entering {
                    *counter += 1;
                } else {
                    *counter = counter.saturating_sub(1);
                }
            }
        }
    }
    let mut ok = true;
    let mut visitor = |pos: VisitorPosition, e: &ExpData| {
        use VisitorPosition::*;
        // Maintain the shadowing state for binders.
        match (e, &pos) {
            (ExpData::Lambda(_, pat, ..), Pre) | (ExpData::Block(_, pat, _, _), BeforeBody) => {
                adjust_pat(pat, true, &fun_param_syms, &mut shadow);
            },
            (ExpData::Lambda(_, pat, ..), Post) | (ExpData::Block(_, pat, _, _), Post) => {
                adjust_pat(pat, false, &fun_param_syms, &mut shadow);
            },
            (ExpData::Match(_, _, arms), BeforeMatchBody(idx)) => {
                adjust_pat(&arms[*idx].pattern, true, &fun_param_syms, &mut shadow);
            },
            (ExpData::Match(_, _, arms), AfterMatchBody(idx)) => {
                adjust_pat(&arms[*idx].pattern, false, &fun_param_syms, &mut shadow);
            },
            (ExpData::Quant(_, _, ranges, ..), Pre) => {
                for (pat, _) in ranges {
                    adjust_pat(pat, true, &fun_param_syms, &mut shadow);
                }
            },
            (ExpData::Quant(_, _, ranges, ..), Post) => {
                for (pat, _) in ranges {
                    adjust_pat(pat, false, &fun_param_syms, &mut shadow);
                }
            },
            _ => {},
        }
        if !matches!(pos, Pre) {
            return ok;
        }
        // Whether the expression denotes the enclosing inline function's
        // own (unshadowed) function-typed parameter.
        let is_own_fun_param = |target: &ExpData| -> bool {
            match target {
                ExpData::LocalVar(_, sym) => {
                    fun_param_syms.contains(sym) && shadow.get(sym).copied().unwrap_or(0) == 0
                },
                ExpData::Temporary(_, idx) => {
                    enclosing_is_inline
                        && enclosing_params
                            .get(*idx)
                            .is_some_and(|p| matches!(p.1.skip_reference(), Type::Fun(..)))
                },
                _ => false,
            }
        };
        match e {
            ExpData::Call(_, oper, args) => match oper {
                Operation::Global(_)
                | Operation::Exists(_)
                | Operation::BorrowGlobal(_)
                | Operation::MoveTo
                | Operation::MoveFrom
                | Operation::SpecPublish(_)
                | Operation::SpecRemove(_)
                | Operation::SpecUpdate(_) => ok = false,
                Operation::MoveFunction(mid, fid) => {
                    // Function-typed arguments must originate in scanned
                    // material (see `fun_has_no_memory_effects`).
                    let callee_params = env.get_function(mid.qualified(*fid)).get_parameters();
                    for (param, arg) in callee_params.iter().zip(args) {
                        if !matches!(param.1.skip_reference(), Type::Fun(..)) {
                            continue;
                        }
                        let arg_ok = match arg.as_ref() {
                            // The lambda body is part of this scan.
                            ExpData::Lambda(..) => true,
                            // The closure target is scanned by the
                            // `Closure` arm below.
                            ExpData::Call(_, Operation::Closure(..), _) => true,
                            // Forwarding an inline function's own function
                            // parameter: inductively bound to scanned
                            // material.
                            other => is_own_fun_param(other),
                        };
                        if !arg_ok {
                            ok = false;
                        }
                    }
                    if ok
                        && !fun_body_is_memory_free(
                            env,
                            mid.qualified(*fid),
                            visited,
                            spec_fun_visited,
                        )
                    {
                        ok = false;
                    }
                },
                Operation::Closure(mid, fid, _) => {
                    // A closure created here may be applied anywhere
                    // downstream; account for its target as if called.
                    if !fun_body_is_memory_free(env, mid.qualified(*fid), visited, spec_fun_visited)
                    {
                        ok = false;
                    }
                },
                Operation::SpecFunction(mid, fid, range) => {
                    // Specification function calls (in spec blocks or
                    // conditions): scan the body. A bodiless (native or
                    // uninterpreted) spec function with empty used memory
                    // is a fixed function of its arguments — the backend
                    // axiomatizes it without memory parameters — and hence
                    // state-independent (consistent with the treatment in
                    // `exps_are_pure_single_state`); with used memory it is
                    // state-dependent.
                    if !range.is_default() {
                        ok = false;
                    } else if spec_fun_visited.insert(mid.qualified(*fid)) {
                        let decl = env.get_spec_fun(mid.qualified(*fid));
                        match decl.body.clone() {
                            Some(body) => {
                                if !exp_is_memory_free(
                                    env,
                                    &body,
                                    &[],
                                    false,
                                    visited,
                                    spec_fun_visited,
                                ) {
                                    ok = false;
                                }
                            },
                            None => {
                                if !decl.used_memory.is_empty() {
                                    ok = false;
                                }
                            },
                        }
                    }
                },
                Operation::Behavior(_, range) => {
                    if !range.is_default() {
                        ok = false;
                    }
                },
                _ => {},
            },
            ExpData::Invoke(_, target, _) => {
                // Applying a function value: only an inline function's own
                // function-typed parameter is accounted for.
                if !is_own_fun_param(target.as_ref()) {
                    ok = false;
                }
            },
            ExpData::Assign(_, pat, _) => {
                // Reassigning a tracked function parameter would break the
                // origin argument for its applications.
                if pat.vars().iter().any(|(_, sym)| {
                    fun_param_syms.contains(sym) && shadow.get(sym).copied().unwrap_or(0) == 0
                }) {
                    ok = false;
                }
            },
            _ => {},
        }
        ok
    };
    exp.visit_positions(&mut visitor);
    ok
}

fn has_functional_result_spec(env: &GlobalEnv, id: QualifiedId<FunId>) -> bool {
    use crate::ast::ConditionKind;

    let fun = env.get_function(id);
    let result_count = fun.get_result_type().flatten().len();
    if result_count == 0 {
        return true;
    }
    let mut_params = fun
        .get_parameters()
        .iter()
        .enumerate()
        .filter(|(_, param)| param.1.is_mutable_reference())
        .map(|(idx, _)| idx)
        .collect();
    let concrete_prop = env
        .symbol_pool()
        .make(crate::pragmas::CONDITION_CONCRETE_PROP);
    let inferred_prop = env
        .symbol_pool()
        .make(crate::pragmas::CONDITION_INFERRED_PROP);
    let determined: BTreeSet<_> = fun
        .get_spec()
        .conditions
        .iter()
        .filter(|cond| cond.kind == ConditionKind::Ensures)
        .filter(|cond| {
            !cond.properties.contains_key(&concrete_prop)
                && !cond.properties.contains_key(&inferred_prop)
        })
        .flat_map(|cond| exp_conjuncts(&cond.exp))
        .filter_map(|conjunct| {
            let ExpData::Call(_, Operation::Eq, args) = conjunct.as_ref() else {
                return None;
            };
            (args.len() == 2)
                .then_some([(&args[0], &args[1]), (&args[1], &args[0])])?
                .into_iter()
                .find_map(|(lhs, rhs)| {
                    let ExpData::Call(_, Operation::Result(idx), _) = lhs.as_ref() else {
                        return None;
                    };
                    spec_value_over_prestate(env, rhs, &mut_params).then_some(*idx)
                })
        })
        .collect();
    determined.len() == result_count && (0..result_count).all(|idx| determined.contains(&idx))
}

fn has_exact_move_value_model(
    env: &GlobalEnv,
    id: QualifiedId<FunId>,
    visited: &mut BTreeSet<QualifiedId<FunId>>,
) -> bool {
    if env
        .get_intrinsics()
        .get_spec_fun_for_move_fun(&id)
        .is_some()
    {
        return true;
    }
    let fun = env.get_function(id);
    if has_functional_result_spec(env, id) {
        return true;
    }
    if fun.is_inline() {
        return true;
    }
    if fun.is_native_or_intrinsic() {
        let name = env.symbol_pool().string(fun.get_name());
        return (fun.module_env.is_std_vector()
            && well_known::is_special_vector_bp_fun_name(name.as_str()))
            || fun.is_well_known(well_known::TYPE_NAME_MOVE)
            || fun.is_well_known(well_known::TYPE_INFO_MOVE)
            || fun.is_well_known(well_known::TYPE_NAME_GET_MOVE);
    }
    if fun.is_opaque() {
        return false;
    }
    let Some(body) = fun.get_def() else {
        return false;
    };
    if !visited.insert(id) {
        return false;
    }
    let exact = !body.any(&mut |exp| matches!(exp, ExpData::Loop(..) | ExpData::LoopCont(..)))
        && body
            .called_funs()
            .into_iter()
            .all(|callee| has_exact_move_value_model(env, callee, visited));
    visited.remove(&id);
    exact
}

/// Whether all named calls in `exp` have transitive value models.
pub fn exp_has_exact_value_model(env: &GlobalEnv, exp: &Exp) -> bool {
    exp.called_funs()
        .into_iter()
        .all(|callee| has_exact_move_value_model(env, callee, &mut BTreeSet::new()))
}

/// Whether a Move function has an exact functional result model.
pub fn move_fun_has_exact_value_model(env: &GlobalEnv, id: QualifiedId<FunId>) -> bool {
    has_exact_move_value_model(env, id, &mut BTreeSet::new())
}

/// Check if a callee has an associated pure spec function (created by the
/// spec rewriter) that can be called directly in spec expressions, returning
/// the spec function id and instantiated result type if so.
///
/// A function qualifies if it has no `&mut` params, no function-type params,
/// and an associated spec function with empty `used_memory`. Native pure
/// functions are accepted even though their derived spec function has no
/// body — the Boogie backend resolves them through prelude theories.
pub fn try_as_pure_spec_call(
    env: &GlobalEnv,
    module_id: ModuleId,
    fun_id: FunId,
    type_inst: &[Type],
) -> Option<(SpecFunId, Type)> {
    let callee = env.get_function(module_id.qualified(fun_id));
    // Must have no &mut params and no function-type params
    if callee
        .get_parameters()
        .iter()
        .any(|p| p.1.is_mutable_reference() || matches!(p.1.skip_reference(), Type::Fun(..)))
    {
        return None;
    }
    // A unit-returning function (e.g. an abort-only validator) has no spec
    // value to substitute; its abort behavior is described by the generic
    // behavioral summary instead.
    if callee.get_result_type().is_unit() {
        return None;
    }
    // Special case: intrinsic Move functions (e.g. SimpleMap::contains_key) are
    // axiomatized in Boogie via their paired spec function (e.g. spec_contains_key).
    // They have no `$name` spec function body, so find_spec_fun() returns None.
    // Instead, look up the spec function through the IntrinsicsAnnotation pairing table.
    let callee_qid = callee.get_qualified_id();
    if let Some(spec_qid) = env.get_intrinsics().get_spec_fun_for_move_fun(&callee_qid) {
        // Use the spec function's return type, not the Move function's (which may be &V).
        let spec_decl = env.get_spec_fun(spec_qid);
        let result_type = spec_decl.result_type.instantiate(type_inst);
        return Some((spec_qid.id, result_type));
    }

    // Must have an associated spec function with a body and no memory use.
    // `spec_rewriter` removes the body (`body = None`) for any spec function that
    // contains imperative expressions (Loop, Assign, Mutate, Return, LoopCont), so
    // a callee whose spec fun has side-effects or a while-loop body is rejected here.
    let (spec_fun_id, decl) = callee.find_spec_fun()?;
    // Non-native functions without a derived body cannot be expressed as a
    // spec call. Native body-less spec functions are resolved by the backend.
    if !decl.is_native && decl.body.is_none() {
        return None;
    }
    // Must not access global memory.
    if !decl.used_memory.is_empty() {
        return None;
    }
    // Additionally check the Move function body for specification-mode purity:
    // no Assign, Return, uninitialized let, or mutable borrows.
    // `FunctionPurenessChecker` traverses all sub-expressions including those
    // nested inside Loop nodes, so a while-loop body containing Assign is caught.
    if let Some(def) = callee.get_def() {
        let mut is_pure = true;
        let mut checker =
            FunctionPurenessChecker::new(FunctionPurenessCheckerMode::Specification, |_, _, _| {
                is_pure = false;
            });
        checker.check_exp(env, def);
        if !is_pure {
            return None;
        }
    }
    let result_type = callee.get_result_type().instantiate(type_inst);
    Some((spec_fun_id, result_type))
}

/// Whether `exp` mentions one of the given symbols outside of an
/// `old(..)` wrapper. In derived per-parameter or capture values, an
/// `old(x)` occurrence denotes the pre-state (transformer material),
/// while a plain occurrence is a post-state self-reference such a value
/// cannot express (see the effects-through-callee checks in the
/// `folds_of` resolution and in the callee body value summary).
pub fn mentions_syms_outside_old(exp: &Exp, syms: &BTreeSet<Symbol>) -> bool {
    let mut old_depth: usize = 0;
    let mut found = false;
    exp.visit_positions(&mut |pos, e| {
        match (e, &pos) {
            (ExpData::Call(_, Operation::Old, _), VisitorPosition::Pre) => old_depth += 1,
            (ExpData::Call(_, Operation::Old, _), VisitorPosition::Post) => old_depth -= 1,
            (ExpData::LocalVar(_, sym), VisitorPosition::Pre)
                if old_depth == 0 && syms.contains(sym) =>
            {
                found = true;
            },
            _ => {},
        }
        !found
    });
    found
}

/// Whether a spec-condition expression qualifies as an exact value over
/// the pre-state of a call: pure and single-state, no `result`
/// references, no free local variables (e.g. from spec `let` bindings),
/// and mentions of the given `&mut` parameters only under `old(..)` (a
/// plain mention denotes the post-state).
fn spec_value_over_prestate(env: &GlobalEnv, exp: &Exp, mut_params: &BTreeSet<usize>) -> bool {
    if !exp.free_vars().is_empty() {
        return false;
    }
    let mut ok = true;
    let mut old_depth: usize = 0;
    exp.visit_positions(&mut |pos, e| {
        match (e, &pos) {
            (ExpData::Call(_, Operation::Old, _), VisitorPosition::Pre) => old_depth += 1,
            (ExpData::Call(_, Operation::Old, _), VisitorPosition::Post) => old_depth -= 1,
            (ExpData::Call(_, Operation::Result(_), _), VisitorPosition::Pre) => ok = false,
            (ExpData::Temporary(_, idx), VisitorPosition::Pre)
                if old_depth == 0 && mut_params.contains(idx) =>
            {
                ok = false;
            },
            _ => {},
        }
        ok
    });
    ok && exps_are_pure_single_state(env, [exp])
}

/// The parameter index a spec expression mentions directly, looking
/// through `Deref`/`Freeze` wrappers (the builder phrases the value
/// mention of a reference parameter as a dereference).
fn param_mention(exp: &Exp) -> Option<usize> {
    match exp.as_ref() {
        ExpData::Temporary(_, idx) => Some(*idx),
        ExpData::Call(_, Operation::Deref | Operation::Freeze(_), args) => {
            param_mention(args.first()?)
        },
        _ => None,
    }
}

/// The root of a place: the local symbol for local-rooted chains, `None`
/// for global-rooted ones (conservatively treated as aliasing any other
/// place by the post-value routing).
fn place_root(place: &Place) -> Option<Symbol> {
    match place {
        Place::Local(sym) => Some(*sym),
        Place::Field(base, _) | Place::VecElem(base, _) => place_root(base),
        Place::Global(..) => None,
    }
}

/// Decomposes a place into its root local symbol and projection steps
/// (root-first order), with element indices canonicalized to plain
/// pre-state mentions. `None` for global-rooted places.
fn place_projection(place: &Place) -> Option<(Symbol, Vec<ProjStep>)> {
    match place {
        Place::Local(sym) => Some((*sym, vec![])),
        Place::Field(base, sel) => {
            let (root, mut steps) = place_projection(base)?;
            steps.push(ProjStep::Field(sel.clone()));
            Some((root, steps))
        },
        Place::VecElem(base, index) => {
            let (root, mut steps) = place_projection(base)?;
            steps.push(ProjStep::VecElem(strip_all_olds(index)));
            Some((root, steps))
        },
        Place::Global(..) => None,
    }
}

/// A memoized value summary of a callee's body: the exact post values of
/// its `&mut` parameters (keyed by parameter index) and, for a callee with
/// a single value-level result, the exact result value; derived by this
/// same analysis and phrased over placeholder parameter symbols denoting
/// the pre-state argument values (all `old(..)` wrappers stripped); pure
/// and single-state.
#[derive(Clone)]
struct CalleeValueSummary {
    param_syms: Vec<Symbol>,
    mut_values: BTreeMap<usize, Exp>,
    result_value: Option<Exp>,
}

/// A projection step of a reference-result summary: how a returned
/// reference descends from its root parameter.
#[derive(Clone)]
enum ProjStep {
    /// A field selection, rebuilt from the stored `Select` template.
    Field(FieldSel),
    /// A vector element selection; the index expression is phrased over
    /// the summary's placeholder parameter symbols (pre-state values).
    VecElem(Exp),
}

/// A flattened result component of a reference-returning pure helper.
#[derive(Clone)]
enum RefResultComp {
    /// A reference component: a projection of the parameter at the given
    /// index. At a call the projection is re-rooted at the argument's
    /// place (or read as a value over a value-level reference argument).
    Place { param: usize, steps: Vec<ProjStep> },
    /// A plain value component, phrased over the placeholder parameter
    /// symbols.
    Value(Exp),
}

/// A memoized place-projection summary of a reference-returning pure
/// helper (e.g. `fun borrow_kv(self: &Entry): (&K, &V) { (&self.key,
/// &self.value) }`): the helper has no memory effects, does not write its
/// parameters, and every returned reference is a field/element projection
/// of a parameter reference. Splicing the projections at a call makes the
/// returned references genuine places of the calling derivation —
/// generalizing the intrinsic `borrow_mut` place handling — so writes
/// through them compose (which `result_of` carriers cannot express).
/// `aborts` are the helper's abort disjuncts (a pure assertion prelude is
/// permitted), phrased over the placeholder symbols.
#[derive(Clone)]
struct CalleeRefResultSummary {
    param_syms: Vec<Symbol>,
    results: Vec<RefResultComp>,
    aborts: Vec<Exp>,
}

/// Cache of callee body value summaries, hosted as a `GlobalEnv`
/// extension: entries are keyed per callee and type instantiation; the
/// in-progress set breaks recursion cycles (shared by both summary kinds,
/// which run the same body derivation).
#[derive(Default)]
struct CalleeValueSummaryCache {
    entries: RefCell<BTreeMap<(QualifiedId<FunId>, Vec<Type>), Option<CalleeValueSummary>>>,
    ref_entries: RefCell<BTreeMap<(QualifiedId<FunId>, Vec<Type>), Option<CalleeRefResultSummary>>>,
    in_progress: RefCell<BTreeSet<QualifiedId<FunId>>>,
}

fn summary_cache(env: &GlobalEnv) -> Rc<CalleeValueSummaryCache> {
    match env.get_extension::<CalleeValueSummaryCache>() {
        Some(cache) => cache,
        None => {
            env.set_extension(CalleeValueSummaryCache::default());
            env.get_extension::<CalleeValueSummaryCache>()
                .expect("extension just set")
        },
    }
}

/// Substitutes a summary's placeholder parameter symbols by the call's
/// pre-state input expressions.
fn substitute_placeholders(
    env: &GlobalEnv,
    exp: &Exp,
    param_syms: &[Symbol],
    inputs: &[Exp],
) -> Exp {
    let map: BTreeMap<Symbol, Exp> = param_syms
        .iter()
        .zip(inputs)
        .map(|(sym, input)| (*sym, input.clone()))
        .collect();
    let mut replacer = |_: NodeId, target: RewriteTarget| match target {
        RewriteTarget::LocalVar(sym) => map.get(&sym).cloned(),
        RewriteTarget::Temporary(_) => None,
    };
    ExpRewriter::new(env, &mut replacer).rewrite_exp(exp.clone())
}

// =================================================================================================
// Analysis state

/// How a parameter of the analyzed body is treated by the analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParamKind {
    /// An ordinary by-value parameter.
    Value,
    /// A `&mut` reference parameter (declared, or a captured `&mut`
    /// reference entering as a literal `&mut` parameter): mentioning it
    /// denotes the reference to its cell, so bindings alias.
    MutRef,
    /// A by-value capture of the enclosing scope entering as an implicit
    /// `&mut` parameter: its value starts at the pre-state `old(c)` and
    /// its final value is exposed like a `&mut` parameter's post-state,
    /// but mentioning it denotes the current value (bindings copy, they
    /// do not alias).
    ValueCapture,
}

#[derive(Clone)]
struct ParamInfo {
    sym: Symbol,
    kind: ParamKind,
}

/// A symbolic value.
#[derive(Clone, Debug)]
enum SymVal {
    /// A first-class value, represented by a spec expression over the
    /// pre-state parameter values and free variables.
    Value(Exp),
    /// A reference to a place.
    Ref(Place),
    /// A tuple of values (from tuple expressions or multi-result calls).
    Tuple(Vec<SymVal>),
    /// A lambda value, closing over the captured values at creation.
    Func(FunVal),
}

/// A lambda value.
#[derive(Clone, Debug)]
struct FunVal {
    /// Identity for joins.
    node_id: NodeId,
    pat: Pattern,
    body: Exp,
    /// Captured free variables, snapshotted at creation (Move lambdas
    /// capture by value).
    captures: BTreeMap<Symbol, SymVal>,
}

/// A place a reference can point to.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Place {
    /// A local variable or parameter cell, identified by its store symbol.
    Local(Symbol),
    /// A field of a place. Rebuilding uses the stored `Select` template
    /// expression (with a hole at argument position 0) for reads and
    /// `UpdateField` for writes.
    Field(Box<Place>, FieldSel),
    /// A global resource cell `R[addr]`.
    Global(QualifiedInstId<StructId>, Exp),
    /// A vector element cell.
    VecElem(Box<Place>, Exp),
}

/// Field selection data: the original `Select` operation and node, used to
/// rebuild reads and updates.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldSel {
    oper: Operation,
    node_ty: Type,
    inst: Vec<Type>,
}

/// The memory state label at the current program point.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelState {
    Concrete(MemoryLabel),
    /// Different labels joined; a subsequent state-referencing operation
    /// cannot be given a concrete pre-state.
    Mixed,
}

/// Branch-local evaluation state.
#[derive(Clone)]
struct Frame {
    /// Current symbolic value per bound symbol.
    store: BTreeMap<Symbol, SymVal>,
    /// Whether this control path has diverged (returned or aborted).
    diverged: bool,
    /// The current memory state label.
    label: LabelState,
}

/// A recorded normal return.
struct ReturnRecord {
    guard: Option<Exp>,
    results: Vec<Exp>,
    /// Current values of `&mut` parameters at the return point.
    param_state: Vec<(Symbol, Exp)>,
}

/// A post-state slot for a `&mut` argument of a summarized call.
enum PostSlot {
    /// A fresh symbol carrying the post-state value (with its value type),
    /// constrained by the canonical `ensures_of` at assembly.
    Sym(Symbol, Type),
    /// The exact post-state value over the call's pre-state inputs,
    /// obtained from the callee's functional ensures or its body value
    /// summary; the canonical `ensures_of` restates it.
    Value(Exp),
}

/// A generic behavioral call summary, recorded for canonical `ensures_of`
/// emission and post-state variable resolution at assembly.
struct CallRecord {
    guard: Option<Exp>,
    fun_exp: Exp,
    /// Pre-state values of all arguments.
    inputs: Vec<Exp>,
    /// The `result_of` carrier expressions (one per result component).
    results: Vec<Exp>,
    /// Post-state slots of the `&mut` arguments, in argument order.
    posts: Vec<PostSlot>,
    /// The pre- and post-state labels of the summarized call; `None` for a
    /// callee without memory effects, whose summary is single-state.
    pre: Option<MemoryLabel>,
    post: Option<MemoryLabel>,
    /// Whether this is a deferred application of a function-typed
    /// parameter of the enclosing function (see
    /// `Deriver::deferred_fun_param_temps`), exposed in
    /// [`DerivedSpec::deferred_applications`].
    deferred: bool,
}

/// Failure of the derivation (body outside the derivable fragment).
struct Unsupported;

type Res<T> = Result<T, Unsupported>;

/// Evaluation result: `None` when all paths through the expression diverge.
type EvalResult = Res<Option<SymVal>>;

struct Deriver<'a, G> {
    builder: &'a mut G,
    params: Vec<ParamInfo>,
    var_types: BTreeMap<Symbol, Type>,
    /// Current path condition (conjunctive).
    path: Vec<Exp>,
    frame: Frame,
    /// Accumulated abort conditions, each guarded by the path at emission.
    aborts: Vec<Exp>,
    /// Accumulated two-state memory effects, guarded.
    effects: Vec<Exp>,
    /// Modified global memory targets, as unlabeled `global<R>(addr)`
    /// expressions.
    modifies: Vec<Exp>,
    /// Whether `modifies` covers all memory the body can modify. Cleared by
    /// generic behavioral call summaries, whose memory footprint is unknown.
    modifies_exact: bool,
    returns: Vec<ReturnRecord>,
    /// Records of generic behavioral call summaries, for canonical
    /// `ensures_of` emission at assembly.
    call_records: Vec<CallRecord>,
    entry_label: MemoryLabel,
    fresh_counter: usize,
    depth: usize,
    /// Function-typed parameters of the enclosing function (temporary
    /// index to symbol) whose applications are deferred; see
    /// [`derive_spec_with_captures`].
    deferred_fun_param_temps: BTreeMap<usize, Symbol>,
}

impl<'env, G: ExpGenerator<'env>> Deriver<'_, G> {
    // =============================================================================================
    // Helpers

    fn fresh_sym(&mut self, prefix: &str) -> Symbol {
        self.fresh_counter += 1;
        self.builder
            .mk_symbol(&format!("$wp_{}{}", prefix, self.fresh_counter))
    }

    fn fresh_label(&mut self) -> MemoryLabel {
        let env = self.builder.global_env();
        let label = MemoryLabel::new(env.new_global_id().as_usize());
        self.fresh_counter += 1;
        env.set_memory_label_name(
            label,
            env.symbol_pool().make(&format!("S{}", self.fresh_counter)),
        );
        label
    }

    /// The current concrete memory label; fails under mixed labels (a
    /// state-referencing operation after a join of different states).
    fn cur_label(&self) -> Res<MemoryLabel> {
        match self.frame.label {
            LabelState::Concrete(label) => Ok(label),
            LabelState::Mixed => Err(Unsupported),
        }
    }

    /// Advances the state to a fresh label, returning `(pre, post)`.
    fn advance_label(&mut self) -> Res<(MemoryLabel, MemoryLabel)> {
        let pre = self.cur_label()?;
        let post = self.fresh_label();
        self.frame.label = LabelState::Concrete(post);
        Ok((pre, post))
    }

    /// Adds a two-state effect condition, guarded by the current path.
    fn add_effect(&mut self, cond: Exp) {
        let guarded = match self.path_cond() {
            Some(p) => self.builder.mk_implies(p, cond),
            None => cond,
        };
        self.effects.push(guarded);
    }

    /// Advances the memory state and records the two-state effect built by
    /// `mk_effect` over the resulting `(pre, post)` label range. `target` is
    /// the modified memory cell as an unlabeled `global<R>(addr)` expression,
    /// recorded in `modifies`.
    fn record_memory_effect(
        &mut self,
        target: Exp,
        mk_effect: impl FnOnce(&G, MemoryRange) -> Exp,
    ) -> Res<()> {
        let (pre, post) = self.advance_label()?;
        let effect = mk_effect(self.builder, MemoryRange {
            pre: Some(pre),
            post: Some(post),
        });
        self.add_effect(effect);
        self.modifies.push(target);
        Ok(())
    }

    /// Resolves the resource type from an operation node's instantiation.
    fn resource_of_node(&self, id: NodeId) -> Res<QualifiedInstId<StructId>> {
        let inst = self.builder.global_env().get_node_instantiation(id);
        match inst.first() {
            Some(Type::Struct(mid, sid, targs)) => Ok(mid.qualified_inst(*sid, targs.clone())),
            _ => Err(Unsupported),
        }
    }

    /// The current path condition, or `None` if unconditional.
    fn path_cond(&self) -> Option<Exp> {
        self.builder
            .mk_join_bool(Operation::And, self.path.iter().cloned())
    }

    /// Adds an abort condition, guarded by the current path.
    fn add_abort(&mut self, cond: Exp) {
        let guarded = match self.path_cond() {
            Some(p) => self.builder.mk_and(p, cond),
            None => cond,
        };
        self.aborts.push(guarded);
    }

    /// Extracts a plain value expression; fails on references, tuples,
    /// and function values.
    fn as_value(&self, val: SymVal) -> Res<Exp> {
        self.as_value_in(&self.frame, val)
    }

    /// Reads the current value stored at a place.
    fn read_place(&self, place: &Place) -> Res<Exp> {
        self.read_place_in(&self.frame, place)
    }

    /// Reads the value stored at a place against the given frame.
    fn read_place_in(&self, frame: &Frame, place: &Place) -> Res<Exp> {
        match place {
            Place::Local(sym) => match frame.store.get(sym) {
                Some(SymVal::Value(e)) => Ok(e.clone()),
                _ => Err(Unsupported),
            },
            Place::Field(base, sel) => {
                let base_exp = self.read_place_in(frame, base)?;
                let env = self.builder.global_env();
                let id = env.new_node(self.builder.get_current_loc(), sel.node_ty.clone());
                if !sel.inst.is_empty() {
                    env.set_node_instantiation(id, sel.inst.clone());
                }
                Ok(ExpData::Call(id, sel.oper.clone(), vec![base_exp]).into_exp())
            },
            Place::Global(resource, addr) => {
                let label = match frame.label {
                    LabelState::Concrete(label) => label,
                    LabelState::Mixed => return Err(Unsupported),
                };
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                Ok(self.builder.mk_global_with_label(
                    &struct_env,
                    &resource.inst,
                    addr.clone(),
                    Some(label),
                ))
            },
            Place::VecElem(base, index) => {
                let base_exp = self.read_place_in(frame, base)?;
                let env = self.builder.global_env();
                let vec_ty = env.get_node_type(base_exp.as_ref().node_id());
                let elem_ty = match vec_ty.skip_reference() {
                    Type::Vector(elem) => (**elem).clone(),
                    _ => return Err(Unsupported),
                };
                Ok(self.builder.mk_index(base_exp, index.clone(), &elem_ty))
            },
        }
    }

    /// Writes a value to a place, composing functional updates up to the
    /// root cell. Fails for roots not in the store (writes through
    /// references of the enclosing scope).
    fn write_place(&mut self, place: &Place, value: Exp) -> Res<()> {
        match place {
            Place::Local(sym) => {
                if self.frame.store.contains_key(sym) {
                    self.frame.store.insert(*sym, SymVal::Value(value));
                    Ok(())
                } else {
                    Err(Unsupported)
                }
            },
            Place::Field(base, sel) => {
                let base_exp = self.read_place(base)?;
                let updated = self.mk_update_field(sel, base_exp, value)?;
                self.write_place(base, updated)
            },
            Place::Global(resource, addr) => {
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                let target = self.builder.mk_global_with_label(
                    &struct_env,
                    &resource.inst,
                    addr.clone(),
                    None,
                );
                self.record_memory_effect(target, |g, range| {
                    g.mk_spec_update(&struct_env, &resource.inst, addr.clone(), value, range)
                })
            },
            Place::VecElem(base, index) => {
                let base_exp = self.read_place(base)?;
                let vec_ty = self
                    .builder
                    .global_env()
                    .get_node_type(base_exp.as_ref().node_id());
                let updated = self
                    .builder
                    .mk_update_vec(base_exp, index.clone(), value, &vec_ty);
                self.write_place(base, updated)
            },
        }
    }

    /// Builds `update_field` for the field denoted by a `Select` operation.
    fn mk_update_field(&mut self, sel: &FieldSel, base: Exp, value: Exp) -> Res<Exp> {
        let (mid, sid, fid) = match &sel.oper {
            Operation::Select(mid, sid, fid) => (*mid, *sid, *fid),
            // Writes through variant field selections are not supported.
            _ => return Err(Unsupported),
        };
        let env = self.builder.global_env();
        // The struct instantiation is not contained in the select operation
        // but in the type of its operand.
        let Type::Struct(_, _, inst) = env
            .get_node_type(base.as_ref().node_id())
            .skip_reference()
            .clone()
        else {
            return Err(Unsupported);
        };
        let struct_env = env.get_struct(mid.qualified(sid));
        let field_env = struct_env.get_field(fid);
        Ok(self.builder.mk_field_update(&field_env, &inst, base, value))
    }

    /// Rebuilds a pure operation call with evaluated arguments, keeping the
    /// original node type and instantiation.
    fn rebuild(&mut self, id: NodeId, oper: &Operation, args: Vec<Exp>) -> Exp {
        let env = self.builder.global_env();
        let ty = env.get_node_type(id);
        let inst = env.get_node_instantiation(id);
        if inst.is_empty() {
            self.builder.mk_call(&ty, oper.clone(), args)
        } else {
            self.builder
                .mk_call_with_inst(&ty, inst, oper.clone(), args)
        }
    }

    fn prim_ty(&self, id: NodeId) -> Res<PrimitiveType> {
        match self.builder.global_env().get_node_type(id) {
            Type::Primitive(p) => Ok(p),
            _ => Err(Unsupported),
        }
    }

    /// Fails for signed integer types at the given node, whose abort
    /// conditions differ from the unsigned encodings emitted here.
    fn reject_signed(&self, id: NodeId) -> Res<()> {
        match self.builder.global_env().get_node_type(id) {
            Type::Primitive(p) if p.is_signed() => Err(Unsupported),
            _ => Ok(()),
        }
    }

    /// Returns an expanded inline call's derivation summary, if present.
    fn inline_call_summary(exp: &Exp) -> Option<(&Exp, &Exp)> {
        let ExpData::Sequence(_, exps) = exp.as_ref() else {
            return None;
        };
        exps.iter().find_map(|candidate| {
            let ExpData::SpecBlock(_, spec) = candidate.as_ref() else {
                return None;
            };
            let [condition] = spec.conditions.as_slice() else {
                return None;
            };
            let ExpData::Call(_, Operation::InlineCallSummary, args) = condition.exp.as_ref()
            else {
                return None;
            };
            let [result, aborts] = args.as_slice() else {
                return None;
            };
            Some((result, aborts))
        })
    }

    /// Instantiates an inline-call summary in the current symbolic frame.
    fn instantiate_inline_call_summary(&self, exp: &Exp) -> Res<Exp> {
        let mut substitutions = BTreeMap::new();
        for sym in exp.free_vars() {
            if let Some(value) = self.frame.store.get(&sym).cloned() {
                substitutions.insert(sym, self.as_value(value)?);
            }
        }
        let env = self.builder.global_env();
        let mut replacer = |_: NodeId, target: RewriteTarget| match target {
            RewriteTarget::LocalVar(sym) => substitutions.get(&sym).cloned(),
            RewriteTarget::Temporary(_) => None,
        };
        Ok(ExpRewriter::new(env, &mut replacer).rewrite_exp(exp.clone()))
    }

    // =============================================================================================
    // Evaluation

    fn eval(&mut self, exp: &Exp) -> EvalResult {
        if self.frame.diverged {
            return Ok(None);
        }
        self.builder.set_loc_from_node(exp.node_id());
        if let Some((result, aborts)) = Self::inline_call_summary(exp) {
            let result = self.instantiate_inline_call_summary(result)?;
            let aborts = self.instantiate_inline_call_summary(aborts)?;
            self.add_abort(aborts);
            return Ok(Some(SymVal::Value(result)));
        }
        use ExpData::*;
        match exp.as_ref() {
            Invalid(_) => Err(Unsupported),
            Value(..) => Ok(Some(SymVal::Value(exp.clone()))),
            LocalVar(_, sym) => {
                if self
                    .params
                    .iter()
                    .any(|p| p.sym == *sym && p.kind == ParamKind::MutRef)
                    && self.frame.store.contains_key(sym)
                {
                    // A `&mut` parameter used as a value is the reference to
                    // its cell; the store holds the current target value.
                    Ok(Some(SymVal::Ref(Place::Local(*sym))))
                } else if let Some(val) = self.frame.store.get(sym) {
                    Ok(Some(val.clone()))
                } else {
                    // Free variable of the enclosing scope: itself.
                    Ok(Some(SymVal::Value(exp.clone())))
                }
            },
            Temporary(..) => {
                // Parameter of the enclosing function (free in the analyzed
                // body): itself.
                Ok(Some(SymVal::Value(exp.clone())))
            },
            Call(id, oper, args) => self.eval_call(*id, oper, args),
            Lambda(id, pat, body, _, _) => {
                // Move lambdas capture by value: snapshot the current values
                // of the free variables bound in the store.
                let mut captures = BTreeMap::new();
                for sym in exp.free_vars() {
                    if let Some(val) = self.frame.store.get(&sym) {
                        captures.insert(sym, val.clone());
                    }
                }
                Ok(Some(SymVal::Func(FunVal {
                    node_id: *id,
                    pat: pat.clone(),
                    body: body.clone(),
                    captures,
                })))
            },
            Invoke(_, target, args) => self.eval_invoke(target, args),
            Quant(..) => Err(Unsupported),
            Block(_, pat, binding, scope) => {
                let bound = match binding {
                    Some(b) => match self.eval(b)? {
                        Some(v) => Some(v),
                        None => return Ok(None),
                    },
                    None => None,
                };
                let saved = self.bind_pattern(pat, bound)?;
                let result = self.eval(scope)?;
                self.restore_bindings(saved);
                Ok(result)
            },
            IfElse(_, cond, if_true, if_false) => {
                let Some(cond_val) = self.eval(cond)? else {
                    return Ok(None);
                };
                let cond_exp = self.as_value(cond_val)?;
                // Constant-fold literal conditions.
                if let ExpData::Value(_, self::Value::Bool(b)) = cond_exp.as_ref() {
                    return if *b {
                        self.eval(if_true)
                    } else {
                        self.eval(if_false)
                    };
                }
                self.eval_branches(cond_exp, if_true, if_false)
            },
            Match(_, disc, arms) => {
                let Some(disc_val) = self.eval(disc)? else {
                    return Ok(None);
                };
                let disc_exp = self.as_value(disc_val)?;
                self.eval_arms(&disc_exp, arms)
            },
            Return(_, val) => {
                let Some(v) = self.eval(val)? else {
                    return Ok(None);
                };
                self.record_return(v)?;
                self.frame.diverged = true;
                Ok(None)
            },
            Sequence(_, exps) => {
                let mut last = Some(SymVal::Value(self.unit_value()));
                for (i, e) in exps.iter().enumerate() {
                    let is_last = i + 1 == exps.len();
                    let v = if is_last {
                        self.eval(e)?
                    } else {
                        self.eval_discarded(e)?
                    };
                    if v.is_none() {
                        return Ok(None);
                    }
                    if is_last {
                        last = v;
                    }
                }
                Ok(last)
            },
            Loop(..) | LoopCont(..) => Err(Unsupported),
            Assign(_, pat, rhs) => {
                let Some(v) = self.eval(rhs)? else {
                    return Ok(None);
                };
                self.assign_pattern(pat, v)?;
                Ok(Some(SymVal::Value(self.unit_value())))
            },
            Mutate(_, lhs, rhs) => {
                let Some(rv) = self.eval(rhs)? else {
                    return Ok(None);
                };
                let value = self.as_value(rv)?;
                // The left-hand side denotes a place; resolve it like a
                // mutable borrow (this also covers field writes rooted at
                // by-value locals, where plain evaluation would yield the
                // struct value instead of the cell).
                let Some(lv) = self.eval_borrow(ReferenceKind::Mutable, lhs)? else {
                    return Ok(None);
                };
                let SymVal::Ref(place) = lv else {
                    return Err(Unsupported);
                };
                self.write_place(&place, value)?;
                Ok(Some(SymVal::Value(self.unit_value())))
            },
            SpecBlock(..) => {
                // Verification-only content; state-neutral for the analysis.
                Ok(Some(SymVal::Value(self.unit_value())))
            },
        }
    }

    fn unit_value(&self) -> Exp {
        let env = self.builder.global_env();
        let id = env.new_node(self.builder.get_current_loc(), Type::unit());
        ExpData::Call(id, Operation::Tuple, vec![]).into_exp()
    }

    /// Evaluates an expression without requiring a place for a returned `&mut`.
    fn eval_discarded(&mut self, exp: &Exp) -> EvalResult {
        let ExpData::Call(id, Operation::MoveFunction(mid, fid), args) = exp.as_ref() else {
            return self.eval(exp);
        };
        self.builder.set_loc_from_node(*id);
        let mut arg_vals = vec![];
        for arg in args {
            let Some(value) = self.eval(arg)? else {
                return Ok(None);
            };
            arg_vals.push(value);
        }
        self.eval_move_function_call(*id, *mid, *fid, arg_vals, false)
    }

    /// Evaluates both sides of a branch in cloned frames and joins.
    fn eval_branches(&mut self, cond: Exp, if_true: &Exp, if_false: &Exp) -> EvalResult {
        let saved_frame = self.frame.clone();
        self.path.push(cond.clone());
        let true_val = self.eval(if_true)?;
        let true_frame = std::mem::replace(&mut self.frame, saved_frame);
        self.path.pop();
        let not_cond = self.builder.mk_not(cond.clone());
        self.path.push(not_cond);
        let false_val = self.eval(if_false)?;
        self.path.pop();
        let false_frame = self.frame.clone();
        self.join(cond, true_frame, true_val, false_frame, false_val)
    }

    /// Joins two branch outcomes into the current frame and result value.
    fn join(
        &mut self,
        cond: Exp,
        true_frame: Frame,
        true_val: Option<SymVal>,
        false_frame: Frame,
        false_val: Option<SymVal>,
    ) -> EvalResult {
        match (true_frame.diverged, false_frame.diverged) {
            (true, true) => {
                self.frame = true_frame;
                self.frame.diverged = true;
                Ok(None)
            },
            (true, false) => {
                self.frame = false_frame;
                Ok(false_val)
            },
            (false, true) => {
                self.frame = true_frame;
                Ok(true_val)
            },
            (false, false) => {
                let mut store = BTreeMap::new();
                for (sym, tv) in &true_frame.store {
                    let Some(fv) = false_frame.store.get(sym) else {
                        continue;
                    };
                    store.insert(*sym, self.join_vals(&cond, tv.clone(), fv.clone())?);
                }
                let label = if true_frame.label == false_frame.label {
                    true_frame.label
                } else {
                    LabelState::Mixed
                };
                self.frame = Frame {
                    store,
                    diverged: false,
                    label,
                };
                let val = match (true_val, false_val) {
                    (Some(tv), Some(fv)) => {
                        // References join only with the identical place; any
                        // other pairing involving a reference is read back to
                        // values against the respective branch frames.
                        let same_ref = matches!(
                            (&tv, &fv),
                            (SymVal::Ref(p), SymVal::Ref(q)) if p == q
                        );
                        let any_ref = matches!(tv, SymVal::Ref(_)) || matches!(fv, SymVal::Ref(_));
                        let (tv, fv) = if any_ref && !same_ref {
                            (
                                SymVal::Value(self.as_value_in(&true_frame, tv)?),
                                SymVal::Value(self.as_value_in(&false_frame, fv)?),
                            )
                        } else {
                            (tv, fv)
                        };
                        Some(self.join_vals(&cond, tv, fv)?)
                    },
                    // A side without a value must have diverged, handled above.
                    _ => None,
                };
                Ok(val)
            },
        }
    }

    fn join_vals(&mut self, cond: &Exp, a: SymVal, b: SymVal) -> Res<SymVal> {
        match (a, b) {
            (SymVal::Value(x), SymVal::Value(y)) => {
                if x.as_ref() == y.as_ref()
                    || self
                        .builder
                        .global_env()
                        .get_node_type(x.as_ref().node_id())
                        .is_unit()
                {
                    Ok(SymVal::Value(x))
                } else {
                    Ok(SymVal::Value(self.builder.mk_ite(
                        cond.as_ref().clone(),
                        x.as_ref().clone(),
                        y.as_ref().clone(),
                    )))
                }
            },
            (SymVal::Ref(p), SymVal::Ref(q)) if p == q => Ok(SymVal::Ref(p)),
            (SymVal::Tuple(xs), SymVal::Tuple(ys)) if xs.len() == ys.len() => {
                let mut joined = vec![];
                for (x, y) in xs.into_iter().zip(ys) {
                    joined.push(self.join_vals(cond, x, y)?);
                }
                Ok(SymVal::Tuple(joined))
            },
            (SymVal::Func(f), SymVal::Func(g)) if f.node_id == g.node_id => {
                // Same lambda; captures must agree structurally.
                if f.captures.len() == g.captures.len()
                    && f.captures
                        .iter()
                        .zip(g.captures.iter())
                        .all(|((s1, v1), (s2, v2))| {
                            s1 == s2
                                && matches!(
                                    (v1, v2),
                                    (SymVal::Value(x), SymVal::Value(y)) if x.as_ref() == y.as_ref()
                                )
                        })
                {
                    Ok(SymVal::Func(f))
                } else {
                    Err(Unsupported)
                }
            },
            _ => Err(Unsupported),
        }
    }

    // =============================================================================================
    // Patterns

    /// Binds a pattern in a new scope; returns the shadowed entries to
    /// restore at scope exit. A `None` value is an uninitialized let.
    fn bind_pattern(
        &mut self,
        pat: &Pattern,
        value: Option<SymVal>,
    ) -> Res<Vec<(Symbol, Option<SymVal>)>> {
        let mut saved = vec![];
        self.bind_pattern_rec(pat, value, true, &mut saved)?;
        Ok(saved)
    }

    fn restore_bindings(&mut self, saved: Vec<(Symbol, Option<SymVal>)>) {
        for (sym, old) in saved.into_iter().rev() {
            match old {
                Some(v) => {
                    self.frame.store.insert(sym, v);
                },
                None => {
                    self.frame.store.remove(&sym);
                },
            }
        }
    }

    /// Assigns to an existing pattern (no new scope).
    fn assign_pattern(&mut self, pat: &Pattern, value: SymVal) -> Res<()> {
        let mut saved = vec![];
        self.bind_pattern_rec(pat, Some(value), false, &mut saved)
    }

    fn bind_pattern_rec(
        &mut self,
        pat: &Pattern,
        value: Option<SymVal>,
        scoped: bool,
        saved: &mut Vec<(Symbol, Option<SymVal>)>,
    ) -> Res<()> {
        match pat {
            Pattern::Var(_, sym) => {
                if scoped {
                    saved.push((*sym, self.frame.store.get(sym).cloned()));
                } else if !self.frame.store.contains_key(sym) {
                    // An assignment to a variable of the enclosing scope:
                    // the analysis cannot express the write — the derived
                    // conditions have no way to name the variable's pre- and
                    // post-state. Inserting it into the store would silently
                    // drop the effect from the derived specification.
                    return Err(Unsupported);
                }
                match value {
                    Some(v) => {
                        self.frame.store.insert(*sym, v);
                    },
                    None => {
                        // Uninitialized let: remove any shadowed entry; a
                        // read before assignment fails as unsupported
                        // free-variable... it would wrongly resolve, so mark
                        // by removing and treating later reads of an
                        // uninitialized local as unsupported is not possible
                        // via absence (absence means free). Reject instead.
                        return Err(Unsupported);
                    },
                }
                Ok(())
            },
            Pattern::Wildcard(_) => Ok(()),
            Pattern::Tuple(_, sub_pats) => {
                // Zero-argument inline calls bind an empty tuple to nothing.
                if sub_pats.is_empty() && value.is_none() {
                    return Ok(());
                }
                let Some(v) = value else {
                    return Err(Unsupported);
                };
                match v {
                    SymVal::Tuple(vals) if vals.len() == sub_pats.len() => {
                        for (p, v) in sub_pats.iter().zip(vals) {
                            self.bind_pattern_rec(p, Some(v), scoped, saved)?;
                        }
                        Ok(())
                    },
                    // Inline parameter blocks use singleton tuple patterns.
                    v if sub_pats.len() == 1 => {
                        self.bind_pattern_rec(&sub_pats[0], Some(v), scoped, saved)
                    },
                    // A unit value matching an empty tuple pattern.
                    SymVal::Value(_) if sub_pats.is_empty() => Ok(()),
                    _ => Err(Unsupported),
                }
            },
            Pattern::Struct(_, struct_qid, variant, sub_pats) => {
                let Some(v) = value else {
                    return Err(Unsupported);
                };
                let value_exp = self.as_value(v)?;
                let env = self.builder.global_env();
                let struct_env = env.get_struct(struct_qid.to_qualified_id());
                if let Some(variant) = variant {
                    // Unpacking a variant aborts if the value is another
                    // variant.
                    let test =
                        self.builder
                            .mk_variant_test(&struct_env, *variant, value_exp.clone());
                    let not_test = self.builder.mk_not(test);
                    self.add_abort(not_test);
                }
                let fields = struct_env
                    .get_fields_optional_variant(*variant)
                    .collect::<Vec<_>>();
                if fields.len() != sub_pats.len() {
                    return Err(Unsupported);
                }
                for (field_env, p) in fields.iter().zip(sub_pats) {
                    let field_val = self.builder.mk_field_select(
                        field_env,
                        &struct_qid.inst,
                        value_exp.clone(),
                    );
                    self.bind_pattern_rec(p, Some(SymVal::Value(field_val)), scoped, saved)?;
                }
                Ok(())
            },
            Pattern::LiteralValue(..) | Pattern::Range(..) | Pattern::Error(_) => Err(Unsupported),
        }
    }

    // =============================================================================================
    // Operations

    fn eval_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> EvalResult {
        use Operation::*;
        // Short-circuit operators guard their second operand.
        if matches!(oper, And | Or) {
            return self.eval_short_circuit(id, oper, args);
        }
        if matches!(oper, Abort(..)) {
            // Evaluate the code operand for its side conditions, then abort.
            if self.eval(&args[0])?.is_none() {
                return Ok(None);
            }
            self.add_abort(self.builder.mk_bool_const(true));
            self.frame.diverged = true;
            return Ok(None);
        }
        // Reference operations are handled before generic argument
        // evaluation, since they operate on places.
        match oper {
            Borrow(kind) => return self.eval_borrow(*kind, &args[0]),
            Deref => {
                let Some(v) = self.eval(&args[0])? else {
                    return Ok(None);
                };
                return Ok(Some(SymVal::Value(self.as_value(v)?)));
            },
            Freeze(_) => return self.eval(&args[0]),
            _ => {},
        }
        // Evaluate arguments left-to-right.
        let mut arg_vals = vec![];
        for arg in args {
            let Some(v) = self.eval(arg)? else {
                return Ok(None);
            };
            arg_vals.push(v);
        }
        match oper {
            Tuple => {
                return Ok(Some(match arg_vals.len() {
                    0 => SymVal::Value(self.unit_value()),
                    1 => arg_vals.pop().unwrap(),
                    _ => SymVal::Tuple(arg_vals),
                }));
            },
            Operation::MoveFunction(mid, fid) => {
                return self.eval_move_function_call(id, *mid, *fid, arg_vals, true);
            },
            Select(..) => {
                // A field selection over a reference denotes a sub-place:
                // writes through it compose functional updates up to the
                // root cell, and reads rebuild the selection over the base
                // value (mirroring `eval_borrow`). Value operands fall
                // through to the generic rebuild below.
                if let [SymVal::Ref(place)] = &arg_vals[..] {
                    let env = self.builder.global_env();
                    return Ok(Some(SymVal::Ref(Place::Field(
                        Box::new(place.clone()),
                        FieldSel {
                            oper: oper.clone(),
                            node_ty: env.get_node_type(id),
                            inst: env.get_node_instantiation(id),
                        },
                    ))));
                }
            },
            _ => {},
        }
        let mut value_args = vec![];
        for v in arg_vals {
            value_args.push(self.as_value(v)?);
        }
        let result = match oper {
            // Arithmetic with abort side conditions, mirroring the
            // bytecode-level inference. Symbolic arithmetic is built in
            // unbounded `num` (like the bytecode inference), since the
            // bounded operand types would make overflow conditions
            // trivially false in spec semantics.
            Add | Mul => {
                // Signed overflow needs two-sided range checks; the
                // overflow-only encoding here covers unsigned types.
                self.reject_signed(id)?;
                let value = if matches!(oper, Add) {
                    self.builder
                        .mk_num_add(value_args[0].clone(), value_args[1].clone())
                } else {
                    self.builder
                        .mk_num_mul(value_args[0].clone(), value_args[1].clone())
                };
                let prim = self.prim_ty(id)?;
                let check = self
                    .builder
                    .mk_range_check(&prim, RangeCheckKind::Overflow, value.clone())
                    .ok_or(Unsupported)?;
                self.add_abort(check);
                value
            },
            Sub => {
                // The underflow encoding `a < b` covers unsigned types only.
                self.reject_signed(id)?;
                let check = self.builder.mk_bool_call(Operation::Lt, vec![
                    value_args[0].clone(),
                    value_args[1].clone(),
                ]);
                self.add_abort(check);
                self.builder
                    .mk_num_sub(value_args[0].clone(), value_args[1].clone())
            },
            Div | Mod => {
                // Signed division additionally aborts on `MIN / -1`; the
                // divisor-zero-only encoding here covers unsigned types.
                self.reject_signed(id)?;
                let zero = self.builder.mk_num_const(BigInt::from(0));
                let check = self
                    .builder
                    .mk_bool_call(Operation::Eq, vec![value_args[1].clone(), zero]);
                self.add_abort(check);
                if matches!(oper, Div) {
                    self.builder
                        .mk_num_div(value_args[0].clone(), value_args[1].clone())
                } else {
                    self.builder
                        .mk_num_mod(value_args[0].clone(), value_args[1].clone())
                }
            },
            Shl | Shr => {
                let lhs_prim = self.prim_ty(args[0].node_id())?;
                let bits = lhs_prim.get_num_bits().ok_or(Unsupported)?;
                let width = self.builder.mk_num_const(BigInt::from(bits));
                let check = self
                    .builder
                    .mk_bool_call(Operation::Ge, vec![value_args[1].clone(), width]);
                self.add_abort(check);
                let (lhs, rhs) = (value_args[0].clone(), value_args[1].clone());
                if matches!(oper, Shl) {
                    self.builder.mk_shl(lhs, rhs)
                } else {
                    self.builder.mk_shr(lhs, rhs)
                }
            },
            Cast => {
                let target = self.prim_ty(id)?;
                let kind = if target.is_signed() {
                    RangeCheckKind::Both
                } else {
                    // A signed source can be negative, which the
                    // overflow-only check for an unsigned target misses.
                    self.reject_signed(args[0].node_id())?;
                    RangeCheckKind::Overflow
                };
                let check = self
                    .builder
                    .mk_range_check(&target, kind, value_args[0].clone())
                    .ok_or(Unsupported)?;
                self.add_abort(check);
                self.rebuild(id, oper, value_args)
            },
            Negate => {
                let prim = self.prim_ty(id)?;
                if prim.is_signed() {
                    let min = self.builder.mk_num_min(&prim).ok_or(Unsupported)?;
                    let check = self
                        .builder
                        .mk_bool_call(Operation::Eq, vec![value_args[0].clone(), min]);
                    self.add_abort(check);
                }
                self.builder.mk_negate(value_args.pop().unwrap())
            },
            // Total operations: rebuild with evaluated arguments.
            Not | Eq | Neq | Lt | Gt | Le | Ge | BitOr | BitAnd | Xor | Copy | Move | Pack(..)
            | Vector | Select(..) | TestVariants(..) | Closure(..) => {
                self.rebuild(id, oper, value_args)
            },
            SelectVariants(mid, sid, fids) => {
                // Reading a field over variants aborts if the value is in
                // none of the variants carrying the field.
                let not_test = {
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(mid.qualified(*sid));
                    let variants = fids
                        .iter()
                        .filter_map(|fid| struct_env.get_field(*fid).get_variant())
                        .collect::<Vec<_>>();
                    let test = self.builder.mk_variant_tests(
                        &struct_env,
                        &variants,
                        value_args[0].clone(),
                    );
                    self.builder.mk_not(test)
                };
                self.add_abort(not_test);
                self.rebuild(id, oper, value_args)
            },
            NoOp => {
                debug_assert!(value_args.len() == 1);
                value_args.pop().unwrap()
            },
            Exists(None) => {
                let label = self.cur_label()?;
                let resource = self.resource_of_node(id)?;
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                self.builder.mk_exists_with_label(
                    &struct_env,
                    &resource.inst,
                    value_args[0].clone(),
                    Some(label),
                )
            },
            BorrowGlobal(kind) => {
                let label = self.cur_label()?;
                let resource = self.resource_of_node(id)?;
                let addr = value_args[0].clone();
                let not_exists = {
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(resource.to_qualified_id());
                    let exists = self.builder.mk_exists_with_label(
                        &struct_env,
                        &resource.inst,
                        addr.clone(),
                        Some(label),
                    );
                    self.builder.mk_not(exists)
                };
                self.add_abort(not_exists);
                if matches!(kind, ReferenceKind::Mutable) {
                    return Ok(Some(SymVal::Ref(Place::Global(resource, addr))));
                }
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                self.builder
                    .mk_global_with_label(&struct_env, &resource.inst, addr, Some(label))
            },
            MoveTo => {
                // srcs: [signer, value]; aborts if already published.
                let resource = self.resource_of_node(id)?;
                let addr = self.builder.signer_to_address(value_args[0].clone());
                let exists = {
                    let label = self.cur_label()?;
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(resource.to_qualified_id());
                    self.builder.mk_exists_with_label(
                        &struct_env,
                        &resource.inst,
                        addr.clone(),
                        Some(label),
                    )
                };
                self.add_abort(exists);
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                let target = self.builder.mk_global_with_label(
                    &struct_env,
                    &resource.inst,
                    addr.clone(),
                    None,
                );
                self.record_memory_effect(target, |g, range| {
                    g.mk_spec_publish(
                        &struct_env,
                        &resource.inst,
                        addr,
                        value_args[1].clone(),
                        range,
                    )
                })?;
                self.unit_value()
            },
            MoveFrom => {
                let resource = self.resource_of_node(id)?;
                let addr = value_args[0].clone();
                let label = self.cur_label()?;
                let (value, not_exists) = {
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(resource.to_qualified_id());
                    let value = self.builder.mk_global_with_label(
                        &struct_env,
                        &resource.inst,
                        addr.clone(),
                        Some(label),
                    );
                    let exists = self.builder.mk_exists_with_label(
                        &struct_env,
                        &resource.inst,
                        addr.clone(),
                        Some(label),
                    );
                    (value, self.builder.mk_not(exists))
                };
                self.add_abort(not_exists);
                let env = self.builder.global_env();
                let struct_env = env.get_struct(resource.to_qualified_id());
                let target = self.builder.mk_global_with_label(
                    &struct_env,
                    &resource.inst,
                    addr.clone(),
                    None,
                );
                self.record_memory_effect(target, |g, range| {
                    g.mk_spec_remove(&struct_env, &resource.inst, addr, range)
                })?;
                value
            },
            // Everything else (spec-only operations in code position, events,
            // etc.) is not supported.
            _ => return Err(Unsupported),
        };
        Ok(Some(SymVal::Value(result)))
    }

    fn eval_short_circuit(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> EvalResult {
        let Some(lhs_val) = self.eval(&args[0])? else {
            return Ok(None);
        };
        let lhs = self.as_value(lhs_val)?;
        let guard = if matches!(oper, Operation::And) {
            lhs.clone()
        } else {
            self.builder.mk_not(lhs.clone())
        };
        let saved_frame = self.frame.clone();
        self.path.push(guard.clone());
        let rhs_val = self.eval(&args[1])?;
        let rhs_frame = std::mem::replace(&mut self.frame, saved_frame);
        self.path.pop();
        // The non-evaluating side keeps the pre-state frame and yields the
        // short-circuit constant.
        let const_val = Some(SymVal::Value(
            self.builder.mk_bool_const(!matches!(oper, Operation::And)),
        ));
        let rhs_out = match rhs_val {
            Some(v) => Some(SymVal::Value({
                let rv = self.as_value_in(&rhs_frame, v)?;
                self.rebuild(id, oper, vec![lhs.clone(), rv])
            })),
            None => None,
        };
        let pre_frame = self.frame.clone();
        self.join(guard, rhs_frame, rhs_out, pre_frame, const_val)
    }

    /// Like `as_value`, but reading places against a given frame.
    fn as_value_in(&self, frame: &Frame, val: SymVal) -> Res<Exp> {
        match val {
            SymVal::Value(e) => Ok(e),
            SymVal::Ref(place) => self.read_place_in(frame, &place),
            SymVal::Tuple(_) | SymVal::Func(_) => Err(Unsupported),
        }
    }

    /// Evaluates a borrow operand to a place. Borrows of rvalues create an
    /// anonymous cell.
    fn eval_borrow(&mut self, kind: ReferenceKind, operand: &Exp) -> EvalResult {
        match operand.as_ref() {
            ExpData::LocalVar(_, sym) if self.frame.store.contains_key(sym) => {
                // A variable bound to a reference (a let-bound borrow) is
                // transparent: borrowing through it denotes the reference's
                // target place, not the binding itself.
                if let Some(SymVal::Ref(place)) = self.frame.store.get(sym) {
                    return Ok(Some(SymVal::Ref(place.clone())));
                }
                Ok(Some(SymVal::Ref(Place::Local(*sym))))
            },
            // A mutable borrow of a variable of the enclosing scope (a free
            // variable of the analyzed body, or a parameter of the enclosing
            // function): the analysis cannot express writes through it — the
            // derived conditions have no way to name the variable's pre- and
            // post-state. The rvalue fallback below would silently model the
            // write against a throwaway copy, losing the effect.
            ExpData::LocalVar(..) | ExpData::Temporary(..) if kind == ReferenceKind::Mutable => {
                Err(Unsupported)
            },
            ExpData::Call(id, oper @ Operation::Select(..), sel_args) => {
                let Some(base) = self.eval_borrow(kind, &sel_args[0])? else {
                    return Ok(None);
                };
                let SymVal::Ref(base_place) = base else {
                    return Err(Unsupported);
                };
                let env = self.builder.global_env();
                Ok(Some(SymVal::Ref(Place::Field(
                    Box::new(base_place),
                    FieldSel {
                        oper: oper.clone(),
                        node_ty: env.get_node_type(*id),
                        inst: env.get_node_instantiation(*id),
                    },
                ))))
            },
            ExpData::Call(_, Operation::Deref, deref_args) => {
                self.eval_borrow(kind, &deref_args[0])
            },
            ExpData::Call(_, Operation::Borrow(_), inner) => self.eval_borrow(kind, &inner[0]),
            // Borrow of a place-producing expression (e.g. `borrow_global`,
            // a call returning a reference), or of a genuine rvalue, which
            // gets an anonymous cell (a write through it goes to a
            // temporary, as in the dynamic semantics).
            _ => {
                let Some(v) = self.eval(operand)? else {
                    return Ok(None);
                };
                if matches!(v, SymVal::Ref(_)) {
                    return Ok(Some(v));
                }
                let value = self.as_value(v)?;
                let cell = self.fresh_sym("cell");
                self.frame.store.insert(cell, SymVal::Value(value));
                Ok(Some(SymVal::Ref(Place::Local(cell))))
            },
        }
    }

    // =============================================================================================
    // Calls

    /// Evaluates a call to a named Move function, using the same summary
    /// cascade as the bytecode-level inference: exact WP for `std::vector`
    /// intrinsics, spec-function substitution for pure callees, and a
    /// generic behavioral summary otherwise.
    fn eval_move_function_call(
        &mut self,
        id: NodeId,
        mid: ModuleId,
        fid: FunId,
        arg_vals: Vec<SymVal>,
        result_is_used: bool,
    ) -> EvalResult {
        let env = self.builder.global_env();
        let callee = env.get_function(mid.qualified(fid));
        if callee.is_inline() && !callee.is_inline_opaque_retained() {
            // Calls to expanded inline functions cannot occur in material
            // this analysis sees; defensive.
            return Err(Unsupported);
        }
        let type_inst = env.get_node_instantiation(id);
        let param_types = callee
            .get_parameters()
            .iter()
            .map(|p| p.1.instantiate(&type_inst))
            .collect::<Vec<_>>();
        let result_type = callee.get_result_type().instantiate(&type_inst);
        // Pre-state argument values and `&mut` argument places.
        let mut inputs = vec![];
        let mut mut_places = vec![];
        for (val, ty) in arg_vals.iter().zip(&param_types) {
            inputs.push(self.as_value(val.clone())?);
            if ty.is_mutable_reference() {
                match val {
                    SymVal::Ref(place) => mut_places.push((place.clone(), ty.clone())),
                    _ => return Err(Unsupported),
                }
            }
        }

        // 1. Exact WP for `std::vector` intrinsics.
        if let Some(wp) = self.try_vector_intrinsic(mid, fid, &type_inst, &inputs) {
            return self.finish_intrinsic_wp(mid, fid, &type_inst, &inputs, &mut_places, wp);
        }

        // 1.5. Exact WP for intrinsic-map mutators (value-level add/del
        // roles), phrased over the map type's declared spec functions.
        if let Some(wp) = well_known::map_intrinsic_wp(
            self.builder.global_env(),
            self.builder,
            mid.qualified(fid),
            &type_inst,
            &inputs,
        ) {
            return self.finish_intrinsic_wp(mid, fid, &type_inst, &inputs, &mut_places, wp);
        }

        // 2. Pure callees with an associated spec function. Multi-result
        // callees are excluded: a tuple-valued spec call is not
        // decomposable by this analysis (there is no tuple projection in
        // its value language); the generic summary's per-component result
        // carriers handle them instead.
        if let Some((spec_fun_id, spec_result_ty)) = self
            .try_as_pure_spec_call(mid, fid, &type_inst)
            .filter(|(_, ty)| !matches!(ty, Type::Tuple(_)))
        {
            let fun_exp = self.mk_callee_closure(mid, fid, &type_inst);
            let aborts = self.builder.mk_aborts_of(fun_exp, inputs.clone());
            self.add_abort(aborts);
            let env = self.builder.global_env();
            // The companion may stem from the pre-inlining derivation stage,
            // where companions are not marked used eagerly; mark it (and its
            // companion callees) used here, where the reference is created.
            env.add_used_spec_fun_transitive(mid.qualified(spec_fun_id));
            let node = env.new_node(self.builder.get_current_loc(), spec_result_ty);
            if !type_inst.is_empty() {
                env.set_node_instantiation(node, type_inst);
            }
            return Ok(Some(SymVal::Value(
                ExpData::Call(
                    node,
                    Operation::SpecFunction(mid, spec_fun_id, MemoryRange::default()),
                    inputs,
                )
                .into_exp(),
            )));
        }

        // 2.5. Place-projection splice for reference-returning pure
        // helpers: returned references which are field/element projections
        // of parameter references become places of this derivation, so
        // reads *and writes* through them compose (e.g. `let (k, v) =
        // e.borrow_kv_mut();` followed by writes through `v`), which the
        // generic summary's `result_of` carriers cannot express.
        let result_comps = result_type.clone().flatten();
        if result_comps.iter().any(|ty| ty.is_reference()) {
            if let Some(spliced) = self.try_splice_ref_result_summary(
                mid.qualified(fid),
                &type_inst,
                &arg_vals,
                &inputs,
                &result_comps,
            ) {
                return Ok(Some(spliced));
            }
        }

        // 3. Generic behavioral summary.
        self.generic_call_summary(
            mid,
            fid,
            &type_inst,
            inputs,
            mut_places,
            &result_type,
            result_is_used,
        )
    }

    /// Computes the vector-intrinsic WP if the callee is a handled
    /// `std::vector` intrinsic.
    fn try_vector_intrinsic(
        &self,
        mid: ModuleId,
        fid: FunId,
        type_inst: &[Type],
        inputs: &[Exp],
    ) -> Option<IntrinsicWp> {
        let output_types = self.vector_output_types(mid, fid, type_inst);
        well_known::vector_intrinsic_wp(
            self.builder.global_env(),
            self.builder,
            mid.qualified(fid),
            type_inst,
            inputs,
            &output_types,
        )
    }

    fn vector_output_types(&self, mid: ModuleId, fid: FunId, type_inst: &[Type]) -> Vec<Type> {
        let env = self.builder.global_env();
        let callee = env.get_function(mid.qualified(fid));
        let mut output_types = callee
            .get_result_type()
            .instantiate(type_inst)
            .flatten()
            .into_iter()
            .map(|ty| ty.skip_reference().clone())
            .collect::<Vec<_>>();
        for param in callee.get_parameters() {
            if param.1.is_mutable_reference() {
                output_types.push(param.1.skip_reference().instantiate(type_inst));
            }
        }
        output_types
    }

    /// Applies an intrinsic WP (`std::vector` or intrinsic-map): aborts,
    /// result values, and `&mut` argument post-state write-back.
    fn finish_intrinsic_wp(
        &mut self,
        mid: ModuleId,
        fid: FunId,
        type_inst: &[Type],
        inputs: &[Exp],
        mut_places: &[(Place, Type)],
        wp: IntrinsicWp,
    ) -> EvalResult {
        self.add_abort(wp.aborts);
        let callee = self.builder.global_env().get_function(mid.qualified(fid));
        let result_count = callee.get_result_type().flatten().len();
        let mut outputs = wp.outputs.into_iter();
        let mut results = vec![];
        for _ in 0..result_count {
            results.push(outputs.next().ok_or(Unsupported)?);
        }
        // Remaining outputs are `&mut` argument post-states, in order.
        for ((place, _), post) in mut_places.iter().zip(outputs) {
            self.write_place(place, post)?;
        }
        // A `&mut`-returning intrinsic (`borrow_mut`) yields a place.
        let result_ref = callee.get_result_type().instantiate(type_inst);
        if result_ref.is_mutable_reference() {
            let Some((place, _)) = mut_places.first() else {
                return Err(Unsupported);
            };
            // `borrow_mut(v, i)` points at element `i` of the vector place.
            let index = inputs.get(1).cloned().ok_or(Unsupported)?;
            return Ok(Some(SymVal::Ref(Place::VecElem(
                Box::new(place.clone()),
                index,
            ))));
        }
        match results.len() {
            0 => Ok(Some(SymVal::Value(self.unit_value()))),
            1 => Ok(Some(SymVal::Value(results.pop().unwrap()))),
            _ => Ok(Some(SymVal::Tuple(
                results.into_iter().map(SymVal::Value).collect(),
            ))),
        }
    }

    /// The pure-spec-call test shared with the bytecode-level inference; see
    /// the module-level [`try_as_pure_spec_call`].
    fn try_as_pure_spec_call(
        &self,
        mid: ModuleId,
        fid: FunId,
        type_inst: &[Type],
    ) -> Option<(SpecFunId, Type)> {
        try_as_pure_spec_call(self.builder.global_env(), mid, fid, type_inst)
    }

    fn mk_callee_closure(&mut self, mid: ModuleId, fid: FunId, type_inst: &[Type]) -> Exp {
        let (fun_exp, _) =
            self.builder
                .mk_closure(mid, fid, type_inst, ClosureMask::empty(), vec![]);
        fun_exp
    }

    /// Generic behavioral summary of a call: results are `result_of`
    /// carriers, `&mut` argument post-states become fresh bound symbols
    /// constrained by a canonical `ensures_of` at assembly, and the
    /// callee's abort behavior is `aborts_of`.
    fn generic_call_summary(
        &mut self,
        mid: ModuleId,
        fid: FunId,
        type_inst: &[Type],
        inputs: Vec<Exp>,
        mut_places: Vec<(Place, Type)>,
        result_type: &Type,
        result_is_used: bool,
    ) -> EvalResult {
        if result_is_used
            && result_type
                .clone()
                .flatten()
                .iter()
                .any(|ty| ty.is_mutable_reference())
        {
            // Returning references from summarized callees is not supported.
            return Err(Unsupported);
        }
        let fun_exp = self.mk_callee_closure(mid, fid, type_inst);
        self.behavioral_summary_over(fun_exp, inputs, mut_places, result_type, result_is_used)
    }

    /// Evaluates an invocation of a function value: recursive evaluation
    /// for lambda values, a generic behavioral summary for closures and
    /// free function-typed variables.
    fn eval_invoke(&mut self, target: &Exp, args: &[Exp]) -> EvalResult {
        let Some(target_val) = self.eval(target)? else {
            return Ok(None);
        };
        let mut arg_vals = vec![];
        for arg in args {
            let Some(v) = self.eval(arg)? else {
                return Ok(None);
            };
            arg_vals.push(v);
        }
        match target_val {
            SymVal::Func(fun_val) => {
                if self.depth >= 32 {
                    return Err(Unsupported);
                }
                self.depth += 1;
                // Bind captures and parameters in a fresh scope over the
                // current store.
                let mut saved = vec![];
                for (sym, val) in &fun_val.captures {
                    saved.push((*sym, self.frame.store.get(sym).cloned()));
                    self.frame.store.insert(*sym, val.clone());
                }
                let value = if arg_vals.len() == 1 {
                    arg_vals.pop().unwrap()
                } else {
                    SymVal::Tuple(arg_vals)
                };
                let pat_saved = self.bind_pattern(&fun_val.pat, Some(value));
                let result = match pat_saved {
                    Ok(pat_saved) => {
                        let r = self.eval(&fun_val.body);
                        self.restore_bindings(pat_saved);
                        r
                    },
                    Err(e) => Err(e),
                };
                self.restore_bindings(saved);
                self.depth -= 1;
                result
            },
            SymVal::Value(fun_exp) => {
                // A closure of a named function or a free function-typed
                // variable: generic behavioral summary over that expression.
                let mut inputs = vec![];
                let mut mut_places = vec![];
                let env = self.builder.global_env();
                let fun_ty = env.get_node_type(fun_exp.as_ref().node_id());
                let Type::Fun(param_ty, result_ty, _) = fun_ty else {
                    return Err(Unsupported);
                };
                let param_tys = param_ty.flatten();
                if param_tys.len() != arg_vals.len() {
                    return Err(Unsupported);
                }
                for (val, ty) in arg_vals.iter().zip(&param_tys) {
                    inputs.push(self.as_value(val.clone())?);
                    if ty.is_mutable_reference() {
                        match val {
                            SymVal::Ref(place) => mut_places.push((place.clone(), ty.clone())),
                            _ => return Err(Unsupported),
                        }
                    }
                }
                self.behavioral_summary_over(fun_exp, inputs, mut_places, &result_ty, true)
            },
            _ => Err(Unsupported),
        }
    }

    /// Generic behavioral summary over an arbitrary function expression.
    fn behavioral_summary_over(
        &mut self,
        fun_exp: Exp,
        inputs: Vec<Exp>,
        mut_places: Vec<(Place, Type)>,
        result_type: &Type,
        result_is_used: bool,
    ) -> EvalResult {
        // A deferred application of a function-typed parameter of the
        // enclosing function: the summary is emitted label-free and its
        // predicates re-resolve in the context which knows the parameter's
        // binding (see `derive_spec_with_captures`). The modifies footprint
        // stays exact modulo the deferred effects, which
        // `DerivedSpec::deferred_applications` exposes for separate
        // accounting.
        let deferred = match fun_exp.as_ref() {
            ExpData::Temporary(_, idx) => self.deferred_fun_param_temps.contains_key(idx),
            // A free `LocalVar` named like an enclosing parameter denotes
            // that parameter (bound locals resolve through the store and
            // never reach here as variables) — the convention the inliner's
            // behavioral-predicate machinery uses throughout.
            ExpData::LocalVar(_, sym) => self.deferred_fun_param_temps.values().any(|s| s == sym),
            _ => false,
        };
        // When the summarized callee provably has no memory effects
        // (memory-pure or abort-only), the summary is single-state: memory
        // cannot change across the call, so the exact modifies footprint is
        // kept and no state labels are introduced (the behavioral
        // predicates get default ranges).
        let memory_free = deferred
            || match fun_exp.as_ref() {
                ExpData::Call(_, Operation::Closure(mid, fid, _), _) => {
                    fun_has_no_memory_effects(self.builder.global_env(), mid.qualified(*fid))
                },
                _ => false,
            };
        let (pre, post) = if memory_free {
            (None, None)
        } else {
            // The summarized callee may modify arbitrary global memory.
            self.modifies_exact = false;
            let (pre, post) = self.advance_label()?;
            (Some(pre), Some(post))
        };
        let aborts =
            self.builder
                .mk_aborts_of_with_state(fun_exp.clone(), inputs.clone(), pre, None);
        self.add_abort(aborts);
        let result_tys = result_type.clone().flatten();
        // The `result_of` carrier is typed with the full (value-level)
        // result type; for a multi-result callee that is the tuple type,
        // which the component extraction destructures (and whose tuple
        // node type the Boogie backend's block translation projects on).
        let carrier_ty = Type::tuple(
            result_tys
                .iter()
                .map(|ty| ty.skip_reference().clone())
                .collect(),
        );
        // Exact result values routed from the callee's attached functional
        // ensures or its body value summary (the result analog of the
        // `&mut` post-value routing below), per value-level component;
        // `result_of` carriers otherwise.
        let exact_results = self.callee_result_values(&fun_exp, &inputs);
        let mut results = vec![];
        for i in 0..result_tys.len() {
            let exact = (!result_tys[i].is_reference())
                .then(|| exact_results.get(&i))
                .flatten();
            let carrier = if let Some(value) = exact {
                value.clone()
            } else if result_tys.len() == 1 {
                self.builder.mk_result_of_with_state(
                    fun_exp.clone(),
                    inputs.clone(),
                    &carrier_ty,
                    pre,
                    post,
                )
            } else {
                self.builder.mk_result_of_at_with_state(
                    fun_exp.clone(),
                    inputs.clone(),
                    &carrier_ty,
                    i,
                    result_tys.len(),
                    pre,
                    post,
                )
            };
            results.push(carrier);
        }
        // Post-state values per `&mut` place: exact values routed from the
        // callee's functional ensures or its body value summary where
        // available, fresh symbols (constrained by the canonical
        // `ensures_of`) otherwise.
        let post_values = self.callee_post_values(&fun_exp, &inputs, &mut_places);
        let mut posts = vec![];
        for (pos, (place, ty)) in mut_places.iter().enumerate() {
            let value_ty = ty.skip_reference().clone();
            if let Some(value) = post_values.get(&pos) {
                self.write_place(place, value.clone())?;
                posts.push(PostSlot::Value(value.clone()));
            } else {
                let sym = self.fresh_sym("m");
                posts.push(PostSlot::Sym(sym, value_ty.clone()));
                let var = self.builder.mk_local_by_sym(sym, value_ty);
                self.write_place(place, var)?;
            }
        }
        self.call_records.push(CallRecord {
            guard: self.path_cond(),
            fun_exp,
            inputs,
            results: results.clone(),
            posts,
            pre,
            post,
            deferred,
        });
        if !result_is_used {
            return Ok(Some(SymVal::Value(self.unit_value())));
        }
        match results.len() {
            0 => Ok(Some(SymVal::Value(self.unit_value()))),
            1 => Ok(Some(SymVal::Value(results.pop().unwrap()))),
            _ => Ok(Some(SymVal::Tuple(
                results.into_iter().map(SymVal::Value).collect(),
            ))),
        }
    }

    // =============================================================================================
    // Post-value routing for `&mut` arguments of summarized calls

    /// The exact post-state values for the `&mut` argument places of a
    /// summarized callee, keyed by position in `mut_places`: per place,
    /// routed from (a) the callee's attached functional ensures
    /// (`p == E(old(p), args)`, or complete per-field conjuncts) or (b)
    /// the memoized value summary of the callee's body. Slots without a
    /// source stay absent; their post-states become fresh symbols
    /// constrained only by the canonical `ensures_of`.
    ///
    /// Routing requires a known callee — a capture-free closure — and
    /// non-overlapping `&mut` places: places sharing a root (or with
    /// global roots, whose addresses this check does not compare) are
    /// conservatively treated as aliasing, since the sequential
    /// write-back of per-place values would not reflect their
    /// interaction.
    fn callee_post_values(
        &mut self,
        fun_exp: &Exp,
        inputs: &[Exp],
        mut_places: &[(Place, Type)],
    ) -> BTreeMap<usize, Exp> {
        if mut_places.is_empty() {
            return BTreeMap::new();
        }
        let ExpData::Call(id, Operation::Closure(mid, fid, mask), _) = fun_exp.as_ref() else {
            return BTreeMap::new();
        };
        if *mask != ClosureMask::empty() {
            return BTreeMap::new();
        }
        let env = self.builder.global_env();
        let callee_qid = mid.qualified(*fid);
        let type_inst = env.get_node_instantiation(*id);
        // Aliasing fallback.
        let roots: Vec<Option<Symbol>> = mut_places.iter().map(|(p, _)| place_root(p)).collect();
        for (i, a) in roots.iter().enumerate() {
            for b in &roots[i + 1..] {
                if a.is_none() || b.is_none() || a == b {
                    return BTreeMap::new();
                }
            }
        }
        // Positions of the `&mut` parameters, aligning `mut_places` with
        // parameter indices.
        let mut_param_indices: Vec<usize> = env
            .get_function(callee_qid)
            .get_parameters()
            .iter()
            .enumerate()
            .filter(|(_, p)| p.1.is_mutable_reference())
            .map(|(i, _)| i)
            .collect();
        if mut_param_indices.len() != mut_places.len() {
            return BTreeMap::new();
        }
        // (a) Functional ensures of the attached specification.
        let mut by_param = self.callee_spec_post_values(callee_qid, &type_inst, inputs);
        // (b) Body value summary for the remaining slots.
        if mut_param_indices
            .iter()
            .any(|idx| !by_param.contains_key(idx))
        {
            if let Some(summary) = self.callee_body_value_summary(callee_qid, &type_inst) {
                let env = self.builder.global_env();
                for (idx, value) in &summary.mut_values {
                    if !by_param.contains_key(idx) {
                        by_param.insert(
                            *idx,
                            substitute_placeholders(env, value, &summary.param_syms, inputs),
                        );
                    }
                }
            }
        }
        mut_param_indices
            .into_iter()
            .enumerate()
            .filter_map(|(pos, idx)| by_param.remove(&idx).map(|e| (pos, e)))
            .collect()
    }

    /// The exact result values of a summarized callee, keyed by result
    /// component — the result analog of [`Self::callee_post_values`]:
    /// routed from (a) the callee's attached functional ensures
    /// `result == E` (per component `result_i == E_i`) or (b) the memoized
    /// value summary of the callee's body. Routing requires a known callee
    /// (a capture-free closure); the caller restricts consumption to
    /// value-level components. Components without a source stay absent and
    /// keep their `result_of` carriers.
    fn callee_result_values(&mut self, fun_exp: &Exp, inputs: &[Exp]) -> BTreeMap<usize, Exp> {
        let ExpData::Call(id, Operation::Closure(mid, fid, mask), _) = fun_exp.as_ref() else {
            return BTreeMap::new();
        };
        if *mask != ClosureMask::empty() {
            return BTreeMap::new();
        }
        let env = self.builder.global_env();
        let callee_qid = mid.qualified(*fid);
        let type_inst = env.get_node_instantiation(*id);
        let mut values = self.callee_spec_result_values(callee_qid, &type_inst, inputs);
        if let btree_map::Entry::Vacant(entry) = values.entry(0) {
            if let Some(CalleeValueSummary {
                param_syms,
                result_value: Some(value),
                ..
            }) = self.callee_body_value_summary(callee_qid, &type_inst)
            {
                entry.insert(substitute_placeholders(
                    self.builder.global_env(),
                    &value,
                    &param_syms,
                    inputs,
                ));
            }
        }
        values
    }

    /// Extracts exact result values from the callee's attached
    /// specification, keyed by result component: unconditional `ensures`
    /// conjuncts `result == E` (or `result_i == E_i` for a multi-result
    /// callee) with each `E` a pure single-state expression over the
    /// parameters (the `&mut` ones only under `old(..)`). Conditions
    /// marked `[concrete]` are invisible to callers and skipped; consuming
    /// `[abstract]` conditions of opaque callees rides the same trust the
    /// prover places in opaque specifications.
    fn callee_spec_result_values(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
        inputs: &[Exp],
    ) -> BTreeMap<usize, Exp> {
        use crate::ast::ConditionKind;
        let env = self.builder.global_env();
        let callee = env.get_function(callee_qid);
        let mut_params: BTreeSet<usize> = callee
            .get_parameters()
            .iter()
            .enumerate()
            .filter(|(_, p)| p.1.is_mutable_reference())
            .map(|(i, _)| i)
            .collect();
        let concrete_prop = env
            .symbol_pool()
            .make(crate::pragmas::CONDITION_CONCRETE_PROP);
        let mut found: BTreeMap<usize, Exp> = BTreeMap::new();
        {
            let spec = callee.get_spec();
            for cond in spec
                .conditions
                .iter()
                .filter(|c| c.kind == ConditionKind::Ensures)
            {
                if cond.properties.contains_key(&concrete_prop) {
                    continue;
                }
                for conjunct in exp_conjuncts(&cond.exp) {
                    let ExpData::Call(_, Operation::Eq, eq_args) = conjunct.as_ref() else {
                        continue;
                    };
                    if eq_args.len() != 2 {
                        continue;
                    }
                    for (lhs, rhs) in [(&eq_args[0], &eq_args[1]), (&eq_args[1], &eq_args[0])] {
                        if let ExpData::Call(_, Operation::Result(i), _) = lhs.as_ref() {
                            if spec_value_over_prestate(env, rhs, &mut_params) {
                                found.entry(*i).or_insert_with(|| rhs.clone());
                            }
                        }
                    }
                }
            }
        }
        found
            .into_iter()
            .map(|(i, value)| (i, self.instantiate_spec_value(&value, type_inst, inputs)))
            .collect()
    }

    /// Extracts exact `&mut`-parameter post values from the callee's
    /// attached specification, keyed by parameter index: an unconditional
    /// `ensures` conjunct `p == E` — or a complete set of per-field
    /// conjuncts `p.f == E_f`, composed via field update over the
    /// parameter's pre-state — where each right-hand side is a pure
    /// single-state expression over the parameters with the `&mut` ones
    /// only under `old(..)`. Conditions marked `[concrete]` are invisible
    /// to callers and skipped; consuming `[abstract]` conditions of opaque
    /// callees rides the same trust the prover places in opaque
    /// specifications.
    fn callee_spec_post_values(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
        inputs: &[Exp],
    ) -> BTreeMap<usize, Exp> {
        use crate::ast::ConditionKind;
        let env = self.builder.global_env();
        let callee = env.get_function(callee_qid);
        let params = callee.get_parameters();
        let mut_params: BTreeSet<usize> = params
            .iter()
            .enumerate()
            .filter(|(_, p)| p.1.is_mutable_reference())
            .map(|(i, _)| i)
            .collect();
        if mut_params.is_empty() {
            return BTreeMap::new();
        }
        let concrete_prop = env
            .symbol_pool()
            .make(crate::pragmas::CONDITION_CONCRETE_PROP);
        // Collect qualifying candidate conjuncts: whole-parameter values
        // and per-field values (the latter keyed by the selected struct
        // and field, to be validated against the parameter type below).
        let mut whole: BTreeMap<usize, Exp> = BTreeMap::new();
        type FieldKey = (ModuleId, StructId, crate::model::FieldId);
        let mut by_field: BTreeMap<usize, BTreeMap<FieldKey, Exp>> = BTreeMap::new();
        {
            let spec = callee.get_spec();
            for cond in spec
                .conditions
                .iter()
                .filter(|c| c.kind == ConditionKind::Ensures)
            {
                if cond.properties.contains_key(&concrete_prop) {
                    continue;
                }
                for conjunct in exp_conjuncts(&cond.exp) {
                    let ExpData::Call(_, Operation::Eq, eq_args) = conjunct.as_ref() else {
                        continue;
                    };
                    if eq_args.len() != 2 {
                        continue;
                    }
                    for (lhs, rhs) in [(&eq_args[0], &eq_args[1]), (&eq_args[1], &eq_args[0])] {
                        // `p == E` (the builder may phrase the value
                        // mention of a `&mut` parameter as a dereference).
                        if let Some(idx) = param_mention(lhs) {
                            if mut_params.contains(&idx)
                                && spec_value_over_prestate(env, rhs, &mut_params)
                            {
                                whole.entry(idx).or_insert_with(|| rhs.clone());
                            }
                            continue;
                        }
                        // `p.f == E_f`
                        if let ExpData::Call(_, Operation::Select(mid, sid, fid), sel_args) =
                            lhs.as_ref()
                        {
                            if let Some(idx) = param_mention(&sel_args[0]) {
                                if mut_params.contains(&idx)
                                    && spec_value_over_prestate(env, rhs, &mut_params)
                                {
                                    by_field
                                        .entry(idx)
                                        .or_default()
                                        .entry((*mid, *sid, *fid))
                                        .or_insert_with(|| rhs.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        // Whole-parameter conjuncts win; per-field conjuncts compose only
        // when they cover every field of the parameter's struct type
        // (variant-less), so the composed value is fully determined.
        let mut result = BTreeMap::new();
        for idx in mut_params {
            if let Some(value) = whole.get(&idx) {
                let value = value.clone();
                result.insert(idx, self.instantiate_spec_value(&value, type_inst, inputs));
                continue;
            }
            let Some(field_values) = by_field.get(&idx) else {
                continue;
            };
            let param_ty = params[idx].1.skip_reference().instantiate(type_inst);
            let Type::Struct(s_mid, s_sid, s_inst) = &param_ty else {
                continue;
            };
            let env = self.builder.global_env();
            let struct_env = env.get_struct(s_mid.qualified(*s_sid));
            if struct_env.has_variants() {
                continue;
            }
            let field_ids: Vec<crate::model::FieldId> =
                struct_env.get_fields().map(|f| f.get_id()).collect();
            if !field_ids
                .iter()
                .all(|fid| field_values.contains_key(&(*s_mid, *s_sid, *fid)))
            {
                continue;
            }
            let field_exps: Vec<(crate::model::FieldId, Exp)> = field_ids
                .iter()
                .map(|fid| {
                    (
                        *fid,
                        field_values
                            .get(&(*s_mid, *s_sid, *fid))
                            .expect("field covered")
                            .clone(),
                    )
                })
                .collect();
            let (s_mid, s_sid, s_inst) = (*s_mid, *s_sid, s_inst.clone());
            let mut value = inputs[idx].clone();
            for (fid, exp) in field_exps {
                let field_value = self.instantiate_spec_value(&exp, type_inst, inputs);
                let env = self.builder.global_env();
                let struct_env = env.get_struct(s_mid.qualified(s_sid));
                let field_env = struct_env.get_field(fid);
                value = self
                    .builder
                    .mk_field_update(&field_env, &s_inst, value, field_value);
            }
            result.insert(idx, value);
        }
        result
    }

    /// Instantiates a qualifying spec-condition expression over a call's
    /// pre-state inputs: `old(..)` wrappers are stripped first (all
    /// content is pre-state by the qualification in
    /// [`spec_value_over_prestate`]), then parameters are substituted by
    /// the input expressions and generic types instantiated. Dereference
    /// wrappers around the substituted values become identities (the
    /// inputs are value-level) and are dropped — they have no spec form.
    fn instantiate_spec_value(&mut self, exp: &Exp, type_inst: &[Type], inputs: &[Exp]) -> Exp {
        let env = self.builder.global_env();
        let stripped = strip_all_olds(exp);
        let mut replacer = |_: NodeId, target: RewriteTarget| match target {
            RewriteTarget::Temporary(idx) => inputs.get(idx).cloned(),
            RewriteTarget::LocalVar(_) => None,
        };
        let mut rewriter = ExpRewriter::new(env, &mut replacer).set_type_args(type_inst);
        let substituted = rewriter.rewrite_exp(stripped);
        struct DerefStrip<'a> {
            env: &'a GlobalEnv,
        }
        impl ExpRewriterFunctions for DerefStrip<'_> {
            fn rewrite_call(&mut self, _id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
                if matches!(oper, Operation::Deref)
                    && args
                        .first()
                        .is_some_and(|a| !self.env.get_node_type(a.node_id()).is_reference())
                {
                    Some(args[0].clone())
                } else {
                    None
                }
            }
        }
        DerefStrip { env }.rewrite_exp(substituted)
    }

    /// The memoized value summary of the callee's body (source (b) of the
    /// post-value routing); `None` when the body is outside the derivable
    /// fragment, the callee is opaque (its attached specification is the
    /// authoritative contract), native or inline, or its derivation is
    /// already in progress (a recursive callee's effect is not a
    /// closed-form value).
    fn callee_body_value_summary(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
    ) -> Option<CalleeValueSummary> {
        if !has_exact_move_value_model(self.builder.global_env(), callee_qid, &mut BTreeSet::new())
        {
            return None;
        }
        let cache = summary_cache(self.builder.global_env());
        let key = (callee_qid, type_inst.to_vec());
        if let Some(entry) = cache.entries.borrow().get(&key) {
            return entry.clone();
        }
        if !cache.in_progress.borrow_mut().insert(callee_qid) {
            return None;
        }
        let computed = self.compute_callee_body_value_summary(callee_qid, type_inst);
        cache.in_progress.borrow_mut().remove(&callee_qid);
        cache.entries.borrow_mut().insert(key, computed.clone());
        computed
    }

    /// Prepares a callee's body for a placeholder-parameter derivation:
    /// generics instantiated and parameter temporaries bound to fresh
    /// placeholder symbols. Shared by the value and reference-result
    /// summary computations. `None` for callees whose body is not the
    /// authoritative contract (native, inline, opaque) or absent.
    fn callee_body_over_placeholders(
        &self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
    ) -> Option<(Vec<Symbol>, Vec<(Symbol, Type)>, Exp)> {
        let env = self.builder.global_env();
        let callee = env.get_function(callee_qid);
        if callee.is_native() || callee.is_inline() || callee.is_opaque() {
            return None;
        }
        let def = callee.get_def()?.clone();
        let params = callee.get_parameters();
        let param_syms: Vec<Symbol> = (0..params.len())
            .map(|i| env.symbol_pool().make(&format!("$cs_p{}", i)))
            .collect();
        let param_decls: Vec<(Symbol, Type)> = params
            .iter()
            .zip(&param_syms)
            .map(|(Parameter(_, ty, _), sym)| (*sym, ty.instantiate(type_inst)))
            .collect();
        // The body references its parameters as temporaries; bind them to
        // the placeholder symbols and instantiate generics.
        let body = {
            // The rewriter instantiates node types itself (`set_type_args`
            // runs before the replacer); the placeholder reuses the node's
            // already-instantiated type.
            let mut replacer = |id: NodeId, target: RewriteTarget| match target {
                RewriteTarget::Temporary(idx) if idx < param_syms.len() => {
                    let node = env.new_node(env.get_node_loc(id), env.get_node_type(id));
                    Some(ExpData::LocalVar(node, param_syms[idx]).into_exp())
                },
                _ => None,
            };
            ExpRewriter::new(env, &mut replacer)
                .set_type_args(type_inst)
                .rewrite_exp(def)
        };
        Some((param_syms, param_decls, body))
    }

    fn compute_callee_body_value_summary(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
    ) -> Option<CalleeValueSummary> {
        let (param_syms, param_decls, body) =
            self.callee_body_over_placeholders(callee_qid, type_inst)?;
        let result_ty = self
            .builder
            .global_env()
            .get_function(callee_qid)
            .get_result_type()
            .instantiate(type_inst);
        let var_types: BTreeMap<Symbol, Type> = param_decls.iter().cloned().collect();
        let derived = derive_spec(self.builder, &param_decls, &var_types, &result_ty, &body)?;
        let mut_syms: BTreeSet<Symbol> = param_decls
            .iter()
            .filter(|(_, ty)| ty.is_mutable_reference())
            .map(|(sym, _)| *sym)
            .collect();
        let env = self.builder.global_env();
        let mut mut_values = BTreeMap::new();
        if let Some(values) = &derived.mut_param_values {
            for (idx, (sym, ty)) in param_decls.iter().enumerate() {
                if !ty.is_mutable_reference() {
                    continue;
                }
                let Some((_, value)) = values.iter().find(|(s, _)| s == sym) else {
                    continue;
                };
                // A plain (un-`old`-wrapped) self-mention is a post-state
                // reference — the callee itself accumulates through another
                // call — and not a value; likewise, state-dependent values
                // are not routable.
                if mentions_syms_outside_old(value, &mut_syms)
                    || !exps_are_pure_single_state(env, [value])
                {
                    continue;
                }
                // Canonicalize to plain pre-state placeholder mentions.
                mut_values.insert(idx, strip_all_olds(value));
            }
        }
        // The exact result value, for a single value-level result phrased
        // over the pre-state placeholders (same qualification as the `&mut`
        // post values). Only closed-form values route: a value embedding a
        // behavioral carrier or a closure references the summarized body's
        // callees directly, which the consuming context may not even be
        // allowed to name (a private native of the callee's module).
        let single_value_result = !result_ty.is_reference()
            && !matches!(result_ty, Type::Tuple(_))
            && !result_ty.is_unit();
        let is_closed_form = |value: &Exp| {
            !value.any(&mut |e| {
                matches!(
                    e,
                    ExpData::Call(_, Operation::Behavior(..) | Operation::Closure(..), _)
                )
            })
        };
        let result_value = derived
            .results
            .as_ref()
            .filter(|_| single_value_result)
            .and_then(|values| match values.as_slice() {
                [value]
                    if !mentions_syms_outside_old(value, &mut_syms)
                        && exps_are_pure_single_state(env, [value])
                        && is_closed_form(value) =>
                {
                    Some(strip_all_olds(value))
                },
                _ => None,
            });
        if mut_values.is_empty() && result_value.is_none() {
            return None;
        }
        Some(CalleeValueSummary {
            param_syms,
            mut_values,
            result_value,
        })
    }

    // =============================================================================================
    // Reference-result place projection for pure helpers

    /// The memoized reference-result summary of a callee (step 2.5 of
    /// [`Self::eval_move_function_call`]); `None` under the same conditions
    /// as [`Self::callee_body_value_summary`], or when the body is not a
    /// pure projection helper (see [`CalleeRefResultSummary`]).
    fn callee_ref_result_summary(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
    ) -> Option<CalleeRefResultSummary> {
        let cache = summary_cache(self.builder.global_env());
        let key = (callee_qid, type_inst.to_vec());
        if let Some(entry) = cache.ref_entries.borrow().get(&key) {
            return entry.clone();
        }
        if !cache.in_progress.borrow_mut().insert(callee_qid) {
            return None;
        }
        let computed = self.compute_callee_ref_result_summary(callee_qid, type_inst);
        cache.in_progress.borrow_mut().remove(&callee_qid);
        cache.ref_entries.borrow_mut().insert(key, computed.clone());
        computed
    }

    fn compute_callee_ref_result_summary(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
    ) -> Option<CalleeRefResultSummary> {
        let (param_syms, param_decls, body) =
            self.callee_body_over_placeholders(callee_qid, type_inst)?;
        let result_tys = self
            .builder
            .global_env()
            .get_function(callee_qid)
            .get_result_type()
            .instantiate(type_inst)
            .flatten();
        if result_tys.is_empty() {
            return None;
        }
        let var_types: BTreeMap<Symbol, Type> = param_decls.iter().cloned().collect();
        // Evaluate the body in a fresh deriver and inspect the terminal
        // symbolic value directly: the assembled specification has no
        // place form for returned references.
        let (terminal, aborts) = {
            let mut deriver = make_deriver(self.builder, &param_decls, &[], &var_types);
            let terminal = deriver.eval(&body).ok()??;
            // The helper must be pure: a single fall-through return, no
            // memory effects, no embedded behavioral call summaries, and
            // unwritten `&mut` parameters (their current value is still
            // the pre-state `old(p)`).
            if !deriver.returns.is_empty()
                || !deriver.effects.is_empty()
                || !deriver.modifies.is_empty()
                || !deriver.modifies_exact
                || !deriver.call_records.is_empty()
            {
                return None;
            }
            for (sym, ty) in &param_decls {
                if !ty.is_mutable_reference() {
                    continue;
                }
                let unwritten = matches!(
                    deriver.frame.store.get(sym),
                    Some(SymVal::Value(cur))
                        if matches!(
                            cur.as_ref(),
                            ExpData::Call(_, Operation::Old, old_args)
                                if matches!(
                                    old_args[0].as_ref(),
                                    ExpData::LocalVar(_, s) if s == sym))
                );
                if !unwritten {
                    return None;
                }
            }
            let aborts: Vec<Exp> = deriver.aborts.iter().map(strip_all_olds).collect();
            (terminal, aborts)
        };
        let comps = match terminal {
            SymVal::Tuple(vals) => vals,
            val => vec![val],
        };
        if comps.len() != result_tys.len() {
            return None;
        }
        let param_index: BTreeMap<Symbol, usize> = param_syms
            .iter()
            .enumerate()
            .map(|(i, sym)| (*sym, i))
            .collect();
        let mut results = vec![];
        // The expressions embedded in the summary (canonicalized to plain
        // pre-state placeholder mentions): validated below for purity and
        // closedness over the placeholders.
        let mut embedded: Vec<Exp> = vec![];
        for (val, ty) in comps.into_iter().zip(&result_tys) {
            match val {
                SymVal::Ref(place) if ty.is_reference() => {
                    let (root, steps) = place_projection(&place)?;
                    let param = *param_index.get(&root)?;
                    for step in &steps {
                        if let ProjStep::VecElem(index) = step {
                            embedded.push(index.clone());
                        }
                    }
                    results.push(RefResultComp::Place { param, steps });
                },
                SymVal::Value(exp) if !ty.is_reference() => {
                    let exp = strip_all_olds(&exp);
                    embedded.push(exp.clone());
                    results.push(RefResultComp::Value(exp));
                },
                _ => return None,
            }
        }
        let env = self.builder.global_env();
        if !exps_are_pure_single_state(env, embedded.iter().chain(&aborts)) {
            return None;
        }
        // Every embedded expression must be closed over the placeholders:
        // other free symbols would be fresh cells of the summary
        // derivation, meaningless at a call.
        let placeholders: BTreeSet<Symbol> = param_syms.iter().copied().collect();
        if embedded
            .iter()
            .chain(&aborts)
            .any(|e| !e.free_vars().is_subset(&placeholders))
        {
            return None;
        }
        Some(CalleeRefResultSummary {
            param_syms,
            results,
            aborts,
        })
    }

    /// Splices a callee's reference-result summary at a call: records the
    /// substituted abort conditions and produces the result components,
    /// with each projected reference re-rooted at the corresponding
    /// argument's place. An argument available only as a value-level
    /// reference expression (e.g. an immutable reference parameter of the
    /// analyzed body) is bound to a fresh cell first, so its projections
    /// are read-only places over that cell. `None` (before any state
    /// change) falls through to the generic summary.
    fn try_splice_ref_result_summary(
        &mut self,
        callee_qid: QualifiedId<FunId>,
        type_inst: &[Type],
        arg_vals: &[SymVal],
        inputs: &[Exp],
        result_tys: &[Type],
    ) -> Option<SymVal> {
        let summary = self.callee_ref_result_summary(callee_qid, type_inst)?;
        if summary.param_syms.len() != inputs.len() {
            return None;
        }
        // Validate before mutating any state. A `&mut` result must splice
        // as a genuine place: rooting it at a value copy would silently
        // drop writes (per borrow rules its root argument is a `&mut`
        // place already; this is defensive).
        for (comp, ty) in summary.results.iter().zip(result_tys) {
            if let RefResultComp::Place { param, .. } = comp {
                match arg_vals.get(*param)? {
                    SymVal::Ref(_) => {},
                    SymVal::Value(_) if !ty.is_mutable_reference() => {},
                    _ => return None,
                }
            }
        }
        let env = self.builder.global_env();
        let aborts: Vec<Exp> = summary
            .aborts
            .iter()
            .map(|a| substitute_placeholders(env, a, &summary.param_syms, inputs))
            .collect();
        for abort in aborts {
            self.add_abort(abort);
        }
        // Roots per projected parameter; value-level arguments get a fresh
        // cell (shared by the parameter's projections).
        let mut roots: BTreeMap<usize, Place> = BTreeMap::new();
        let mut comps = vec![];
        for comp in &summary.results {
            match comp {
                RefResultComp::Place { param, steps } => {
                    if !roots.contains_key(param) {
                        let root = match &arg_vals[*param] {
                            SymVal::Ref(place) => place.clone(),
                            SymVal::Value(_) => {
                                let cell = self.fresh_sym("cell");
                                self.frame
                                    .store
                                    .insert(cell, SymVal::Value(inputs[*param].clone()));
                                Place::Local(cell)
                            },
                            _ => unreachable!("validated above"),
                        };
                        roots.insert(*param, root);
                    }
                    let mut place = roots.get(param).expect("just inserted").clone();
                    for step in steps {
                        place = match step {
                            ProjStep::Field(sel) => Place::Field(Box::new(place), sel.clone()),
                            ProjStep::VecElem(index) => {
                                let env = self.builder.global_env();
                                Place::VecElem(
                                    Box::new(place),
                                    substitute_placeholders(
                                        env,
                                        index,
                                        &summary.param_syms,
                                        inputs,
                                    ),
                                )
                            },
                        };
                    }
                    comps.push(SymVal::Ref(place));
                },
                RefResultComp::Value(exp) => {
                    let env = self.builder.global_env();
                    comps.push(SymVal::Value(substitute_placeholders(
                        env,
                        exp,
                        &summary.param_syms,
                        inputs,
                    )));
                },
            }
        }
        Some(
            if comps.len() == 1 {
                comps.pop().unwrap()
            } else {
                SymVal::Tuple(comps)
            },
        )
    }

    // =============================================================================================
    // Match

    /// Evaluates a match as a chain of tests over the discriminant value,
    /// arm by arm.
    fn eval_arms(&mut self, disc: &Exp, arms: &[MatchArm]) -> EvalResult {
        let Some((arm, rest)) = arms.split_first() else {
            // No arm matched: the match aborts. For exhaustive matches this
            // path is unreachable and the guarded condition is vacuous.
            self.add_abort(self.builder.mk_bool_const(true));
            self.frame.diverged = true;
            return Ok(None);
        };
        let test = self.pattern_test(&arm.pattern, disc)?;
        // Evaluate the arm body with bindings; the guard condition (if any)
        // is evaluated under the pattern test and becomes part of the
        // branch condition.
        let saved_frame = self.frame.clone();
        if let Some(test) = &test {
            self.path.push(test.clone());
        }
        let saved_bindings = self.bind_match_pattern(&arm.pattern, disc)?;
        let guard_val = match &arm.condition {
            Some(guard) => match self.eval(guard)? {
                Some(v) => Some(self.as_value(v)?),
                None => None,
            },
            None => None,
        };
        if let Some(g) = &guard_val {
            self.path.push(g.clone());
        }
        let arm_val = self.eval(&arm.body)?;
        if guard_val.is_some() {
            self.path.pop();
        }
        self.restore_bindings(saved_bindings);
        if test.is_some() {
            self.path.pop();
        }
        let arm_frame = std::mem::replace(&mut self.frame, saved_frame);
        // Full condition for taking this arm.
        let cond = match (test, guard_val) {
            (Some(t), Some(g)) => Some(self.builder.mk_and(t, g)),
            (Some(t), None) => Some(t),
            (None, Some(g)) => Some(g),
            (None, None) => None,
        };
        let Some(cond) = cond else {
            // Unconditional catch-all arm.
            self.frame = arm_frame;
            return Ok(arm_val);
        };
        let not_cond = self.builder.mk_not(cond.clone());
        self.path.push(not_cond);
        let rest_val = self.eval_arms(disc, rest)?;
        self.path.pop();
        let rest_frame = self.frame.clone();
        self.join(cond, arm_frame, arm_val, rest_frame, rest_val)
    }

    /// The boolean test whether the pattern matches the value; `None` for
    /// irrefutable patterns.
    fn pattern_test(&mut self, pat: &Pattern, value: &Exp) -> Res<Option<Exp>> {
        match pat {
            Pattern::Var(..) | Pattern::Wildcard(_) => Ok(None),
            Pattern::Tuple(..) => Err(Unsupported),
            Pattern::Struct(_, struct_qid, variant, sub_pats) => {
                let mut conds = vec![];
                if let Some(variant) = variant {
                    let test = {
                        let env = self.builder.global_env();
                        let struct_env = env.get_struct(struct_qid.to_qualified_id());
                        self.builder
                            .mk_variant_test(&struct_env, *variant, value.clone())
                    };
                    conds.push(test);
                }
                let fields = {
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(struct_qid.to_qualified_id());
                    struct_env
                        .get_fields_optional_variant(*variant)
                        .map(|f| f.get_id())
                        .collect::<Vec<_>>()
                };
                if fields.len() != sub_pats.len() {
                    return Err(Unsupported);
                }
                for (fid, sub) in fields.iter().zip(sub_pats) {
                    if matches!(sub, Pattern::Var(..) | Pattern::Wildcard(_)) {
                        continue;
                    }
                    let field_val = {
                        let env = self.builder.global_env();
                        let struct_env = env.get_struct(struct_qid.to_qualified_id());
                        let field_env = struct_env.get_field(*fid);
                        self.builder
                            .mk_field_select(&field_env, &struct_qid.inst, value.clone())
                    };
                    if let Some(sub_test) = self.pattern_test(sub, &field_val)? {
                        conds.push(sub_test);
                    }
                }
                Ok(self.builder.mk_join_bool(Operation::And, conds.into_iter()))
            },
            Pattern::LiteralValue(_, lit) => {
                let env = self.builder.global_env();
                let value_ty = env.get_node_type(value.as_ref().node_id());
                let lit_exp = ExpData::Value(
                    env.new_node(self.builder.get_current_loc(), value_ty),
                    lit.clone(),
                )
                .into_exp();
                Ok(Some(self.builder.mk_eq(value.clone(), lit_exp)))
            },
            Pattern::Range(_, lo, hi, inclusive) => {
                let env = self.builder.global_env();
                let value_ty = env.get_node_type(value.as_ref().node_id());
                let mut conds = vec![];
                if let Some(lo) = lo {
                    let lo_exp = ExpData::Value(
                        env.new_node(self.builder.get_current_loc(), value_ty.clone()),
                        lo.clone(),
                    )
                    .into_exp();
                    conds.push(
                        self.builder
                            .mk_bool_call(Operation::Ge, vec![value.clone(), lo_exp]),
                    );
                }
                if let Some(hi) = hi {
                    let hi_exp = ExpData::Value(
                        env.new_node(self.builder.get_current_loc(), value_ty),
                        hi.clone(),
                    )
                    .into_exp();
                    let cmp = if *inclusive {
                        Operation::Le
                    } else {
                        Operation::Lt
                    };
                    conds.push(self.builder.mk_bool_call(cmp, vec![value.clone(), hi_exp]));
                }
                Ok(self.builder.mk_join_bool(Operation::And, conds.into_iter()))
            },
            Pattern::Error(_) => Err(Unsupported),
        }
    }

    /// Binds the variables of a match pattern to selections over the
    /// discriminant (without emitting variant aborts — mismatch falls
    /// through to the next arm).
    fn bind_match_pattern(
        &mut self,
        pat: &Pattern,
        value: &Exp,
    ) -> Res<Vec<(Symbol, Option<SymVal>)>> {
        let mut saved = vec![];
        self.bind_match_pattern_rec(pat, value, &mut saved)?;
        Ok(saved)
    }

    fn bind_match_pattern_rec(
        &mut self,
        pat: &Pattern,
        value: &Exp,
        saved: &mut Vec<(Symbol, Option<SymVal>)>,
    ) -> Res<()> {
        match pat {
            Pattern::Var(_, sym) => {
                saved.push((*sym, self.frame.store.get(sym).cloned()));
                self.frame.store.insert(*sym, SymVal::Value(value.clone()));
                Ok(())
            },
            Pattern::Wildcard(_) | Pattern::LiteralValue(..) | Pattern::Range(..) => Ok(()),
            Pattern::Struct(_, struct_qid, variant, sub_pats) => {
                let fields = {
                    let env = self.builder.global_env();
                    let struct_env = env.get_struct(struct_qid.to_qualified_id());
                    struct_env
                        .get_fields_optional_variant(*variant)
                        .map(|f| f.get_id())
                        .collect::<Vec<_>>()
                };
                if fields.len() != sub_pats.len() {
                    return Err(Unsupported);
                }
                for (fid, sub) in fields.iter().zip(sub_pats) {
                    let field_val = {
                        let env = self.builder.global_env();
                        let struct_env = env.get_struct(struct_qid.to_qualified_id());
                        let field_env = struct_env.get_field(*fid);
                        self.builder
                            .mk_field_select(&field_env, &struct_qid.inst, value.clone())
                    };
                    self.bind_match_pattern_rec(sub, &field_val, saved)?;
                }
                Ok(())
            },
            Pattern::Tuple(..) | Pattern::Error(_) => Err(Unsupported),
        }
    }

    // =============================================================================================
    // Returns and assembly

    /// Records a normal return with the given (possibly tuple) value.
    fn record_return(&mut self, val: SymVal) -> Res<()> {
        let results = match val {
            SymVal::Tuple(vals) => vals
                .into_iter()
                .map(|v| self.as_value(v))
                .collect::<Res<Vec<_>>>()?,
            SymVal::Value(e) => {
                if e.as_ref().is_unit_exp() {
                    vec![]
                } else {
                    vec![e]
                }
            },
            SymVal::Ref(ref place) => vec![self.read_place(place)?],
            SymVal::Func(_) => return Err(Unsupported),
        };
        let mut param_state = vec![];
        for info in &self.params {
            if info.kind != ParamKind::Value {
                match self.frame.store.get(&info.sym) {
                    Some(SymVal::Value(e)) => param_state.push((info.sym, e.clone())),
                    _ => return Err(Unsupported),
                }
            }
        }
        self.returns.push(ReturnRecord {
            guard: self.path_cond(),
            results,
            param_state,
        });
        Ok(())
    }

    /// The final value of a `&mut` parameter or capture at normal return:
    /// the recorded per-return values folded into a conditional over the
    /// return guards. `None` if a return record lacks the value or an
    /// unconditional return is not the only one.
    fn fold_final_param_value(&mut self, sym: Symbol) -> Option<Exp> {
        let mut acc: Option<Exp> = None;
        for record in self.returns.iter().rev() {
            let val = record
                .param_state
                .iter()
                .find(|(s, _)| *s == sym)
                .map(|(_, e)| e.clone())?;
            acc = Some(match (acc, &record.guard) {
                (None, _) => val,
                (Some(other), Some(guard)) => self.builder.mk_ite(
                    guard.as_ref().clone(),
                    val.as_ref().clone(),
                    other.as_ref().clone(),
                ),
                (Some(_), None) => return None,
            });
        }
        acc
    }

    /// Existentially quantifies `body` over the given value variables.
    fn mk_exists_over(&mut self, vars: &[(Symbol, Type)], body: Exp) -> Exp {
        let env = self.builder.global_env();
        let loc = self.builder.get_current_loc();
        let ranges = vars
            .iter()
            .map(|(sym, ty)| {
                let pat_id = env.new_node(loc.clone(), ty.clone());
                (
                    Pattern::Var(pat_id, *sym),
                    self.builder.mk_type_domain(ty.clone()),
                )
            })
            .collect::<Vec<_>>();
        let node = env.new_node(loc, BOOL_TYPE.clone());
        ExpData::Quant(node, QuantKind::Exists, ranges, vec![], None, body).into_exp()
    }

    /// Assembles the final specification from the accumulated state.
    fn assemble(mut self, result_type: &Type) -> Option<DerivedSpec> {
        let result_tys = result_type.clone().flatten();
        let mut results = if self.returns.is_empty() {
            None
        } else if result_tys.is_empty() {
            // Unit result: terminal values are irrelevant.
            Some(vec![])
        } else {
            let count = result_tys.len();
            if self.returns.iter().any(|r| r.results.len() != count) {
                return None;
            }
            let mut acc: Vec<Exp> = self.returns.last().unwrap().results.clone();
            for record in self.returns.iter().rev().skip(1) {
                let Some(guard) = &record.guard else {
                    // An unconditional return must be the only one.
                    return None;
                };
                for (i, res) in record.results.iter().enumerate() {
                    acc[i] = self.builder.mk_ite(
                        guard.as_ref().clone(),
                        res.as_ref().clone(),
                        acc[i].as_ref().clone(),
                    );
                }
            }
            Some(acc)
        };

        let mut ensures = vec![];
        let mut mut_param_values = None;
        if let Some(results) = &results {
            for (i, (val, ty)) in results.iter().zip(&result_tys).enumerate() {
                let result = self.builder.mk_result(i, ty);
                ensures.push(self.builder.mk_eq(result, val.clone()));
            }
            // Post-state of `&mut` parameters and captures: the final value
            // per parameter, emitted as the condition `p == <value>` and
            // collected for `DerivedSpec::mut_param_values`.
            let param_infos = self.params.clone();
            let mut final_values = vec![];
            for info in param_infos.iter().filter(|p| p.kind != ParamKind::Value) {
                let value_ty = self
                    .var_types
                    .get(&info.sym)
                    .map(|ty| ty.skip_reference().clone())?;
                let final_val = self.fold_final_param_value(info.sym)?;
                let var = self.builder.mk_local_by_sym(info.sym, value_ty);
                ensures.push(self.builder.mk_eq(var, final_val.clone()));
                final_values.push((info.sym, final_val));
            }
            mut_param_values = Some(final_values);
        }
        ensures.append(&mut self.effects);

        // Canonical `ensures_of` conditions for behavioral call summaries:
        // needed whenever the call has `&mut` argument post-states, is void,
        // or its result carriers do not occur in the output (anchor rule of
        // the bytecode inference).
        let call_records = std::mem::take(&mut self.call_records);
        let mut deferred_applications: Vec<(Exp, Vec<Exp>, Option<Exp>)> = call_records
            .iter()
            .filter(|record| record.deferred)
            .map(|record| {
                (
                    record.fun_exp.clone(),
                    record.inputs.clone(),
                    record.guard.clone(),
                )
            })
            .collect();
        for record in &call_records {
            let needed = !record.posts.is_empty()
                || record.results.is_empty()
                || !record.results.iter().any(|carrier| {
                    ensures
                        .iter()
                        .chain(results.iter().flatten())
                        .any(|e| e.any(&mut |n| n == carrier.as_ref()))
                });
            if !needed {
                continue;
            }
            let mut bp_args = record.inputs.clone();
            bp_args.extend(record.results.iter().cloned());
            for slot in &record.posts {
                bp_args.push(match slot {
                    PostSlot::Sym(sym, ty) => self.builder.mk_local_by_sym(*sym, ty.clone()),
                    PostSlot::Value(value) => value.clone(),
                });
            }
            let canonical = self.builder.mk_ensures_of_with_state(
                record.fun_exp.clone(),
                bp_args,
                record.pre,
                record.post,
            );
            let guarded = match &record.guard {
                Some(g) => self.builder.mk_implies(g.clone(), canonical),
                None => canonical,
            };
            ensures.push(guarded);
        }

        // Resolve post-state symbols: where the output pins `m == X` (or the
        // symbol flowed into a parameter/result equation), substitute; any
        // remaining symbols are existentially quantified together with their
        // defining conditions. Exact-value slots carry no symbols.
        let post_syms: Vec<(Symbol, Type)> = call_records
            .iter()
            .flat_map(|r| r.posts.iter())
            .filter_map(|slot| match slot {
                PostSlot::Sym(sym, ty) => Some((*sym, ty.clone())),
                PostSlot::Value(_) => None,
            })
            .collect();
        let mut aborts = std::mem::take(&mut self.aborts);
        let mut modifies = std::mem::take(&mut self.modifies);
        let mut modifies_exact = self.modifies_exact;
        if !post_syms.is_empty() {
            let mut subst: BTreeMap<Symbol, Exp> = BTreeMap::new();
            // Solve simple equations `lhs == m` / `m == rhs` whose other side
            // is free of post symbols. Post symbols are fresh and never
            // shadowed, so free-variable mention is exact.
            let is_free = |e: &Exp, syms: &[(Symbol, Type)]| {
                let used = e.free_vars();
                !syms.iter().any(|(sym, _)| used.contains(sym))
            };
            for e in &ensures {
                if let ExpData::Call(_, Operation::Eq, eq_args) = e.as_ref() {
                    for (a, b) in [(&eq_args[0], &eq_args[1]), (&eq_args[1], &eq_args[0])] {
                        if let ExpData::LocalVar(_, sym) = b.as_ref() {
                            if post_syms.iter().any(|(s, _)| s == sym)
                                && !subst.contains_key(sym)
                                && is_free(a, &post_syms)
                            {
                                subst.insert(*sym, a.clone());
                            }
                        }
                    }
                }
            }
            if !subst.is_empty() {
                let env = self.builder.global_env();
                let mut replacer = |_: NodeId, target: RewriteTarget| {
                    if let RewriteTarget::LocalVar(sym) = target {
                        subst.get(&sym).cloned()
                    } else {
                        None
                    }
                };
                let mut rewriter = ExpRewriter::new(env, &mut replacer);
                ensures = ensures
                    .into_iter()
                    .map(|e| rewriter.rewrite_exp(e))
                    .collect();
                aborts = aborts
                    .into_iter()
                    .map(|a| rewriter.rewrite_exp(a))
                    .collect();
                modifies = modifies
                    .into_iter()
                    .map(|m| rewriter.rewrite_exp(m))
                    .collect();
                if let Some(rs) = &mut results {
                    *rs = rs.iter().map(|r| rewriter.rewrite_exp(r.clone())).collect();
                }
                if let Some(vals) = &mut mut_param_values {
                    *vals = vals
                        .iter()
                        .map(|(sym, e)| (*sym, rewriter.rewrite_exp(e.clone())))
                        .collect();
                }
                deferred_applications = deferred_applications
                    .into_iter()
                    .map(|(target, inputs, guard)| {
                        (
                            target,
                            inputs
                                .into_iter()
                                .map(|e| rewriter.rewrite_exp(e))
                                .collect(),
                            guard.map(|g| rewriter.rewrite_exp(g)),
                        )
                    })
                    .collect();
            }
            // Existentially close over any remaining post symbols.
            let remaining: Vec<(Symbol, Type)> = post_syms
                .iter()
                .filter(|(sym, _)| !subst.contains_key(sym))
                .cloned()
                .collect();
            if !remaining.is_empty() {
                let mentions = |e: &Exp| !is_free(e, &remaining);
                if results.iter().flatten().any(mentions) {
                    // A result value depending on an unresolved intermediate
                    // state cannot be expressed.
                    return None;
                }
                if deferred_applications
                    .iter()
                    .any(|(_, inputs, guard)| inputs.iter().chain(guard.iter()).any(mentions))
                {
                    // A deferred application's arguments depending on an
                    // unresolved intermediate value cannot be re-stated at
                    // consumption sites; without the application's deferred
                    // effects the footprint is not usable either.
                    deferred_applications.clear();
                    modifies_exact = false;
                }
                if mut_param_values.iter().flatten().any(|(_, e)| mentions(e)) {
                    // A final parameter value depending on an unresolved
                    // intermediate value cannot be expressed as a value;
                    // the `p == <value>` condition remains exact via the
                    // existential closure below.
                    mut_param_values = None;
                }
                if modifies.iter().any(mentions) {
                    // A modified cell's address depending on an unresolved
                    // intermediate value cannot be re-evaluated at
                    // consumption sites.
                    modifies_exact = false;
                }
                let (bound_ensures, free_ensures): (Vec<_>, Vec<_>) =
                    ensures.into_iter().partition(mentions);
                let (bound_aborts, free_aborts): (Vec<_>, Vec<_>) =
                    aborts.into_iter().partition(mentions);
                ensures = free_ensures;
                aborts = free_aborts;
                if !bound_ensures.is_empty() {
                    let body = self.builder.mk_and_n(bound_ensures.clone());
                    ensures.push(self.mk_exists_over(&remaining, body));
                }
                for abort in bound_aborts {
                    // Each abort disjunct is closed together with the
                    // defining conditions of the intermediate values it
                    // mentions.
                    let mut conjuncts = bound_ensures.clone();
                    conjuncts.push(abort);
                    let body = self.builder.mk_and_n(conjuncts);
                    aborts.push(self.mk_exists_over(&remaining, body));
                }
            }
        }

        // Normalize memory labels: entry-state references become `old(..)`
        // in ensures and implicit in aborts; the final state is implicit;
        // intermediate labels are preserved.
        {
            let all: Vec<&Exp> = ensures.iter().chain(aborts.iter()).collect();
            let label_info = MemoryLabelInfo::from_conditions(&all, Some(self.entry_label));
            let env = self.builder.global_env();
            ensures = ensures
                .iter()
                .map(|e| label_info.normalize(env, e, true))
                .collect();
            aborts = aborts
                .iter()
                .map(|a| label_info.normalize(env, a, false))
                .collect();
            modifies = modifies
                .iter()
                .map(|m| label_info.normalize(env, m, true))
                .collect();
            if let Some(rs) = &mut results {
                *rs = rs
                    .iter()
                    .map(|r| label_info.normalize(env, r, true))
                    .collect();
            }
            if let Some(vals) = &mut mut_param_values {
                *vals = vals
                    .iter()
                    .map(|(sym, e)| (*sym, label_info.normalize(env, e, true)))
                    .collect();
            }
            deferred_applications = deferred_applications
                .into_iter()
                .map(|(target, inputs, guard)| {
                    (
                        target,
                        inputs
                            .iter()
                            .map(|e| label_info.normalize(env, e, true))
                            .collect(),
                        guard.map(|g| label_info.normalize(env, &g, true)),
                    )
                })
                .collect();
        }

        // The `modifies` footprint is usable only if every target is
        // expressible in entry-state terms: an address depending on an
        // intermediate memory state cannot be re-evaluated at consumption
        // sites.
        if modifies.iter().any(memory_labels::references_labeled_state) {
            modifies_exact = false;
        }

        // Aborts are pre-state phrased: strip `old(..)` wrappers.
        let aborts = aborts.iter().map(strip_all_olds).collect::<Vec<_>>();

        // Simplify and drop trivial conditions.
        let mut simplifier = ExpSimplifier::new(self.builder);
        let ensures = ensures
            .into_iter()
            .map(|e| simplifier.simplify(e))
            .filter(|e| !matches!(e.as_ref(), ExpData::Value(_, Value::Bool(true))))
            .collect::<Vec<_>>();
        let aborts = aborts
            .into_iter()
            .map(|a| simplifier.simplify(a))
            .filter(|a| !matches!(a.as_ref(), ExpData::Value(_, Value::Bool(false))))
            .collect::<Vec<_>>();
        let mut_param_values = mut_param_values.map(|vals| {
            vals.into_iter()
                .map(|(sym, e)| (sym, simplifier.simplify(e)))
                .collect()
        });

        Some(DerivedSpec {
            requires: vec![],
            aborts,
            ensures,
            results,
            modifies: modifies_exact.then_some(modifies),
            mut_param_values,
            deferred_applications,
        })
    }
}

// =================================================================================================
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{Address, BehaviorKind, ModuleName, Spec, SpecFunDecl},
        exp_generator::FunExpGenerator,
        model::{FieldData, FieldId, FunId, FunctionData, GlobalEnv, Loc, ModuleId, NodeId},
        ty::BOOL_TYPE,
    };
    use move_core_types::account_address::AccountAddress;
    use std::cell::RefCell;

    fn test_env() -> GlobalEnv {
        let mut env = GlobalEnv::new();
        let loc = Loc::default();
        let fun_name = env.symbol_pool().make("test_fun");
        let fun_id = FunId::new(fun_name);
        let mut function_data = BTreeMap::new();
        function_data.insert(fun_id, FunctionData::new(fun_name, loc.clone()));
        // A resource `struct R { v: u64 }` for tests of global state effects.
        let struct_name = env.symbol_pool().make("R");
        let field_name = env.symbol_pool().make("v");
        let mut struct_data = crate::model::StructData::new(struct_name, loc.clone());
        struct_data
            .field_data
            .insert(FieldId::new(field_name), FieldData {
                name: field_name,
                loc: loc.clone(),
                offset: 0,
                variant: None,
                ty: u64_ty(),
                is_ghost: false,
                init: None,
            });
        let mut struct_map = BTreeMap::new();
        struct_map.insert(StructId::new(struct_name), struct_data);
        let addr = Address::Numerical(AccountAddress::ZERO);
        let module_name = ModuleName::new(addr, env.symbol_pool().make("test_mod"));
        env.add(
            loc,
            module_name,
            vec![],
            vec![],
            vec![],
            BTreeMap::new(),
            struct_map,
            function_data,
            vec![],
            vec![],
            vec![],
            Spec::default(),
            vec![],
        );
        env
    }

    fn test_gen(env: &GlobalEnv) -> FunExpGenerator<'_> {
        let module = env.get_module(ModuleId::new(0));
        let fun_name = env.symbol_pool().make("test_fun");
        let fun_env = module.into_function(FunId::new(fun_name));
        FunExpGenerator::new(fun_env, Loc::default())
    }

    fn u64_ty() -> Type {
        Type::Primitive(PrimitiveType::U64)
    }

    fn node(env: &GlobalEnv, ty: Type) -> NodeId {
        env.new_node(Loc::default(), ty)
    }

    fn var(env: &GlobalEnv, name: &str, ty: Type) -> Exp {
        let sym = env.symbol_pool().make(name);
        ExpData::LocalVar(node(env, ty), sym).into_exp()
    }

    fn num(env: &GlobalEnv, val: i64) -> Exp {
        ExpData::Value(node(env, u64_ty()), Value::Number(BigInt::from(val))).into_exp()
    }

    fn call(env: &GlobalEnv, ty: Type, oper: Operation, args: Vec<Exp>) -> Exp {
        ExpData::Call(node(env, ty), oper, args).into_exp()
    }

    fn derive(
        env: &GlobalEnv,
        params: &[(&str, Type)],
        result_ty: Type,
        body: Exp,
    ) -> Option<DerivedSpec> {
        derive_with_captures(env, params, &[], result_ty, body)
    }

    fn derive_with_captures(
        env: &GlobalEnv,
        params: &[(&str, Type)],
        captures: &[(&str, Type)],
        result_ty: Type,
        body: Exp,
    ) -> Option<DerivedSpec> {
        let mut generator = test_gen(env);
        let make = |decls: &[(&str, Type)]| {
            decls
                .iter()
                .map(|(name, ty)| (env.symbol_pool().make(name), ty.clone()))
                .collect::<Vec<_>>()
        };
        let params = make(params);
        let captures = make(captures);
        let var_types: BTreeMap<Symbol, Type> = params
            .iter()
            .cloned()
            .chain(captures.iter().cloned())
            .collect();
        derive_spec_with_captures(
            &mut generator,
            &params,
            &captures,
            &var_types,
            &result_ty,
            &body,
            &BTreeMap::new(),
        )
    }

    fn render(env: &GlobalEnv, exps: &[Exp]) -> Vec<String> {
        exps.iter().map(|e| format!("{}", e.display(env))).collect()
    }

    fn render_vals(env: &GlobalEnv, vals: &[(Symbol, Exp)]) -> Vec<(String, String)> {
        vals.iter()
            .map(|(sym, e)| {
                (
                    sym.display(env.symbol_pool()).to_string(),
                    format!("{}", e.display(env)),
                )
            })
            .collect()
    }

    fn syms(env: &GlobalEnv, names: &[&str]) -> BTreeSet<Symbol> {
        names
            .iter()
            .map(|name| env.symbol_pool().make(name))
            .collect()
    }

    fn rendered_syms(env: &GlobalEnv, set: &BTreeSet<Symbol>) -> Vec<String> {
        set.iter()
            .map(|sym| sym.display(env.symbol_pool()).to_string())
            .collect()
    }

    #[test]
    fn add_overflow() {
        let env = test_env();
        let body = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "x", u64_ty()),
            num(&env, 1),
        ]);
        let spec = derive(&env, &[("x", u64_ty())], u64_ty(), body).unwrap();
        assert_eq!(render(&env, &spec.aborts), vec![
            "Gt(x, 18446744073709551614)"
        ]);
        assert_eq!(render(&env, &spec.ensures), vec![
            "Eq(result0(), Add(x, 1))"
        ]);
        assert_eq!(render(&env, spec.results.as_ref().unwrap()), vec![
            "Add(x, 1)"
        ]);
    }

    #[test]
    fn div_by_zero_short_circuit() {
        let env = test_env();
        // x < 5 && 10 / x > 1: the division abort is guarded by x < 5.
        let div = call(&env, u64_ty(), Operation::Div, vec![
            num(&env, 10),
            var(&env, "x", u64_ty()),
        ]);
        let cmp = call(&env, BOOL_TYPE.clone(), Operation::Gt, vec![
            div,
            num(&env, 1),
        ]);
        let lt = call(&env, BOOL_TYPE.clone(), Operation::Lt, vec![
            var(&env, "x", u64_ty()),
            num(&env, 5),
        ]);
        let body = call(&env, BOOL_TYPE.clone(), Operation::And, vec![lt, cmp]);
        let spec = derive(&env, &[("x", u64_ty())], BOOL_TYPE.clone(), body).unwrap();
        // `x < 5 && x == 0` is simplified to `x == 0` (subsumption).
        assert_eq!(render(&env, &spec.aborts), vec!["Eq(x, 0)"]);
    }

    #[test]
    fn if_else_join() {
        let env = test_env();
        // if (x > 10) x + 1 else x
        let cond = call(&env, BOOL_TYPE.clone(), Operation::Gt, vec![
            var(&env, "x", u64_ty()),
            num(&env, 10),
        ]);
        let add = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "x", u64_ty()),
            num(&env, 1),
        ]);
        let body =
            ExpData::IfElse(node(&env, u64_ty()), cond, add, var(&env, "x", u64_ty())).into_exp();
        let spec = derive(&env, &[("x", u64_ty())], u64_ty(), body).unwrap();
        // `x > 10 && x + 1 > MAX` is simplified to the stronger conjunct.
        assert_eq!(render(&env, &spec.aborts), vec![
            "Gt(x, 18446744073709551614)"
        ]);
        let results = render(&env, spec.results.as_ref().unwrap());
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("Add(x, 1)"), "{}", results[0]);
    }

    #[test]
    fn let_binding() {
        let env = test_env();
        // { let y = x + 1; y * 2 }
        let sym_y = env.symbol_pool().make("y");
        let add = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "x", u64_ty()),
            num(&env, 1),
        ]);
        let mul = call(&env, u64_ty(), Operation::Mul, vec![
            var(&env, "y", u64_ty()),
            num(&env, 2),
        ]);
        let body = ExpData::Block(
            node(&env, u64_ty()),
            Pattern::Var(node(&env, u64_ty()), sym_y),
            Some(add),
            mul,
        )
        .into_exp();
        let spec = derive(&env, &[("x", u64_ty())], u64_ty(), body).unwrap();
        assert_eq!(render(&env, spec.results.as_ref().unwrap()), vec![
            "Mul(Add(x, 1), 2)"
        ]);
        assert_eq!(spec.aborts.len(), 2);
    }

    #[test]
    fn empty_tuple_without_binding() {
        let env = test_env();
        // A zero-argument expansion has an empty, uninitialized binding.
        let body = ExpData::Block(
            node(&env, u64_ty()),
            Pattern::Tuple(node(&env, Type::unit()), vec![]),
            None,
            num(&env, 7),
        )
        .into_exp();
        let spec = derive(&env, &[], u64_ty(), body).unwrap();
        assert_eq!(render(&env, spec.results.as_ref().unwrap()), vec!["7"]);
        assert!(spec.aborts.is_empty());
    }

    #[test]
    fn mut_param_increment() {
        let env = test_env();
        // *p = *p + 1  where p: &mut u64
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let deref = call(&env, u64_ty(), Operation::Deref, vec![var(
            &env,
            "p",
            p_ref_ty.clone(),
        )]);
        let add = call(&env, u64_ty(), Operation::Add, vec![deref, num(&env, 1)]);
        let body = ExpData::Mutate(
            node(&env, Type::unit()),
            var(&env, "p", p_ref_ty.clone()),
            add,
        )
        .into_exp();
        let spec = derive(&env, &[("p", p_ref_ty)], Type::unit(), body).unwrap();
        assert_eq!(render(&env, &spec.ensures), vec!["Eq(p, Add(Old(p), 1))"]);
        assert_eq!(render(&env, &spec.aborts), vec![
            "Gt(p, 18446744073709551614)"
        ]);
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("p".to_string(), "Add(Old(p), 1)".to_string())]
        );
    }

    #[test]
    fn unwritten_mut_param() {
        let env = test_env();
        // body: *p  (read only)
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let body = call(&env, u64_ty(), Operation::Deref, vec![var(
            &env,
            "p",
            p_ref_ty.clone(),
        )]);
        let spec = derive(&env, &[("p", p_ref_ty)], u64_ty(), body).unwrap();
        assert_eq!(render(&env, &spec.ensures), vec![
            "Eq(result0(), Old(p))",
            "Eq(p, Old(p))"
        ]);
    }

    #[test]
    fn abort_in_branch() {
        let env = test_env();
        // if (x > 10) abort 1 else (); x  -- via sequence
        let cond = call(&env, BOOL_TYPE.clone(), Operation::Gt, vec![
            var(&env, "x", u64_ty()),
            num(&env, 10),
        ]);
        let abort = call(
            &env,
            Type::unit(),
            Operation::Abort(crate::ast::AbortKind::Code),
            vec![num(&env, 1)],
        );
        let unit = call(&env, Type::unit(), Operation::Tuple, vec![]);
        let if_exp = ExpData::IfElse(node(&env, Type::unit()), cond, abort, unit).into_exp();
        let body = ExpData::Sequence(node(&env, u64_ty()), vec![if_exp, var(&env, "x", u64_ty())])
            .into_exp();
        let spec = derive(&env, &[("x", u64_ty())], u64_ty(), body).unwrap();
        assert_eq!(render(&env, &spec.aborts), vec!["Gt(x, 10)"]);
        assert_eq!(render(&env, spec.results.as_ref().unwrap()), vec!["x"]);
    }

    #[test]
    fn clamp_shape() {
        let env = test_env();
        // { let cur = *e; if (cur > cap) { *e = cap } }  with e: &mut u64
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let deref = call(&env, u64_ty(), Operation::Deref, vec![var(
            &env,
            "e",
            p_ref_ty.clone(),
        )]);
        let cond = call(&env, BOOL_TYPE.clone(), Operation::Gt, vec![
            var(&env, "cur", u64_ty()),
            var(&env, "cap", u64_ty()),
        ]);
        let mutate = ExpData::Mutate(
            node(&env, Type::unit()),
            var(&env, "e", p_ref_ty.clone()),
            var(&env, "cap", u64_ty()),
        )
        .into_exp();
        let unit = call(&env, Type::unit(), Operation::Tuple, vec![]);
        let if_exp = ExpData::IfElse(node(&env, Type::unit()), cond, mutate, unit).into_exp();
        let body = ExpData::Block(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), env.symbol_pool().make("cur")),
            Some(deref),
            if_exp,
        )
        .into_exp();
        let spec = derive(&env, &[("e", p_ref_ty)], Type::unit(), body).unwrap();
        let ensures = render(&env, &spec.ensures);
        assert_eq!(ensures.len(), 1);
        assert!(
            ensures[0].contains("if") && ensures[0].contains("Old(e)"),
            "{}",
            ensures[0]
        );
        assert!(spec.aborts.is_empty());
    }

    #[test]
    fn field_write_through_global_ref() {
        let env = test_env();
        // { let r = &mut R[a]; r.v = r.v + 1; }  with a: address
        let addr_ty = Type::Primitive(PrimitiveType::Address);
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let borrow_id = node(&env, ref_ty.clone());
        env.set_node_instantiation(borrow_id, vec![struct_ty.clone()]);
        let borrow = ExpData::Call(
            borrow_id,
            Operation::BorrowGlobal(ReferenceKind::Mutable),
            vec![var(&env, "a", addr_ty.clone())],
        )
        .into_exp();
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let mk_select = || {
            let id = node(&env, u64_ty());
            env.set_node_instantiation(id, vec![struct_ty.clone()]);
            ExpData::Call(
                id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![var(&env, "r", ref_ty.clone())],
            )
            .into_exp()
        };
        let add = call(&env, u64_ty(), Operation::Add, vec![
            mk_select(),
            num(&env, 1),
        ]);
        let mutate = ExpData::Mutate(node(&env, Type::unit()), mk_select(), add).into_exp();
        let body = ExpData::Block(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, ref_ty), env.symbol_pool().make("r")),
            Some(borrow),
            mutate,
        )
        .into_exp();
        let spec = derive(&env, &[("a", addr_ty)], Type::unit(), body).unwrap();
        assert!(
            spec.ensures.iter().any(
                |e| e.any(&mut |n| matches!(n, ExpData::Call(_, Operation::SpecUpdate(..), _)))
            ),
            "expected a memory update effect in {:?}",
            render(&env, &spec.ensures)
        );
        let modifies = spec.modifies.as_ref().expect("exact modifies footprint");
        assert_eq!(modifies.len(), 1);
        assert!(matches!(
            modifies[0].as_ref(),
            ExpData::Call(_, Operation::Global(None), _)
        ));
        // The write aborts if the resource is absent or the increment
        // overflows.
        assert_eq!(spec.aborts.len(), 2);
    }

    #[test]
    fn loops_unsupported() {
        let env = test_env();
        let body = ExpData::Loop(
            node(&env, Type::unit()),
            call(&env, Type::unit(), Operation::Tuple, vec![]),
        )
        .into_exp();
        assert!(derive(&env, &[], Type::unit(), body).is_none());
    }

    // ============================================================================================
    // Capture discovery and derivation with captures

    /// `sum = sum + e` with `e` bound: `sum` is a mutated free variable.
    #[test]
    fn collect_assigned_capture() {
        let env = test_env();
        let sym_sum = env.symbol_pool().make("sum");
        let add = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "sum", u64_ty()),
            var(&env, "e", u64_ty()),
        ]);
        let body = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            add,
        )
        .into_exp();
        let found = collect_mutated_free_vars(&env, &body, &syms(&env, &["e"]));
        assert_eq!(rendered_syms(&env, &found), vec!["sum"]);
    }

    /// `f(&mut c)`: the mutable borrow of the free `c` in a call argument.
    #[test]
    fn collect_mut_borrow_to_callee() {
        let env = test_env();
        let ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let borrow = call(
            &env,
            ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", u64_ty())],
        );
        let fun_id = FunId::new(env.symbol_pool().make("test_fun"));
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), fun_id),
            vec![borrow],
        );
        let found = collect_mutated_free_vars(&env, &body, &syms(&env, &[]));
        assert_eq!(rendered_syms(&env, &found), vec!["c"]);
    }

    /// `r.v = r.v + e` through the captured reference `r`: the write is
    /// rooted at the free `r`.
    #[test]
    fn collect_through_ref_field_write() {
        let env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let mk_select = || {
            let id = node(&env, u64_ty());
            env.set_node_instantiation(id, vec![struct_ty.clone()]);
            ExpData::Call(
                id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![var(&env, "r", ref_ty.clone())],
            )
            .into_exp()
        };
        let add = call(&env, u64_ty(), Operation::Add, vec![
            mk_select(),
            var(&env, "e", u64_ty()),
        ]);
        let body = ExpData::Mutate(node(&env, Type::unit()), mk_select(), add).into_exp();
        let found = collect_mutated_free_vars(&env, &body, &syms(&env, &["e"]));
        assert_eq!(rendered_syms(&env, &found), vec!["r"]);
    }

    /// A let-bound `sum` shadows the free variable: the assignment is not a
    /// capture mutation.
    #[test]
    fn collect_respects_shadowing() {
        let env = test_env();
        let sym_sum = env.symbol_pool().make("sum");
        let assign = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            num(&env, 1),
        )
        .into_exp();
        let body = ExpData::Block(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            Some(num(&env, 0)),
            assign,
        )
        .into_exp();
        let found = collect_mutated_free_vars(&env, &body, &syms(&env, &[]));
        assert!(found.is_empty(), "{:?}", rendered_syms(&env, &found));
    }

    /// The essential accumulation `sum = sum + e` with `sum` captured: the
    /// transformer value is exact and pure.
    #[test]
    fn capture_accumulation() {
        let env = test_env();
        let sym_sum = env.symbol_pool().make("sum");
        let add = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "sum", u64_ty()),
            var(&env, "e", u64_ty()),
        ]);
        let body = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            add,
        )
        .into_exp();
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("sum", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("sum".to_string(), "Add(Old(sum), e)".to_string())]
        );
        assert_eq!(render(&env, &spec.ensures), vec![
            "Eq(sum, Add(Old(sum), e))"
        ]);
        assert_eq!(render(&env, &spec.aborts), vec![
            "Gt(Add(sum, e), 18446744073709551615)"
        ]);
        // The transformer values and aborts are pure single-state.
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
                .chain(&spec.aborts)
        ));
    }

    /// Coupled multi-capture updates `sum = sum + count * e; count = count
    /// + 1`: both transformer values are phrased over the pre-state.
    #[test]
    fn multi_capture_coupled() {
        let env = test_env();
        let sym_sum = env.symbol_pool().make("sum");
        let sym_count = env.symbol_pool().make("count");
        let mul = call(&env, u64_ty(), Operation::Mul, vec![
            var(&env, "count", u64_ty()),
            var(&env, "e", u64_ty()),
        ]);
        let add_sum = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "sum", u64_ty()),
            mul,
        ]);
        let assign_sum = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            add_sum,
        )
        .into_exp();
        let add_count = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "count", u64_ty()),
            num(&env, 1),
        ]);
        let assign_count = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_count),
            add_count,
        )
        .into_exp();
        let body =
            ExpData::Sequence(node(&env, Type::unit()), vec![assign_sum, assign_count]).into_exp();
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("sum", u64_ty()), ("count", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![
                (
                    "sum".to_string(),
                    "Add(Old(sum), Mul(Old(count), e))".to_string()
                ),
                ("count".to_string(), "Add(Old(count), 1)".to_string())
            ]
        );
    }

    /// `r.v = r.v + e` with `r` a captured `&mut` reference: the final
    /// value is a functional field update over the pre-state.
    #[test]
    fn through_ref_capture() {
        let env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let mk_select = || {
            let id = node(&env, u64_ty());
            env.set_node_instantiation(id, vec![struct_ty.clone()]);
            ExpData::Call(
                id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![var(&env, "r", ref_ty.clone())],
            )
            .into_exp()
        };
        let add = call(&env, u64_ty(), Operation::Add, vec![
            mk_select(),
            var(&env, "e", u64_ty()),
        ]);
        let body = ExpData::Mutate(node(&env, Type::unit()), mk_select(), add).into_exp();
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("r", ref_ty)],
            Type::unit(),
            body,
        )
        .unwrap();
        let vals = render_vals(&env, spec.mut_param_values.as_ref().unwrap());
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].0, "r");
        assert!(
            vals[0].1.contains("Old(r)") && vals[0].1.to_lowercase().contains("update"),
            "{}",
            vals[0].1
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
        ));
    }

    // ============================================================================================
    // Vector intrinsic WPs

    /// Extends the test env with a `std::vector` module (address 0x1)
    /// declaring the given function names, returning its module id.
    fn add_vector_module(env: &mut GlobalEnv, names: &[&str]) -> ModuleId {
        let loc = Loc::default();
        let mut function_data = BTreeMap::new();
        for name in names {
            let sym = env.symbol_pool().make(name);
            function_data.insert(FunId::new(sym), FunctionData::new(sym, loc.clone()));
        }
        let addr = Address::Numerical(AccountAddress::ONE);
        let module_name = ModuleName::new(addr, env.symbol_pool().make("vector"));
        env.add(
            loc,
            module_name,
            vec![],
            vec![],
            vec![],
            BTreeMap::new(),
            BTreeMap::new(),
            function_data,
            vec![],
            vec![],
            vec![],
            Spec::default(),
            vec![],
        );
        ModuleId::new(1)
    }

    fn vector_wp(
        env: &GlobalEnv,
        vec_mid: ModuleId,
        name: &str,
        args: &[Exp],
        output_types: &[Type],
    ) -> IntrinsicWp {
        let generator = test_gen(env);
        let fid = FunId::new(env.symbol_pool().make(name));
        well_known::vector_intrinsic_wp(
            env,
            &generator,
            vec_mid.qualified(fid),
            &[u64_ty()],
            args,
            output_types,
        )
        .expect("intrinsic WP")
    }

    #[test]
    fn vector_index_of_wp() {
        let mut env = test_env();
        let vec_mid = add_vector_module(&mut env, &["index_of"]);
        let vec_ty = Type::Vector(Box::new(u64_ty()));
        let v = var(&env, "v", vec_ty);
        let e = var(&env, "e", u64_ty());
        let wp = vector_wp(&env, vec_mid, "index_of", &[v, e], &[
            BOOL_TYPE.clone(),
            u64_ty(),
        ]);
        assert_eq!(render(&env, std::slice::from_ref(&wp.aborts)), vec![
            "false"
        ]);
        let outputs = render(&env, &wp.outputs);
        assert_eq!(outputs[0], "ContainsVec<u64>(v, e)");
        // `if contains(v, e) then index_of(v, e) else 0`.
        assert!(
            outputs[1].contains("IndexOfVec<u64>(v, e)")
                && outputs[1].contains("ContainsVec<u64>(v, e)"),
            "{}",
            outputs[1]
        );
    }

    #[test]
    fn vector_swap_remove_wp() {
        let mut env = test_env();
        let vec_mid = add_vector_module(&mut env, &["swap_remove"]);
        let vec_ty = Type::Vector(Box::new(u64_ty()));
        let v = var(&env, "v", vec_ty.clone());
        let i = var(&env, "i", u64_ty());
        let wp = vector_wp(&env, vec_mid, "swap_remove", &[v, i], &[u64_ty(), vec_ty]);
        assert_eq!(render(&env, std::slice::from_ref(&wp.aborts)), vec![
            "Not(InRangeVec(v, i))"
        ]);
        let outputs = render(&env, &wp.outputs);
        // Result: the removed element.
        assert_eq!(outputs[0], "Index(v, i)");
        // Post-state: the last element swapped into place, then truncated.
        assert_eq!(
            outputs[1],
            "Slice(UpdateVec(v, i, Index(v, Sub(Len(v), 1))), Range(0, Sub(Len(v), 1)))"
        );
    }

    #[test]
    fn vector_append_wp() {
        let mut env = test_env();
        let vec_mid = add_vector_module(&mut env, &["append"]);
        let vec_ty = Type::Vector(Box::new(u64_ty()));
        let v = var(&env, "v", vec_ty.clone());
        let other = var(&env, "other", vec_ty.clone());
        let wp = vector_wp(&env, vec_mid, "append", &[v, other], &[vec_ty]);
        assert_eq!(render(&env, std::slice::from_ref(&wp.aborts)), vec![
            "false"
        ]);
        assert_eq!(render(&env, &wp.outputs), vec!["ConcatVec(v, other)"]);
    }

    #[test]
    fn vector_remove_wp() {
        let mut env = test_env();
        let vec_mid = add_vector_module(&mut env, &["remove"]);
        let vec_ty = Type::Vector(Box::new(u64_ty()));
        let v = var(&env, "v", vec_ty.clone());
        let i = var(&env, "i", u64_ty());
        let wp = vector_wp(&env, vec_mid, "remove", &[v, i], &[u64_ty(), vec_ty]);
        assert_eq!(render(&env, std::slice::from_ref(&wp.aborts)), vec![
            "Not(InRangeVec(v, i))"
        ]);
        let outputs = render(&env, &wp.outputs);
        assert_eq!(outputs[0], "Index(v, i)");
        assert_eq!(
            outputs[1],
            "ConcatVec(Slice(v, Range(0, i)), Slice(v, Range(Add(i, 1), Len(v))))"
        );
    }

    #[test]
    fn vector_insert_wp() {
        let mut env = test_env();
        let vec_mid = add_vector_module(&mut env, &["insert"]);
        let vec_ty = Type::Vector(Box::new(u64_ty()));
        let v = var(&env, "v", vec_ty.clone());
        let i = var(&env, "i", u64_ty());
        let e = var(&env, "e", u64_ty());
        let wp = vector_wp(&env, vec_mid, "insert", &[v, i, e], &[vec_ty]);
        assert_eq!(render(&env, std::slice::from_ref(&wp.aborts)), vec![
            "Gt(i, Len(v))"
        ]);
        assert_eq!(render(&env, &wp.outputs), vec![
            "ConcatVec(ConcatVec(Slice(v, Range(0, i)), SingleVec<u64>(e)), Slice(v, Range(i, Len(v))))"
        ]);
    }

    // ============================================================================================
    // Callee summaries

    /// Adds a Move function to the test module. `def` may reference the
    /// parameters as `Temporary(i)`.
    fn add_function(
        env: &mut GlobalEnv,
        name: &str,
        params: &[(&str, Type)],
        result_type: Type,
        def: Option<Exp>,
        is_native: bool,
    ) -> FunId {
        let sym = env.symbol_pool().make(name);
        let mut data = FunctionData::new(sym, Loc::default());
        data.params = params
            .iter()
            .map(|(name, ty)| Parameter(env.symbol_pool().make(name), ty.clone(), Loc::default()))
            .collect();
        data.result_type = result_type;
        data.def = def;
        data.is_native = is_native;
        env.add_function_def_from_data(ModuleId::new(0), data)
    }

    /// A summarized callee without memory effects (`*p = *p + 1` through a
    /// `&mut` parameter) keeps the exact (empty) modifies footprint and
    /// produces single-state conditions — no state labels are introduced.
    #[test]
    fn memory_free_callee_summary_keeps_modifies_exact() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let bump_def = {
            let p = || ExpData::Temporary(node(&env, p_ref_ty.clone()), 0).into_exp();
            let deref = call(&env, u64_ty(), Operation::Deref, vec![p()]);
            let add = call(&env, u64_ty(), Operation::Add, vec![deref, num(&env, 1)]);
            ExpData::Mutate(node(&env, Type::unit()), p(), add).into_exp()
        };
        let bump_fid = add_function(
            &mut env,
            "bump",
            &[("p", p_ref_ty.clone())],
            Type::unit(),
            Some(bump_def),
            false,
        );
        // Lambda body: `bump(&mut c)` with `c` a captured local.
        let borrow = call(
            &env,
            p_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", u64_ty())],
        );
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), bump_fid),
            vec![borrow],
        );
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("c", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        // Exact empty footprint, and single-state (default-range) summary
        // conditions.
        assert!(
            spec.modifies.as_ref().is_some_and(|m| m.is_empty()),
            "{:?}",
            spec.modifies.as_ref().map(|m| render(&env, m))
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.aborts.iter().chain(&spec.ensures)
        ));
    }

    /// A summarized callee with unknown effects (a native outside the
    /// memory-free whitelist) clears the modifies footprint.
    #[test]
    fn unknown_callee_summary_clears_modifies_exact() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let opaque_fid = add_function(
            &mut env,
            "opaque",
            &[("p", p_ref_ty.clone())],
            Type::unit(),
            None,
            true,
        );
        let borrow = call(
            &env,
            p_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", u64_ty())],
        );
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), opaque_fid),
            vec![borrow],
        );
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("c", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        assert!(spec.modifies.is_none());
    }

    /// A pure callee with a companion spec function resolves to a direct
    /// spec-function call: the exact footprint is kept, and the companion
    /// is marked used.
    #[test]
    fn pure_spec_call_resolution_in_lambda_callee() {
        let mut env = test_env();
        let callee_def = {
            let x = ExpData::Temporary(node(&env, u64_ty()), 0).into_exp();
            call(&env, u64_ty(), Operation::Add, vec![x, num(&env, 1)])
        };
        let callee_fid = add_function(
            &mut env,
            "callee",
            &[("x", u64_ty())],
            u64_ty(),
            Some(callee_def),
            false,
        );
        // The companion, as derived by the (pre-inlining) spec rewriter.
        let companion_body = num(&env, 1);
        let companion_qid = env.add_spec_function_def(ModuleId::new(0), SpecFunDecl {
            loc: Loc::default(),
            name: env.symbol_pool().make("$callee"),
            type_params: vec![],
            params: vec![Parameter(
                env.symbol_pool().make("x"),
                u64_ty(),
                Loc::default(),
            )],
            result_type: u64_ty(),
            used_memory: BTreeSet::new(),
            old_memory: BTreeSet::new(),
            uninterpreted: false,
            is_move_fun: true,
            is_native: false,
            body: Some(companion_body),
            callees: BTreeSet::new(),
            is_recursive: RefCell::new(None),
            uses_old: false,
            frame_spec: None,
            insts_using_generic_type_reflection: Default::default(),
            spec: RefCell::new(Spec::default()),
        });
        // Lambda body: `callee(e)`.
        let body = call(
            &env,
            u64_ty(),
            Operation::MoveFunction(ModuleId::new(0), callee_fid),
            vec![var(&env, "e", u64_ty())],
        );
        let spec = derive(&env, &[("e", u64_ty())], u64_ty(), body).unwrap();
        let results = render(&env, spec.results.as_ref().unwrap());
        assert!(results[0].contains("$callee"), "{}", results[0]);
        assert!(
            spec.modifies.as_ref().is_some_and(|m| m.is_empty()),
            "exact footprint kept"
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.aborts.iter().chain(&spec.ensures)
        ));
        assert!(env.is_spec_fun_used(companion_qid));
    }

    // ============================================================================================
    // Post-value routing (D1): callee functional ensures and body value
    // summaries

    /// Adds a Move function with an attached specification.
    #[allow(clippy::too_many_arguments)]
    fn add_function_with_spec(
        env: &mut GlobalEnv,
        name: &str,
        params: &[(&str, Type)],
        result_type: Type,
        def: Option<Exp>,
        is_native: bool,
        spec: Spec,
    ) -> FunId {
        let sym = env.symbol_pool().make(name);
        let mut data = FunctionData::new(sym, Loc::default());
        data.params = params
            .iter()
            .map(|(name, ty)| Parameter(env.symbol_pool().make(name), ty.clone(), Loc::default()))
            .collect();
        data.result_type = result_type;
        data.def = def;
        data.is_native = is_native;
        data.spec = RefCell::new(spec);
        env.add_function_def_from_data(ModuleId::new(0), data)
    }

    /// A spec with the given (unconditional) `ensures` conditions.
    fn ensures_spec(exps: Vec<Exp>) -> Spec {
        let mut spec = Spec::default();
        for exp in exps {
            spec.conditions.push(crate::ast::Condition {
                loc: Loc::default(),
                kind: crate::ast::ConditionKind::Ensures,
                properties: Default::default(),
                exp,
                additional_exps: vec![],
            });
        }
        spec
    }

    /// An underivable but memory-free function body (a loop).
    fn loop_def(env: &GlobalEnv) -> Exp {
        ExpData::Loop(
            node(env, Type::unit()),
            call(env, Type::unit(), Operation::Tuple, vec![]),
        )
        .into_exp()
    }

    fn temp(env: &GlobalEnv, idx: usize, ty: Type) -> Exp {
        ExpData::Temporary(node(env, ty), idx).into_exp()
    }

    /// Derives the spec of a lambda body `callee(&mut c, e)`.
    fn derive_mut_capture_call(env: &GlobalEnv, callee: FunId) -> DerivedSpec {
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let borrow = call(
            env,
            p_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(env, "c", u64_ty())],
        );
        let body = call(
            env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), callee),
            vec![borrow, var(env, "e", u64_ty())],
        );
        derive_with_captures(
            env,
            &[("e", u64_ty())],
            &[("c", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap()
    }

    /// A callee whose body is underivable (a loop) but whose attached spec
    /// carries the functional ensures `p == old(p) + x`: the post value
    /// routes from the spec, so the capture's final value is exact
    /// transformer material.
    #[test]
    fn callee_functional_ensures_consumed() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let old_t0 = call(&env, u64_ty(), Operation::Old, vec![temp(
            &env,
            0,
            u64_ty(),
        )]);
        let add = call(&env, u64_ty(), Operation::Add, vec![
            old_t0,
            temp(&env, 1, u64_ty()),
        ]);
        let ens = call(&env, BOOL_TYPE.clone(), Operation::Eq, vec![
            temp(&env, 0, u64_ty()),
            add,
        ]);
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "bump",
            &[("p", p_ref_ty), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![ens]),
        );
        let spec = derive_mut_capture_call(&env, fid);
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("c".to_string(), "Add(Old(c), e)".to_string())]
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
                .chain(&spec.aborts)
        ));
        // Memory-free callee: exact (empty) modifies footprint kept.
        assert!(spec.modifies.as_ref().is_some_and(|m| m.is_empty()));
    }

    /// Discarding `&mut` results retains callee aborts and argument effects.
    #[test]
    fn discarded_mut_ref_result_keeps_callee_effects() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let old_t0 = call(&env, u64_ty(), Operation::Old, vec![temp(
            &env,
            0,
            u64_ty(),
        )]);
        let add = call(&env, u64_ty(), Operation::Add, vec![
            old_t0,
            temp(&env, 1, u64_ty()),
        ]);
        let ens = call(&env, BOOL_TYPE.clone(), Operation::Eq, vec![
            temp(&env, 0, u64_ty()),
            add,
        ]);
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "bump_and_return",
            &[("p", p_ref_ty.clone()), ("x", u64_ty())],
            p_ref_ty.clone(),
            Some(def),
            false,
            ensures_spec(vec![ens]),
        );
        let borrow = call(
            &env,
            p_ref_ty.clone(),
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", u64_ty())],
        );
        let discarded_call = call(
            &env,
            p_ref_ty,
            Operation::MoveFunction(ModuleId::new(0), fid),
            vec![borrow, var(&env, "e", u64_ty())],
        );
        let body = ExpData::Sequence(node(&env, Type::unit()), vec![
            discarded_call,
            call(&env, Type::unit(), Operation::Tuple, vec![]),
        ])
        .into_exp();
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("c", u64_ty())],
            Type::unit(),
            body,
        )
        .expect("the unused reference result must not prevent derivation");
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("c".to_string(), "Add(Old(c), e)".to_string())]
        );
    }

    /// Complete per-field functional ensures (the `coin::merge` shape,
    /// `p.v == old(p.v) + x`) compose into a field update over the
    /// parameter's pre-state.
    #[test]
    fn callee_per_field_ensures_composed() {
        let mut env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let mk_select = |env: &GlobalEnv, base: Exp| {
            let id = node(env, u64_ty());
            env.set_node_instantiation(id, vec![struct_ty.clone()]);
            ExpData::Call(
                id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![base],
            )
            .into_exp()
        };
        // ensures p.v == old(p.v) + x
        let old_sel = call(&env, u64_ty(), Operation::Old, vec![mk_select(
            &env,
            temp(&env, 0, struct_ty.clone()),
        )]);
        let add = call(&env, u64_ty(), Operation::Add, vec![
            old_sel,
            temp(&env, 1, u64_ty()),
        ]);
        let ens = call(&env, BOOL_TYPE.clone(), Operation::Eq, vec![
            mk_select(&env, temp(&env, 0, struct_ty.clone())),
            add,
        ]);
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "merge_like",
            &[("p", p_ref_ty.clone()), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![ens]),
        );
        // Lambda body: `merge_like(&mut acc, e)` with `acc: R` captured.
        let borrow = call(
            &env,
            p_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "acc", struct_ty.clone())],
        );
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), fid),
            vec![borrow, var(&env, "e", u64_ty())],
        );
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("acc", struct_ty)],
            Type::unit(),
            body,
        )
        .unwrap();
        let vals = render_vals(&env, spec.mut_param_values.as_ref().unwrap());
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].0, "acc");
        assert!(
            vals[0].1.to_lowercase().contains("update") && vals[0].1.contains("Old(acc)"),
            "{}",
            vals[0].1
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
        ));
    }

    /// A relational-only spec (`p >= old(p)`) has no functional value: the
    /// post slot stays symbolic, and the final capture value degenerates to
    /// the post-state self-reference the fold transformer rejects.
    #[test]
    fn callee_relational_ensures_not_consumed() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let old_t0 = call(&env, u64_ty(), Operation::Old, vec![temp(
            &env,
            0,
            u64_ty(),
        )]);
        let ens = call(&env, BOOL_TYPE.clone(), Operation::Ge, vec![
            temp(&env, 0, u64_ty()),
            old_t0,
        ]);
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "relaxed",
            &[("p", p_ref_ty), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![ens]),
        );
        let spec = derive_mut_capture_call(&env, fid);
        let vals = spec.mut_param_values.as_ref().unwrap();
        let capture_syms = syms(&env, &["c"]);
        assert!(
            vals.iter()
                .any(|(_, e)| mentions_syms_outside_old(e, &capture_syms)),
            "expected a post-state self-reference, got {:?}",
            render_vals(&env, vals)
        );
    }

    /// `[concrete]` conditions are invisible to callers and must not be
    /// consumed.
    #[test]
    fn callee_concrete_ensures_not_consumed() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let old_t0 = call(&env, u64_ty(), Operation::Old, vec![temp(
            &env,
            0,
            u64_ty(),
        )]);
        let add = call(&env, u64_ty(), Operation::Add, vec![
            old_t0,
            temp(&env, 1, u64_ty()),
        ]);
        let ens = call(&env, BOOL_TYPE.clone(), Operation::Eq, vec![
            temp(&env, 0, u64_ty()),
            add,
        ]);
        let mut spec = ensures_spec(vec![ens]);
        let concrete = env
            .symbol_pool()
            .make(crate::pragmas::CONDITION_CONCRETE_PROP);
        spec.conditions[0].properties.insert(
            concrete,
            crate::ast::PropertyValue::Value(crate::ast::Value::Bool(true)),
        );
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "hidden",
            &[("p", p_ref_ty), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
            spec,
        );
        let spec = derive_mut_capture_call(&env, fid);
        let vals = spec.mut_param_values.as_ref().unwrap();
        let capture_syms = syms(&env, &["c"]);
        assert!(
            vals.iter()
                .any(|(_, e)| mentions_syms_outside_old(e, &capture_syms)),
            "concrete condition must not route: {:?}",
            render_vals(&env, vals)
        );
    }

    /// Aliasing `&mut` arguments (same root in two slots) fall back to
    /// symbolic post-states even when functional ensures exist; distinct
    /// roots route.
    #[test]
    fn aliasing_mut_args_fall_back() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let mk_ens = |env: &GlobalEnv, idx: usize, delta: i64| {
            let old_t = call(env, u64_ty(), Operation::Old, vec![temp(
                env,
                idx,
                u64_ty(),
            )]);
            let add = call(env, u64_ty(), Operation::Add, vec![old_t, num(env, delta)]);
            call(env, BOOL_TYPE.clone(), Operation::Eq, vec![
                temp(env, idx, u64_ty()),
                add,
            ])
        };
        let ens_a = mk_ens(&env, 0, 1);
        let ens_b = mk_ens(&env, 1, 2);
        let def = loop_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "two_muts",
            &[("a", p_ref_ty.clone()), ("b", p_ref_ty.clone())],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![ens_a, ens_b]),
        );
        let mk_borrow = |env: &GlobalEnv, name: &str| {
            call(
                env,
                p_ref_ty.clone(),
                Operation::Borrow(ReferenceKind::Mutable),
                vec![var(env, name, u64_ty())],
            )
        };
        // Aliasing: `two_muts(&mut c, &mut c)`.
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), fid),
            vec![mk_borrow(&env, "c"), mk_borrow(&env, "c")],
        );
        let spec = derive_with_captures(&env, &[], &[("c", u64_ty())], Type::unit(), body).unwrap();
        let vals = spec.mut_param_values.as_ref().unwrap();
        let capture_syms = syms(&env, &["c"]);
        assert!(
            vals.iter()
                .any(|(_, e)| mentions_syms_outside_old(e, &capture_syms)),
            "aliasing places must not route: {:?}",
            render_vals(&env, vals)
        );
        // Distinct roots: `two_muts(&mut c1, &mut c2)` routes both.
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), fid),
            vec![mk_borrow(&env, "c1"), mk_borrow(&env, "c2")],
        );
        let spec = derive_with_captures(
            &env,
            &[],
            &[("c1", u64_ty()), ("c2", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![
                ("c1".to_string(), "Add(Old(c1), 1)".to_string()),
                ("c2".to_string(), "Add(Old(c2), 2)".to_string())
            ]
        );
    }

    /// Without a spec, the post value routes from the callee's own body
    /// (`*r = *r + x`): the memoized body value summary.
    #[test]
    fn callee_body_value_summary_routes() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let def = {
            let p = || temp(&env, 0, p_ref_ty.clone());
            let deref = call(&env, u64_ty(), Operation::Deref, vec![p()]);
            let add = call(&env, u64_ty(), Operation::Add, vec![
                deref,
                temp(&env, 1, u64_ty()),
            ]);
            ExpData::Mutate(node(&env, Type::unit()), p(), add).into_exp()
        };
        let fid = add_function(
            &mut env,
            "add_to",
            &[("r", p_ref_ty.clone()), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
        );
        let spec = derive_mut_capture_call(&env, fid);
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("c".to_string(), "Add(Old(c), e)".to_string())]
        );
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
                .chain(&spec.aborts)
        ));
        // Transitively: a wrapper calling `add_to` routes through the
        // nested summary.
        let wrapper_def = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), fid),
            vec![temp(&env, 0, p_ref_ty.clone()), temp(&env, 1, u64_ty())],
        );
        let wrapper_fid = add_function(
            &mut env,
            "outer_add_to",
            &[("r", p_ref_ty), ("x", u64_ty())],
            Type::unit(),
            Some(wrapper_def),
            false,
        );
        let spec = derive_mut_capture_call(&env, wrapper_fid);
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![("c".to_string(), "Add(Old(c), e)".to_string())]
        );
    }

    /// A self-recursive callee cannot be summarized as a value: the
    /// in-progress guard falls back to the symbolic post-state.
    #[test]
    fn recursive_callee_body_summary_falls_back() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let name = env.symbol_pool().make("rec");
        let rec_fid = FunId::new(name);
        let def = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), rec_fid),
            vec![temp(&env, 0, p_ref_ty.clone()), temp(&env, 1, u64_ty())],
        );
        let fid = add_function(
            &mut env,
            "rec",
            &[("r", p_ref_ty), ("x", u64_ty())],
            Type::unit(),
            Some(def),
            false,
        );
        assert_eq!(fid, rec_fid);
        let spec = derive_mut_capture_call(&env, fid);
        let vals = spec.mut_param_values.as_ref().unwrap();
        let capture_syms = syms(&env, &["c"]);
        assert!(
            vals.iter()
                .any(|(_, e)| mentions_syms_outside_old(e, &capture_syms)),
            "recursion must not route: {:?}",
            render_vals(&env, vals)
        );
    }

    // ============================================================================================
    // Intrinsic-map WPs (D2)

    /// Environment with a map-intrinsic declaration over `R`, binding
    /// `m_add` to `map_add_no_override` and `m_remove` to
    /// `map_del_return_key`, with declared spec and abort functions.
    fn map_intrinsic_env() -> (GlobalEnv, FunId, FunId, Type) {
        let mut env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let map_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let add_fid = add_function(
            &mut env,
            "m_add",
            &[("m", map_ref_ty.clone()), ("k", u64_ty()), ("v", u64_ty())],
            Type::unit(),
            None,
            true,
        );
        let remove_fid = add_function(
            &mut env,
            "m_remove",
            &[("m", map_ref_ty), ("k", u64_ty())],
            Type::Tuple(vec![u64_ty(), u64_ty()]),
            None,
            true,
        );
        let add_spec_fun = |env: &mut GlobalEnv, name: &str, params: Vec<Type>, result: Type| {
            let params = params
                .into_iter()
                .enumerate()
                .map(|(i, ty)| {
                    Parameter(
                        env.symbol_pool().make(&format!("p{}", i)),
                        ty,
                        Loc::default(),
                    )
                })
                .collect();
            env.add_spec_function_def(ModuleId::new(0), SpecFunDecl {
                loc: Loc::default(),
                name: env.symbol_pool().make(name),
                type_params: vec![],
                params,
                result_type: result,
                used_memory: BTreeSet::new(),
                old_memory: BTreeSet::new(),
                uninterpreted: true,
                is_move_fun: false,
                is_native: false,
                body: None,
                callees: BTreeSet::new(),
                is_recursive: RefCell::new(None),
                uses_old: false,
                frame_spec: None,
                insts_using_generic_type_reflection: Default::default(),
                spec: RefCell::new(Spec::default()),
            })
        };
        let set_qid = add_spec_fun(
            &mut env,
            "spec_set_t",
            vec![struct_ty.clone(), u64_ty(), u64_ty()],
            struct_ty.clone(),
        );
        let get_qid = add_spec_fun(
            &mut env,
            "spec_get_t",
            vec![struct_ty.clone(), u64_ty()],
            u64_ty(),
        );
        let del_qid = add_spec_fun(
            &mut env,
            "spec_del_t",
            vec![struct_ty.clone(), u64_ty()],
            struct_ty.clone(),
        );
        let aborts_add_qid = add_spec_fun(
            &mut env,
            "aborts_add_t",
            vec![struct_ty.clone(), u64_ty(), u64_ty()],
            BOOL_TYPE.clone(),
        );
        let aborts_del_qid = add_spec_fun(
            &mut env,
            "aborts_del_t",
            vec![struct_ty.clone(), u64_ty()],
            BOOL_TYPE.clone(),
        );
        let s = |name: &str| env.symbol_pool().make(name);
        let decl = crate::intrinsics::IntrinsicDecl::new_for_test(
            ModuleId::new(0).qualified(struct_id),
            s(crate::pragmas::INTRINSIC_TYPE_MAP),
            vec![
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE),
                    ModuleId::new(0).qualified(add_fid),
                ),
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_DEL_RETURN_KEY),
                    ModuleId::new(0).qualified(remove_fid),
                ),
            ],
            vec![
                (s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_SET), set_qid),
                (s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_GET), get_qid),
                (s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_DEL), del_qid),
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD),
                    aborts_add_qid,
                ),
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL),
                    aborts_del_qid,
                ),
            ],
            vec![
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_ADD_NO_OVERRIDE),
                    s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_ABORTS_ADD),
                ),
                (
                    s(crate::pragmas::INTRINSIC_FUN_MAP_DEL_RETURN_KEY),
                    s(crate::pragmas::INTRINSIC_FUN_MAP_SPEC_ABORTS_DEL),
                ),
            ],
        );
        env.intrinsics.add_decl(&decl);
        (env, add_fid, remove_fid, struct_ty)
    }

    /// `m_add(&mut c, e, 7)` — the map add WP: post value `spec_set`,
    /// abort per the declared condition, no state labels.
    #[test]
    fn map_intrinsic_add_wp() {
        let (env, add_fid, _, struct_ty) = map_intrinsic_env();
        let map_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let borrow = call(
            &env,
            map_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", struct_ty.clone())],
        );
        let body = call(
            &env,
            Type::unit(),
            Operation::MoveFunction(ModuleId::new(0), add_fid),
            vec![borrow, var(&env, "e", u64_ty()), num(&env, 7)],
        );
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("c", struct_ty)],
            Type::unit(),
            body,
        )
        .unwrap();
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![(
                "c".to_string(),
                "test_mod::spec_set_t(Old(c), e, 7)".to_string()
            )]
        );
        assert_eq!(render(&env, &spec.aborts), vec![
            "test_mod::aborts_add_t(c, e, 7)"
        ]);
        assert!(spec.modifies.as_ref().is_some_and(|m| m.is_empty()));
        assert!(exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
                .chain(&spec.aborts)
        ));
    }

    /// `let (rk, rv) = m_remove(&mut c, e); rv` — the del-return-key WP:
    /// results `(k, spec_get)`, post value `spec_del`, abort per the
    /// declared condition.
    #[test]
    fn map_intrinsic_del_return_key_wp() {
        let (env, _, remove_fid, struct_ty) = map_intrinsic_env();
        let map_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let borrow = call(
            &env,
            map_ref_ty,
            Operation::Borrow(ReferenceKind::Mutable),
            vec![var(&env, "c", struct_ty.clone())],
        );
        let callee_call = call(
            &env,
            Type::Tuple(vec![u64_ty(), u64_ty()]),
            Operation::MoveFunction(ModuleId::new(0), remove_fid),
            vec![borrow, var(&env, "e", u64_ty())],
        );
        let pat = Pattern::Tuple(node(&env, Type::Tuple(vec![u64_ty(), u64_ty()])), vec![
            Pattern::Var(node(&env, u64_ty()), env.symbol_pool().make("rk")),
            Pattern::Var(node(&env, u64_ty()), env.symbol_pool().make("rv")),
        ]);
        let body = ExpData::Block(
            node(&env, u64_ty()),
            pat,
            Some(callee_call),
            var(&env, "rv", u64_ty()),
        )
        .into_exp();
        let spec = derive_with_captures(
            &env,
            &[("e", u64_ty())],
            &[("c", struct_ty)],
            u64_ty(),
            body,
        )
        .unwrap();
        assert_eq!(render(&env, spec.results.as_ref().unwrap()), vec![
            "test_mod::spec_get_t(Old(c), e)"
        ]);
        assert_eq!(
            render_vals(&env, spec.mut_param_values.as_ref().unwrap()),
            vec![(
                "c".to_string(),
                "test_mod::spec_del_t(Old(c), e)".to_string()
            )]
        );
        assert_eq!(render(&env, &spec.aborts), vec![
            "test_mod::aborts_del_t(c, e)"
        ]);
    }

    // ============================================================================================
    // Frame-conjunct allowance in the memory-effect analysis (D3)

    /// A structurally identical two-state read equation (`R[0x1].v ==
    /// old(R[0x1].v)`, the `coin::merge` supply-frame shape).
    fn frame_conjunct(env: &GlobalEnv) -> Exp {
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let mk_read = |env: &GlobalEnv| {
            let addr_ty = Type::Primitive(PrimitiveType::Address);
            let addr = ExpData::Value(
                node(env, addr_ty),
                Value::Address(crate::ast::Address::Numerical(
                    move_core_types::account_address::AccountAddress::ONE,
                )),
            )
            .into_exp();
            let global_id = node(env, struct_ty.clone());
            env.set_node_instantiation(global_id, vec![struct_ty.clone()]);
            let global = ExpData::Call(global_id, Operation::Global(None), vec![addr]).into_exp();
            let sel_id = node(env, u64_ty());
            env.set_node_instantiation(sel_id, vec![struct_ty.clone()]);
            ExpData::Call(
                sel_id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![global],
            )
            .into_exp()
        };
        let old_read = call(env, u64_ty(), Operation::Old, vec![mk_read(env)]);
        call(env, BOOL_TYPE.clone(), Operation::Eq, vec![
            mk_read(env),
            old_read,
        ])
    }

    /// An exact frame conjunct `X == old(X)` in the attached spec does not
    /// disqualify a memory-free callee; an inexact two-state condition
    /// does.
    #[test]
    fn frame_conjunct_allowed_in_memory_free_spec() {
        let mut env = test_env();
        let p_ref_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let mk_def = |env: &GlobalEnv| {
            let p = || temp(env, 0, p_ref_ty.clone());
            let deref = call(env, u64_ty(), Operation::Deref, vec![p()]);
            let add = call(env, u64_ty(), Operation::Add, vec![deref, num(env, 1)]);
            ExpData::Mutate(node(env, Type::unit()), p(), add).into_exp()
        };
        let frame = frame_conjunct(&env);
        let def = mk_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "with_frame",
            &[("p", p_ref_ty.clone())],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![frame]),
        );
        assert!(fun_has_no_memory_effects(
            &env,
            ModuleId::new(0).qualified(fid)
        ));
        // Control: a non-frame two-state memory condition disqualifies.
        let frame = frame_conjunct(&env);
        let ExpData::Call(_, Operation::Eq, eq_args) = frame.as_ref() else {
            unreachable!()
        };
        let inexact = call(&env, BOOL_TYPE.clone(), Operation::Ge, vec![
            eq_args[0].clone(),
            eq_args[1].clone(),
        ]);
        let def = mk_def(&env);
        let fid = add_function_with_spec(
            &mut env,
            "with_inexact",
            &[("p", p_ref_ty)],
            Type::unit(),
            Some(def),
            false,
            ensures_spec(vec![inexact]),
        );
        assert!(!fun_has_no_memory_effects(
            &env,
            ModuleId::new(0).qualified(fid)
        ));
    }

    /// The `let (key, value) = elem.borrow_kv();` shape: a pure helper
    /// returning a tuple of references into its receiver. The companion
    /// spec function the pre-inlining derivation establishes is
    /// tuple-valued and therefore excluded from the pure-spec-call
    /// substitution (not decomposable here); the reference-result summary
    /// (D7) evaluates the helper's body to place projections over the
    /// receiver instead, so the results are exact field reads (and writes
    /// through returned `&mut` projections compose, see the `_mut` test).
    #[test]
    fn reference_returning_helper_splices_places() {
        let mut env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let recv_ty = Type::Reference(ReferenceKind::Immutable, Box::new(struct_ty.clone()));
        let u64_ref_ty = Type::Reference(ReferenceKind::Immutable, Box::new(u64_ty()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let result_ty = Type::Tuple(vec![u64_ref_ty.clone(), u64_ref_ty.clone()]);
        // fun borrow_kv(self: &R): (&u64, &u64) { (&self.v, &self.v) }
        let mk_field_ref = |env: &GlobalEnv| {
            let sel_id = node(env, u64_ty());
            env.set_node_instantiation(sel_id, vec![struct_ty.clone()]);
            let select = ExpData::Call(
                sel_id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![ExpData::Temporary(node(env, recv_ty.clone()), 0).into_exp()],
            )
            .into_exp();
            call(
                env,
                u64_ref_ty.clone(),
                Operation::Borrow(ReferenceKind::Immutable),
                vec![select],
            )
        };
        let def = call(&env, result_ty.clone(), Operation::Tuple, vec![
            mk_field_ref(&env),
            mk_field_ref(&env),
        ]);
        let callee_fid = add_function(
            &mut env,
            "borrow_kv",
            &[("self", recv_ty.clone())],
            result_ty.clone(),
            Some(def),
            false,
        );
        // The companion, as the pre-inlining companion derivation creates
        // it (references eliminated from the converted body).
        let mk_field_select = |env: &GlobalEnv| {
            let sel_id = node(env, u64_ty());
            env.set_node_instantiation(sel_id, vec![struct_ty.clone()]);
            ExpData::Call(
                sel_id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![var(env, "self", struct_ty.clone())],
            )
            .into_exp()
        };
        let companion_body = call(
            &env,
            Type::Tuple(vec![u64_ty(), u64_ty()]),
            Operation::Tuple,
            vec![mk_field_select(&env), mk_field_select(&env)],
        );
        env.add_spec_function_def(ModuleId::new(0), SpecFunDecl {
            loc: Loc::default(),
            name: env.symbol_pool().make("$borrow_kv"),
            type_params: vec![],
            params: vec![Parameter(
                env.symbol_pool().make("self"),
                struct_ty.clone(),
                Loc::default(),
            )],
            result_type: Type::Tuple(vec![u64_ty(), u64_ty()]),
            used_memory: BTreeSet::new(),
            old_memory: BTreeSet::new(),
            uninterpreted: false,
            is_move_fun: true,
            is_native: false,
            body: Some(companion_body),
            callees: BTreeSet::new(),
            is_recursive: RefCell::new(None),
            uses_old: false,
            frame_spec: None,
            insts_using_generic_type_reflection: Default::default(),
            spec: RefCell::new(Spec::default()),
        });
        // Lambda body: `let (k, v) = borrow_kv(&e); *k + *v`.
        let borrow_e = call(
            &env,
            recv_ty,
            Operation::Borrow(ReferenceKind::Immutable),
            vec![var(&env, "e", struct_ty.clone())],
        );
        let callee_call = call(
            &env,
            result_ty,
            Operation::MoveFunction(ModuleId::new(0), callee_fid),
            vec![borrow_e],
        );
        let k_deref = call(&env, u64_ty(), Operation::Deref, vec![var(
            &env,
            "k",
            u64_ref_ty.clone(),
        )]);
        let v_deref = call(&env, u64_ty(), Operation::Deref, vec![var(
            &env,
            "v",
            u64_ref_ty.clone(),
        )]);
        let use_kv = call(&env, u64_ty(), Operation::Add, vec![k_deref, v_deref]);
        let pat = Pattern::Tuple(
            node(
                &env,
                Type::Tuple(vec![u64_ref_ty.clone(), u64_ref_ty.clone()]),
            ),
            vec![
                Pattern::Var(node(&env, u64_ref_ty.clone()), env.symbol_pool().make("k")),
                Pattern::Var(node(&env, u64_ref_ty), env.symbol_pool().make("v")),
            ],
        );
        let body = ExpData::Block(node(&env, u64_ty()), pat, Some(callee_call), use_kv).into_exp();
        let spec = derive(&env, &[("e", struct_ty)], u64_ty(), body)
            .expect("derives via the place-projection splice");
        let results = render(&env, spec.results.as_ref().unwrap());
        assert!(
            !results[0].contains("result_of"),
            "place projections, not carriers: {}",
            results[0]
        );
        assert_eq!(
            results[0],
            "Add(select test_mod::R.v<0x0::test_mod::R>(e), \
             select test_mod::R.v<0x0::test_mod::R>(e))"
        );
        // The only abort condition is the addition's overflow check — the
        // helper itself contributes none.
        assert_eq!(spec.aborts.len(), 1);
    }

    /// `let (k, v) = borrow_kv_mut(e); *v = 7` over a `&mut` parameter `e`:
    /// the returned references splice as places of the calling derivation
    /// (D7), so the write through `v` composes into a field update of `e`.
    #[test]
    fn mut_reference_returning_helper_write_composes() {
        let mut env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let recv_ty = Type::Reference(ReferenceKind::Mutable, Box::new(struct_ty.clone()));
        let u64_mut_ty = Type::Reference(ReferenceKind::Mutable, Box::new(u64_ty()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        // fun borrow_v_mut(self: &mut R): &mut u64 { &mut self.v }
        let def = {
            let sel_id = node(&env, u64_ty());
            env.set_node_instantiation(sel_id, vec![struct_ty.clone()]);
            let select = ExpData::Call(
                sel_id,
                Operation::Select(ModuleId::new(0), struct_id, field_id),
                vec![ExpData::Temporary(node(&env, recv_ty.clone()), 0).into_exp()],
            )
            .into_exp();
            call(
                &env,
                u64_mut_ty.clone(),
                Operation::Borrow(ReferenceKind::Mutable),
                vec![select],
            )
        };
        let callee_fid = add_function(
            &mut env,
            "borrow_v_mut",
            &[("self", recv_ty.clone())],
            u64_mut_ty.clone(),
            Some(def),
            false,
        );
        // Body: `let v = borrow_v_mut(e); *v = 7;` with `e: &mut R`.
        let callee_call = call(
            &env,
            u64_mut_ty.clone(),
            Operation::MoveFunction(ModuleId::new(0), callee_fid),
            vec![var(&env, "e", recv_ty.clone())],
        );
        let write = ExpData::Mutate(
            node(&env, Type::unit()),
            var(&env, "v", u64_mut_ty.clone()),
            num(&env, 7),
        )
        .into_exp();
        let pat = Pattern::Var(node(&env, u64_mut_ty), env.symbol_pool().make("v"));
        let body =
            ExpData::Block(node(&env, Type::unit()), pat, Some(callee_call), write).into_exp();
        let spec = derive(&env, &[("e", recv_ty)], Type::unit(), body)
            .expect("derives via the place-projection splice");
        let values = render_vals(&env, spec.mut_param_values.as_ref().unwrap());
        assert_eq!(values, vec![(
            "e".to_string(),
            "update test_mod::R.v<0x0::test_mod::R>(Old(e), 7)".to_string()
        )]);
        assert!(spec.aborts.is_empty());
    }

    /// A helper with an assertion prelude before returning the projection:
    /// the abort condition is substituted at the call; the projection still
    /// splices as a place.
    #[test]
    fn reference_returning_helper_with_abort_prelude() {
        let mut env = test_env();
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let recv_ty = Type::Reference(ReferenceKind::Immutable, Box::new(struct_ty.clone()));
        let u64_ref_ty = Type::Reference(ReferenceKind::Immutable, Box::new(u64_ty()));
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        // fun checked_v(self: &R): &u64 {
        //     if (self.v == 0) abort 1; &self.v
        // }
        let def = {
            let mk_select = |env: &GlobalEnv| {
                let sel_id = node(env, u64_ty());
                env.set_node_instantiation(sel_id, vec![struct_ty.clone()]);
                ExpData::Call(
                    sel_id,
                    Operation::Select(ModuleId::new(0), struct_id, field_id),
                    vec![ExpData::Temporary(node(env, recv_ty.clone()), 0).into_exp()],
                )
                .into_exp()
            };
            let cond = call(&env, BOOL_TYPE.clone(), Operation::Eq, vec![
                mk_select(&env),
                num(&env, 0),
            ]);
            let abort = call(
                &env,
                u64_ref_ty.clone(),
                Operation::Abort(crate::ast::AbortKind::Code),
                vec![num(&env, 1)],
            );
            let proj = call(
                &env,
                u64_ref_ty.clone(),
                Operation::Borrow(ReferenceKind::Immutable),
                vec![mk_select(&env)],
            );
            ExpData::IfElse(node(&env, u64_ref_ty.clone()), cond, abort, proj).into_exp()
        };
        let callee_fid = add_function(
            &mut env,
            "checked_v",
            &[("self", recv_ty.clone())],
            u64_ref_ty.clone(),
            Some(def),
            false,
        );
        // Body: `*checked_v(&e)` with `e: R`.
        let borrow_e = call(
            &env,
            recv_ty,
            Operation::Borrow(ReferenceKind::Immutable),
            vec![var(&env, "e", struct_ty.clone())],
        );
        let callee_call = call(
            &env,
            u64_ref_ty,
            Operation::MoveFunction(ModuleId::new(0), callee_fid),
            vec![borrow_e],
        );
        let body = call(&env, u64_ty(), Operation::Deref, vec![callee_call]);
        let spec = derive(&env, &[("e", struct_ty)], u64_ty(), body)
            .expect("derives via the place-projection splice");
        let results = render(&env, spec.results.as_ref().unwrap());
        assert_eq!(results[0], "select test_mod::R.v<0x0::test_mod::R>(e)");
        let aborts = render(&env, &spec.aborts);
        assert_eq!(aborts, vec![
            "Eq(select test_mod::R.v<0x0::test_mod::R>(e), 0)".to_string()
        ]);
    }

    /// `sum = sum + R[a].v`: a state-reading accumulation derives, but its
    /// transformer value is classified as impure.
    #[test]
    fn state_reading_accumulation_impure() {
        let env = test_env();
        let addr_ty = Type::Primitive(PrimitiveType::Address);
        let sym_sum = env.symbol_pool().make("sum");
        let struct_id = StructId::new(env.symbol_pool().make("R"));
        let struct_ty = Type::Struct(ModuleId::new(0), struct_id, vec![]);
        let field_id = FieldId::new(env.symbol_pool().make("v"));
        let borrow_id = node(&env, struct_ty.clone());
        env.set_node_instantiation(borrow_id, vec![struct_ty.clone()]);
        let borrow = ExpData::Call(
            borrow_id,
            Operation::BorrowGlobal(ReferenceKind::Immutable),
            vec![var(&env, "a", addr_ty.clone())],
        )
        .into_exp();
        let select_id = node(&env, u64_ty());
        env.set_node_instantiation(select_id, vec![struct_ty]);
        let select = ExpData::Call(
            select_id,
            Operation::Select(ModuleId::new(0), struct_id, field_id),
            vec![borrow],
        )
        .into_exp();
        let add = call(&env, u64_ty(), Operation::Add, vec![
            var(&env, "sum", u64_ty()),
            select,
        ]);
        let body = ExpData::Assign(
            node(&env, Type::unit()),
            Pattern::Var(node(&env, u64_ty()), sym_sum),
            add,
        )
        .into_exp();
        let spec = derive_with_captures(
            &env,
            &[("a", addr_ty)],
            &[("sum", u64_ty())],
            Type::unit(),
            body,
        )
        .unwrap();
        assert!(!exps_are_pure_single_state(
            &env,
            spec.mut_param_values
                .as_ref()
                .unwrap()
                .iter()
                .map(|(_, e)| e)
                .chain(&spec.aborts)
        ));
    }

    /// Even an unlabeled behavioral predicate can read the target function's
    /// current memory through its evaluator, so it is not a safe
    /// per-iteration value for `folds_of`.
    #[test]
    fn default_range_behavior_is_not_single_state_for_folds() {
        let env = test_env();
        let behavior = call(
            &env,
            u64_ty(),
            Operation::Behavior(BehaviorKind::ResultOf, MemoryRange::default()),
            vec![],
        );
        assert!(exps_are_pure_single_state(&env, [&behavior]));
        assert!(!exps_are_pure_single_state_for_folds(&env, [&behavior]));
    }
}
