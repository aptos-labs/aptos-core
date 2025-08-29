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
//!   pure_function();
//! Third, it looks for complex expressions that contain internal state changes
//! that are not propagated outside of themselves.
//! Consider this example:
//!
//!   fun impure(x: &mut u64){
//!       *x += 1;
//!   }
//!
//!   fun pure(x: u64): u64{
//!       x + 1
//!   }
//!
//!   fun f(){
//!       // This statement can be removed.
//!       {
//!           let x = 0;
//!           x += 1;
//!           x
//!       };
//!
//!       // This statement can be removed.
//!       {
//!           let x = 0;
//!           impure(&mut x);
//!           x
//!       };
//!
//!       let x = 0;
//!       // This statement cannot be removed...
//!       {
//!           impure(&mut x);
//!          // ...but this one can.
//!           x
//!       };
//!
//!       // This statement can be removed.
//!       pure({
//!           let x = 0;
//!           {
//!               impure(&mut x);
//!               x
//!           }
//!       });
//!   }
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
    model::{FunId, FunctionEnv, NodeId, Parameter},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
};
use std::collections::HashSet;

static NO_EFFECT_STMT: &str = "This statement has no effect and can be removed";
static NO_EFFECT_OR_ABORT_STMT: &str = "This statement has no effect besides possibly aborting due to arithmetic errors and can be refactored or removed";
static NO_EFFECT_ASSIGN: &str = "This assignment has no effect and can be removed";
static NO_EFFECT_OR_ABORT_ASSIGN: &str = "This assignment has no effect besides possibly aborting due to arithmetic errors and can be refactored or removed";

#[derive(Default)]
pub struct NoOp {
    //Marks nodes to be skipped during visits.
    skip: HashSet<NodeId>,
}

impl ExpChecker for NoOp {
    fn get_name(&self) -> String {
        "no_op".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        match expr {
            ExpData::Sequence(_, _) => {
                self.visit_subexpression(function, expr, false, 0);
            },
            ExpData::Block(_, _, _, _) => {
                self.visit_subexpression(function, expr, false, 0);
            },
            _ => {},
        }
    }
}

impl NoOp {
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

//Represents the locality of an object with respect to a specific scope, an
//"object" being a variable, reference, argument, or result of an evaluation.
//The scope will usually be narrower than a complete function, often just a
//sequence or a statement.
#[derive(PartialEq, Clone, Debug)]
enum Locality {
    //The locality cannot be determined.
    Unknown,
    //The object is local to the scope.
    Local,
    //The object is non-local.
    NonLocal,
}

//Represents the "referenceness" of a function argument. Note that this applies
//to function *arguments*, so this can only be determined at call sites.
#[derive(PartialEq, Clone, Debug)]
enum ArgumentReference {
    //The referenceness cannot be determined.
    Unknown,
    //The argument is a value. Either a literal or the result of an evaluation.
    Value,
    //The argument is an immutable reference.
    ImmutableRef,
    //The argument is a mutable reference.
    MutableRef(Locality),
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

fn combine_localities(left: Locality, right: Locality) -> Locality {
    if left == Locality::Unknown || right == Locality::Unknown {
        return Locality::Unknown;
    }
    if left == Locality::NonLocal || right == Locality::NonLocal {
        return Locality::NonLocal;
    }
    Locality::Local
}

fn combine_references(left: ArgumentReference, right: ArgumentReference) -> ArgumentReference {
    if left == right {
        return left;
    }
    if left == ArgumentReference::Unknown || right == ArgumentReference::Unknown {
        return ArgumentReference::Unknown;
    }
    if let ArgumentReference::MutableRef(left2) = &left {
        if let ArgumentReference::MutableRef(right2) = right {
            let combined = combine_localities(left2.clone(), right2);
            return ArgumentReference::MutableRef(combined);
        }
        return left;
    }
    if let ArgumentReference::MutableRef(_) = right {
        return right;
    }
    if left == ArgumentReference::ImmutableRef || right == ArgumentReference::ImmutableRef {
        ArgumentReference::ImmutableRef
    } else {
        ArgumentReference::Value
    }
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

fn combine_references_opt(
    left: Option<ArgumentReference>,
    right: ArgumentReference,
) -> Option<ArgumentReference> {
    Some(match left {
        None => right,
        Some(left) => combine_references(left, right),
    })
}

fn classify_call_locality(
    op: &Operation,
    _arguments: &[Exp],
    _local_symbols: &HashSet<Symbol>,
) -> Locality {
    match op {
        Operation::Add
        | Operation::Sub
        | Operation::Mul
        | Operation::Mod
        | Operation::Div
        | Operation::BitOr
        | Operation::BitAnd
        | Operation::Xor
        | Operation::Shl
        | Operation::Shr
        | Operation::And
        | Operation::Or
        | Operation::Eq
        | Operation::Neq
        | Operation::Lt
        | Operation::Gt
        | Operation::Le
        | Operation::Ge
        | Operation::Not
        | Operation::Exists(_)
        | Operation::Len
        | Operation::Vector
        | Operation::EmptyVec
        | Operation::MaxU8
        | Operation::MaxU16
        | Operation::MaxU32
        | Operation::MaxU64
        | Operation::MaxU128
        | Operation::MaxU256 => Locality::Local,
        Operation::BorrowGlobal(_) | Operation::Global(_) => Locality::NonLocal,
        _ => Locality::Unknown,
    }
}

fn classify_expression_locality(expr: &ExpData, local_symbols: &HashSet<Symbol>) -> Locality {
    match expr {
        ExpData::Value(_, _) => Locality::Local,
        ExpData::LocalVar(_, symbol) => {
            if local_symbols.contains(symbol) {
                Locality::Local
            } else {
                Locality::NonLocal
            }
        },
        ExpData::Temporary(_, _) => Locality::Local,
        ExpData::Call(_, operation, args) => classify_call_locality(operation, args, local_symbols),
        _ => Locality::Unknown,
    }
}

fn classify_expression_reference(
    expr: &ExpData,
    local_symbols: &HashSet<Symbol>,
    function: &FunctionEnv,
) -> ArgumentReference {
    match expr {
        ExpData::Value(_, _) => ArgumentReference::Value,
        ExpData::Temporary(_, _) => ArgumentReference::Value,
        ExpData::Call(_, Borrow(ReferenceKind::Immutable), _) => ArgumentReference::ImmutableRef,
        ExpData::Call(_, Borrow(ReferenceKind::Mutable), exps) => {
            if exps.len() != 1 {
                ArgumentReference::MutableRef(Locality::NonLocal)
            } else {
                ArgumentReference::MutableRef(classify_expression_locality(
                    exps.first().unwrap(),
                    local_symbols,
                ))
            }
        },
        ExpData::Block(_, pattern, rhs, expr) => {
            let mut local_symbols = local_symbols.clone();
            add_declared_symbols(&mut local_symbols, rhs, pattern, function);
            classify_expression_reference(expr, &local_symbols, function)
        },
        ExpData::IfElse(_, _, true_block, false_block) => {
            let t = classify_expression_reference(true_block, local_symbols, function);
            let f = classify_expression_reference(false_block, local_symbols, function);
            combine_references(t, f)
        },
        ExpData::Match(_, _, arms) => arms
            .iter()
            .map(|x| classify_expression_reference(&x.body, local_symbols, function))
            .fold(None, combine_references_opt)
            .unwrap_or(ArgumentReference::Unknown),
        ExpData::Sequence(_, exprs) => classify_sequence_locality(exprs, local_symbols, function),
        _ => ArgumentReference::Unknown,
    }
}

fn classify_sequence_locality(
    exprs: &[Exp],
    local_symbols: &HashSet<Symbol>,
    function: &FunctionEnv,
) -> ArgumentReference {
    if exprs.is_empty() {
        return ArgumentReference::Unknown;
    }
    classify_expression_reference(exprs.last().unwrap(), local_symbols, function)
}

fn classify_argument(
    expr: &ExpData,
    local_symbols: &HashSet<Symbol>,
    function: &FunctionEnv,
) -> ArgumentReference {
    classify_expression_reference(expr, local_symbols, function)
}

fn argument_reference_to_locality(arg_ref: ArgumentReference) -> bool {
    matches!(
        arg_ref,
        ArgumentReference::Value
            | ArgumentReference::ImmutableRef
            | ArgumentReference::MutableRef(Locality::Local)
    )
}

fn argument_is_local(
    expr: &ExpData,
    local_symbols: &HashSet<Symbol>,
    function: &FunctionEnv,
) -> bool {
    argument_reference_to_locality(classify_argument(expr, local_symbols, function))
}

fn add_declared_symbols(
    dst: &mut HashSet<Symbol>,
    rhs: &Option<Exp>,
    pattern: &Pattern,
    function: &FunctionEnv,
) {
    match pattern {
        Pattern::Var(id, symbol) => match function.env().get_node_type(*id) {
            Type::Primitive(_) => {
                dst.insert(*symbol);
            },
            Type::Reference(reference_kind, _) => match reference_kind {
                ReferenceKind::Mutable => {
                    if let Some(rhs) = rhs {
                        match rhs as &ExpData {
                            ExpData::LocalVar(_, symbol2) => {
                                if dst.contains(symbol2) {
                                    dst.insert(*symbol);
                                }
                            },
                            ExpData::Temporary(_, idx) => {
                                if dst.contains(&function.get_local_name(*idx)) {
                                    dst.insert(*symbol);
                                }
                            },
                            _ => {},
                        }
                    }
                },
                ReferenceKind::Immutable => {
                    dst.insert(*symbol);
                },
            },
            _ => {},
        },
        Pattern::Wildcard(_) => {},
        Pattern::Tuple(_, _) => {},
        Pattern::Struct(_, _, _, _) => {},
        Pattern::Error(_) => {},
    }
}

struct EffectsAnalyzer {
    pub visited: HashSet<FunId>,
}

impl EffectsAnalyzer {
    pub fn new() -> EffectsAnalyzer {
        EffectsAnalyzer {
            visited: HashSet::new(),
        }
    }

    fn classify_unary_op(
        &mut self,
        args: &[Exp],
        function: &FunctionEnv,
        local_symbols: &HashSet<Symbol>,
    ) -> EffectClass {
        if args.len() != 1 {
            EffectClass::Unknown
        } else {
            self.classify_expression_with_symbols(args.first().unwrap(), function, local_symbols)
        }
    }

    fn classify_binary_op(
        &mut self,
        args: &[Exp],
        function: &FunctionEnv,
        local_symbols: &HashSet<Symbol>,
    ) -> EffectClass {
        if args.len() != 2 {
            EffectClass::Unknown
        } else {
            let left = args.first().unwrap();
            let right = args.get(1).unwrap();
            let left = self.classify_expression_with_symbols(left, function, local_symbols);
            let right = self.classify_expression_with_symbols(right, function, local_symbols);
            combine_effects(left, right)
        }
    }

    fn classify_call(
        &mut self,
        op: &Operation,
        args: &[Exp],
        function: &FunctionEnv,
        local_symbols: &HashSet<Symbol>,
    ) -> EffectClass {
        match op {
            Operation::Not => self.classify_unary_op(args, function, local_symbols),
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
            | Operation::Le => self.classify_binary_op(args, function, local_symbols),
            Operation::MoveFunction(mid, fid) => {
                let env = function.env();
                let argument_purity = args
                    .iter()
                    .map(|x| self.classify_expression_with_symbols(x, function, local_symbols))
                    .fold(EffectClass::Pure, combine_effects);
                if argument_purity != EffectClass::Pure {
                    return argument_purity;
                }
                let args = args
                    .iter()
                    .map(|x| argument_is_local(x, local_symbols, function))
                    .collect::<Vec<_>>();
                let pure =
                    self.function_call_is_pure(&env.get_function(mid.qualified(*fid)), &args);
                match pure {
                    Some(true) => EffectClass::Pure,
                    Some(false) => EffectClass::Mutation,
                    None => EffectClass::Unknown,
                }
            },
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
                .map(|x| self.classify_expression_with_symbols(x, function, local_symbols))
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

    /// classify_expression_with_symbols() works similarly to
    /// classify_expression_purity(), except it takes a set of symbols. If it
    /// finds a mutation of a symbol in that set, it does not classify the
    /// expression as a mutation.
    fn classify_expression_with_symbols(
        &mut self,
        expr: &ExpData,
        function: &FunctionEnv,
        local_symbols: &HashSet<Symbol>,
    ) -> EffectClass {
        match expr {
            ExpData::Value(_, _) => EffectClass::Pure,
            ExpData::LocalVar(_, _) => EffectClass::Pure,
            ExpData::Temporary(_, _) => EffectClass::Pure,
            ExpData::Call(_, op, args) => self.classify_call(op, args, function, local_symbols),
            ExpData::IfElse(_, condition, then_block, else_block) => {
                [condition, then_block, else_block]
                    .iter()
                    .map(|x| self.classify_expression_with_symbols(x, function, local_symbols))
                    .fold(EffectClass::Pure, combine_effects)
            },
            ExpData::Loop(_, body) => {
                let mut ret = self.classify_expression_with_symbols(body, function, local_symbols);
                if let EffectClass::ControlFlow(depth) = ret {
                    ret = if depth > 0 {
                        EffectClass::ControlFlow(depth - 1)
                    } else {
                        EffectClass::Pure
                    };
                }
                ret
            },
            ExpData::LoopCont(_, depth, _) => EffectClass::ControlFlow(*depth),
            ExpData::Block(_, pattern, rhs, body) => {
                let rhs_class = if let Some(rhs) = rhs {
                    self.classify_expression_with_symbols(rhs, function, local_symbols)
                } else {
                    EffectClass::Pure
                };
                if matches!(pattern, Pattern::Struct(_, _, _, _)) {
                    return EffectClass::Unknown;
                }
                let mut local_symbols = local_symbols.clone();
                add_declared_symbols(&mut local_symbols, rhs, pattern, function);
                let body = self.classify_expression_with_symbols(body, function, &local_symbols);
                combine_effects(rhs_class, body)
            },
            ExpData::Assign(_, pattern, rhs) => {
                let lhs = match pattern {
                    Pattern::Var(_, symbol) => {
                        if local_symbols.contains(symbol) {
                            EffectClass::Pure
                        } else {
                            EffectClass::Mutation
                        }
                    },
                    _ => EffectClass::Unknown,
                };
                let rhs = self.classify_expression_with_symbols(rhs, function, local_symbols);
                combine_effects(lhs, rhs)
            },
            ExpData::Mutate(_, lhs, rhs) => {
                let lhs = match lhs as &ExpData {
                    ExpData::LocalVar(_, symbol) => {
                        if local_symbols.contains(symbol) {
                            EffectClass::Pure
                        } else {
                            EffectClass::Mutation
                        }
                    },
                    _ => EffectClass::Unknown,
                };
                let rhs = self.classify_expression_with_symbols(rhs, function, local_symbols);
                combine_effects(lhs, rhs)
            },
            ExpData::Sequence(_, exprs) => exprs
                .iter()
                .map(|x| self.classify_expression_with_symbols(x, function, local_symbols))
                .fold(EffectClass::Pure, combine_effects),
            ExpData::Match(_, exp, match_arms) => {
                let exp = self.classify_expression_with_symbols(exp, function, local_symbols);
                let arms = match_arms
                    .iter()
                    .map(|arm| {
                        let condition = if let Some(condition) = &arm.condition {
                            self.classify_expression_with_symbols(
                                condition,
                                function,
                                local_symbols,
                            )
                        } else {
                            EffectClass::Pure
                        };
                        let body = self.classify_expression_with_symbols(
                            &arm.body,
                            function,
                            local_symbols,
                        );
                        combine_effects(condition, body)
                    })
                    .fold(EffectClass::Pure, combine_effects);
                combine_effects(exp, arms)
            },
            ExpData::Return(_, _) => EffectClass::ControlFlow(usize::MAX),
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
        self.classify_expression_with_symbols(expr, function, &HashSet::new())
    }

    //Returns true if the type definitely does not contain mutable references.
    fn type_is_pure(ty: &Type) -> bool {
        match ty {
            Type::Primitive(_) => true,
            Type::Tuple(items) => items.iter().all(Self::type_is_pure),
            Type::Vector(subtype) => Self::type_is_pure(subtype),
            Type::Struct(_, _, subtypes) => subtypes.iter().all(Self::type_is_pure),
            Type::TypeParameter(_) => false,
            Type::Fun(_, _, _) => false,
            Type::Reference(reference_kind, t) => match reference_kind {
                ReferenceKind::Immutable => Self::type_is_pure(t),
                ReferenceKind::Mutable => false,
            },
            Type::TypeDomain(_) => false,
            Type::ResourceDomain(_, _, _) => false,
            Type::Error => true,
            Type::Var(_) => true,
        }
    }

    fn function_parameter_is_pure(param: &Parameter) -> bool {
        Self::type_is_pure(&param.1)
    }

    /// function_call_is_pure() analyzes the side-effects of calling a
    /// function. Note that this function analyzes function *calls* not
    /// *functions*.
    ///  * None: The side-effects could not be analyzed.
    ///  * Some(true): The function call does not affect any state external to
    ///                itself. This includes both global state as well as
    ///                mutable reference arguments.
    ///  * Some(false): The function call does affect state external to itself.
    ///
    /// `argument_locality` is a vector of booleans corresponding to the
    /// locality of call's arguments. Mutations to those arguments do not make
    /// a pure call impure.
    pub fn function_call_is_pure(
        &mut self,
        function: &FunctionEnv,
        argument_locality: &[bool],
    ) -> Option<bool> {
        let id = function.get_id();
        if !self.visited.insert(id) {
            return None;
        }
        let def = function.get_def()?;

        let mut ret = Some(true);
        let mut local_symbols = HashSet::<Symbol>::new();
        let params = function.get_parameters();
        if argument_locality.len() < params.len() {
            ret = None;
        } else {
            for (locality, param) in argument_locality
                .iter()
                .take(params.len())
                .zip(params.iter())
            {
                if *locality || Self::function_parameter_is_pure(param) {
                    local_symbols.insert(param.0);
                } else {
                    ret = Some(false);
                    break;
                }
            }
        }
        let local_symbols = local_symbols;

        if ret.unwrap_or(false) {
            ret = Some(
                self.classify_expression_with_symbols(def, function, &local_symbols)
                    == EffectClass::Pure,
            );
        }

        self.visited.remove(&id);

        ret
    }
}
