// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{names::Identifier, types::Type};
use arbitrary::Arbitrary;
use num_bigint::BigUint;

#[derive(Debug, Clone)]
pub struct CompileUnit {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone)]
pub struct Module {
    // pub attributes: Vec<Attributes>,
    // pub address: Option<LeadingNameAccess>,
    pub name: Identifier,
    // pub is_spec_module: bool,
    pub functions: Vec<Function>,
    pub structs: Vec<StructDefinition>,
    // pub uses: Vec<UseDecl>,
    // pub friends: Vec<FriendDecl>,
    // pub constants: Vec<Constant>,
    // pub specs: Vec<SpecBlock>,
}

#[derive(Debug, Clone)]
pub struct Function {
    // pub attributes: Vec<Attributes>,
    // pub visibility: Visibility,
    pub signature: FunctionSignature,
    /// `None` indicates no specifiers given, `Some([])` indicates the `pure` keyword has been
    /// used.
    // pub access_specifiers: Option<Vec<AccessSpecifier>>,
    pub name: Identifier,
    // pub inline: bool,
    pub body: Option<FunctionBody>,

    pub return_stmt: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    // pub type_parameters: Vec<(Name, Vec<Ability>)>,
    pub parameters: Vec<(Identifier, Type)>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct StructDefinition {
    pub name: Identifier,
    // pub attributes: Vec<Attributes>,
    pub abilities: Vec<Ability>,
    // pub type_parameters: Vec<StructTypeParameter>,
    pub fields: Vec<(Identifier, Type)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Arbitrary)]
pub enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

#[derive(Debug, Clone)]
pub enum Statement {
    // If(If),
    // While(While),
    // For(For),
    // Break,
    // Continue,
    // Assign(Assign),
    Decl(Declaration),
    Expr(Expression),
}

// TODO: Support multiple declarations in a single statement
#[derive(Debug, Clone)]
pub struct Declaration {
    pub typ: Type,
    pub name: Identifier,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    NumberLiteral(NumberLiteral),
    Variable(Identifier),
    Boolean(bool),
    FunctionCall(FunctionCall),
    StructInitialization(StructInitialization),
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub value: BigUint,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct StructInitialization {
    pub name: Identifier,
    pub fields: Vec<(Identifier, Expression)>,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: Identifier,
    pub args: Vec<Expression>,
}
