// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for statements that
//! do nothing. In particular, it looks for three kinds of statements:
//! First, assignments where the destination is an unused temporary:
//!   *(&mut 0) = pure_expression;
//!   *(&mut 1) = pure_expression;
//!   *(&mut (x + 1)) = pure_expression;
//! Second, statements consisting of a single expression that doesn't change any
//! state:
//!   0;
//!   x;
//!   x + 1;
//!
//! Of note, possible aborts caused by arithmetic errors are ignored by the
//! linter. As such, the suggestions offered may not strictly preserve
//! semantics. This is not a problem, as a statement that has no
//! externally-observable effect other than implicitly aborting under certain
//! states is almost certainly not the user's intention.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{
        Exp, ExpData,
        Operation::{self, Borrow},
        Pattern,
    },
    model::{FunctionEnv, NodeId},
    ty::{ReferenceKind, Type},
};
use std::collections::HashSet;

static NO_EFFECT_STMT: &str = "This statement has no effect and can be removed";
static NO_EFFECT_OR_ABORT_STMT: &str = "This statement has no effect besides possibly aborting due to arithmetic errors and might be refactored or removed";
static NO_EFFECT_ASSIGN: &str = "This assignment has no effect and can be removed";
static NO_EFFECT_OR_ABORT_ASSIGN: &str = "This assignment has no effect besides possibly aborting due to arithmetic errors and might be refactored or removed";

#[derive(Default)]
pub struct NoEffectOp {
    //Marks nodes to be skipped during visits.
    skip: HashSet<NodeId>,
}

impl ExpChecker for NoEffectOp {
    fn get_name(&self) -> String {
        "no_effect_op".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        if matches!(expr, ExpData::Sequence(..)) || matches!(expr, ExpData::Block(..)) {
            self.visit_subexpression(function, expr, false, 0);
        }
    }
}

impl NoEffectOp {
    fn visit_subexpression(
        &mut self,
        function: &FunctionEnv,
        expr: &ExpData,
        only_sequence: bool,
        depth: u64,
    ) {
        if only_sequence && !matches!(expr, ExpData::Sequence(_, _)) {
            return;
        }

        let exp_id = expr.node_id();
        if !self.skip.insert(exp_id) {
            return;
        }
        match expr {
            ExpData::Value(id, _) | ExpData::Lambda(id, _, _, _, _) => {
                let env = function.env();
                self.report(env, &env.get_node_loc(*id), NO_EFFECT_STMT);
            },
            ExpData::Mutate(id, left, right) => {
                if let Some(bitbucket) = Self::get_bitbucket_expression(left, function) {
                    let mut analyzer = EffectsAnalyzer::new();
                    if analyzer.classify_expression_purity(right, function) == EffectClass::Pure {
                        let env = function.env();

                        let msg = if Self::expression_is_trivial(bitbucket)
                            && Self::expression_is_trivial(right)
                        {
                            NO_EFFECT_ASSIGN
                        } else {
                            NO_EFFECT_OR_ABORT_ASSIGN
                        };

                        self.report(env, &env.get_node_loc(*id), msg);
                    }
                }
            },
            ExpData::Block(id, _, _, block_expr) => {
                self.visit_subexpression(function, block_expr, true, depth + 1);
                let mut analyzer = EffectsAnalyzer::new();
                if analyzer.classify_expression_purity(expr, function) == EffectClass::Pure {
                    let env = function.env();
                    if depth > 0 || env.get_node_type(*id) == Type::Tuple(vec![]) {
                        self.report(env, &env.get_node_loc(*id), NO_EFFECT_OR_ABORT_STMT);
                    }
                }
            },
            ExpData::Sequence(id, seq) => {
                for i in seq.iter().take(seq.len() - 1) {
                    self.visit_subexpression(function, i, false, depth + 1);
                }

                let mut analyzer = EffectsAnalyzer::new();
                if analyzer.classify_expression_purity(expr, function) == EffectClass::Pure {
                    let env = function.env();
                    if depth > 0 || env.get_node_type(*id) == Type::Tuple(vec![]) {
                        self.report(env, &env.get_node_loc(*id), NO_EFFECT_OR_ABORT_STMT);
                    }
                }
            },
            ExpData::Loop(_, body) => {
                self.visit_subexpression(function, body, true, depth + 1);
            },
            ExpData::LoopCont(_, _, _) => {},
            ExpData::Assign(_, _, _) => {},
            ExpData::SpecBlock(_, _) => {},
            _ => {
                if let ExpData::Call(_, Operation::Tuple, args) = expr {
                    if args.is_empty() {
                        return;
                    }
                }
                let env = function.env();
                let mut analyzer = EffectsAnalyzer::new();
                if analyzer.classify_expression_purity(expr, function) == EffectClass::Pure {
                    self.report(env, &env.get_node_loc(exp_id), NO_EFFECT_OR_ABORT_STMT);
                }
            },
        }
    }

    // If the left side of an assignment is `*(&mut x)`, where `x` is any
    // literal or the result of some operation, this function returns Some(x).
    // Otherwise, it returns None.
    fn get_bitbucket_expression<'a>(
        left: &'a ExpData,
        function: &FunctionEnv,
    ) -> Option<&'a ExpData> {
        if let ExpData::Call(_, op, args) = left {
            if *op != Borrow(ReferenceKind::Mutable) || args.len() != 1 {
                return None;
            }
            let e: &ExpData = args.first().unwrap();
            match e {
                ExpData::Value(_, _) => return Some(e),
                ExpData::Call(_, op, _) => match op {
                    Operation::Not
                    | Operation::Add
                    | Operation::Sub
                    | Operation::Mul
                    | Operation::Div
                    | Operation::Mod
                    | Operation::And
                    | Operation::Or
                    | Operation::BitAnd
                    | Operation::BitOr
                    | Operation::Xor
                    | Operation::Shl
                    | Operation::Shr
                    | Operation::Eq
                    | Operation::Neq
                    | Operation::Lt
                    | Operation::Gt
                    | Operation::Ge
                    | Operation::Le => return Some(e),
                    _ => {},
                },
                _ => {},
            }
            let mut analyzer = EffectsAnalyzer::new();
            if analyzer.classify_expression_purity(e, function) == EffectClass::Pure {
                return Some(e);
            }
        }
        None
    }

    fn expression_is_trivial(e: &ExpData) -> bool {
        matches!(e, ExpData::Value(_, _))
    }
}

//Represents the kind of effects an expression has on program state.
#[derive(PartialEq, Clone, Debug)]
enum EffectClass {
    //The object could not be analyzed.
    Unknown,
    //The object has no side-effects.
    Pure,
    //The object causes state mutation side-effects.
    Mutation,
    //The object affects control flow in a non-sequential fashion.
    ControlFlow(usize),
    //The object affects both control flow and state.
    Complex(usize),
}

fn combine_effects(left: EffectClass, right: EffectClass) -> EffectClass {
    if left == EffectClass::Unknown || right == EffectClass::Unknown {
        return EffectClass::Unknown;
    }

    //Neither is Unknown

    if let EffectClass::Complex(left) = left {
        if let EffectClass::Complex(right) = right {
            return EffectClass::Complex(left.max(right));
        }
        return EffectClass::Complex(left);
    }
    if let EffectClass::Complex(_) = right {
        return right;
    }

    //Neither is Unknown
    //Neither is Complex(x)

    if left == EffectClass::Pure {
        return right;
    }
    if right == EffectClass::Pure {
        return left;
    }

    //Neither is Unknown
    //Neither is Complex(x)
    //Neither is Pure

    if left == right {
        return left;
    }

    //Neither is Unknown
    //Neither is Complex(x)
    //Neither is Pure
    //They're not equal
    //Exactly one of these is true:
    //* left == Mutation       && right == ControlFlow(x)
    //* left == ControlFlow(x) && right == Mutation
    //* left == ControlFlow(x) && right == ControlFlow(y) && x != y

    if left == EffectClass::Mutation {
        if let EffectClass::ControlFlow(right) = right {
            return EffectClass::Complex(right);
        }
        unreachable!();
    }

    //Neither is Unknown
    //Neither is Complex(x)
    //Neither is Pure
    //They're not equal
    //Exactly one of these is true:
    //* left == ControlFlow(x) && right == Mutation
    //* left == ControlFlow(x) && right == ControlFlow(y) && x != y

    if right == EffectClass::Mutation {
        if let EffectClass::ControlFlow(left) = left {
            return EffectClass::Complex(left);
        }
    }

    //Neither is Unknown
    //Neither is Complex(x)
    //Neither is Pure
    //They're not equal
    //Exactly one of these is true:
    //* left == ControlFlow(x) && right == ControlFlow(y) && x != y

    if let EffectClass::ControlFlow(left) = left {
        if let EffectClass::ControlFlow(right) = right {
            return EffectClass::ControlFlow(left.max(right));
        }
    }

    //Neither is Unknown
    //Neither is Complex(x)
    //Neither is Pure
    //Neither is Mutation
    //Neither is ControlFlow(x)

    //Control flow can never reach this point.
    unreachable!();
}

struct EffectsAnalyzer;

impl EffectsAnalyzer {
    pub fn new() -> EffectsAnalyzer {
        EffectsAnalyzer {}
    }

    fn classify_unary_op(&mut self, args: &[Exp], function: &FunctionEnv) -> EffectClass {
        if args.len() != 1 {
            EffectClass::Unknown
        } else {
            self.classify_expression_purity(args.first().unwrap(), function)
        }
    }

    fn classify_binary_op(&mut self, args: &[Exp], function: &FunctionEnv) -> EffectClass {
        if args.len() != 2 {
            EffectClass::Unknown
        } else {
            let left = args.first().unwrap();
            let right = args.get(1).unwrap();
            let left = self.classify_expression_purity(left, function);
            let right = self.classify_expression_purity(right, function);
            combine_effects(left, right)
        }
    }

    fn classify_call(
        &mut self,
        op: &Operation,
        args: &[Exp],
        function: &FunctionEnv,
    ) -> EffectClass {
        match op {
            Operation::Not => self.classify_unary_op(args, function),
            Operation::Add
            | Operation::Sub
            | Operation::Mul
            | Operation::Div
            | Operation::Mod
            | Operation::And
            | Operation::Or
            | Operation::BitAnd
            | Operation::BitOr
            | Operation::Xor
            | Operation::Shl
            | Operation::Shr
            | Operation::Eq
            | Operation::Neq
            | Operation::Lt
            | Operation::Gt
            | Operation::Ge
            | Operation::Le => self.classify_binary_op(args, function),
            Operation::Borrow(_) => {
                if args.len() != 1 {
                    return EffectClass::Unknown;
                }
                self.classify_expression_purity(args.first().unwrap(), function)
            },
            Operation::BorrowGlobal(t) => match t {
                ReferenceKind::Immutable => EffectClass::Pure,
                ReferenceKind::Mutable => EffectClass::Mutation,
            },
            Operation::Select(_, _, _) | Operation::Vector => args
                .iter()
                .map(|x| self.classify_expression_purity(x, function))
                .fold(EffectClass::Pure, combine_effects),
            Operation::Deref => EffectClass::Pure,
            Operation::Tuple => {
                if args.is_empty() {
                    EffectClass::Pure
                } else {
                    EffectClass::Unknown
                }
            },
            _ => EffectClass::Unknown,
        }
    }

    /// classify_expression_purity() returns the class of effects that `expr`
    /// has. For example,
    ///  (x + 1)          // EffectClass::Pure. The expression does not affect
    ///                   // state.
    ///  x = 1;           // EffectClass::Mutation. The expression mutates
    ///                   // state.
    ///  break;           // EffectClass::ControlFlow(x). The expression alters
    ///                      control flow.
    ///  if (foo){
    ///      x = 1;
    ///  }else{
    ///      break;
    ///  }                // EffectClass::Complex(x). The if expression may both
    ///                   // mutate state as well as alter control flow.
    ///  recursive_call() // EffectClass::Unknown. Some constructs are not
    ///                   // analyzed. Recursive functions are among them.
    pub fn classify_expression_purity(
        &mut self,
        expr: &ExpData,
        function: &FunctionEnv,
    ) -> EffectClass {
        match expr {
            ExpData::Value(_, _) => EffectClass::Pure,
            ExpData::LocalVar(_, _) => EffectClass::Pure,
            ExpData::Temporary(_, _) => EffectClass::Pure,
            ExpData::Call(_, op, args) => self.classify_call(op, args, function),
            ExpData::Assign(_, pattern, rhs) => {
                let lhs = match pattern {
                    Pattern::Var(_, _) => EffectClass::Mutation,
                    _ => EffectClass::Unknown,
                };
                let rhs = self.classify_expression_purity(rhs, function);
                combine_effects(lhs, rhs)
            },
            ExpData::Mutate(_, lhs, rhs) => {
                let lhs = match lhs as &ExpData {
                    ExpData::LocalVar(_, _) => EffectClass::Mutation,
                    _ => EffectClass::Unknown,
                };
                let rhs = self.classify_expression_purity(rhs, function);
                combine_effects(lhs, rhs)
            },
            _ => EffectClass::Unknown,
        }
    }
}
