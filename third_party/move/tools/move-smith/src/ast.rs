// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use num_bigint::BigUint;

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
}

#[derive(Debug)]
pub struct IdentifierPool {
    var_count: u32,
    struct_count: u32,
    function_count: u32,
    module_count: u32,
    script_count: u32,
    constant_count: u32,
}

pub enum IdentifierType {
    Var,
    Struct,
    Function,
    Module,
    Script,
    Constant,
}

impl IdentifierPool {
    pub fn new() -> Self {
        Self {
            var_count: 0,
            struct_count: 0,
            function_count: 0,
            module_count: 0,
            script_count: 0,
            constant_count: 0,
        }
    }

    pub fn next_identifier(&mut self, typ: IdentifierType) -> Identifier {
        use IdentifierType as T;
        let name = match typ {
            T::Var => {
                self.var_count += 1;
                format!("var{}", self.var_count)
            },
            T::Struct => {
                self.struct_count += 1;
                format!("Struct{}", self.struct_count)
            },
            T::Function => {
                self.function_count += 1;
                format!("function{}", self.function_count)
            },
            T::Module => {
                self.module_count += 1;
                format!("Module{}", self.module_count)
            },
            T::Script => {
                self.script_count += 1;
                format!("Script{}", self.script_count)
            },
            T::Constant => {
                self.constant_count += 1;
                format!("CONST{}", self.constant_count)
            },
        };
        Identifier { name }
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    // pub attributes: Vec<Attributes>,
    // pub address: Option<LeadingNameAccess>,
    pub name: Identifier,
    // pub is_spec_module: bool,
    pub members: Vec<ModuleMember>,
}

#[derive(Debug, Clone)]
pub enum ModuleMember {
    Function(Function),
    // Struct(StructDefinition),
    // Use(UseDecl),
    // Friend(FriendDecl),
    // Constant(Constant),
    // Spec(SpecBlock),
}

#[derive(Debug, Clone)]
pub struct Function {
    // pub attributes: Vec<Attributes>,
    // pub visibility: Visibility,
    // pub signature: FunctionSignature,
    /// `None` indicates no specifiers given, `Some([])` indicates the `pure` keyword has been
    /// used.
    // pub access_specifiers: Option<Vec<AccessSpecifier>>,
    pub name: Identifier,
    // pub inline: bool,
    pub body: FunctionBody,
}

#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    // Return(Option<Expression>),
    // If(If),
    // While(While),
    // For(For),
    // Break,
    // Continue,
    // Assign(Assign),
    // Decl(Decl),
    Expr(Expression),
}

#[derive(Debug, Clone)]
pub enum Expression {
    NumberLiteral(NumberLiteral),
}

#[derive(Debug, Clone)]
pub struct NumberLiteral {
    pub value: BigUint,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Type {
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
}
