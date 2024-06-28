// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! An abstract syntax tree for the Move language used by the MoveSmith fuzzer.
//! The AST is taken mostly from `third_party/move/move-compiler/src/parser/ast.rs`.
//! Ideally when the fuzzer becomes more mature, this AST will converge to the
//! parser's AST and we might be able to reuse the parser's AST directly.

use crate::{
    names::{Identifier, IdentifierKind as IDKind},
    types::{Ability, HasType, Type, TypeArgs, TypeParameters},
    CodeGenerator,
};
use arbitrary::Arbitrary;
use num_bigint::BigUint;
use std::cell::RefCell;

/// The collection of modules and scripts that make up a Move program.
/// This is the final output of the MoveSmith fuzzer.
/// This should be runnable as a transactional test.
#[derive(Debug, Clone)]
pub struct CompileUnit {
    pub modules: Vec<Module>,
    pub scripts: Vec<Script>,
    pub runs: Vec<Identifier>,
}

/// A Move module.
#[derive(Debug, Clone)]
pub struct Module {
    // pub attributes: Vec<Attributes>,
    // pub address: Option<LeadingNameAccess>,
    pub name: Identifier,
    pub functions: Vec<RefCell<Function>>,
    pub structs: Vec<RefCell<StructDefinition>>,
    // pub constants: Vec<Constant>,
}

/// A simplified Move Script.
/// The script only contains a `main` function.
/// The `main` function only consists of a sequence of function calls.
#[derive(Debug, Clone)]
pub struct Script {
    pub main: Vec<FunctionCall>,
}

/// A function definition.
/// The return statement is separated from the body to simplify verifying the
/// generated function has a valid return.
#[derive(Debug, Clone)]
pub struct Function {
    pub visibility: Visibility,
    pub signature: FunctionSignature,
    // pub inline: bool,
    pub body: Option<Block>,
}

/// The Visibility
#[derive(Debug, Clone)]
pub struct Visibility {
    pub public: bool,
}

/// A function signature.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub type_parameters: TypeParameters,
    pub name: Identifier,
    pub parameters: Vec<(Identifier, Type)>,
    pub return_type: Option<Type>,
}

/// An expression block
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Statement>,
    pub return_expr: Option<Expression>,
}

/// The definition of a struct.
/// Cyclic data is not allowed.
/// Struct used in fields must have the all the abilities of the parent struct.
#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub name: Identifier,
    pub abilities: Vec<Ability>,
    pub type_parameters: TypeParameters,
    pub fields: Vec<(Identifier, Type)>,
}

impl HasType for StructDefinition {
    fn get_type(&self) -> Type {
        Type::new_struct(&self.name, Some(&self.type_parameters))
    }
}

/// A statement in a function body.
#[derive(Debug, Clone)]
pub enum Statement {
    // While(While),
    // For(For),
    // Break,
    // Continue,
    Decl(Declaration),
    Expr(Expression),
}

/// An inline struct initialization.
#[derive(Debug, Clone)]
pub struct StructPack {
    pub name: Identifier,
    pub type_args: TypeArgs,
    pub fields: Vec<(Identifier, Expression)>,
}

impl HasType for StructPack {
    fn get_type(&self) -> Type {
        let name = format!("{}{}", self.name.inline(), self.type_args.inline());
        let kind = IDKind::StructConcrete;
        Type::new_concrete_struct(&Identifier::new(name, kind), Some(&self.type_args))
    }
}

/// Declare a new variable.
/// Optionally initialize the variable with an expression.
/// Currently type annotations will always be generated.
// TODO: Support multiple declarations in a single statement
// TODO: Randomly ignore type annotation
#[derive(Debug, Clone)]
pub struct Declaration {
    pub typ: Type,
    pub name: Identifier,
    pub value: Option<Expression>,
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expression {
    NumberLiteral(NumberLiteral),
    Variable(VariableAccess),
    Boolean(bool),
    FunctionCall(FunctionCall),
    StructPack(StructPack),
    Block(Box<Block>),
    Assign(Box<Assignment>),
    BinaryOperation(Box<BinaryOperation>),
    IfElse(Box<IfExpr>),
}

/// Represents a variable access
#[derive(Debug, Clone)]
pub struct VariableAccess {
    pub name: Identifier,
    pub copy: bool,
}

// If Expression
#[derive(Debug, Clone)]
pub struct IfExpr {
    pub condition: Expression,
    pub body: Block,
    pub else_expr: Option<ElseExpr>,
}

// Else Expression
// Should only be contained in an IfExpr
#[derive(Debug, Clone)]
pub struct ElseExpr {
    pub typ: Option<Type>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct BinaryOperation {
    pub op: BinaryOperator,
    pub lhs: Expression,
    pub rhs: Expression,
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Numerical(NumericalBinaryOperator),
    Boolean(BooleanBinaryOperator),
    Equality(EqualityBinaryOperator),
}

#[derive(Debug, Clone, Arbitrary)]
pub enum NumericalBinaryOperator {
    Add,
    Sub,
    Mul,
    Mod,
    Div,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Le,
    Ge,
    Leq,
    Geq,
}

#[derive(Debug, Clone, Arbitrary)]
pub enum BooleanBinaryOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Arbitrary)]
pub enum EqualityBinaryOperator {
    Eq,
    Neq,
}

/// An assignment expression
#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: Identifier,
    pub value: Expression,
}

/// A number literal.
/// Currently the number literal will always have the type suffix.
#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub value: BigUint,
    pub typ: Type,
}

/// A function call.
/// Currently the generated doesn't allow the argument to be another function call.
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: Identifier,
    pub type_args: TypeArgs,
    pub args: Vec<Expression>,
}
