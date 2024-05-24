// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ast::*, config::Config};
use arbitrary::{Arbitrary, Result, Unstructured};
use num_bigint::BigUint;

pub struct MoveSmith {
    pub config: Config,

    // The output code
    pub modules: Vec<Module>,

    // Bookkeeping
    pub id_pool: IdentifierPool,
}

impl Default for MoveSmith {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl MoveSmith {
    pub fn new(config: Config) -> Self {
        Self {
            modules: Vec::new(),
            config,
            id_pool: IdentifierPool::new(),
        }
    }

    pub fn generate_module(&mut self, u: &mut Unstructured) -> Result<Module> {
        let len = u.int_in_range(0..=self.config.max_members_in_module)?;
        let mut members = Vec::new();
        for _ in 0..len {
            members.push(self.generate_module_member(u)?);
        }
        Ok(Module {
            name: self.id_pool.next_identifier(IdentifierType::Module),
            members,
        })
    }

    fn generate_module_member(&mut self, u: &mut Unstructured) -> Result<ModuleMember> {
        match u.int_in_range(0..=0).unwrap() {
            0 => Ok(ModuleMember::Function(self.generate_function(u)?)),
            _ => panic!("Invalid module member type"),
        }
    }

    fn generate_function(&mut self, u: &mut Unstructured) -> Result<Function> {
        Ok(Function {
            name: self.id_pool.next_identifier(IdentifierType::Function),
            body: self.generate_function_body(u)?,
        })
    }

    fn generate_function_body(&mut self, u: &mut Unstructured) -> Result<FunctionBody> {
        let len = u.int_in_range(0..=self.config.max_stmt_in_func)?;
        let mut stmts = Vec::new();
        for _ in 0..len {
            stmts.push(self.generate_statement(u)?);
        }
        Ok(FunctionBody { stmts })
    }

    fn generate_statement(&mut self, u: &mut Unstructured) -> Result<Statement> {
        match u.int_in_range(0..=0)? {
            0 => Ok(Statement::Expr(self.generate_expression(u)?)),
            _ => panic!("Invalid statement type"),
        }
    }

    fn generate_expression(&mut self, u: &mut Unstructured) -> Result<Expression> {
        match u.int_in_range(0..=0)? {
            0 => Ok(Expression::NumberLiteral(self.generate_number_literal(u)?)),
            _ => panic!("Invalid expression type"),
        }
    }

    fn generate_number_literal(&mut self, u: &mut Unstructured) -> Result<NumberLiteral> {
        let typ = self.generate_type(u)?;
        let value = match typ {
            Type::U8 => BigUint::from(u8::arbitrary(u)?),
            Type::U16 => BigUint::from(u16::arbitrary(u)?),
            Type::U32 => BigUint::from(u32::arbitrary(u)?),
            Type::U64 => BigUint::from(u64::arbitrary(u)?),
            Type::U128 => BigUint::from(u128::arbitrary(u)?),
            Type::U256 => BigUint::from_bytes_be(u.bytes(32)?),
        };
        Ok(NumberLiteral { value, typ })
    }

    fn generate_type(&mut self, u: &mut Unstructured) -> Result<Type> {
        match u.int_in_range(0..=5)? {
            0 => Ok(Type::U8),
            1 => Ok(Type::U16),
            2 => Ok(Type::U32),
            3 => Ok(Type::U64),
            4 => Ok(Type::U128),
            5 => Ok(Type::U256),
            _ => panic!("Invalid type"),
        }
    }
}
