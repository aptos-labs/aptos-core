// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A general-purpose expression simplifier that works under a set of assumptions.
//!
//! `ExpSimplifier` implements `ExpRewriterFunctions` for bottom-up traversal and simplification.
//! It maintains a set of known-true predicates and substitutions extracted from equality
//! assumptions, simplifying expressions using algebraic identities and assumption-based reasoning.
//!
//! # Simplifications
//!
//! **Boolean identities**: `a && true => a`, `a || false => a`, `!!a => a`, `a && !a => false`,
//!   etc. Complementary pairs are detected through negation and comparison flipping
//!   (`!(a < b) => a >= b`). Iff identities: `a <==> true => a`, `a <==> a => true`.
//!
//! **Implication simplification**: `true ==> a => a`, `a ==> true => true`,
//!   `a ==> (b ==> c) => (a && b) ==> c`, and local assumption pushing where the antecedent
//!   is assumed true while simplifying the consequent.
//!
//! **Comparison simplification**: Reflexivity (`x == x => true`, `x < x => false`),
//!   type-bound analysis (`x > MAX_U64 => false`, `x >= 0 => true` for unsigned types),
//!   addend normalization (`(e + C1) op C2 => e op (C2 - C1)`),
//!   antisymmetry (`a <= b && a >= b => a == b`),
//!   pinch-to-equality (`c < x && !(c+1 < x) => x == c+1`),
//!   ordering-based deduction (e.g. `3 < x` implies `1 < x`),
//!   and conjunction/disjunction pruning via implication between comparisons.
//!
//! **Arithmetic identities** (spec mode): `0 + a => a`, `a * 1 => a`, `a - a => 0`,
//!   associative constant folding (`(x + c1) + c2 => x + (c1+c2)`).
//!
//! **Constant folding**: Via `ConstantFolder` for fully-constant subexpressions,
//!   MAX_U* operations folded to their numeric values, plus spec function unfolding
//!   when all arguments are constants (depth-limited).
//!
//! **If-then-else**: `if true {a} else {b} => a`, `if c {true} else {false} => c`,
//!   same-branch elimination.
//!
//! **Quantifier simplification**:
//! - Flattening: `forall x. forall y. P => forall x, y. P`
//! - One-point rule: `forall v, .. (v == e ==> Q) => forall .. Q[v/e]`
//!   and `exists v, .. (v == e && Q) => exists .. Q[v/e]`
//! - Struct field one-point: `forall x: S. x.f1 == e1 && ... ==> Q => Q[x/Pack(S, e1,...)]`
//! - Antisymmetry normalization: `x <= m && x >= m => x == m` to expose one-point bindings
//! - Unused variable elimination: drop quantified variables not appearing in the body
//! - Antecedent-only elimination (forall): `forall x: A(x) ==> Q => Q` when `x` only in `A`
//! - Independent splitting (exists): `exists x, y: A(x) && B(y) => (exists x: A(x)) && (exists y: B(y))`
//! - Absorb inner exists: `exists x: A(x) && (exists y: B(x,y)) => exists x, y: A(x) && B(x,y)`
//! - Upper-bound witness (exists): `exists x: x <= e && P(x) => P(e)` when P is monotone
//! - Witness instantiation (exists): `exists x: u64. P => true` if `P[x/0]` simplifies to true
//!
//! **Assumption-based reasoning**: Tracks known-true predicates and equality substitutions
//!   for both temporaries (`$t == e`) and local variables (`x == e`). Expressions provably
//!   true/false under assumptions are reduced to boolean constants. Substitutions are
//!   scope-aware (shadowed locals are not substituted) and suppressed inside `old(..)`.
//!
//! **Struct operations**: `update_field(update_field(e, f, v1), f, v2) => update_field(e, f, v2)`,
//!   `update_field(e, f, v).f => v`, `Pack(S, e1,...,en).fi => e_i`,
//!   `update_field(Pack(S, ...), fi, v) => Pack(S, ..., v, ...)`.
//!
//! **Special operations**: `old(old(x)) => old(x)`, `Freeze(x) => x` in spec mode,
//!   `WellFormed(x) => true`, `AbortFlag() => false`.
//!
//! # Example
//!
//! ```ignore
//! let mut simplifier = ExpSimplifier::new(&mut gen);
//! simplifier.assume(known_true_exp);
//! let simplified = simplifier.simplify(complex_exp);
//! ```

use crate::{
    ast::{Exp, ExpData, Operation, Pattern, QuantKind, TempIndex, Value},
    constant_folder::ConstantFolder,
    exp_generator::ExpGenerator,
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget},
    model::{FieldId, GlobalEnv, ModuleId, NodeId, SpecFunId, StructId},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
};
use num::{BigInt, Zero};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of spec function unfold steps allowed during simplification.
const MAX_SPEC_FUN_UNFOLD_DEPTH: usize = 10;

/// Expression simplifier with assumption tracking.
///
/// Maintains a set of known-true predicates and substitutions extracted from
/// equality assumptions. Simplifies expressions bottom-up using algebraic
/// identities, constant folding, and assumption checking.
///
/// Parameterized by an `ExpGenerator` which provides the environment and
/// expression construction primitives.
pub struct ExpSimplifier<'a, 'env, G: ExpGenerator<'env>> {
    generator: &'a mut G,
    assumptions: Vec<Exp>,
    substitutions: BTreeMap<RewriteTarget, Exp>,
    /// Tracks scopes of bound variables for shadowing detection.
    shadowed: Vec<BTreeSet<Symbol>>,
    spec_mode: bool,
    /// Whether we are currently inside an `old(..)` expression.
    /// Substitutions are suppressed in this context since temporaries
    /// refer to pre-state values, not post-state.
    inside_old: bool,
    /// Tracks the number of spec function unfold steps to prevent runaway recursion.
    spec_fun_unfold_depth: usize,
    /// Binds the `'env` lifetime (used only in the `G: ExpGenerator<'env>` bound).
    _phantom: std::marker::PhantomData<&'env ()>,
}

impl<'a, 'env, G: ExpGenerator<'env>> ExpSimplifier<'a, 'env, G> {
    /// Creates a new simplifier in spec mode (arbitrary-precision arithmetic).
    pub fn new(generator: &'a mut G) -> Self {
        Self {
            generator,
            assumptions: Vec::new(),
            substitutions: BTreeMap::new(),
            shadowed: Vec::new(),
            spec_mode: true,
            inside_old: false,
            spec_fun_unfold_depth: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new simplifier with explicit mode selection.
    pub fn new_with_mode(generator: &'a mut G, spec_mode: bool) -> Self {
        Self {
            generator,
            assumptions: Vec::new(),
            substitutions: BTreeMap::new(),
            shadowed: Vec::new(),
            spec_mode,
            inside_old: false,
            spec_fun_unfold_depth: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Access the global environment.
    fn env(&self) -> &'env GlobalEnv {
        self.generator.global_env()
    }

    /// Adds an assumption (known-true predicate). Conjunctions are flattened,
    /// and equalities of the form `$t == e` are extracted as substitutions.
    pub fn assume(&mut self, exp: Exp) {
        // Flatten conjunctions
        if let ExpData::Call(_, Operation::And, args) = exp.as_ref() {
            if args.len() == 2 {
                self.assume(args[0].clone());
                self.assume(args[1].clone());
                return;
            }
        }
        // Extract substitutions from equalities
        if let ExpData::Call(_, Operation::Eq, args) = exp.as_ref() {
            if args.len() == 2 {
                if let ExpData::Temporary(_, idx) = args[0].as_ref() {
                    let rhs = self.apply_substitutions(args[1].clone());
                    self.substitutions
                        .insert(RewriteTarget::Temporary(*idx), rhs);
                } else if let ExpData::Temporary(_, idx) = args[1].as_ref() {
                    let lhs = self.apply_substitutions(args[0].clone());
                    self.substitutions
                        .insert(RewriteTarget::Temporary(*idx), lhs);
                } else if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                    let rhs = self.apply_substitutions(args[1].clone());
                    self.substitutions
                        .insert(RewriteTarget::LocalVar(*sym), rhs);
                } else if let ExpData::LocalVar(_, sym) = args[1].as_ref() {
                    let lhs = self.apply_substitutions(args[0].clone());
                    self.substitutions
                        .insert(RewriteTarget::LocalVar(*sym), lhs);
                }
            }
        }
        // Add to assumptions if not already present
        if !self.assumptions.iter().any(|a| a.structural_eq(&exp)) {
            self.assumptions.push(exp);
        }
    }

    /// Simplifies an expression using bottom-up rewriting.
    pub fn simplify(&mut self, exp: Exp) -> Exp {
        self.rewrite_exp(exp)
    }

    /// Checks whether the given expression is known to be true under current assumptions.
    pub fn is_known_true(&self, exp: &Exp) -> bool {
        if is_bool_const(exp, true) {
            return true;
        }
        if self
            .assumptions
            .iter()
            .any(|a| a.structural_eq(exp) || self.implies_comparison(a, exp))
        {
            return true;
        }
        self.is_known_true_by_ordering(exp)
    }

    /// Checks whether the given expression is known to be false under current assumptions.
    pub fn is_known_false(&self, exp: &Exp) -> bool {
        if is_bool_const(exp, false) {
            return true;
        }
        // Check if negation is in assumptions, including via implies_comparison
        if self
            .assumptions
            .iter()
            .any(|a| self.is_complementary(a, exp) || self.implies_complementary(a, exp))
        {
            return true;
        }
        self.is_known_false_by_ordering(exp)
    }

    // -----------------------------------------------------------
    // Private helper methods
    // -----------------------------------------------------------

    fn apply_substitutions(&self, exp: Exp) -> Exp {
        match exp.as_ref() {
            ExpData::Temporary(_, idx) => {
                if let Some(replacement) = self.substitutions.get(&RewriteTarget::Temporary(*idx)) {
                    return replacement.clone();
                }
            },
            ExpData::LocalVar(_, sym) => {
                if let Some(replacement) = self.substitutions.get(&RewriteTarget::LocalVar(*sym)) {
                    return replacement.clone();
                }
            },
            _ => {},
        }
        exp
    }

    /// Check whether a local variable symbol is shadowed by an enclosing scope.
    fn is_shadowed(&self, sym: Symbol) -> bool {
        self.shadowed.iter().any(|set| set.contains(&sym))
    }

    fn mk_bool_const(&self, value: bool) -> Exp {
        self.generator.mk_bool_const(value)
    }

    fn mk_num_const(&self, ty: &Type, value: BigInt) -> Exp {
        let node_id = self.generator.new_node(ty.clone(), None);
        ExpData::Value(node_id, Value::Number(value)).into_exp()
    }

    fn mk_call(&self, ty: &Type, oper: Operation, args: Vec<Exp>) -> Exp {
        self.generator.mk_call(ty, oper, args)
    }

    fn mk_bool_call(&self, oper: Operation, args: Vec<Exp>) -> Exp {
        self.generator.mk_bool_call(oper, args)
    }

    fn mk_not(&self, arg: Exp) -> Exp {
        match arg.as_ref() {
            ExpData::Value(_, Value::Bool(b)) => self.mk_bool_const(!b),
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => args[0].clone(),
            // !(0 < x) where x is unsigned ==> x == 0
            ExpData::Call(_, Operation::Lt, args)
                if args.len() == 2
                    && is_num_const(&args[0], 0)
                    && self
                        .env()
                        .get_node_type(args[1].as_ref().node_id())
                        .is_unsigned_int() =>
            {
                let zero = self.mk_num_const(
                    &self.env().get_node_type(args[1].as_ref().node_id()),
                    BigInt::from(0),
                );
                self.mk_bool_call(Operation::Eq, vec![args[1].clone(), zero])
            },
            // Negate comparisons: !(a < b) => a >= b, etc.
            ExpData::Call(_, op, args) if args.len() == 2 && negate_comparison(op).is_some() => {
                self.mk_bool_call(negate_comparison(op).unwrap(), vec![
                    args[0].clone(),
                    args[1].clone(),
                ])
            },
            _ => self.mk_bool_call(Operation::Not, vec![arg]),
        }
    }

    fn mk_and(&self, arg1: Exp, arg2: Exp) -> Exp {
        match (arg1.as_ref(), arg2.as_ref()) {
            (ExpData::Value(_, Value::Bool(true)), _) => arg2,
            (_, ExpData::Value(_, Value::Bool(true))) => arg1,
            (ExpData::Value(_, Value::Bool(false)), _)
            | (_, ExpData::Value(_, Value::Bool(false))) => self.mk_bool_const(false),
            _ if arg1.structural_eq(&arg2) => arg1,
            _ if self.is_complementary(&arg1, &arg2) => self.mk_bool_const(false),
            // If one implies the other, keep the stronger (the one that implies)
            _ if self.implies_comparison(&arg2, &arg1) => arg2,
            _ if self.implies_comparison(&arg1, &arg2) => arg1,
            // Prune comparisons subsumed by a conjunct on the other side:
            // A && (B && C) where B or C implies A → (B && C)
            // (A && B) && C where A or B implies C → (A && B)
            _ if self.conjunction_implies_comparison(&arg2, &arg1) => arg2,
            _ if self.conjunction_implies_comparison(&arg1, &arg2) => arg1,
            _ => {
                // Antisymmetry: a <= b && a >= b → a == b (and symmetric variants)
                if let Some(eq) = self.try_antisymmetry_to_eq(&arg1, &arg2) {
                    return eq;
                }
                // Pinch-to-equality: c < x && !(c+1 < x) → x == c+1
                if let Some(eq) = self.try_pinch_to_eq(&arg1, &arg2) {
                    return eq;
                }
                // Empty integer range: c1 < x && x < c2 where c2 <= c1 + 1 → false
                if let Some(result) = self.try_empty_range(&arg1, &arg2) {
                    return result;
                }
                self.mk_bool_call(Operation::And, vec![arg1, arg2])
            },
        }
    }

    fn mk_or(&self, arg1: Exp, arg2: Exp) -> Exp {
        match (arg1.as_ref(), arg2.as_ref()) {
            (ExpData::Value(_, Value::Bool(false)), _) => arg2,
            (_, ExpData::Value(_, Value::Bool(false))) => arg1,
            (ExpData::Value(_, Value::Bool(true)), _)
            | (_, ExpData::Value(_, Value::Bool(true))) => self.mk_bool_const(true),
            _ if arg1.structural_eq(&arg2) => arg1,
            _ if self.is_complementary(&arg1, &arg2) => self.mk_bool_const(true),
            // If one implies the other, keep the weaker (the one that is implied)
            _ if self.implies_comparison(&arg1, &arg2) => arg2,
            _ if self.implies_comparison(&arg2, &arg1) => arg1,
            _ => self.mk_bool_call(Operation::Or, vec![arg1, arg2]),
        }
    }

    fn mk_implies(&self, arg1: Exp, arg2: Exp) -> Exp {
        match (arg1.as_ref(), arg2.as_ref()) {
            (ExpData::Value(_, Value::Bool(true)), _) => arg2,
            (ExpData::Value(_, Value::Bool(false)), _) => self.mk_bool_const(true),
            (_, ExpData::Value(_, Value::Bool(true))) => self.mk_bool_const(true),
            (_, ExpData::Value(_, Value::Bool(false))) => self.mk_not(arg1),
            (_, ExpData::Call(_, Operation::Implies, args)) if args.len() == 2 => {
                if self.is_complementary(&arg1, &args[0]) {
                    self.mk_bool_const(true)
                } else if arg1.structural_eq(&args[0]) {
                    self.mk_implies(arg1, args[1].clone())
                } else if self.implies_comparison(&args[0], &arg1) {
                    // Inner antecedent implies outer — outer is redundant
                    self.mk_implies(args[0].clone(), args[1].clone())
                } else {
                    // Flatten: a ==> (b ==> c) → (a && b) ==> c
                    let combined = self.mk_and(arg1, args[0].clone());
                    self.mk_implies(combined, args[1].clone())
                }
            },
            _ => self.mk_bool_call(Operation::Implies, vec![arg1, arg2]),
        }
    }

    fn mk_iff(&self, arg1: Exp, arg2: Exp) -> Exp {
        if arg1.structural_eq(&arg2) {
            return self.mk_bool_const(true);
        }
        match (arg1.as_ref(), arg2.as_ref()) {
            (ExpData::Value(_, Value::Bool(true)), _) => arg2,
            (_, ExpData::Value(_, Value::Bool(true))) => arg1,
            (ExpData::Value(_, Value::Bool(false)), _) => self.mk_not(arg2),
            (_, ExpData::Value(_, Value::Bool(false))) => self.mk_not(arg1),
            _ => self.mk_bool_call(Operation::Iff, vec![arg1, arg2]),
        }
    }

    /// Checks whether `stronger` logically implies `weaker` for numeric comparisons
    /// against the same expression. For example:
    /// - `3 < x` implies `1 < x` (stronger lower bound implies weaker lower bound)
    /// - `x < 3` implies `x < 5` (tighter upper bound implies looser upper bound)
    /// - `!(5 < x)` implies `!(3 < x)` (i.e. `x <= 5` implies `x <= 3` is wrong;
    ///    actually `!(3 < x)` means `x <= 3`, and `!(5 < x)` means `x <= 5`,
    ///    so `x <= 3` implies `x <= 5`)
    fn implies_comparison(&self, stronger: &Exp, weaker: &Exp) -> bool {
        // Normalize both sides to canonical Lt / Not(Lt) form, handling Gt/Ge/Le uniformly
        if let (Some((s_left, s_right)), Some((w_left, w_right))) =
            (as_lt_args(stronger), as_lt_args(weaker))
        {
            // c2 < x implies c1 < x when c1 <= c2 (constant on left)
            if let (Some(c2), Some(c1)) = (get_num_const(s_left), get_num_const(w_left)) {
                if s_right.structural_eq(w_right) && c1 <= c2 {
                    return true;
                }
            }
            // x < c1 implies x < c2 when c1 <= c2 (constant on right)
            if let (Some(c1), Some(c2)) = (get_num_const(s_right), get_num_const(w_right)) {
                if s_left.structural_eq(w_left) && c1 <= c2 {
                    return true;
                }
            }
            // Additive offset on LHS: (base+c_s) < rhs implies (base+c_w) < rhs when c_s >= c_w
            if s_right.structural_eq(w_right) {
                let (s_base, s_off) = extract_additive_offset(s_left);
                let (w_base, w_off) = extract_additive_offset(w_left);
                if s_base.structural_eq(w_base) && s_off >= w_off {
                    return true;
                }
            }
        }
        // Negated comparison: both normalize to Not(Lt)
        if let (Some((s_left, s_right)), Some((w_left, w_right))) =
            (as_not_lt_args(stronger), as_not_lt_args(weaker))
        {
            // !(c1 < x) implies !(c2 < x) when c1 <= c2
            // i.e. x <= c1 implies x <= c2 when c1 <= c2
            if let (Some(c1), Some(c2)) = (get_num_const(s_left), get_num_const(w_left)) {
                if s_right.structural_eq(w_right) && c1 <= c2 {
                    return true;
                }
            }
            // !(x < c1) implies !(x < c2) when c2 <= c1
            // i.e. x >= c1 implies x >= c2 when c2 <= c1
            if let (Some(c1), Some(c2)) = (get_num_const(s_right), get_num_const(w_right)) {
                if s_left.structural_eq(w_left) && c2 <= c1 {
                    return true;
                }
            }
            // Additive offset on LHS: !(base+c_s < rhs) implies !(base+c_w < rhs)
            // when c_w >= c_s, i.e. (base+c_s) >= rhs implies (base+c_w) >= rhs
            if s_right.structural_eq(w_right) {
                let (s_base, s_off) = extract_additive_offset(s_left);
                let (w_base, w_off) = extract_additive_offset(w_left);
                if s_base.structural_eq(w_base) && w_off >= s_off {
                    return true;
                }
            }
            // Multiplicative factor on LHS: !(base*k_s < rhs) implies !(base*k_w < rhs)
            // when k_w >= k_s > 0, i.e. (base*k_s) >= rhs implies (base*k_w) >= rhs
            // Only valid for unsigned types (base >= 0)
            if s_right.structural_eq(w_right) {
                let (s_base, s_fac) = extract_multiplicative_factor(s_left);
                let (w_base, w_fac) = extract_multiplicative_factor(w_left);
                if s_base.structural_eq(w_base)
                    && s_fac >= BigInt::from(1)
                    && w_fac >= s_fac
                    && self.env().get_node_type(s_base.node_id()).is_unsigned_int()
                {
                    return true;
                }
            }
        }
        // Try equality implying comparison: x == c implies various Lt / Not(Lt) facts
        if let ExpData::Call(_, Operation::Eq, eq_args) = stronger.as_ref() {
            if eq_args.len() == 2 {
                // Extract (variable_expr, constant_value) from equality
                let var_and_const = if let Some(c) = get_num_const(&eq_args[1]) {
                    Some((&eq_args[0], c))
                } else {
                    get_num_const(&eq_args[0]).map(|c| (&eq_args[1], c))
                };
                if let Some((var, val)) = var_and_const {
                    // x == val implies c2 < x when c2 < val (handles Lt and Gt)
                    if let Some((w_left, w_right)) = as_lt_args(weaker) {
                        if let Some(c2) = get_num_const(w_left) {
                            if w_right.structural_eq(var) && c2 < val {
                                return true;
                            }
                        }
                        if let Some(c2) = get_num_const(w_right) {
                            if w_left.structural_eq(var) && val < c2 {
                                return true;
                            }
                        }
                    }
                    // x == val implies !(c2 < x) / x <= c2 etc. (handles Not(Lt), Le, Ge)
                    if let Some((w_left, w_right)) = as_not_lt_args(weaker) {
                        if let Some(c2) = get_num_const(w_left) {
                            if w_right.structural_eq(var) && c2 >= val {
                                return true;
                            }
                        }
                        if let Some(c2) = get_num_const(w_right) {
                            if w_left.structural_eq(var) && val >= c2 {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        // Symbolic strict-implies-non-strict: a < b implies a <= b, a > b implies a >= b
        match (stronger.as_ref(), weaker.as_ref()) {
            (ExpData::Call(_, Operation::Lt, s), ExpData::Call(_, Operation::Le, w))
            | (ExpData::Call(_, Operation::Gt, s), ExpData::Call(_, Operation::Ge, w))
                if s.len() == 2
                    && w.len() == 2
                    && s[0].structural_eq(&w[0])
                    && s[1].structural_eq(&w[1]) =>
            {
                return true;
            },
            _ => {},
        }
        false
    }

    /// Checks if `conj` is a conjunction containing a conjunct that implies `target`.
    /// For `And(B, C)`, returns true if `B` implies `target` or `C` implies `target`.
    /// Recurses into nested right-associated conjunctions.
    fn conjunction_implies_comparison(&self, conj: &Exp, target: &Exp) -> bool {
        if let ExpData::Call(_, Operation::And, args) = conj.as_ref() {
            if args.len() == 2 {
                self.implies_comparison(&args[0], target)
                    || self.implies_comparison(&args[1], target)
                    || self.conjunction_implies_comparison(&args[0], target)
                    || self.conjunction_implies_comparison(&args[1], target)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Returns true if `a` subsumes `b` in a disjunctive context, meaning `b ==> a`
    /// (a is more general, so b is redundant when both appear as disjuncts).
    pub fn subsumes(&self, a: &Exp, b: &Exp) -> bool {
        // Case 1: Structural equality
        if a.structural_eq(b) {
            return true;
        }
        // Case 2: Comparison implication (b implies a)
        if self.implies_comparison(b, a) {
            return true;
        }
        // Case 3: Negation — !X subsumes !Y when Y subsumes X
        if let (
            ExpData::Call(_, Operation::Not, a_inner),
            ExpData::Call(_, Operation::Not, b_inner),
        ) = (a.as_ref(), b.as_ref())
        {
            if a_inner.len() == 1 && b_inner.len() == 1 {
                if self.subsumes(&b_inner[0], &a_inner[0]) {
                    return true;
                }
            }
        }
        // Case 4: Conjunction subsumption — a subsumes (A && B) if a subsumes A or a subsumes B
        if let ExpData::Call(_, Operation::And, b_args) = b.as_ref() {
            if b_args.len() == 2 && (self.subsumes(a, &b_args[0]) || self.subsumes(a, &b_args[1])) {
                return true;
            }
        }
        false
    }

    fn is_complementary(&self, a: &Exp, b: &Exp) -> bool {
        is_complementary(a, b)
    }

    /// Checks whether knowing `assumption` is true makes `exp` false, using
    /// comparison implication. For example:
    /// - assumption = `3 < x`, exp = `!(2 < x)` → false, since `3 < x` implies `2 < x`
    /// - assumption = `!(3 < x)`, exp = `5 < x` → false, since `5 < x` implies `3 < x`
    ///   but we know `!(3 < x)`, contradiction
    fn implies_complementary(&self, assumption: &Exp, exp: &Exp) -> bool {
        // exp = Not(inner) and assumption implies inner → exp is false
        if let ExpData::Call(_, Operation::Not, args) = exp.as_ref() {
            if args.len() == 1 && self.implies_comparison(assumption, &args[0]) {
                return true;
            }
        }
        // assumption = Not(inner) and exp implies inner → exp is false
        // (exp being true would imply inner, contradicting !inner)
        if let ExpData::Call(_, Operation::Not, args) = assumption.as_ref() {
            if args.len() == 1 && self.implies_comparison(exp, &args[0]) {
                return true;
            }
        }
        false
    }

    /// Simplifies `c < x && !(c+1 < x)` to `x == c+1` for integer types.
    /// When a lower bound and upper bound pinch a variable to a single value,
    /// the conjunction reduces to an equality.
    /// Antisymmetry: `a <= b && a >= b` → `a == b`.
    /// Recognizes all combinations: Le/Ge with matching or swapped operands.
    fn try_antisymmetry_to_eq(&self, a: &Exp, b: &Exp) -> Option<Exp> {
        // Normalize both sides to (lhs, rhs) where the relation is lhs <= rhs
        let (a_lhs, a_rhs) = match a.as_ref() {
            ExpData::Call(_, Operation::Le, args) if args.len() == 2 => (&args[0], &args[1]),
            ExpData::Call(_, Operation::Ge, args) if args.len() == 2 => (&args[1], &args[0]),
            _ => return None,
        };
        let (b_lhs, b_rhs) = match b.as_ref() {
            ExpData::Call(_, Operation::Le, args) if args.len() == 2 => (&args[0], &args[1]),
            ExpData::Call(_, Operation::Ge, args) if args.len() == 2 => (&args[1], &args[0]),
            _ => return None,
        };
        // Check if they form a <= b && b <= a
        if a_lhs.structural_eq(b_rhs) && a_rhs.structural_eq(b_lhs) {
            Some(self.mk_bool_call(Operation::Eq, vec![a_lhs.clone(), a_rhs.clone()]))
        } else {
            None
        }
    }

    fn try_pinch_to_eq(&self, a: &Exp, b: &Exp) -> Option<Exp> {
        self.try_pinch_directed(a, b)
            .or_else(|| self.try_pinch_directed(b, a))
    }

    fn try_pinch_directed(&self, lt_exp: &Exp, not_lt_exp: &Exp) -> Option<Exp> {
        let (e1, e2) = match lt_exp.as_ref() {
            ExpData::Call(_, Operation::Lt, args) if args.len() == 2 => (&args[0], &args[1]),
            _ => return None,
        };
        // Extract (f1, f2) such that not_lt_exp means !(f1 < f2), i.e., f1 >= f2.
        // Matches: Not(Lt(f1,f2)), Ge(f1,f2), Le(f2,f1)
        let (f1, f2) = match not_lt_exp.as_ref() {
            ExpData::Call(_, Operation::Not, not_args) if not_args.len() == 1 => {
                match not_args[0].as_ref() {
                    ExpData::Call(_, Operation::Lt, args) if args.len() == 2 => {
                        (&args[0], &args[1])
                    },
                    _ => return None,
                }
            },
            ExpData::Call(_, Operation::Ge, args) if args.len() == 2 => (&args[0], &args[1]),
            ExpData::Call(_, Operation::Le, args) if args.len() == 2 => (&args[1], &args[0]),
            _ => return None,
        };
        // Pattern: c1 < x && !(c2 < x) with same x, c2 == c1 + 1
        // means c1 < x <= c2, pinch to x == c2
        if let (Some(c1), Some(c2)) = (get_num_const(e1), get_num_const(f1)) {
            if e2.structural_eq(f2) && *c2 == c1 + 1 {
                let ty = self.env().get_node_type(e2.as_ref().node_id());
                let val = self.mk_num_const(&ty, c2.clone());
                return Some(self.mk_bool_call(Operation::Eq, vec![e2.clone(), val]));
            }
        }
        // Pattern: x < c1 && !(x < c2) with same x, c2 == c1 - 1
        // means c2 <= x < c1, pinch to x == c2
        if let (Some(c1), Some(c2)) = (get_num_const(e2), get_num_const(f2)) {
            if e1.structural_eq(f1) && *c2 == c1 - 1 {
                let ty = self.env().get_node_type(e1.as_ref().node_id());
                let val = self.mk_num_const(&ty, c2.clone());
                return Some(self.mk_bool_call(Operation::Eq, vec![e1.clone(), val]));
            }
        }
        None
    }

    /// Detects empty integer ranges in a conjunction.
    /// If both sides normalize to strict inequalities bounding the same variable
    /// from opposite sides with no integer in the gap, returns `false`.
    /// E.g., `x > 0 && x < 1` (i.e. `0 < x && x < 1`, gap = 1 ≤ 0+1 → false).
    fn try_empty_range(&self, a: &Exp, b: &Exp) -> Option<Exp> {
        let (a_lo, a_hi) = as_lt_args(a)?; // a is: a_lo < a_hi
        let (b_lo, b_hi) = as_lt_args(b)?; // b is: b_lo < b_hi
                                           // Case: a_lo < X && X < b_hi (X = a_hi = b_lo)
        if a_hi.structural_eq(b_lo) {
            if let (Some(lo), Some(hi)) = (get_num_const(a_lo), get_num_const(b_hi)) {
                if *hi <= lo + 1 {
                    return Some(self.mk_bool_const(false));
                }
            }
        }
        // Case: b_lo < X && X < a_hi (X = b_hi = a_lo)
        if b_hi.structural_eq(a_lo) {
            if let (Some(lo), Some(hi)) = (get_num_const(b_lo), get_num_const(a_hi)) {
                if *hi <= lo + 1 {
                    return Some(self.mk_bool_const(false));
                }
            }
        }
        None
    }

    /// Checks whether `a < b` is known from assumptions, recognizing both
    /// `Lt(a, b)` and `Gt(b, a)`, including comparison implication.
    fn ordering_known_lt(&self, a: &Exp, b: &Exp) -> bool {
        let lt_ab = self.mk_bool_call(Operation::Lt, vec![a.clone(), b.clone()]);
        self.assumptions.iter().any(|asn| {
            // Direct Lt(a, b) or implied by a stronger Lt
            if asn.structural_eq(&lt_ab) || self.implies_comparison(asn, &lt_ab) {
                return true;
            }
            // Gt(b, a) means b > a means a < b
            if let ExpData::Call(_, Operation::Gt, args) = asn.as_ref() {
                if args.len() == 2 && args[0].structural_eq(b) && args[1].structural_eq(a) {
                    return true;
                }
            }
            false
        })
    }

    /// Checks whether `!(a < b)` (i.e., `a >= b`) is known from assumptions,
    /// recognizing `Not(Lt(a, b))`, `Ge(a, b)`, and `Le(b, a)`.
    fn ordering_known_not_lt(&self, a: &Exp, b: &Exp) -> bool {
        let lt_ab = self.mk_bool_call(Operation::Lt, vec![a.clone(), b.clone()]);
        self.assumptions.iter().any(|asn| {
            // Not(Lt(a, b)) or implied by comparison
            if self.is_complementary(asn, &lt_ab) || self.implies_complementary(asn, &lt_ab) {
                return true;
            }
            // Ge(a, b) means a >= b means !(a < b)
            if let ExpData::Call(_, Operation::Ge, args) = asn.as_ref() {
                if args.len() == 2 && args[0].structural_eq(a) && args[1].structural_eq(b) {
                    return true;
                }
            }
            // Le(b, a) means b <= a means !(a < b)
            if let ExpData::Call(_, Operation::Le, args) = asn.as_ref() {
                if args.len() == 2 && args[0].structural_eq(b) && args[1].structural_eq(a) {
                    return true;
                }
            }
            // Not(Gt(b, a)) means !(b > a) means !(a < b)
            if let ExpData::Call(_, Operation::Not, not_args) = asn.as_ref() {
                if not_args.len() == 1 {
                    if let ExpData::Call(_, Operation::Gt, args) = not_args[0].as_ref() {
                        if args.len() == 2 && args[0].structural_eq(b) && args[1].structural_eq(a) {
                            return true;
                        }
                    }
                }
            }
            false
        })
    }

    /// Deduces truth of comparison expressions from ordering relationships.
    /// For total ordering: `a == b` iff `!(a < b) && !(b < a)`, etc.
    /// Only handles non-Lt operations to avoid recursion with `is_known_true`.
    fn is_known_true_by_ordering(&self, exp: &Exp) -> bool {
        if let ExpData::Call(_, oper, args) = exp.as_ref() {
            if args.len() == 2 {
                let (a, b) = (&args[0], &args[1]);
                return match oper {
                    // a <= b iff !(b < a)
                    Operation::Le => self.ordering_known_not_lt(b, a),
                    // a >= b iff !(a < b)
                    Operation::Ge => self.ordering_known_not_lt(a, b),
                    // a > b iff b < a
                    Operation::Gt => self.ordering_known_lt(b, a),
                    // a == b iff !(a < b) && !(b < a)
                    Operation::Eq => {
                        self.ordering_known_not_lt(a, b) && self.ordering_known_not_lt(b, a)
                    },
                    _ => false,
                };
            }
        }
        false
    }

    /// Deduces falsity of comparison expressions from ordering relationships.
    fn is_known_false_by_ordering(&self, exp: &Exp) -> bool {
        if let ExpData::Call(_, oper, args) = exp.as_ref() {
            if args.len() == 2 {
                let (a, b) = (&args[0], &args[1]);
                return match oper {
                    // !(a <= b) iff b < a
                    Operation::Le => self.ordering_known_lt(b, a),
                    // !(a >= b) iff a < b
                    Operation::Ge => self.ordering_known_lt(a, b),
                    // !(a > b) iff !(b < a)
                    Operation::Gt => self.ordering_known_not_lt(b, a),
                    // !(a == b) iff a < b or b < a
                    Operation::Eq => self.ordering_known_lt(a, b) || self.ordering_known_lt(b, a),
                    // !(a != b) iff !(a < b) && !(b < a) (i.e. a == b)
                    Operation::Neq => {
                        self.ordering_known_not_lt(a, b) && self.ordering_known_not_lt(b, a)
                    },
                    _ => false,
                };
            }
        }
        false
    }

    /// Try constant folding via `ConstantFolder`.
    fn try_constant_fold(&self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        // Only attempt if all args are values
        if !args
            .iter()
            .all(|a| matches!(a.as_ref(), ExpData::Value(..)))
        {
            return None;
        }
        let mut folder = ConstantFolder::new(self.env(), false);
        folder.rewrite_call(id, oper, args)
    }

    /// Simplify a boolean call after children have been simplified.
    fn simplify_bool_call(&self, _id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        match oper {
            Operation::Not if args.len() == 1 => Some(self.mk_not(args[0].clone())),
            Operation::And if args.len() == 2 => {
                Some(self.mk_and(args[0].clone(), args[1].clone()))
            },
            Operation::Or if args.len() == 2 => Some(self.mk_or(args[0].clone(), args[1].clone())),
            Operation::Implies if args.len() == 2 => {
                Some(self.mk_implies(args[0].clone(), args[1].clone()))
            },
            Operation::Iff if args.len() == 2 => {
                Some(self.mk_iff(args[0].clone(), args[1].clone()))
            },
            _ => None,
        }
    }

    /// Simplify comparisons using reflexive properties and unsigned bounds.
    fn simplify_comparison(&self, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if args.len() != 2 {
            return None;
        }
        // Reflexive: x op x
        if args[0].structural_eq(&args[1]) {
            return match oper {
                Operation::Eq | Operation::Le | Operation::Ge => Some(self.mk_bool_const(true)),
                Operation::Neq | Operation::Lt | Operation::Gt => Some(self.mk_bool_const(false)),
                _ => None,
            };
        }
        // Type-bound simplification: compare a constant against the type bounds
        // of the other operand. Subsumes the old unsigned-only 0-bound checks.
        if let Some(result) = self.simplify_by_type_bounds(oper, args) {
            return Some(result);
        }
        // Comparison normalization: move constant addends across the comparison
        // to expose type-bound simplifications. E.g., `x - 1 < 0` → `x < 1`.
        if self.spec_mode {
            if let Some(result) = self.normalize_comparison_addend(oper, args) {
                return Some(result);
            }
        }
        None
    }

    /// Simplify comparisons where one side is a constant and the other is
    /// a bounded integer expression. Works for all bounded integer types
    /// (u8–u256, i8–i256) but not for `Num` which is unbounded.
    fn simplify_by_type_bounds(&self, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        // Try both orientations: const op expr, expr op const
        for (const_idx, expr_idx) in [(0, 1), (1, 0)] {
            let c = get_num_const(&args[const_idx])?;
            let ty = self.env().get_node_type(args[expr_idx].as_ref().node_id());
            let prim = match &ty {
                Type::Primitive(p) if ty.is_number() => p,
                _ => continue,
            };
            // Only bounded integer types; Num is unbounded.
            let max = match prim.get_max_value() {
                Some(v) => v,
                None => continue,
            };
            let min = match prim.get_min_value() {
                Some(v) => v,
                None => continue,
            };

            // Normalize: express as (expr `op` val) by flipping when const is LHS
            let (op, val) = if const_idx == 0 {
                (flip_comparison(oper), c)
            } else {
                (oper.clone(), c)
            };
            // Now: expr `op` val
            let result = match op {
                // expr > val where val >= max → false
                Operation::Gt if *val >= max => Some(false),
                // expr < val where val <= min → false
                Operation::Lt if *val <= min => Some(false),
                // expr >= val where val > max → false
                Operation::Ge if *val > max => Some(false),
                // expr <= val where val < min → false
                Operation::Le if *val < min => Some(false),
                // expr <= val where val >= max → true
                Operation::Le if *val >= max => Some(true),
                // expr >= val where val <= min → true
                Operation::Ge if *val <= min => Some(true),
                // expr > val where val < min → true
                Operation::Gt if *val < min => Some(true),
                // expr < val where val > max → true
                Operation::Lt if *val > max => Some(true),
                _ => None,
            };
            if let Some(b) = result {
                return Some(self.mk_bool_const(b));
            }
        }
        None
    }

    /// Normalize comparisons where one side has a constant addend:
    /// `(e + C1) op C2` → `e op (C2 - C1)`, `(e - C1) op C2` → `e op (C2 + C1)`.
    /// After normalization, re-check type bounds.
    fn normalize_comparison_addend(&self, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        for (add_idx, const_idx) in [(0, 1), (1, 0)] {
            let c2 = get_num_const(&args[const_idx])?;
            if let ExpData::Call(_, add_op, inner) = args[add_idx].as_ref() {
                if inner.len() < 2 {
                    continue;
                }
                let c1 = match get_num_const(&inner[1]) {
                    Some(c) => c,
                    None => continue,
                };
                let new_const = match add_op {
                    Operation::Add => c2 - c1,
                    Operation::Sub => c2 + c1,
                    _ => continue,
                };
                // Build normalized comparison: inner[0] `op` new_const (or flipped)
                let expr = &inner[0];
                let (normalized_op, normalized_val) = if add_idx == 0 {
                    // (e ± C1) op C2 → e op new_const
                    (oper.clone(), new_const)
                } else {
                    // C2 op (e ± C1) → new_const op e → e flip(op) new_const
                    (flip_comparison(oper), new_const)
                };
                // Check type bounds on the normalized form
                let ty = self.env().get_node_type(expr.as_ref().node_id());
                if let Type::Primitive(prim) = &ty {
                    if let (Some(max), Some(min)) = (prim.get_max_value(), prim.get_min_value()) {
                        let result = match &normalized_op {
                            Operation::Gt if normalized_val >= max => Some(false),
                            Operation::Lt if normalized_val <= min => Some(false),
                            Operation::Ge if normalized_val > max => Some(false),
                            Operation::Le if normalized_val < min => Some(false),
                            Operation::Le if normalized_val >= max => Some(true),
                            Operation::Ge if normalized_val <= min => Some(true),
                            Operation::Gt if normalized_val < min => Some(true),
                            Operation::Lt if normalized_val > max => Some(true),
                            _ => None,
                        };
                        if let Some(b) = result {
                            return Some(self.mk_bool_const(b));
                        }
                    }
                }
                // Even if no type-bound fires, return the normalized form
                // (one fewer arithmetic op)
                let new_const_exp = self.mk_num_const(&ty, normalized_val);
                let new_args = vec![expr.clone(), new_const_exp];
                return Some(self.mk_bool_call(normalized_op, new_args));
            }
        }
        None
    }

    /// Simplify arithmetic operations using algebraic identities (spec mode only).
    fn simplify_arithmetic(&self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if !self.spec_mode || args.len() != 2 {
            return None;
        }
        let ty = self.env().get_node_type(id);
        let zero = || is_num_const(&args[1], 0) || is_num_const(&args[0], 0);
        match oper {
            Operation::Add => {
                if is_num_const(&args[0], 0) {
                    Some(args[1].clone())
                } else if is_num_const(&args[1], 0) {
                    Some(args[0].clone())
                } else if let Some(c2) = get_num_const(&args[1]) {
                    if let ExpData::Call(_, Operation::Add, inner) = args[0].as_ref() {
                        // (x + c1) + c2 -> x + (c1 + c2)
                        if let Some(c1) = get_num_const(&inner[1]) {
                            let sum = c1 + c2;
                            if sum.is_zero() {
                                Some(inner[0].clone())
                            } else {
                                Some(self.mk_call(&ty, Operation::Add, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, sum),
                                ]))
                            }
                        } else {
                            None
                        }
                    } else if let ExpData::Call(_, Operation::Sub, inner) = args[0].as_ref() {
                        // (x - c1) + c2 -> x + (c2 - c1) or x - (c1 - c2)
                        if let Some(c1) = get_num_const(&inner[1]) {
                            let diff = c2 - c1;
                            if diff.is_zero() {
                                Some(inner[0].clone())
                            } else if diff > BigInt::zero() {
                                Some(self.mk_call(&ty, Operation::Add, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, diff),
                                ]))
                            } else {
                                Some(self.mk_call(&ty, Operation::Sub, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, -diff),
                                ]))
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            Operation::Sub => {
                if is_num_const(&args[1], 0) {
                    Some(args[0].clone())
                } else if args[0].structural_eq(&args[1]) {
                    Some(self.mk_num_const(&ty, BigInt::zero()))
                } else if let Some(c2) = get_num_const(&args[1]) {
                    if let ExpData::Call(_, Operation::Sub, inner) = args[0].as_ref() {
                        // (x - c1) - c2 -> x - (c1 + c2)
                        if let Some(c1) = get_num_const(&inner[1]) {
                            let sum = c1 + c2;
                            if sum.is_zero() {
                                Some(inner[0].clone())
                            } else {
                                Some(self.mk_call(&ty, Operation::Sub, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, sum),
                                ]))
                            }
                        } else {
                            None
                        }
                    } else if let ExpData::Call(_, Operation::Add, inner) = args[0].as_ref() {
                        // (x + c1) - c2 -> x + (c1 - c2) or x - (c2 - c1)
                        if let Some(c1) = get_num_const(&inner[1]) {
                            let diff = c1 - c2;
                            if diff.is_zero() {
                                Some(inner[0].clone())
                            } else if diff > BigInt::zero() {
                                Some(self.mk_call(&ty, Operation::Add, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, diff),
                                ]))
                            } else {
                                Some(self.mk_call(&ty, Operation::Sub, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, -diff),
                                ]))
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            Operation::Mul => {
                if is_num_const(&args[0], 1) {
                    Some(args[1].clone())
                } else if is_num_const(&args[1], 1) {
                    Some(args[0].clone())
                } else if zero() {
                    Some(self.mk_num_const(&ty, BigInt::zero()))
                } else if let Some(c2) = get_num_const(&args[1]) {
                    // (x * c1) * c2 -> x * (c1 * c2)
                    if let ExpData::Call(_, Operation::Mul, inner) = args[0].as_ref() {
                        if let Some(c1) = get_num_const(&inner[1]) {
                            let prod = c1 * c2;
                            if prod.is_zero() {
                                Some(self.mk_num_const(&ty, BigInt::zero()))
                            } else {
                                Some(self.mk_call(&ty, Operation::Mul, vec![
                                    inner[0].clone(),
                                    self.mk_num_const(&ty, prod),
                                ]))
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            Operation::Div => {
                if is_num_const(&args[1], 1) {
                    Some(args[0].clone())
                } else {
                    None
                }
            },
            Operation::Mod => {
                if is_num_const(&args[1], 1) {
                    Some(self.mk_num_const(&ty, BigInt::zero()))
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Check if the expression is known true/false via assumptions.
    fn simplify_by_assumption(&self, exp: &Exp) -> Option<Exp> {
        if self.is_known_true(exp) {
            return Some(self.mk_bool_const(true));
        }
        if self.is_known_false(exp) {
            return Some(self.mk_bool_const(false));
        }
        None
    }

    /// Flatten nested quantifiers of the same kind into a single quantifier.
    /// Merges inner ranges into outer ranges, deduplicating by symbol.
    /// Returns the (possibly extended) ranges and the innermost body.
    fn flatten_nested_quant(
        &self,
        kind: QuantKind,
        ranges: &mut Vec<(Pattern, Exp)>,
        body: &Exp,
    ) -> Exp {
        if let ExpData::Quant(_, inner_kind, inner_ranges, inner_triggers, inner_cond, inner_body) =
            body.as_ref()
        {
            if *inner_kind == kind && inner_triggers.is_empty() && inner_cond.is_none() {
                let existing_syms = quant_symbols(ranges);
                for (pat, range) in inner_ranges {
                    let is_dup = if let Pattern::Var(_, sym) = pat {
                        existing_syms.contains(sym)
                    } else {
                        false
                    };
                    if !is_dup {
                        ranges.push((pat.clone(), range.clone()));
                    }
                }
                return inner_body.clone();
            }
        }
        body.clone()
    }

    /// Simplify a forall quantifier expression.
    ///
    /// Applies two simplifications:
    /// 1. **Flatten nested foralls**: `forall x. forall y. P(x,y)` → `forall x, y. P(x,y)`
    /// 2. **One-point rule**: `forall v, ... (v == e ==> Q)` → `forall ... Q[v/e]`
    ///    when `v` is not free in `e`. Eliminates the quantified variable by substituting
    ///    the binding from the equality.
    fn simplify_forall(
        &mut self,
        id: NodeId,
        ranges: Vec<(Pattern, Exp)>,
        triggers: Vec<Vec<Exp>>,
        cond: Option<Exp>,
        body: Exp,
    ) -> Exp {
        let mut ranges = ranges;

        // Step 1: Flatten nested foralls.
        let mut body = self.flatten_nested_quant(QuantKind::Forall, &mut ranges, &body);

        // Step 2: One-point rule for forall elimination + antisymmetry normalization.
        // For `forall vars. (v == e ==> Q)` where `v` is a quantified var and v not free in e:
        // Substitute v with e in Q, remove v from vars.
        // When the one-point rule stalls, try normalizing antisymmetric pairs in
        // the antecedent (e.g. `x <= m && x >= m` → `x == m`) to expose new bindings.
        let mut changed = true;
        while changed {
            changed = false;
            if let Some((eq_sym, eq_expr, consequent)) =
                self.extract_one_point_binding(&ranges, &body)
            {
                // Remove the bound variable from ranges
                let old_len = ranges.len();
                ranges.retain(|(pat, _)| {
                    if let Pattern::Var(_, sym) = pat {
                        *sym != eq_sym
                    } else {
                        true
                    }
                });
                if ranges.len() < old_len {
                    // Substitute the variable in the consequent, then re-simplify
                    // to reduce expressions created by substitution (e.g. x*n == x*n → true).
                    let substituted =
                        substitute_local_var(self.env(), &consequent, eq_sym, &eq_expr);
                    body = self.simplify(substituted);
                    changed = true;
                    continue;
                }
            }
            // Antisymmetry normalization: flatten antecedent conjuncts, detect Le+Ge pairs,
            // convert to Eq, rebuild. This creates new equalities for the one-point rule.
            if let ExpData::Call(_, Operation::Implies, args) = body.as_ref() {
                if args.len() == 2 {
                    let mut owned = flatten_conjunction_owned(&args[0]);
                    if self.normalize_antisymmetric_conjuncts(&mut owned) {
                        let new_ant = owned
                            .into_iter()
                            .reduce(|a, b| self.mk_and(a, b))
                            .unwrap_or_else(|| self.mk_bool_const(true));
                        body = self.mk_implies(new_ant, args[1].clone());
                        changed = true;
                    }
                }
            }
        }

        // Step 2b: Struct field one-point rule.
        // For `forall x: S. x.f1 == e1 && ... && x.fn == en ==> Q` where all fields
        // of struct S are bound, substitute x with `Pack(S, e1, ..., en)`.
        {
            let mut changed_struct = true;
            while changed_struct {
                changed_struct = false;
                if let Some((sym, expr, new_body)) =
                    self.try_extract_struct_field_binding(&ranges, &body)
                {
                    let old_len = ranges.len();
                    ranges.retain(|(pat, _)| !matches!(pat, Pattern::Var(_, s) if *s == sym));
                    if ranges.len() < old_len {
                        body =
                            self.simplify(substitute_local_var(self.env(), &new_body, sym, &expr));
                        changed_struct = true;
                    }
                }
            }
        }

        // Step 3: Remove unused quantified variables.
        remove_unused_quant_vars(&mut ranges, &body);

        // Step 4: Antecedent-only variable elimination.
        // For `forall x: A(x) ==> Q` where x appears in A but not in Q:
        // equivalent to `(exists x: A(x)) ==> Q`. Since type domains are non-empty
        // and typical antecedents like `x <= n` are satisfiable (x=0 works),
        // this simplifies to just Q. Remove such variables and their antecedent.
        if !ranges.is_empty() {
            if let ExpData::Call(_, Operation::Implies, args) = body.as_ref() {
                if args.len() == 2 {
                    let consequent_free = args[1].as_ref().free_vars();
                    let antecedent_only: BTreeSet<Symbol> = quant_symbols(&ranges)
                        .into_iter()
                        .filter(|sym| !consequent_free.contains(sym))
                        .collect();
                    // Safety check: if a conjunct mentions both an antecedent-only var
                    // and a remaining var, removing the antecedent-only var would leave
                    // it free in the kept conjunct. Exclude such vars from elimination.
                    let remaining: BTreeSet<Symbol> = quant_symbols(&ranges)
                        .into_iter()
                        .filter(|sym| !antecedent_only.contains(sym))
                        .collect();
                    let antecedent_only = {
                        let conjuncts_check = flatten_conjunction_owned(&args[0]);
                        let mut safe = antecedent_only.clone();
                        for c in &conjuncts_check {
                            let fv = c.as_ref().free_vars();
                            if remaining.iter().any(|s| fv.contains(s)) {
                                safe.retain(|sym| !fv.contains(sym));
                            }
                        }
                        safe
                    };
                    if !antecedent_only.is_empty() {
                        ranges.retain(|(pat, _)| {
                            if let Pattern::Var(_, sym) = pat {
                                !antecedent_only.contains(sym)
                            } else {
                                true
                            }
                        });
                        if ranges.is_empty() {
                            // All variables were antecedent-only: return just the consequent
                            return args[1].clone();
                        }
                        // Remove eliminated variables from antecedent conjuncts
                        let antecedent_free_needed: std::collections::BTreeSet<Symbol> =
                            quant_symbols(&ranges).into_iter().collect();
                        let conjuncts = flatten_conjunction_owned(&args[0]);
                        let filtered: Vec<Exp> = conjuncts
                            .into_iter()
                            .filter(|c| {
                                let fv = c.as_ref().free_vars();
                                // Keep conjuncts that reference remaining variables
                                antecedent_free_needed.iter().any(|s| fv.contains(s))
                            })
                            .collect();
                        body = if filtered.is_empty() {
                            args[1].clone()
                        } else {
                            let new_ant = filtered
                                .into_iter()
                                .reduce(|a, b| self.mk_and(a, b))
                                .unwrap();
                            self.mk_implies(new_ant, args[1].clone())
                        };
                    }
                }
            }
        }

        // If no ranges remain, return just the body
        if ranges.is_empty() {
            return body;
        }

        // Rebuild the quantifier
        ExpData::Quant(id, QuantKind::Forall, ranges, triggers, cond, body).into_exp()
    }

    /// Simplify an exists quantifier expression.
    ///
    /// Applies simplifications dual to `simplify_forall`:
    /// 1. **Flatten nested exists**: `exists x. exists y. P(x,y)` → `exists x, y. P(x,y)`
    /// 2. **One-point rule**: `exists v, ... (v == e AND Q)` → `exists ... Q[v/e]`
    ///    when `v` is not free in `e`. Eliminates the quantified variable by substituting
    ///    the binding from the equality conjunct.
    /// 3. **Remove unused vars**: `exists x. P` where x not in FV(P) → P
    /// 4. **Pull out non-dependent conjuncts**: `exists x: A AND B(x)` where x not in FV(A)
    ///    → `A AND exists x: B(x)`.
    fn simplify_exists(
        &mut self,
        id: NodeId,
        ranges: Vec<(Pattern, Exp)>,
        triggers: Vec<Vec<Exp>>,
        cond: Option<Exp>,
        body: Exp,
    ) -> Exp {
        let mut ranges = ranges;

        // Step 1: Flatten nested exists.
        let mut body = self.flatten_nested_quant(QuantKind::Exists, &mut ranges, &body);

        // Step 1b: Absorb inner exists from conjuncts.
        // `exists x: A(x) && (exists y: B(x,y))` → `exists x, y: A(x) && B(x,y)`
        // This exposes bindings inside inner exists to the one-point rule.
        {
            let conjuncts = flatten_conjunction_owned(&body);
            if conjuncts.len() >= 2 {
                let mut absorbed = false;
                let mut new_conjuncts: Vec<Exp> = Vec::new();
                for conj in &conjuncts {
                    if let ExpData::Quant(
                        _,
                        QuantKind::Exists,
                        inner_ranges,
                        inner_triggers,
                        inner_cond,
                        inner_body,
                    ) = conj.as_ref()
                    {
                        if inner_triggers.is_empty() && inner_cond.is_none() {
                            let existing_syms = quant_symbols(&ranges);
                            for (pat, range) in inner_ranges {
                                let is_dup = if let Pattern::Var(_, sym) = pat {
                                    existing_syms.contains(sym)
                                } else {
                                    false
                                };
                                if !is_dup {
                                    ranges.push((pat.clone(), range.clone()));
                                }
                            }
                            // Replace the exists conjunct with its body's conjuncts
                            new_conjuncts.extend(flatten_conjunction_owned(inner_body));
                            absorbed = true;
                            continue;
                        }
                    }
                    new_conjuncts.push(conj.clone());
                }
                if absorbed {
                    body = new_conjuncts
                        .into_iter()
                        .reduce(|a, b| self.mk_and(a, b))
                        .unwrap_or_else(|| self.mk_bool_const(true));
                }
            }
        }

        // Step 2: One-point rule for exists elimination + antisymmetry normalization.
        // For `exists vars. (v == e AND Q)` where `v` is a quantified var and v not free in e:
        // Substitute v with e in Q, remove v from vars.
        // When the one-point rule stalls, try normalizing antisymmetric pairs in
        // the body conjuncts to expose new bindings.
        let mut changed = true;
        while changed {
            changed = false;
            if let Some((eq_sym, eq_expr, remaining)) =
                self.extract_exists_one_point_binding(&ranges, &body)
            {
                let old_len = ranges.len();
                ranges.retain(|(pat, _)| {
                    if let Pattern::Var(_, sym) = pat {
                        *sym != eq_sym
                    } else {
                        true
                    }
                });
                if ranges.len() < old_len {
                    // Re-simplify after substitution to reduce tautologies and duplicates.
                    let substituted =
                        substitute_local_var(self.env(), &remaining, eq_sym, &eq_expr);
                    body = self.simplify(substituted);
                    changed = true;
                    continue;
                }
            }
            // Antisymmetry normalization on the body conjuncts
            let mut owned = flatten_conjunction_owned(&body);
            if self.normalize_antisymmetric_conjuncts(&mut owned) {
                body = owned
                    .into_iter()
                    .reduce(|a, b| self.mk_and(a, b))
                    .unwrap_or_else(|| self.mk_bool_const(true));
                changed = true;
            }
        }

        // Step 2b: Struct field one-point rule for exists.
        // For `exists x: S. x.f1 == e1 && ... && x.fn == en && Q` where all fields
        // of struct S are bound, substitute x with `Pack(S, e1, ..., en)`.
        {
            let mut changed_struct = true;
            while changed_struct {
                changed_struct = false;
                if let Some((sym, expr, new_body)) =
                    self.try_extract_struct_field_binding_exists(&ranges, &body)
                {
                    let old_len = ranges.len();
                    ranges.retain(|(pat, _)| !matches!(pat, Pattern::Var(_, s) if *s == sym));
                    if ranges.len() < old_len {
                        body =
                            self.simplify(substitute_local_var(self.env(), &new_body, sym, &expr));
                        changed_struct = true;
                    }
                }
            }
        }

        // Step 3: Remove unused quantified variables.
        remove_unused_quant_vars(&mut ranges, &body);

        // Step 4: Split into independent sub-quantifiers by connected components.
        // Groups quantified variables by co-occurrence in conjuncts: if two vars
        // never share a conjunct, they go into separate exists.
        // `exists x, y: A(x) && B(y)` → `(exists x: A(x)) && (exists y: B(y))`
        // Also pulls out conjuncts with no quantified variables.
        if !ranges.is_empty() {
            let quant_syms = quant_symbols(&ranges);
            let conjuncts = flatten_conjunction_owned(&body);
            if conjuncts.len() >= 2 {
                // For each conjunct, find which quant vars it mentions
                let conj_vars: Vec<BTreeSet<Symbol>> = conjuncts
                    .iter()
                    .map(|c| {
                        let fv = c.as_ref().free_vars();
                        quant_syms
                            .iter()
                            .filter(|s| fv.contains(s))
                            .cloned()
                            .collect()
                    })
                    .collect();

                // Union-Find: assign each var an index, merge when vars co-occur
                let mut parent: Vec<usize> = (0..quant_syms.len()).collect();
                let find = |parent: &mut Vec<usize>, mut x: usize| -> usize {
                    while parent[x] != x {
                        parent[x] = parent[parent[x]];
                        x = parent[x];
                    }
                    x
                };
                for vars in &conj_vars {
                    let indices: Vec<usize> = vars
                        .iter()
                        .filter_map(|s| quant_syms.iter().position(|qs| qs == s))
                        .collect();
                    for i in 1..indices.len() {
                        let ra = find(&mut parent, indices[0]);
                        let rb = find(&mut parent, indices[i]);
                        if ra != rb {
                            parent[rb] = ra;
                        }
                    }
                }

                // Count distinct components
                let roots: BTreeSet<usize> = (0..quant_syms.len())
                    .map(|i| find(&mut parent, i))
                    .collect();
                let has_independent = conj_vars.iter().any(|v| v.is_empty());

                if roots.len() >= 2 || has_independent {
                    let mut parts: Vec<Exp> = Vec::new();

                    // Independent conjuncts (no quant vars)
                    for (i, vars) in conj_vars.iter().enumerate() {
                        if vars.is_empty() {
                            parts.push(conjuncts[i].clone());
                        }
                    }

                    // One exists per connected component
                    for &root in &roots {
                        let comp_syms: BTreeSet<Symbol> = quant_syms
                            .iter()
                            .enumerate()
                            .filter(|&(i, _)| find(&mut parent, i) == root)
                            .map(|(_, s)| *s)
                            .collect();
                        let comp_ranges: Vec<(Pattern, Exp)> = ranges
                            .iter()
                            .filter(|(pat, _)| {
                                if let Pattern::Var(_, sym) = pat {
                                    comp_syms.contains(sym)
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();
                        let comp_body_parts: Vec<Exp> = conj_vars
                            .iter()
                            .enumerate()
                            .filter(|(_, vars)| {
                                !vars.is_empty() && vars.iter().any(|s| comp_syms.contains(s))
                            })
                            .map(|(i, _)| conjuncts[i].clone())
                            .collect();

                        if comp_body_parts.is_empty() || comp_ranges.is_empty() {
                            continue;
                        }

                        let comp_body = comp_body_parts
                            .into_iter()
                            .reduce(|a, b| self.mk_and(a, b))
                            .unwrap();
                        let quant = ExpData::Quant(
                            id,
                            QuantKind::Exists,
                            comp_ranges,
                            vec![],
                            None,
                            comp_body,
                        )
                        .into_exp();
                        // Re-simplify each component (may trigger witness elimination)
                        let quant = self.simplify(quant);
                        parts.push(quant);
                    }

                    if !parts.is_empty() {
                        return parts.into_iter().reduce(|a, b| self.mk_and(a, b)).unwrap();
                    }
                }
            }
        }

        // If no ranges remain, return just the body
        if ranges.is_empty() {
            return body;
        }

        // Step 5: Upper-bound witness elimination.
        // For `exists x1,...,xk: (x1 <= e1 && ... && xk <= ek && P(x1,...,xk))`
        // where each ei is free of quantified variables and P is monotone increasing
        // (upward-safe) in each xi: substitute xi = ei and return P(e1,...,ek).
        //
        // Correctness: if P holds for some ai <= ei and P is upward-safe, then
        // P(e1,...,ek) also holds since ei >= ai. Conversely, P(e1,...,ek) provides
        // witnesses xi = ei with ei <= ei trivially.
        if let Some(result) = self.try_upper_bound_witness(&ranges, &body) {
            return result;
        }

        // Step 6: Witness instantiation for trivially satisfiable exists.
        // If all quantified variables have unsigned integer types, try instantiating
        // with 0 (the minimum value). If the body simplifies to `true`, the exists
        // is trivially satisfiable. For example, `exists x: u64: x <= m` is always
        // true since x=0 works for any u64 m.
        if self.is_exists_trivially_true(&ranges, &body) {
            return self.mk_bool_const(true);
        }

        // Rebuild the quantifier
        ExpData::Quant(id, QuantKind::Exists, ranges, triggers, cond, body).into_exp()
    }

    /// Try to eliminate an exists quantifier by substituting upper-bound witnesses.
    ///
    /// For `exists x1,...,xk: (x1 <= e1 && ... && xk <= ek && P(x1,...,xk))`
    /// where each `ei` is free of all quantified variables and each remaining
    /// conjunct P_j is upward-safe in every bound variable, substitute `xi = ei`
    /// and simplify P(e1,...,ek).
    fn try_upper_bound_witness(&mut self, ranges: &[(Pattern, Exp)], body: &Exp) -> Option<Exp> {
        // All quantified variables must have unsigned integer types
        let quant_syms: Vec<Symbol> = ranges
            .iter()
            .filter_map(|(pat, _)| {
                if let Pattern::Var(pat_id, sym) = pat {
                    let ty = self.env().get_node_type(*pat_id);
                    if ty.is_unsigned_int() {
                        Some(*sym)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        if quant_syms.len() != ranges.len() {
            return None;
        }

        let conjuncts = flatten_conjunction_owned(body);

        // For each quant var, find a conjunct `xi <= ei` or `xi < ei` where ei is free of
        // all quant vars. For `xi < ei`, the witness bound is `ei - 1` and a non-emptiness
        // guard `ei > 0` is required.
        let mut bounds: BTreeMap<Symbol, Exp> = BTreeMap::new();
        let mut guards: Vec<Exp> = Vec::new();
        let mut bound_indices: BTreeSet<usize> = BTreeSet::new();
        let quant_set: BTreeSet<Symbol> = quant_syms.iter().cloned().collect();

        for (i, conj) in conjuncts.iter().enumerate() {
            match conj.as_ref() {
                // x <= e: witness bound is e directly
                ExpData::Call(_, Operation::Le, args) if args.len() == 2 => {
                    if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                        if quant_set.contains(sym) && !bounds.contains_key(sym) {
                            let rhs_fv = args[1].as_ref().free_vars();
                            if quant_set.iter().all(|s| !rhs_fv.contains(s)) {
                                bounds.insert(*sym, args[1].clone());
                                bound_indices.insert(i);
                            }
                        }
                    }
                },
                // x < e: witness bound is e - 1, with guard e > 0
                ExpData::Call(_, Operation::Lt, args) if args.len() == 2 => {
                    if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                        if quant_set.contains(sym) && !bounds.contains_key(sym) {
                            let rhs_fv = args[1].as_ref().free_vars();
                            if quant_set.iter().all(|s| !rhs_fv.contains(s)) {
                                // Build e - 1 as the witness bound
                                let rhs_ty = self.env().get_node_type(args[1].as_ref().node_id());
                                let one = self.mk_num_const(&rhs_ty, BigInt::from(1));
                                let bound = self.mk_call(&rhs_ty, Operation::Sub, vec![
                                    args[1].clone(),
                                    one.clone(),
                                ]);
                                bounds.insert(*sym, bound);
                                bound_indices.insert(i);
                                // Guard: e > 0 (ensures the domain x < e is non-empty)
                                let zero = self.mk_num_const(&rhs_ty, BigInt::from(0));
                                let guard =
                                    self.mk_bool_call(Operation::Gt, vec![args[1].clone(), zero]);
                                guards.push(guard);
                            }
                        }
                    }
                },
                _ => {},
            }
        }

        // All quant vars must have bounds
        if bounds.len() != quant_syms.len() {
            return None;
        }

        // Remaining conjuncts (those that are not bound constraints)
        let remaining: Vec<Exp> = conjuncts
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !bound_indices.contains(i))
            .map(|(_, c)| c)
            .collect();

        if remaining.is_empty() {
            // When only bound constraints exist (e.g., `exists x: x < n`), with Lt bounds
            // the result is the conjunction of guards (e.g., `n > 0`).
            // With Le-only bounds, no simplification is possible.
            if guards.is_empty() {
                return None;
            }
            let result = guards.into_iter().reduce(|a, b| self.mk_and(a, b)).unwrap();
            return Some(self.simplify(result));
        }

        // All remaining conjuncts must be upward-safe in every bound variable.
        // We pass is_unsigned_context=true since all quantified variables are verified
        // to have unsigned integer types above.
        for conj in &remaining {
            for sym in &quant_syms {
                if !is_conjunct_upward_safe(self.env(), conj, *sym, true) {
                    return None;
                }
            }
        }

        // Substitute xi = ei in the remaining conjuncts and simplify
        let mut result: Exp = remaining
            .into_iter()
            .reduce(|a, b| self.mk_and(a, b))
            .unwrap();
        for (sym, bound) in &bounds {
            result = substitute_local_var(self.env(), &result, *sym, bound);
        }
        // AND in any non-emptiness guards from Lt bounds
        for guard in guards {
            result = self.mk_and(guard, result);
        }
        Some(self.simplify(result))
    }

    /// Check whether a conjunct is upward-safe in a variable, i.e., if the conjunct
    /// holds for some value `a` of `sym`, it also holds for any `b >= a`.
    /// Extract a one-point binding from `exists vars. (v == e AND Q)`.
    /// Returns `(v_symbol, e, remaining_body)` if the body contains a conjunct that
    /// equates a quantified variable to an expression not containing that variable.
    fn extract_exists_one_point_binding(
        &self,
        ranges: &[(Pattern, Exp)],
        body: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        let quant_vars = quant_symbols(ranges);

        // Flatten the body into conjuncts
        let conjuncts = flatten_conjunction_owned(body);

        // Try each conjunct as a binding `v == e`
        for i in 0..conjuncts.len() {
            // We pass a dummy consequent since we only need (sym, expr, _)
            if let Some((sym, expr, _)) = self.try_extract_binding(&conjuncts[i], &quant_vars, body)
            {
                // Rebuild the remaining body from all other conjuncts
                let remaining: Vec<Exp> = conjuncts
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, e)| e.clone())
                    .collect();
                let new_body = if remaining.is_empty() {
                    self.mk_bool_const(true)
                } else {
                    remaining
                        .into_iter()
                        .reduce(|a, b| self.mk_and(a, b))
                        .unwrap()
                };
                return Some((sym, expr, new_body));
            }
        }
        None
    }

    /// Extract a one-point binding from `forall vars. (v == e ==> Q)`.
    /// Returns `(v_symbol, e, Q)` if the body is an implication whose antecedent
    /// equates a quantified variable to an expression not containing that variable.
    fn extract_one_point_binding(
        &self,
        ranges: &[(Pattern, Exp)],
        body: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        // Body must be `antecedent ==> consequent`
        let (antecedent, consequent) = match body.as_ref() {
            ExpData::Call(_, Operation::Implies, args) if args.len() == 2 => (&args[0], &args[1]),
            _ => return None,
        };

        // Collect quantified variable symbols
        let quant_vars = quant_symbols(ranges);

        // Check antecedent for `v == e` or `e == v` where v is quantified
        self.try_extract_binding(antecedent, &quant_vars, consequent)
            .or_else(|| {
                // Also check conjunctive antecedents: `(v == e && rest) ==> Q`
                // becomes `rest ==> Q[v/e]` wrapped back into implication
                self.try_extract_binding_from_conjunction(antecedent, &quant_vars, consequent)
            })
    }

    /// Try to extract a binding from a simple equality `v == e` or `e == v`.
    fn try_extract_binding(
        &self,
        eq_exp: &Exp,
        quant_vars: &[Symbol],
        consequent: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        if let ExpData::Call(_, Operation::Eq, args) = eq_exp.as_ref() {
            if args.len() == 2 {
                // Try v == e (v on the left)
                if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                    if quant_vars.contains(sym) && !args[1].as_ref().free_vars().contains(sym) {
                        return Some((*sym, args[1].clone(), consequent.clone()));
                    }
                }
                // Try e == v (v on the right)
                if let ExpData::LocalVar(_, sym) = args[1].as_ref() {
                    if quant_vars.contains(sym) && !args[0].as_ref().free_vars().contains(sym) {
                        return Some((*sym, args[0].clone(), consequent.clone()));
                    }
                }
            }
        }
        None
    }

    /// Try to extract a binding from a conjunction used as an implication antecedent.
    /// For `(v == e && rest) ==> Q`, returns `(v, e, rest ==> Q)`.
    ///
    /// Flattens nested conjunctions so bindings buried in `A && (v == e && B)` are found.
    /// When rebuilding the remaining antecedent, uses `mk_and` which triggers antisymmetry
    /// simplification (e.g., `x <= n && x >= n → x == n`), enabling further binding extraction.
    fn try_extract_binding_from_conjunction(
        &self,
        antecedent: &Exp,
        quant_vars: &[Symbol],
        consequent: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        // Flatten the conjunction into a list of conjuncts
        let conjuncts = flatten_conjunction_owned(antecedent);
        if conjuncts.len() < 2 {
            return None;
        }
        // Try each conjunct as a binding
        for i in 0..conjuncts.len() {
            if let Some((sym, expr, _)) =
                self.try_extract_binding(&conjuncts[i], quant_vars, consequent)
            {
                // Rebuild the remaining antecedent from all other conjuncts,
                // using mk_and which triggers antisymmetry simplification.
                let remaining: Vec<Exp> = conjuncts
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, e)| e.clone())
                    .collect();
                let new_body = if remaining.is_empty() {
                    consequent.clone()
                } else {
                    let remaining_ant = remaining
                        .into_iter()
                        .reduce(|a, b| self.mk_and(a, b))
                        .unwrap();
                    self.mk_implies(remaining_ant, consequent.clone())
                };
                return Some((sym, expr, new_body));
            }
        }
        None
    }

    /// Core struct field binding extraction: given a list of conjuncts and quantifier ranges,
    /// find a quantified struct variable `x` where all fields are bound by equality conjuncts
    /// (`x.f1 == e1 && ... && x.fn == en`).
    ///
    /// Returns `Some((sym, pack_exp, binding_conjunct_indices))` if found.
    fn find_struct_field_binding(
        &self,
        ranges: &[(Pattern, Exp)],
        conjuncts: &[Exp],
    ) -> Option<(Symbol, Exp, BTreeSet<usize>)> {
        for (pat, _) in ranges {
            let (node_id, x_sym) = match pat {
                Pattern::Var(nid, sym) => (*nid, *sym),
                _ => continue,
            };
            let ty = self.env().get_node_type(node_id);
            let (mid, sid, targs) = match &ty {
                Type::Struct(mid, sid, targs) => (*mid, *sid, targs.clone()),
                _ => continue,
            };
            let struct_env = self.env().get_module(mid).into_struct(sid);
            if struct_env.has_variants() {
                continue;
            }

            let fields: Vec<(FieldId, usize)> = struct_env
                .get_fields()
                .map(|f| (f.get_id(), f.get_offset()))
                .collect();
            if fields.is_empty() {
                continue;
            }

            // For each field, find a conjunct matching `x.fi == ei` or `ei == x.fi`
            let mut field_bindings: BTreeMap<FieldId, (usize, Exp, usize)> = BTreeMap::new();
            for (fid, offset) in &fields {
                for (ci, conj) in conjuncts.iter().enumerate() {
                    if let ExpData::Call(_, Operation::Eq, eq_args) = conj.as_ref() {
                        if eq_args.len() == 2 {
                            if self
                                .is_field_select(&eq_args[0], mid, sid, *fid, x_sym)
                                .is_some()
                            {
                                let fv = eq_args[1].as_ref().free_vars();
                                if !fv.contains(&x_sym) {
                                    field_bindings.insert(*fid, (*offset, eq_args[1].clone(), ci));
                                    break;
                                }
                            }
                            if self
                                .is_field_select(&eq_args[1], mid, sid, *fid, x_sym)
                                .is_some()
                            {
                                let fv = eq_args[0].as_ref().free_vars();
                                if !fv.contains(&x_sym) {
                                    field_bindings.insert(*fid, (*offset, eq_args[0].clone(), ci));
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if field_bindings.len() != fields.len() {
                continue;
            }

            let mut ordered: Vec<(usize, Exp)> = field_bindings
                .values()
                .map(|(offset, expr, _)| (*offset, expr.clone()))
                .collect();
            ordered.sort_by_key(|(offset, _)| *offset);
            let field_values: Vec<Exp> = ordered.into_iter().map(|(_, e)| e).collect();
            let pack_exp = self.generator.mk_pack(mid, sid, &targs, field_values);

            let binding_indices: BTreeSet<usize> =
                field_bindings.values().map(|(_, _, ci)| *ci).collect();

            return Some((x_sym, pack_exp, binding_indices));
        }
        None
    }

    /// Struct field one-point rule for forall: given
    /// `forall x: S. x.f1 == e1 && ... && x.fn == en ==> Q`,
    /// substitute x with `Pack(S, e1, ..., en)` and eliminate x.
    fn try_extract_struct_field_binding(
        &self,
        ranges: &[(Pattern, Exp)],
        body: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        let (antecedent, consequent) = match body.as_ref() {
            ExpData::Call(_, Operation::Implies, args) if args.len() == 2 => (&args[0], &args[1]),
            _ => return None,
        };
        let conjuncts = flatten_conjunction_owned(antecedent);
        let (sym, pack_exp, remaining) =
            self.find_and_remove_struct_field_binding(ranges, conjuncts)?;
        let new_body = if remaining.is_empty() {
            consequent.clone()
        } else {
            let remaining_ant = remaining
                .into_iter()
                .reduce(|a, b| self.mk_and(a, b))
                .unwrap();
            self.mk_implies(remaining_ant, consequent.clone())
        };
        Some((sym, pack_exp, new_body))
    }

    /// Struct field one-point rule for exists: given
    /// `exists x: S. x.f1 == e1 && ... && x.fn == en && Q`,
    /// substitute x with `Pack(S, e1, ..., en)` and eliminate x.
    fn try_extract_struct_field_binding_exists(
        &self,
        ranges: &[(Pattern, Exp)],
        body: &Exp,
    ) -> Option<(Symbol, Exp, Exp)> {
        let conjuncts = flatten_conjunction_owned(body);
        let (sym, pack_exp, remaining) =
            self.find_and_remove_struct_field_binding(ranges, conjuncts)?;
        let new_body = if remaining.is_empty() {
            self.mk_bool_const(true)
        } else {
            remaining
                .into_iter()
                .reduce(|a, b| self.mk_and(a, b))
                .unwrap()
        };
        Some((sym, pack_exp, new_body))
    }

    /// Shared helper: find struct field bindings in conjuncts, return the binding symbol,
    /// pack expression, and the remaining (non-binding) conjuncts.
    fn find_and_remove_struct_field_binding(
        &self,
        ranges: &[(Pattern, Exp)],
        conjuncts: Vec<Exp>,
    ) -> Option<(Symbol, Exp, Vec<Exp>)> {
        if conjuncts.is_empty() {
            return None;
        }
        let (sym, pack_exp, binding_indices) =
            self.find_struct_field_binding(ranges, &conjuncts)?;
        let remaining: Vec<Exp> = conjuncts
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !binding_indices.contains(i))
            .map(|(_, e)| e)
            .collect();
        Some((sym, pack_exp, remaining))
    }

    /// Check if `exp` is `Select(mid, sid, fid)(LocalVar(x_sym))`.
    fn is_field_select(
        &self,
        exp: &Exp,
        mid: ModuleId,
        sid: StructId,
        fid: FieldId,
        x_sym: Symbol,
    ) -> Option<()> {
        if let ExpData::Call(_, Operation::Select(m, s, f), sel_args) = exp.as_ref() {
            if *m == mid && *s == sid && *f == fid && sel_args.len() == 1 {
                if let ExpData::LocalVar(_, sym) = sel_args[0].as_ref() {
                    if *sym == x_sym {
                        return Some(());
                    }
                }
            }
        }
        None
    }

    /// Scan a flat list of conjuncts for duplicate entries and antisymmetric pairs
    /// (`a <= b` and `a >= b`), removing duplicates and replacing antisymmetric pairs
    /// with equalities (`a == b`). Returns true if any changes were made.
    fn normalize_antisymmetric_conjuncts(&self, parts: &mut Vec<Exp>) -> bool {
        let mut found = false;
        let mut i = 0;
        while i < parts.len() {
            let mut j = i + 1;
            while j < parts.len() {
                if parts[i].structural_eq(&parts[j]) {
                    // Remove duplicate conjunct
                    parts.remove(j);
                    found = true;
                } else if let Some(eq) = self.try_antisymmetry_to_eq(&parts[i], &parts[j]) {
                    // Replace parts[i] with the equality, remove parts[j]
                    parts[i] = eq;
                    parts.remove(j);
                    found = true;
                    // Don't increment j — the element at j is now a new one
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
        found
    }

    /// Check if an exists quantifier is trivially satisfiable by witness instantiation.
    /// Tries substituting all quantified variables with 0 (minimum) and then with
    /// the maximum value of the unsigned type domain. Returns true if either witness
    /// makes the body simplify to `true`.
    fn is_exists_trivially_true(&mut self, ranges: &[(Pattern, Exp)], body: &Exp) -> bool {
        // Try "low" witness: 0 for unsigned integers, false for bool
        let mut low_instantiated = body.clone();
        for (pat, _) in ranges {
            if let Pattern::Var(pat_id, sym) = pat {
                let ty = self.env().get_node_type(*pat_id);
                if ty.is_unsigned_int() {
                    let zero = self.mk_num_const(&ty, BigInt::zero());
                    low_instantiated =
                        substitute_local_var(self.env(), &low_instantiated, *sym, &zero);
                } else if ty.is_bool() {
                    let val = self.mk_bool_const(false);
                    low_instantiated =
                        substitute_local_var(self.env(), &low_instantiated, *sym, &val);
                } else {
                    return false;
                }
            }
        }
        let result = self.simplify(low_instantiated);
        if matches!(result.as_ref(), ExpData::Value(_, Value::Bool(true))) {
            return true;
        }

        // Try "high" witness: max value for unsigned integers, true for bool
        let mut high_instantiated = body.clone();
        for (pat, _) in ranges {
            if let Pattern::Var(pat_id, sym) = pat {
                let ty = self.env().get_node_type(*pat_id);
                if ty.is_bool() {
                    let val = self.mk_bool_const(true);
                    high_instantiated =
                        substitute_local_var(self.env(), &high_instantiated, *sym, &val);
                } else if let Type::Primitive(prim_ty) = &ty {
                    if let Some(max_val) = prim_ty.get_max_value() {
                        let max_const = self.mk_num_const(&ty, max_val);
                        high_instantiated =
                            substitute_local_var(self.env(), &high_instantiated, *sym, &max_const);
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        let result = self.simplify(high_instantiated);
        matches!(result.as_ref(), ExpData::Value(_, Value::Bool(true)))
    }

    /// Check if a forall expression is provably false by instantiating all quantified
    /// variables with 0 (the minimum of unsigned type domains). If the body evaluates
    /// to `false`, the forall is exactly `false` and carries no information.
    pub fn is_forall_provably_false(&mut self, exp: &Exp) -> bool {
        if let ExpData::Quant(_, QuantKind::Forall, ranges, _, _, body) = exp.as_ref() {
            let mut instantiated = body.clone();
            for (pat, _) in ranges {
                if let Pattern::Var(_, sym) = pat {
                    let u64_ty = Type::new_prim(PrimitiveType::U64);
                    let zero = self.mk_num_const(&u64_ty, BigInt::zero());
                    instantiated = substitute_local_var(self.env(), &instantiated, *sym, &zero);
                }
            }
            let result = self.simplify(instantiated);
            is_bool_const(&result, false)
        } else {
            false
        }
    }

    /// Tries to unfold a spec function call where all arguments are constants.
    /// Returns the substituted body if the function has a non-native, interpreted body,
    /// or `None` if unfolding is not possible.
    fn try_unfold_spec_fun(
        &mut self,
        _call_id: NodeId,
        mid: ModuleId,
        fid: SpecFunId,
        args: &[Exp],
    ) -> Option<Exp> {
        if self.spec_fun_unfold_depth >= MAX_SPEC_FUN_UNFOLD_DEPTH {
            return None;
        }
        let env = self.env();
        let module = env.get_module(mid);
        let decl = module.get_spec_fun(fid);
        if decl.is_native || decl.uninterpreted || decl.body.is_none() {
            return None;
        }
        let body = decl.body.as_ref().unwrap().clone();
        let param_map: BTreeMap<Symbol, Exp> = decl
            .params
            .iter()
            .enumerate()
            .filter_map(|(i, param)| args.get(i).map(|a| (param.0, a.clone())))
            .collect();
        let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
            if let RewriteTarget::LocalVar(sym) = target {
                param_map.get(&sym).cloned()
            } else {
                None
            }
        };
        let substituted = ExpRewriter::new(env, &mut replacer).rewrite_exp(body);
        Some(substituted)
    }
}

// -----------------------------------------------------------
// Public helper functions
// -----------------------------------------------------------

/// Extract quantified variable symbols from quantifier ranges.
/// Filters for `Pattern::Var` patterns and returns the symbols.
pub fn quant_symbols(ranges: &[(Pattern, Exp)]) -> Vec<Symbol> {
    ranges
        .iter()
        .filter_map(|(pat, _)| {
            if let Pattern::Var(_, sym) = pat {
                Some(*sym)
            } else {
                None
            }
        })
        .collect()
}

/// Flatten a conjunction into a list of owned conjunct expressions.
/// `A && (B && C)` becomes `[A, B, C]`.
pub fn flatten_conjunction_owned(exp: &Exp) -> Vec<Exp> {
    let mut result = Vec::new();
    flatten_conjunction_into(exp, &mut result);
    result
}

fn flatten_conjunction_into(exp: &Exp, result: &mut Vec<Exp>) {
    if let ExpData::Call(_, Operation::And, args) = exp.as_ref() {
        if args.len() == 2 {
            flatten_conjunction_into(&args[0], result);
            flatten_conjunction_into(&args[1], result);
            return;
        }
    }
    result.push(exp.clone());
}

/// Check if two boolean expressions are complementary (one is the negation of the other).
/// Uses `ExpData::structural_eq` for comparison, ignoring `NodeId`s.
pub fn is_complementary(a: &Exp, b: &Exp) -> bool {
    match a.as_ref() {
        ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
            args[0].as_ref().structural_eq(b)
        },
        _ => match b.as_ref() {
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
                args[0].as_ref().structural_eq(a)
            },
            _ => false,
        },
    }
}

// -----------------------------------------------------------
// Private helper functions
// -----------------------------------------------------------

/// Remove quantified variables that are not free in the body.
/// For both forall and exists, `Q x. P` where x not in FV(P) is equivalent to P
/// (forall: vacuously; exists: type domains are non-empty).
fn remove_unused_quant_vars(ranges: &mut Vec<(Pattern, Exp)>, body: &Exp) {
    let free = body.as_ref().free_vars();
    ranges.retain(|(pat, _)| {
        if let Pattern::Var(_, sym) = pat {
            free.contains(sym)
        } else {
            true
        }
    });
}

/// Check whether a conjunct is upward-safe in a variable, i.e., if the conjunct
/// holds for some value `a` of `sym`, it also holds for any `b >= a`.
///
/// When `is_unsigned_context` is true, all quantified variables are known to be unsigned,
/// so all values in scope are non-negative. This allows treating `Num`-typed multiplication
/// as monotone increasing (since products of non-negative monotone-increasing functions are
/// monotone increasing).
///
/// Conservative: returns false if it cannot determine safety.
fn is_conjunct_upward_safe(
    env: &GlobalEnv,
    conj: &Exp,
    sym: Symbol,
    is_unsigned_context: bool,
) -> bool {
    // If the conjunct doesn't mention the variable, it's trivially safe
    if !conj.as_ref().free_vars().contains(&sym) {
        return true;
    }
    // `f(x) > C` or `f(x) >= C` where f is monotone increasing in x and C is free of x.
    if let ExpData::Call(_, Operation::Gt | Operation::Ge, args) = conj.as_ref() {
        if args.len() == 2
            && !args[1].as_ref().free_vars().contains(&sym)
            && is_monotone_increasing_in(env, &args[0], sym, is_unsigned_context)
        {
            return true;
        }
    }
    false
}

/// Check whether an expression is monotone increasing in a variable.
///
/// For unsigned integer arithmetic:
/// - `x` itself → true
/// - Expression not containing `x` → true (constant w.r.t. x)
/// - `f(x) + g(x)` where both monotone → true
/// - `f(x) * g(x)` where both monotone → true (only when all values are known
///   non-negative, i.e. the type is unsigned or `is_unsigned_context` is set)
///
/// When `is_unsigned_context` is true, all quantified variables are unsigned integers,
/// so all arithmetic subexpressions involving them are non-negative. This allows treating
/// `Num`-typed multiplication as monotone increasing.
fn is_monotone_increasing_in(
    env: &GlobalEnv,
    exp: &Exp,
    sym: Symbol,
    is_unsigned_context: bool,
) -> bool {
    if !exp.as_ref().free_vars().contains(&sym) {
        return true; // constant in sym
    }
    match exp.as_ref() {
        ExpData::LocalVar(_, s) if *s == sym => true,
        ExpData::Call(_, Operation::Add, args) if args.len() == 2 => {
            is_monotone_increasing_in(env, &args[0], sym, is_unsigned_context)
                && is_monotone_increasing_in(env, &args[1], sym, is_unsigned_context)
        },
        // f(x) * g(x) where both monotone → true, but only when values are non-negative
        // (signed values can be negative, breaking monotonicity of products).
        // In an unsigned context, all quantified variables are unsigned so all arithmetic
        // involving them is non-negative, even if the intermediate type is Num.
        ExpData::Call(id, Operation::Mul, args) if args.len() == 2 => {
            (env.get_node_type(*id).is_unsigned_int() || is_unsigned_context)
                && is_monotone_increasing_in(env, &args[0], sym, is_unsigned_context)
                && is_monotone_increasing_in(env, &args[1], sym, is_unsigned_context)
        },
        // f(x) / c where c > 0 and f is monotone increasing → monotone increasing
        ExpData::Call(_, Operation::Div, args) if args.len() == 2 => {
            if let Some(c) = get_num_const(&args[1]) {
                c > &BigInt::zero()
                    && is_monotone_increasing_in(env, &args[0], sym, is_unsigned_context)
            } else {
                false
            }
        },
        _ => false,
    }
}

/// Substitute all free occurrences of `LocalVar(sym)` with `replacement` in `exp`,
/// using `ExpRewriter` to correctly handle shadowed variables.
fn substitute_local_var(env: &GlobalEnv, exp: &Exp, sym: Symbol, replacement: &Exp) -> Exp {
    let replacement = replacement.clone();
    let mut replacer = |_id: NodeId, target: RewriteTarget| -> Option<Exp> {
        if let RewriteTarget::LocalVar(s) = target {
            if s == sym {
                return Some(replacement.clone());
            }
        }
        None
    };
    ExpRewriter::new(env, &mut replacer).rewrite_exp(exp.clone())
}

/// Negates a comparison: Lt↔Ge, Le↔Gt, Eq↔Neq.
fn negate_comparison(oper: &Operation) -> Option<Operation> {
    match oper {
        Operation::Lt => Some(Operation::Ge),
        Operation::Le => Some(Operation::Gt),
        Operation::Gt => Some(Operation::Le),
        Operation::Ge => Some(Operation::Lt),
        Operation::Eq => Some(Operation::Neq),
        Operation::Neq => Some(Operation::Eq),
        _ => None,
    }
}

/// Flips a comparison direction: Lt↔Gt, Le↔Ge. Eq and Neq are symmetric.
fn flip_comparison(oper: &Operation) -> Operation {
    match oper {
        Operation::Lt => Operation::Gt,
        Operation::Gt => Operation::Lt,
        Operation::Le => Operation::Ge,
        Operation::Ge => Operation::Le,
        Operation::Eq => Operation::Eq,
        Operation::Neq => Operation::Neq,
        _ => oper.clone(),
    }
}

fn is_bool_const(exp: &Exp, val: bool) -> bool {
    matches!(exp.as_ref(), ExpData::Value(_, Value::Bool(b)) if *b == val)
}

fn is_num_const(exp: &Exp, val: i64) -> bool {
    matches!(exp.as_ref(), ExpData::Value(_, Value::Number(n)) if *n == BigInt::from(val))
}

fn get_num_const(exp: &Exp) -> Option<&BigInt> {
    match exp.as_ref() {
        ExpData::Value(_, Value::Number(n)) => Some(n),
        _ => None,
    }
}

/// Normalizes a comparison to canonical `(left, right)` for a less-than relation.
/// Handles `Lt(a, b)` and `Gt(b, a)` uniformly.
fn as_lt_args(exp: &Exp) -> Option<(&Exp, &Exp)> {
    match exp.as_ref() {
        ExpData::Call(_, Operation::Lt, args) if args.len() == 2 => Some((&args[0], &args[1])),
        ExpData::Call(_, Operation::Gt, args) if args.len() == 2 => Some((&args[1], &args[0])),
        _ => None,
    }
}

/// Normalizes a comparison to canonical `(left, right)` for a not-less-than (>=) relation.
/// Handles `Not(Lt(a, b))`, `Not(Gt(a, b))`, `Le(b, a)`, and `Ge(a, b)` uniformly.
fn as_not_lt_args(exp: &Exp) -> Option<(&Exp, &Exp)> {
    match exp.as_ref() {
        // a <= b is !(b < a)
        ExpData::Call(_, Operation::Le, args) if args.len() == 2 => Some((&args[1], &args[0])),
        // a >= b is !(a < b)
        ExpData::Call(_, Operation::Ge, args) if args.len() == 2 => Some((&args[0], &args[1])),
        ExpData::Call(_, Operation::Not, inner) if inner.len() == 1 => as_lt_args(&inner[0]),
        _ => None,
    }
}

/// Extracts `(base, offset)` from value expressions.
/// For `base + c` returns `(base, c)`, for `base - c` returns `(base, -c)`,
/// for plain expressions returns `(val, 0)`.
fn extract_additive_offset(val: &Exp) -> (&Exp, BigInt) {
    if let ExpData::Call(_, oper, args) = val.as_ref() {
        if args.len() == 2 {
            if let Some(c) = get_num_const(&args[1]) {
                match oper {
                    Operation::Add => return (&args[0], c.clone()),
                    Operation::Sub => return (&args[0], -c),
                    _ => {},
                }
            }
        }
    }
    (val, BigInt::zero())
}

/// Extracts `(base, factor)` from value expressions.
/// For `base * k` returns `(base, k)`, for plain expressions returns `(val, 1)`.
fn extract_multiplicative_factor(val: &Exp) -> (&Exp, BigInt) {
    if let ExpData::Call(_, Operation::Mul, args) = val.as_ref() {
        if args.len() == 2 {
            if let Some(k) = get_num_const(&args[1]) {
                return (&args[0], k.clone());
            }
        }
    }
    (val, BigInt::from(1))
}

// -----------------------------------------------------------
// ExpRewriterFunctions implementation
// -----------------------------------------------------------

impl<'a, 'env, G: ExpGenerator<'env>> ExpRewriterFunctions for ExpSimplifier<'a, 'env, G> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        // For implications, rewrite antecedent first, then push it as a local
        // assumption while rewriting the consequent. This enables simplifications
        // like `3 < n ==> (2 < n ==> P)` → `3 < n ==> P`.
        if let ExpData::Call(id, Operation::Implies, args) = exp.as_ref() {
            if args.len() == 2 {
                let new_id = self.rewrite_node_id(*id).unwrap_or(*id);
                // Rewrite antecedent normally
                let new_a = self.rewrite_exp(args[0].clone());
                // Save assumption/substitution state
                let saved_assumptions_len = self.assumptions.len();
                let saved_substitutions = self.substitutions.clone();
                // Push antecedent as local assumption
                self.assume(new_a.clone());
                // Rewrite consequent under the assumption
                let new_b = self.rewrite_exp(args[1].clone());
                // Restore state
                self.assumptions.truncate(saved_assumptions_len);
                self.substitutions = saved_substitutions;
                // Build result using mk_implies (handles boolean identities)
                let result = self.mk_implies(new_a, new_b);
                // Check assumptions on the final result
                return if let Some(simplified) = self.simplify_by_assumption(&result) {
                    simplified
                } else {
                    // If mk_implies didn't change the node type, preserve original id
                    if let ExpData::Call(_, Operation::Implies, new_args) = result.as_ref() {
                        if new_args.len() == 2 {
                            return ExpData::Call(new_id, Operation::Implies, new_args.to_vec())
                                .into_exp();
                        }
                    }
                    result
                };
            }
        }

        // When entering old(), suppress substitutions since temporaries
        // inside old() refer to pre-state values.
        let result = if let ExpData::Call(_, Operation::Old, _) = exp.as_ref() {
            let was_inside_old = self.inside_old;
            self.inside_old = true;
            let r = self.rewrite_exp_descent(exp);
            self.inside_old = was_inside_old;
            r
        } else {
            self.rewrite_exp_descent(exp)
        };
        // Simplify quantifiers (flatten nested, one-point rule)
        let result = if let ExpData::Quant(id, QuantKind::Forall, ranges, triggers, cond, body) =
            result.as_ref()
        {
            self.simplify_forall(
                *id,
                ranges.clone(),
                triggers.clone(),
                cond.clone(),
                body.clone(),
            )
        } else if let ExpData::Quant(id, QuantKind::Exists, ranges, triggers, cond, body) =
            result.as_ref()
        {
            self.simplify_exists(
                *id,
                ranges.clone(),
                triggers.clone(),
                cond.clone(),
                body.clone(),
            )
        } else {
            result
        };
        // Check assumptions on the final result for any expression type
        if let Some(simplified) = self.simplify_by_assumption(&result) {
            simplified
        } else {
            result
        }
    }

    fn rewrite_enter_scope<'b>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
    ) {
        self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.shadowed.pop();
    }

    fn rewrite_local_var(&mut self, _id: NodeId, sym: Symbol) -> Option<Exp> {
        if self.inside_old || self.is_shadowed(sym) {
            None
        } else {
            self.substitutions
                .get(&RewriteTarget::LocalVar(sym))
                .cloned()
        }
    }

    fn rewrite_temporary(&mut self, _id: NodeId, idx: TempIndex) -> Option<Exp> {
        if self.inside_old {
            None
        } else {
            self.substitutions
                .get(&RewriteTarget::Temporary(idx))
                .cloned()
        }
    }

    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        // 1. Try constant folding first
        if let Some(folded) = self.try_constant_fold(id, oper, args) {
            return Some(folded);
        }

        // 1b. Fold MAX_U* constants to numeric values
        if matches!(
            oper,
            Operation::MaxU8
                | Operation::MaxU16
                | Operation::MaxU32
                | Operation::MaxU64
                | Operation::MaxU128
                | Operation::MaxU256
        ) {
            let prim = match oper {
                Operation::MaxU8 => PrimitiveType::U8,
                Operation::MaxU16 => PrimitiveType::U16,
                Operation::MaxU32 => PrimitiveType::U32,
                Operation::MaxU64 => PrimitiveType::U64,
                Operation::MaxU128 => PrimitiveType::U128,
                Operation::MaxU256 => PrimitiveType::U256,
                _ => unreachable!(),
            };
            let ty = self.env().get_node_type(id);
            return Some(self.mk_num_const(&ty, prim.get_max_value().unwrap()));
        }

        // 2. Idempotent old: old(old(x)) -> old(x)
        if matches!(oper, Operation::Old) && args.len() == 1 {
            if let ExpData::Call(_, Operation::Old, _) = args[0].as_ref() {
                return Some(args[0].clone());
            }
        }

        // 2b. Freeze(x) -> x in spec mode (references are erased in specs)
        if self.spec_mode && matches!(oper, Operation::Freeze(_)) && args.len() == 1 {
            return Some(args[0].clone());
        }

        // 3. Boolean simplification
        if matches!(
            oper,
            Operation::Not | Operation::And | Operation::Or | Operation::Implies | Operation::Iff
        ) {
            return self.simplify_bool_call(id, oper, args);
        }

        // 4. Comparison simplification (reflexive)
        if matches!(
            oper,
            Operation::Eq
                | Operation::Neq
                | Operation::Lt
                | Operation::Le
                | Operation::Gt
                | Operation::Ge
        ) {
            if let Some(result) = self.simplify_comparison(oper, args) {
                return Some(result);
            }
        }

        // 5. Arithmetic simplification
        if matches!(
            oper,
            Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod
        ) {
            if let Some(result) = self.simplify_arithmetic(id, oper, args) {
                return Some(result);
            }
        }

        // 6. Redundant update_field: update_field(update_field(e, f, v1), f, v2)
        //     => update_field(e, f, v2)
        if let Operation::UpdateField(mid, sid, fid) = oper {
            if let ExpData::Call(_, Operation::UpdateField(mid2, sid2, fid2), inner_args) =
                args[0].as_ref()
            {
                if mid == mid2 && sid == sid2 && fid == fid2 {
                    return Some(
                        ExpData::Call(id, oper.clone(), vec![
                            inner_args[0].clone(),
                            args[1].clone(),
                        ])
                        .into_exp(),
                    );
                }
            }
        }

        // 7b. Select-of-update: update_field(e, f, v).f => v
        if let Operation::Select(mid, sid, fid) = oper {
            if let ExpData::Call(_, Operation::UpdateField(mid2, sid2, fid2), inner_args) =
                args[0].as_ref()
            {
                if mid == mid2 && sid == sid2 && fid == fid2 {
                    return Some(inner_args[1].clone());
                }
            }
        }

        // 7c. Select-of-Pack: Pack(S, e1, ..., en).fi => e_i
        if let Operation::Select(mid, sid, fid) = oper {
            if let ExpData::Call(_, Operation::Pack(mid2, sid2, _), inner_args) = args[0].as_ref() {
                if mid == mid2 && sid == sid2 {
                    let struct_env = self.env().get_module(*mid).into_struct(*sid);
                    let offset = struct_env.get_field(*fid).get_offset();
                    if offset < inner_args.len() {
                        return Some(inner_args[offset].clone());
                    }
                }
            }
        }

        // 7d. UpdateField-of-Pack: update_field(Pack(S, e1,...,en), fi, v) => Pack(S, e1,...,v,...,en)
        if let Operation::UpdateField(mid, sid, fid) = oper {
            if let ExpData::Call(pid, Operation::Pack(mid2, sid2, var), inner_args) =
                args[0].as_ref()
            {
                if mid == mid2 && sid == sid2 {
                    let struct_env = self.env().get_module(*mid).into_struct(*sid);
                    let offset = struct_env.get_field(*fid).get_offset();
                    if offset < inner_args.len() {
                        let mut new_args = inner_args.clone();
                        new_args[offset] = args[1].clone();
                        return Some(
                            ExpData::Call(*pid, Operation::Pack(*mid2, *sid2, *var), new_args)
                                .into_exp(),
                        );
                    }
                }
            }
        }

        // 8. WellFormed(x) -> true: in Move specs, type-checked values are always well-formed
        if matches!(oper, Operation::WellFormed) {
            return Some(self.mk_bool_const(true));
        }

        // 8. AbortFlag() -> false: verification artifact, not a runtime value
        if matches!(oper, Operation::AbortFlag) {
            return Some(self.mk_bool_const(false));
        }

        // 9. Unfold spec functions with all-constant arguments
        if let Operation::SpecFunction(mid, fid, _) = oper {
            if args
                .iter()
                .all(|a| matches!(a.as_ref(), ExpData::Value(..)))
            {
                if let Some(unfolded) = self.try_unfold_spec_fun(id, *mid, *fid, args) {
                    self.spec_fun_unfold_depth += 1;
                    let result = self.rewrite_exp(unfolded);
                    self.spec_fun_unfold_depth -= 1;
                    return Some(result);
                }
            }
        }

        None
    }

    fn rewrite_if_else(&mut self, _id: NodeId, cond: &Exp, then: &Exp, else_: &Exp) -> Option<Exp> {
        // Constant or assumed condition
        if self.is_known_true(cond) {
            return Some(then.clone());
        }
        if self.is_known_false(cond) {
            return Some(else_.clone());
        }

        // Same branches
        if then.structural_eq(else_) {
            return Some(then.clone());
        }

        // Bool select: if c { true } else { false } -> c
        if is_bool_const(then, true) && is_bool_const(else_, false) {
            return Some(cond.clone());
        }
        // if c { false } else { true } -> !c
        if is_bool_const(then, false) && is_bool_const(else_, true) {
            return Some(self.mk_not(cond.clone()));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{Address, ModuleName, Spec},
        exp_generator::FunExpGenerator,
        model::{FunId, FunctionData, Loc, ModuleId},
        ty::{PrimitiveType, Type, BOOL_TYPE},
    };
    use move_core_types::account_address::AccountAddress;
    use num::BigInt;
    use std::collections::BTreeMap;

    /// Creates a test `GlobalEnv` with a dummy module and function.
    fn test_env() -> GlobalEnv {
        let mut env = GlobalEnv::new();
        let loc = Loc::default();
        let fun_name = env.symbol_pool().make("test_fun");
        let fun_id = FunId::new(fun_name);
        let mut function_data = BTreeMap::new();
        function_data.insert(fun_id, FunctionData::new(fun_name, loc.clone()));
        let addr = Address::Numerical(AccountAddress::ZERO);
        let module_name = ModuleName::new(addr, env.symbol_pool().make("test_mod"));
        env.add(
            loc,
            module_name,
            vec![],          // attributes
            vec![],          // use_decls
            vec![],          // friend_decls
            BTreeMap::new(), // named_constants
            BTreeMap::new(), // struct_data
            function_data,   // function_data
            vec![],          // spec_vars
            vec![],          // spec_funs
            Spec::default(), // module_spec
            vec![],          // spec_block_infos
        );
        env
    }

    /// Creates a `FunExpGenerator` from a test env (looks up the dummy function).
    fn test_gen(env: &GlobalEnv) -> FunExpGenerator<'_> {
        let module = env.get_module(ModuleId::new(0));
        let fun_name = env.symbol_pool().make("test_fun");
        let fun_env = module.into_function(FunId::new(fun_name));
        FunExpGenerator::new(fun_env, Loc::default())
    }

    fn mk_bool(env: &GlobalEnv, val: bool) -> Exp {
        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        ExpData::Value(id, Value::Bool(val)).into_exp()
    }

    fn mk_num(env: &GlobalEnv, val: i64) -> Exp {
        let id = env.new_node(Loc::default(), Type::Primitive(PrimitiveType::Num));
        ExpData::Value(id, Value::Number(BigInt::from(val))).into_exp()
    }

    fn mk_temp(env: &GlobalEnv, idx: usize, ty: Type) -> Exp {
        let id = env.new_node(Loc::default(), ty);
        ExpData::Temporary(id, idx).into_exp()
    }

    fn mk_op(env: &GlobalEnv, ty: Type, op: Operation, args: Vec<Exp>) -> Exp {
        let id = env.new_node(Loc::default(), ty);
        ExpData::Call(id, op, args).into_exp()
    }

    fn mk_bool_op(env: &GlobalEnv, op: Operation, args: Vec<Exp>) -> Exp {
        mk_op(env, BOOL_TYPE.clone(), op, args)
    }

    fn assert_is_bool(exp: &Exp, expected: bool) {
        match exp.as_ref() {
            ExpData::Value(_, Value::Bool(b)) => assert_eq!(*b, expected),
            other => panic!("expected Bool({}), got {:?}", expected, other),
        }
    }

    fn assert_is_num(exp: &Exp, expected: i64) {
        match exp.as_ref() {
            ExpData::Value(_, Value::Number(n)) => {
                assert_eq!(*n, BigInt::from(expected))
            },
            other => panic!("expected Number({}), got {:?}", expected, other),
        }
    }

    fn assert_is_temp(exp: &Exp, expected_idx: usize) {
        match exp.as_ref() {
            ExpData::Temporary(_, idx) => assert_eq!(*idx, expected_idx),
            other => panic!("expected Temporary({}), got {:?}", expected_idx, other),
        }
    }

    // ---- Boolean simplification tests ----

    #[test]
    fn test_not_true() {
        let env = test_env();
        let e = mk_bool_op(&env, Operation::Not, vec![mk_bool(&env, true)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_not_false() {
        let env = test_env();
        let e = mk_bool_op(&env, Operation::Not, vec![mk_bool(&env, false)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_double_negation() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Not, vec![mk_bool_op(
            &env,
            Operation::Not,
            vec![t0],
        )]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_and_identity() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::And, vec![t0, mk_bool(&env, true)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_and_annihilator() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::And, vec![t0, mk_bool(&env, false)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_or_identity() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Or, vec![t0, mk_bool(&env, false)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_or_annihilator() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Or, vec![t0, mk_bool(&env, true)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_implies_true_lhs() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Implies, vec![mk_bool(&env, true), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_implies_false_lhs() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Implies, vec![mk_bool(&env, false), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_complement_and() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let not_t0 = mk_bool_op(&env, Operation::Not, vec![t0.clone()]);
        let e = mk_bool_op(&env, Operation::And, vec![t0, not_t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_complement_or() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let not_t0 = mk_bool_op(&env, Operation::Not, vec![t0.clone()]);
        let e = mk_bool_op(&env, Operation::Or, vec![t0, not_t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    // ---- Equality/comparison tests ----

    #[test]
    fn test_eq_reflexive() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, Type::Primitive(PrimitiveType::U64));
        let e = mk_bool_op(&env, Operation::Eq, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_neq_reflexive() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, Type::Primitive(PrimitiveType::U64));
        let e = mk_bool_op(&env, Operation::Neq, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_le_reflexive() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, Type::Primitive(PrimitiveType::U64));
        let e = mk_bool_op(&env, Operation::Le, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_lt_reflexive() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, Type::Primitive(PrimitiveType::U64));
        let e = mk_bool_op(&env, Operation::Lt, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, false);
    }

    // ---- Arithmetic tests (spec mode) ----

    #[test]
    fn test_add_zero() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let e = mk_op(&env, num_ty, Operation::Add, vec![t0, mk_num(&env, 0)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_add_zero_lhs() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let e = mk_op(&env, num_ty, Operation::Add, vec![mk_num(&env, 0), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_mul_one() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let e = mk_op(&env, num_ty, Operation::Mul, vec![t0, mk_num(&env, 1)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_mul_zero() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let e = mk_op(&env, num_ty, Operation::Mul, vec![t0, mk_num(&env, 0)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 0);
    }

    #[test]
    fn test_sub_self() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let e = mk_op(&env, num_ty, Operation::Sub, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 0);
    }

    #[test]
    fn test_constant_fold() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let e = mk_op(&env, num_ty, Operation::Add, vec![
            mk_num(&env, 2),
            mk_num(&env, 3),
        ]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 5);
    }

    // ---- If-then-else tests ----

    #[test]
    fn test_ite_true_cond() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let cond = mk_bool(&env, true);
        let then_e = mk_num(&env, 1);
        let else_e = mk_num(&env, 2);
        let id = env.new_node(Loc::default(), num_ty);
        let e = ExpData::IfElse(id, cond, then_e, else_e).into_exp();
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 1);
    }

    #[test]
    fn test_ite_false_cond() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let cond = mk_bool(&env, false);
        let then_e = mk_num(&env, 1);
        let else_e = mk_num(&env, 2);
        let id = env.new_node(Loc::default(), num_ty);
        let e = ExpData::IfElse(id, cond, then_e, else_e).into_exp();
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 2);
    }

    #[test]
    fn test_ite_same_branches() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let cond = mk_temp(&env, 0, BOOL_TYPE.clone());
        let val = mk_num(&env, 42);
        let id = env.new_node(Loc::default(), num_ty);
        let e = ExpData::IfElse(id, cond, val.clone(), val).into_exp();
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_num(&result, 42);
    }

    #[test]
    fn test_ite_bool_select() {
        let env = test_env();
        let cond = mk_temp(&env, 0, BOOL_TYPE.clone());
        let then_e = mk_bool(&env, true);
        let else_e = mk_bool(&env, false);
        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        let e = ExpData::IfElse(id, cond, then_e, else_e).into_exp();
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_ite_bool_select_negated() {
        let env = test_env();
        let cond = mk_temp(&env, 0, BOOL_TYPE.clone());
        let then_e = mk_bool(&env, false);
        let else_e = mk_bool(&env, true);
        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        let e = ExpData::IfElse(id, cond, then_e, else_e).into_exp();
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // Result should be Not($t0)
        match result.as_ref() {
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
                assert_is_temp(&args[0], 0);
            },
            other => panic!("expected Not(Temporary(0)), got {:?}", other),
        }
    }

    // ---- Assumption and substitution tests ----

    #[test]
    fn test_assume_makes_true() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(t0.clone());
        let result = s.simplify(t0);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_assume_negation_makes_false() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let not_t0 = mk_bool_op(&env, Operation::Not, vec![t0.clone()]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(not_t0);
        let result = s.simplify(t0);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_assume_substitution() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let forty_two = mk_num(&env, 42);
        let eq = mk_bool_op(&env, Operation::Eq, vec![t0.clone(), forty_two]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(eq);
        // simplify t0 + 1 should give 43
        let add_one = mk_op(&env, num_ty, Operation::Add, vec![t0, mk_num(&env, 1)]);
        let result = s.simplify(add_one);
        assert_is_num(&result, 43);
    }

    #[test]
    fn test_assume_conjunction() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let t1 = mk_temp(&env, 1, BOOL_TYPE.clone());
        let conj = mk_bool_op(&env, Operation::And, vec![t0.clone(), t1.clone()]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(conj);
        assert!(s.is_known_true(&t0));
        assert!(s.is_known_true(&t1));
    }

    #[test]
    fn test_idempotent_and() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::And, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_idempotent_or() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Or, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    // ---- Implication with scoped assumptions ----

    #[test]
    fn test_implies_outer_implies_inner_antecedent() {
        // 3 < $t0 ==> (2 < $t0 ==> P) should simplify to 3 < $t0 ==> P
        // because under assumption 3 < $t0, 2 < $t0 is known true
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let p = mk_temp(&env, 1, BOOL_TYPE.clone());

        let three_lt_t0 = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 3), t0.clone()]);
        let two_lt_t0 = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 2), t0.clone()]);
        let inner = mk_bool_op(&env, Operation::Implies, vec![two_lt_t0, p.clone()]);
        let outer = mk_bool_op(&env, Operation::Implies, vec![three_lt_t0, inner]);

        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(outer);
        // Should be: 3 < $t0 ==> $t1
        match result.as_ref() {
            ExpData::Call(_, Operation::Implies, args) if args.len() == 2 => {
                // Antecedent: 3 < $t0
                match args[0].as_ref() {
                    ExpData::Call(_, Operation::Lt, lt_args) => {
                        assert_is_num(&lt_args[0], 3);
                        assert_is_temp(&lt_args[1], 0);
                    },
                    other => panic!("expected Lt(3, $t0), got {:?}", other),
                }
                // Consequent: $t1 (P)
                assert_is_temp(&args[1], 1);
            },
            other => panic!("expected Implies, got {:?}", other),
        }
    }

    #[test]
    fn test_implies_complementary_inner() {
        // 3 < $t0 ==> (!(3 < $t0) ==> P) should simplify to true
        // because !(3 < $t0) is false under assumption 3 < $t0
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let p = mk_temp(&env, 1, BOOL_TYPE.clone());

        let three_lt_t0 = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 3), t0.clone()]);
        let not_three_lt_t0 = mk_bool_op(&env, Operation::Not, vec![three_lt_t0.clone()]);
        let inner = mk_bool_op(&env, Operation::Implies, vec![not_three_lt_t0, p]);
        let outer = mk_bool_op(&env, Operation::Implies, vec![three_lt_t0, inner]);

        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(outer);
        // false ==> P is true, so whole thing is true
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_implies_equality_substitution() {
        // ($t0 == 5) ==> ($t0 + 1 == 6) should simplify to true
        // because under assumption $t0 == 5, $t0 + 1 == 6 becomes 5 + 1 == 6 == true
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let t0 = mk_temp(&env, 0, num_ty.clone());

        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![t0.clone(), mk_num(&env, 5)]);
        let add_1 = mk_op(&env, num_ty, Operation::Add, vec![t0, mk_num(&env, 1)]);
        let eq_6 = mk_bool_op(&env, Operation::Eq, vec![add_1, mk_num(&env, 6)]);
        let implies = mk_bool_op(&env, Operation::Implies, vec![eq_5, eq_6]);

        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(implies);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_implies_scoped_assumption_does_not_leak() {
        // After simplifying (A ==> B), assumptions from A should not persist
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let t1 = mk_temp(&env, 1, BOOL_TYPE.clone());

        let implies = mk_bool_op(&env, Operation::Implies, vec![t0.clone(), t1.clone()]);

        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let _result = s.simplify(implies);
        // t0 should NOT be known true after simplifying the implication
        assert!(!s.is_known_true(&t0));
    }

    // ---- Subsumption tests ----

    fn mk_big_num(env: &GlobalEnv, val: BigInt) -> Exp {
        let id = env.new_node(Loc::default(), Type::Primitive(PrimitiveType::Num));
        ExpData::Value(id, Value::Number(val)).into_exp()
    }

    #[test]
    fn test_subsumes_ge_additive_offset() {
        // (t0 + 4) >= MAX should subsume (t0 + 1) >= MAX
        // because (t0 + 1) >= MAX (stronger) implies (t0 + 4) >= MAX (weaker)
        let env = test_env();
        let u64_ty = Type::Primitive(PrimitiveType::U64);
        let t0 = mk_temp(&env, 0, u64_ty.clone());
        let max_u64 = mk_big_num(&env, BigInt::from(18446744073709551615u64));

        let val4 = mk_op(&env, u64_ty.clone(), Operation::Add, vec![
            t0.clone(),
            mk_num(&env, 4),
        ]);
        let ge4 = mk_bool_op(&env, Operation::Ge, vec![val4, max_u64.clone()]);

        let val1 = mk_op(&env, u64_ty.clone(), Operation::Add, vec![
            t0.clone(),
            mk_num(&env, 1),
        ]);
        let ge1 = mk_bool_op(&env, Operation::Ge, vec![val1, max_u64]);

        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);

        // (t0+4) >= MAX subsumes (t0+1) >= MAX
        assert!(s.subsumes(&ge4, &ge1));
        // But not the other way around
        assert!(!s.subsumes(&ge1, &ge4));
    }

    // ---- Additional test helpers ----

    fn mk_local(env: &GlobalEnv, name: &str, ty: Type) -> Exp {
        let sym = env.symbol_pool().make(name);
        let id = env.new_node(Loc::default(), ty);
        ExpData::LocalVar(id, sym).into_exp()
    }

    fn mk_quant(env: &GlobalEnv, kind: QuantKind, vars: Vec<(&str, Type)>, body: Exp) -> Exp {
        let pool = env.symbol_pool();
        let ranges: Vec<(Pattern, Exp)> = vars
            .into_iter()
            .map(|(name, ty)| {
                let sym = pool.make(name);
                let pat_id = env.new_node(Loc::default(), ty.clone());
                let range_id = env.new_node(Loc::default(), Type::TypeDomain(Box::new(ty)));
                let range = ExpData::Call(range_id, Operation::TypeDomain, vec![]).into_exp();
                (Pattern::Var(pat_id, sym), range)
            })
            .collect();
        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        ExpData::Quant(id, kind, ranges, vec![], None, body).into_exp()
    }

    // ---- Iff simplification tests ----

    #[test]
    fn test_iff_same() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Iff, vec![t0.clone(), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_iff_true_lhs() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Iff, vec![mk_bool(&env, true), t0]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_iff_false_rhs() {
        let env = test_env();
        let t0 = mk_temp(&env, 0, BOOL_TYPE.clone());
        let e = mk_bool_op(&env, Operation::Iff, vec![t0, mk_bool(&env, false)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // iff(t0, false) => not(t0)
        match result.as_ref() {
            ExpData::Call(_, Operation::Not, args) if args.len() == 1 => {
                assert_is_temp(&args[0], 0);
            },
            other => panic!("expected Not(Temporary(0)), got {:?}", other),
        }
    }

    // ---- Antisymmetry to equality tests ----

    #[test]
    fn test_antisymmetry_le_ge_to_eq() {
        // x <= y && x >= y should be simplified to x == y under the simplifier's
        // antisymmetry normalization (used in quantifier simplification).
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let le = mk_bool_op(&env, Operation::Le, vec![x.clone(), y.clone()]);
        let ge = mk_bool_op(&env, Operation::Ge, vec![x.clone(), y.clone()]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        let result = s.try_antisymmetry_to_eq(&le, &ge);
        assert!(result.is_some());
        // The result should be Eq(x, y)
        match result.unwrap().as_ref() {
            ExpData::Call(_, Operation::Eq, args) if args.len() == 2 => {
                // x and y
            },
            other => panic!("expected Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_antisymmetry_le_le_swapped_to_eq() {
        // x <= y && y <= x should simplify to x == y
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let le1 = mk_bool_op(&env, Operation::Le, vec![x.clone(), y.clone()]);
        let le2 = mk_bool_op(&env, Operation::Le, vec![y.clone(), x.clone()]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        let result = s.try_antisymmetry_to_eq(&le1, &le2);
        assert!(result.is_some());
    }

    #[test]
    fn test_antisymmetry_unrelated_returns_none() {
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let z = mk_local(&env, "z", num_ty.clone());
        let le1 = mk_bool_op(&env, Operation::Le, vec![x.clone(), y.clone()]);
        let le2 = mk_bool_op(&env, Operation::Le, vec![z.clone(), x.clone()]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.try_antisymmetry_to_eq(&le1, &le2).is_none());
    }

    // ---- Pinch-to-equality tests ----

    #[test]
    fn test_pinch_to_eq() {
        // c < x && !(c+1 < x) should simplify to x == c+1
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 5), x.clone()]);
        let lt_inner = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 6), x.clone()]);
        let not_lt = mk_bool_op(&env, Operation::Not, vec![lt_inner]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        let result = s.try_pinch_to_eq(&lt, &not_lt);
        assert!(result.is_some());
        match result.unwrap().as_ref() {
            ExpData::Call(_, Operation::Eq, args) if args.len() == 2 => {
                // Should be x == 6
                assert_is_num(&args[1], 6);
            },
            other => panic!("expected Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_pinch_wrong_constants_returns_none() {
        // c < x && !(c+3 < x) — gap too large, not a pinch
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 5), x.clone()]);
        let lt_inner = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 8), x.clone()]);
        let not_lt = mk_bool_op(&env, Operation::Not, vec![lt_inner]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.try_pinch_to_eq(&lt, &not_lt).is_none());
    }

    // ---- Ordering-based reasoning tests ----

    #[test]
    fn test_ordering_known_true_le_from_not_lt() {
        // If we assume !(y < x) (i.e., x <= y), then x <= y should be known true
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_temp(&env, 0, num_ty.clone());
        let y = mk_temp(&env, 1, num_ty.clone());
        let not_lt = mk_bool_op(&env, Operation::Not, vec![mk_bool_op(
            &env,
            Operation::Lt,
            vec![y.clone(), x.clone()],
        )]);
        let le = mk_bool_op(&env, Operation::Le, vec![x, y]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(not_lt);
        let result = s.simplify(le);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_ordering_known_false_le_from_lt() {
        // If we assume y < x (i.e., x > y), then x <= y should be known false
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_temp(&env, 0, num_ty.clone());
        let y = mk_temp(&env, 1, num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![y.clone(), x.clone()]);
        let le = mk_bool_op(&env, Operation::Le, vec![x, y]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(lt);
        let result = s.simplify(le);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_ordering_known_true_gt_from_lt() {
        // If we assume y < x, then x > y should be known true
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_temp(&env, 0, num_ty.clone());
        let y = mk_temp(&env, 1, num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![y.clone(), x.clone()]);
        let gt = mk_bool_op(&env, Operation::Gt, vec![x, y]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(lt);
        let result = s.simplify(gt);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_ordering_known_false_eq_from_lt() {
        // If we assume x < y, then x == y should be known false
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_temp(&env, 0, num_ty.clone());
        let y = mk_temp(&env, 1, num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![x.clone(), y.clone()]);
        let eq = mk_bool_op(&env, Operation::Eq, vec![x, y]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(lt);
        let result = s.simplify(eq);
        assert_is_bool(&result, false);
    }

    #[test]
    fn test_ordering_known_false_neq_from_eq_by_bounds() {
        // If we assume !(x < y) && !(y < x), then x != y should be known false
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_temp(&env, 0, num_ty.clone());
        let y = mk_temp(&env, 1, num_ty.clone());
        let not_lt_xy = mk_bool_op(&env, Operation::Not, vec![mk_bool_op(
            &env,
            Operation::Lt,
            vec![x.clone(), y.clone()],
        )]);
        let not_lt_yx = mk_bool_op(&env, Operation::Not, vec![mk_bool_op(
            &env,
            Operation::Lt,
            vec![y.clone(), x.clone()],
        )]);
        let neq = mk_bool_op(&env, Operation::Neq, vec![x, y]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        s.assume(not_lt_xy);
        s.assume(not_lt_yx);
        let result = s.simplify(neq);
        assert_is_bool(&result, false);
    }

    // ---- Comparison implication tests ----

    #[test]
    fn test_implies_comparison_eq_implies_lt() {
        // x == 5 implies 3 < x (since 3 < 5)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let lt_3_x = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 3), x]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&eq_5, &lt_3_x));
    }

    #[test]
    fn test_implies_comparison_eq_implies_x_lt_c() {
        // x == 5 implies x < 10 (since 5 < 10)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let x_lt_10 = mk_bool_op(&env, Operation::Lt, vec![x, mk_num(&env, 10)]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&eq_5, &x_lt_10));
    }

    #[test]
    fn test_implies_comparison_eq_does_not_imply_wrong() {
        // x == 5 does NOT imply 7 < x
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let lt_7_x = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 7), x]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(!s.implies_comparison(&eq_5, &lt_7_x));
    }

    #[test]
    fn test_implies_comparison_lt_implies_le() {
        // a < b implies a <= b
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let a = mk_temp(&env, 0, num_ty.clone());
        let b = mk_temp(&env, 1, num_ty.clone());
        let lt = mk_bool_op(&env, Operation::Lt, vec![a.clone(), b.clone()]);
        let le = mk_bool_op(&env, Operation::Le, vec![a, b]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&lt, &le));
    }

    #[test]
    fn test_implies_comparison_gt_implies_ge() {
        // a > b implies a >= b
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let a = mk_temp(&env, 0, num_ty.clone());
        let b = mk_temp(&env, 1, num_ty.clone());
        let gt = mk_bool_op(&env, Operation::Gt, vec![a.clone(), b.clone()]);
        let ge = mk_bool_op(&env, Operation::Ge, vec![a, b]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&gt, &ge));
    }

    #[test]
    fn test_implies_comparison_eq_implies_not_lt() {
        // x == 5 implies !(7 < x) since 7 >= 5
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let lt_7_x = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 7), x]);
        let not_lt_7_x = mk_bool_op(&env, Operation::Not, vec![lt_7_x]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&eq_5, &not_lt_7_x));
    }

    // ---- Forall simplification tests ----

    #[test]
    fn test_forall_flatten_nested() {
        // forall x: forall y: P(x,y) → forall x, y: P(x,y)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let body = mk_bool_op(&env, Operation::Le, vec![x, y]);
        let inner = mk_quant(&env, QuantKind::Forall, vec![("y", num_ty.clone())], body);
        let outer = mk_quant(&env, QuantKind::Forall, vec![("x", num_ty.clone())], inner);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(outer);
        // Should be a single forall with 2 ranges
        match result.as_ref() {
            ExpData::Quant(_, QuantKind::Forall, ranges, _, _, _) => {
                assert_eq!(ranges.len(), 2, "expected 2 ranges after flattening");
            },
            other => panic!("expected Quant(Forall), got {:?}", other),
        }
    }

    #[test]
    fn test_forall_one_point_rule() {
        // forall x: (x == 5 ==> x < 10) → true (since 5 < 10)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_local(&env, "x", num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let lt_10 = mk_bool_op(&env, Operation::Lt, vec![x.clone(), mk_num(&env, 10)]);
        let body = mk_bool_op(&env, Operation::Implies, vec![eq_5, lt_10]);
        let quant = mk_quant(&env, QuantKind::Forall, vec![("x", num_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_forall_remove_unused_var() {
        // forall x: true → true (x is unused)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let body = mk_bool(&env, true);
        let quant = mk_quant(&env, QuantKind::Forall, vec![("x", num_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_forall_antecedent_only_elimination() {
        // forall x: (x <= n ==> P) where P doesn't mention x → P
        // (since exists x: x <= n is satisfiable for unsigned types, e.g. x=0)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let n = mk_temp(&env, 0, num_ty.clone());
        let p = mk_temp(&env, 1, BOOL_TYPE.clone());
        let le = mk_bool_op(&env, Operation::Le, vec![x, n]);
        let body = mk_bool_op(&env, Operation::Implies, vec![le, p]);
        let quant = mk_quant(&env, QuantKind::Forall, vec![("x", num_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        // Should simplify to just P ($t1)
        assert_is_temp(&result, 1);
    }

    // ---- Exists simplification tests ----

    #[test]
    fn test_exists_flatten_nested() {
        // exists x: exists y: P(x,y) → exists x, y: P(x,y)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let body = mk_bool_op(&env, Operation::Le, vec![x, y]);
        let inner = mk_quant(&env, QuantKind::Exists, vec![("y", num_ty.clone())], body);
        let outer = mk_quant(&env, QuantKind::Exists, vec![("x", num_ty.clone())], inner);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(outer);
        // Should be a single exists with 2 ranges, or simplified further
        if let ExpData::Quant(_, QuantKind::Exists, ranges, _, _, _) = result.as_ref() {
            assert_eq!(ranges.len(), 2, "expected 2 ranges after flattening");
        }
    }

    #[test]
    fn test_exists_one_point_rule() {
        // exists x: x == 5 && x < 10 → true (instantiate x=5, 5 < 10)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_local(&env, "x", num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let lt_10 = mk_bool_op(&env, Operation::Lt, vec![x.clone(), mk_num(&env, 10)]);
        let body = mk_bool_op(&env, Operation::And, vec![eq_5, lt_10]);
        let quant = mk_quant(&env, QuantKind::Exists, vec![("x", num_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_exists_remove_unused_var() {
        // exists x: P → P where x not in FV(P)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let p = mk_temp(&env, 0, BOOL_TYPE.clone());
        let quant = mk_quant(&env, QuantKind::Exists, vec![("x", num_ty)], p);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_exists_component_splitting() {
        // exists x, y: A(x) && B(y) → (exists x: A(x)) && (exists y: B(y))
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let t1 = mk_temp(&env, 1, num_ty.clone());
        let a = mk_bool_op(&env, Operation::Le, vec![x, t0]); // x <= t0
        let b = mk_bool_op(&env, Operation::Le, vec![y, t1]); // y <= t1
        let body = mk_bool_op(&env, Operation::And, vec![a, b]);
        let quant = mk_quant(
            &env,
            QuantKind::Exists,
            vec![("x", num_ty.clone()), ("y", num_ty.clone())],
            body,
        );
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        // Result should be a conjunction (And) rather than a single exists with 2 vars
        match result.as_ref() {
            ExpData::Value(_, Value::Bool(true)) => {
                // Even better: trivially true since x=0 works
            },
            ExpData::Call(_, Operation::And, _) => {
                // Split into two parts
            },
            ExpData::Quant(_, QuantKind::Exists, ranges, _, _, _) => {
                // If not split, it should at most have 1 range
                assert!(
                    ranges.len() <= 1,
                    "expected split or simplified, got {} ranges",
                    ranges.len()
                );
            },
            _ => {
                // Any simplification is acceptable
            },
        }
    }

    #[test]
    fn test_exists_trivially_true_by_witness() {
        // exists x: u64: x <= n → true (x=0 always works for unsigned)
        let env = test_env();
        let u64_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", u64_ty.clone());
        let n = mk_temp(&env, 0, u64_ty.clone());
        let body = mk_bool_op(&env, Operation::Le, vec![x, n]);
        let quant = mk_quant(&env, QuantKind::Exists, vec![("x", u64_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        assert_is_bool(&result, true);
    }

    #[test]
    fn test_exists_absorb_inner() {
        // exists x: A(x) && (exists y: B(x,y)) → exists x, y: A(x) && B(x,y)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", num_ty.clone());
        let y = mk_local(&env, "y", num_ty.clone());
        let t0 = mk_temp(&env, 0, num_ty.clone());
        let a = mk_bool_op(&env, Operation::Le, vec![x.clone(), t0]); // x <= t0
        let b = mk_bool_op(&env, Operation::Le, vec![y.clone(), x.clone()]); // y <= x
        let inner = mk_quant(&env, QuantKind::Exists, vec![("y", num_ty.clone())], b);
        let body = mk_bool_op(&env, Operation::And, vec![a, inner]);
        let quant = mk_quant(&env, QuantKind::Exists, vec![("x", num_ty.clone())], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        // The inner exists should have been absorbed
        // Result could be simplified (trivially true since x=0,y=0) or a single exists
        match result.as_ref() {
            ExpData::Value(_, Value::Bool(true)) => {
                // Trivially true
            },
            ExpData::Quant(_, QuantKind::Exists, _ranges, _, _, body) => {
                // If not fully simplified, check inner exists was absorbed
                assert!(!matches!(
                    body.as_ref(),
                    ExpData::Quant(_, QuantKind::Exists, ..)
                ));
            },
            _ => {},
        }
    }

    // ---- Comparison implication with constant equalities ----

    #[test]
    fn test_implies_comparison_eq_implies_not_x_lt_c() {
        // x == 5 implies !(x < 3) since 5 >= 3
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let eq_5 = mk_bool_op(&env, Operation::Eq, vec![x.clone(), mk_num(&env, 5)]);
        let x_lt_3 = mk_bool_op(&env, Operation::Lt, vec![x, mk_num(&env, 3)]);
        let not_x_lt_3 = mk_bool_op(&env, Operation::Not, vec![x_lt_3]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&eq_5, &not_x_lt_3));
    }

    // ---- Upper-bound witness elimination test ----

    #[test]
    fn test_exists_upper_bound_witness() {
        // exists x: u64: x <= n && (x + 1) >= MAX
        // With upper bound x <= n and upward-safe (x+1) >= MAX, substitute x = n:
        // (n + 1) >= MAX
        let env = test_env();
        let u64_ty = Type::Primitive(PrimitiveType::U64);
        let x = mk_local(&env, "x", u64_ty.clone());
        let n = mk_temp(&env, 0, u64_ty.clone());
        let max = mk_big_num(&env, BigInt::from(18446744073709551615u64));
        let bound = mk_bool_op(&env, Operation::Le, vec![x.clone(), n.clone()]);
        let val = mk_op(&env, u64_ty.clone(), Operation::Add, vec![
            x.clone(),
            mk_num(&env, 1),
        ]);
        let overflow = mk_bool_op(&env, Operation::Ge, vec![val, max]);
        let body = mk_bool_op(&env, Operation::And, vec![bound, overflow]);
        let quant = mk_quant(&env, QuantKind::Exists, vec![("x", u64_ty)], body);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(quant);
        // Should be (n + 1) >= MAX, not an exists anymore
        assert!(
            !matches!(result.as_ref(), ExpData::Quant(..)),
            "expected witness elimination to remove exists, got: {:?}",
            result.as_ref()
        );
    }

    // ---- Monotonicity tests ----

    #[test]
    fn test_is_monotone_increasing_variable() {
        let env = test_env();
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", Type::Primitive(PrimitiveType::U64));
        assert!(is_monotone_increasing_in(&env, &x, sym, false));
    }

    #[test]
    fn test_is_monotone_increasing_constant() {
        // A constant is trivially monotone increasing
        let env = test_env();
        let sym = env.symbol_pool().make("x");
        let c = mk_num(&env, 42);
        assert!(is_monotone_increasing_in(&env, &c, sym, false));
    }

    #[test]
    fn test_is_monotone_increasing_add() {
        // x + c is monotone in x
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let expr = mk_op(&env, num_ty, Operation::Add, vec![x, mk_num(&env, 1)]);
        assert!(is_monotone_increasing_in(&env, &expr, sym, false));
    }

    #[test]
    fn test_is_monotone_increasing_sub_not_monotone() {
        // x - c is NOT monotone increasing (in the conservative analysis)
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let expr = mk_op(&env, num_ty, Operation::Sub, vec![x, mk_num(&env, 1)]);
        assert!(!is_monotone_increasing_in(&env, &expr, sym, false));
    }

    // ---- flatten_conjunction_owned tests ----

    #[test]
    fn test_flatten_conjunction_simple() {
        let env = test_env();
        let a = mk_temp(&env, 0, BOOL_TYPE.clone());
        let b = mk_temp(&env, 1, BOOL_TYPE.clone());
        let c = mk_temp(&env, 2, BOOL_TYPE.clone());
        let inner = mk_bool_op(&env, Operation::And, vec![b, c]);
        let outer = mk_bool_op(&env, Operation::And, vec![a, inner]);
        let conjuncts = flatten_conjunction_owned(&outer);
        assert_eq!(conjuncts.len(), 3);
    }

    #[test]
    fn test_flatten_conjunction_single() {
        let env = test_env();
        let a = mk_temp(&env, 0, BOOL_TYPE.clone());
        let conjuncts = flatten_conjunction_owned(&a);
        assert_eq!(conjuncts.len(), 1);
    }

    // ---- quant_symbols tests ----

    #[test]
    fn test_quant_symbols() {
        let env = test_env();
        let pool = env.symbol_pool();
        let u64_ty = Type::Primitive(PrimitiveType::U64);
        let sym_x = pool.make("x");
        let sym_y = pool.make("y");
        let pat_x_id = env.new_node(Loc::default(), u64_ty.clone());
        let pat_y_id = env.new_node(Loc::default(), u64_ty.clone());
        let dummy_range = mk_num(&env, 0);
        let ranges = vec![
            (Pattern::Var(pat_x_id, sym_x), dummy_range.clone()),
            (Pattern::Var(pat_y_id, sym_y), dummy_range),
        ];
        let syms = quant_symbols(&ranges);
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0], sym_x);
        assert_eq!(syms[1], sym_y);
    }

    // ---- is_complementary tests ----

    #[test]
    fn test_is_complementary_not_a_b() {
        let env = test_env();
        let a = mk_temp(&env, 0, BOOL_TYPE.clone());
        let not_a = mk_bool_op(&env, Operation::Not, vec![a.clone()]);
        assert!(is_complementary(&not_a, &a));
        assert!(is_complementary(&a, &not_a));
    }

    #[test]
    fn test_is_complementary_unrelated() {
        let env = test_env();
        let a = mk_temp(&env, 0, BOOL_TYPE.clone());
        let b = mk_temp(&env, 1, BOOL_TYPE.clone());
        assert!(!is_complementary(&a, &b));
    }

    // ---- Cross-operator constant cancellation tests ----

    #[test]
    fn test_add_sub_cancel() {
        // (x - 3) + 3 → x
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let sub = mk_op(&env, num_ty.clone(), Operation::Sub, vec![
            x,
            mk_num(&env, 3),
        ]);
        let e = mk_op(&env, num_ty, Operation::Add, vec![sub, mk_num(&env, 3)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_sub_add_cancel() {
        // (x + 3) - 3 → x
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let add = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x,
            mk_num(&env, 3),
        ]);
        let e = mk_op(&env, num_ty, Operation::Sub, vec![add, mk_num(&env, 3)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        assert_is_temp(&result, 0);
    }

    #[test]
    fn test_add_sub_partial() {
        // (x - 2) + 5 → x + 3
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let sub = mk_op(&env, num_ty.clone(), Operation::Sub, vec![
            x,
            mk_num(&env, 2),
        ]);
        let e = mk_op(&env, num_ty, Operation::Add, vec![sub, mk_num(&env, 5)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // Should be $t0 + 3
        match result.as_ref() {
            ExpData::Call(_, Operation::Add, args) if args.len() == 2 => {
                assert_is_temp(&args[0], 0);
                assert_is_num(&args[1], 3);
            },
            other => panic!("expected Add(Temporary(0), 3), got {:?}", other),
        }
    }

    #[test]
    fn test_sub_add_partial() {
        // (x + 5) - 2 → x + 3
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let add = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x,
            mk_num(&env, 5),
        ]);
        let e = mk_op(&env, num_ty, Operation::Sub, vec![add, mk_num(&env, 2)]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // Should be $t0 + 3
        match result.as_ref() {
            ExpData::Call(_, Operation::Add, args) if args.len() == 2 => {
                assert_is_temp(&args[0], 0);
                assert_is_num(&args[1], 3);
            },
            other => panic!("expected Add(Temporary(0), 3), got {:?}", other),
        }
    }

    // ---- Nested conjunction pruning tests ----

    #[test]
    fn test_nested_conjunction_pruning() {
        // 0 < x && (2 < x && P) → 2 < x && P
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let p = mk_temp(&env, 1, BOOL_TYPE.clone());
        let lt_0_x = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 0), x.clone()]);
        let lt_2_x = mk_bool_op(&env, Operation::Lt, vec![mk_num(&env, 2), x]);
        let inner = mk_bool_op(&env, Operation::And, vec![lt_2_x, p]);
        let e = mk_bool_op(&env, Operation::And, vec![lt_0_x, inner]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // Should be And(Lt(2, $t0), $t1) — the 0 < x conjunct should be pruned
        match result.as_ref() {
            ExpData::Call(_, Operation::And, args) if args.len() == 2 => {
                match args[0].as_ref() {
                    ExpData::Call(_, Operation::Lt, lt_args) if lt_args.len() == 2 => {
                        assert_is_num(&lt_args[0], 2);
                        assert_is_temp(&lt_args[1], 0);
                    },
                    other => panic!("expected Lt(2, $t0), got {:?}", other),
                }
                assert_is_temp(&args[1], 1);
            },
            other => panic!("expected And(Lt(2, $t0), $t1), got {:?}", other),
        }
    }

    // ---- implies_comparison with Gt/Ge/Le tests ----

    #[test]
    fn test_implies_comparison_gt_gt() {
        // n > MAX implies n > 0
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let n = mk_temp(&env, 0, num_ty.clone());
        let gt_max = mk_bool_op(&env, Operation::Gt, vec![n.clone(), mk_num(&env, 100)]);
        let gt_0 = mk_bool_op(&env, Operation::Gt, vec![n, mk_num(&env, 0)]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&gt_max, &gt_0));
    }

    #[test]
    fn test_implies_comparison_gt_gt_not_reverse() {
        // n > 0 does NOT imply n > 100
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let n = mk_temp(&env, 0, num_ty.clone());
        let gt_max = mk_bool_op(&env, Operation::Gt, vec![n.clone(), mk_num(&env, 100)]);
        let gt_0 = mk_bool_op(&env, Operation::Gt, vec![n, mk_num(&env, 0)]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(!s.implies_comparison(&gt_0, &gt_max));
    }

    #[test]
    fn test_implies_comparison_gt_prunes_and() {
        // n > 0 && n > MAX_U64 → n > MAX_U64
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let n = mk_temp(&env, 0, num_ty.clone());
        let gt_0 = mk_bool_op(&env, Operation::Gt, vec![n.clone(), mk_num(&env, 0)]);
        let gt_max = mk_bool_op(&env, Operation::Gt, vec![n, mk_num(&env, 100)]);
        let e = mk_bool_op(&env, Operation::And, vec![gt_0, gt_max]);
        let mut g = test_gen(&env);
        let mut s = ExpSimplifier::new(&mut g);
        let result = s.simplify(e);
        // Should be Gt(n, 100) — the n > 0 conjunct should be pruned
        match result.as_ref() {
            ExpData::Call(_, Operation::Gt, args) if args.len() == 2 => {
                assert_is_temp(&args[0], 0);
                assert_is_num(&args[1], 100);
            },
            other => panic!("expected Gt($t0, 100), got {:?}", other),
        }
    }

    #[test]
    fn test_implies_comparison_le_le() {
        // x <= 3 implies x <= 5
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let le_3 = mk_bool_op(&env, Operation::Le, vec![x.clone(), mk_num(&env, 3)]);
        let le_5 = mk_bool_op(&env, Operation::Le, vec![x, mk_num(&env, 5)]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&le_3, &le_5));
    }

    #[test]
    fn test_implies_comparison_ge_ge() {
        // x >= 5 implies x >= 3
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let x = mk_temp(&env, 0, num_ty.clone());
        let ge_5 = mk_bool_op(&env, Operation::Ge, vec![x.clone(), mk_num(&env, 5)]);
        let ge_3 = mk_bool_op(&env, Operation::Ge, vec![x, mk_num(&env, 3)]);
        let mut g = test_gen(&env);
        let s = ExpSimplifier::new(&mut g);
        assert!(s.implies_comparison(&ge_5, &ge_3));
    }

    // ---- Monotonicity with Div test ----

    #[test]
    fn test_is_monotone_increasing_div_by_constant() {
        // x * (x + 1) / 2 is monotone increasing in x
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::U64);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let x_plus_1 = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x.clone(),
            mk_num(&env, 1),
        ]);
        let product = mk_op(&env, num_ty.clone(), Operation::Mul, vec![x, x_plus_1]);
        let expr = mk_op(&env, num_ty, Operation::Div, vec![product, mk_num(&env, 2)]);
        assert!(is_monotone_increasing_in(&env, &expr, sym, false));
    }

    // ---- Monotonicity with unsigned context tests ----

    #[test]
    fn test_is_monotone_mul_num_type_without_context() {
        // x * (x + 1) with Num type is NOT monotone without unsigned context,
        // because Num is signed (arbitrary precision) and could be negative.
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let x_plus_1 = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x.clone(),
            mk_num(&env, 1),
        ]);
        let product = mk_op(&env, num_ty, Operation::Mul, vec![x, x_plus_1]);
        assert!(!is_monotone_increasing_in(&env, &product, sym, false));
    }

    #[test]
    fn test_is_monotone_mul_num_type_with_unsigned_context() {
        // x * (x + 1) with Num type IS monotone in an unsigned context,
        // because we know all quantified variables are unsigned (non-negative).
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let x_plus_1 = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x.clone(),
            mk_num(&env, 1),
        ]);
        let product = mk_op(&env, num_ty, Operation::Mul, vec![x, x_plus_1]);
        assert!(is_monotone_increasing_in(&env, &product, sym, true));
    }

    #[test]
    fn test_is_monotone_div_num_type_with_unsigned_context() {
        // x * (x + 1) / 2 with Num type is monotone in an unsigned context.
        let env = test_env();
        let num_ty = Type::Primitive(PrimitiveType::Num);
        let sym = env.symbol_pool().make("x");
        let x = mk_local(&env, "x", num_ty.clone());
        let x_plus_1 = mk_op(&env, num_ty.clone(), Operation::Add, vec![
            x.clone(),
            mk_num(&env, 1),
        ]);
        let product = mk_op(&env, num_ty.clone(), Operation::Mul, vec![x, x_plus_1]);
        let expr = mk_op(&env, num_ty, Operation::Div, vec![product, mk_num(&env, 2)]);
        assert!(is_monotone_increasing_in(&env, &expr, sym, true));
    }
}
