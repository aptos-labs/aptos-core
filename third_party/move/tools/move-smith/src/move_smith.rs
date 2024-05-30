// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::*,
    config::Config,
    names::{is_in_scope, Identifier, IdentifierPool, IdentifierType, Scope},
    types::{Type, TypePool},
};
use arbitrary::{Arbitrary, Result, Unstructured};
use num_bigint::BigUint;

pub struct MoveSmith {
    pub config: Config,

    // The output code
    pub modules: Vec<Module>,

    // Bookkeeping
    pub id_pool: IdentifierPool,
    pub type_pool: TypePool,
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
            type_pool: TypePool::new(),
        }
    }

    pub fn get_compile_unit(&self) -> CompileUnit {
        CompileUnit {
            modules: self.modules.clone(),
        }
    }

    pub fn generate(&mut self, u: &mut Unstructured) -> Result<()> {
        let num_modules = u.int_in_range(1..=self.config.max_num_modules)?;

        let mut modules = Vec::new();
        for _ in 0..num_modules {
            modules.push(self.generate_module_skeleton(u)?);
        }
        self.modules = modules;

        let filled_modules = self
            .modules
            .clone()
            .into_iter()
            .map(|m| self.fill_module(u, m))
            .collect::<Result<Vec<Module>>>()?;
        self.modules = filled_modules;
        Ok(())
    }

    pub fn generate_module_skeleton(&mut self, u: &mut Unstructured) -> Result<Module> {
        let (name, scope) = self.id_pool.next_identifier(IdentifierType::Module, &None);

        // Function signatures
        let mut functions = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.max_num_functions_in_module)? {
            functions.push(self.generate_function_skeleton(u, &scope)?);
        }
        Ok(Module { name, functions })
    }

    pub fn fill_module(&mut self, u: &mut Unstructured, mut module: Module) -> Result<Module> {
        // Function bodies
        for f in module.functions.iter_mut() {
            self.fill_function(u, f)?;
        }

        Ok(module)
    }

    fn generate_function_skeleton(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Function> {
        let (name, scope) = self
            .id_pool
            .next_identifier(IdentifierType::Function, parent_scope);
        let signature = self.generate_function_signature(u, &scope)?;

        Ok(Function {
            signature,
            name,
            body: None,
            return_stmt: None,
        })
    }

    fn fill_function(&mut self, u: &mut Unstructured, function: &mut Function) -> Result<()> {
        let scope = self.id_pool.get_scope_for_children(&function.name);
        function.body = Some(self.generate_function_body(u, &scope)?);
        function.return_stmt = self.generate_return_stmt(u, &scope, &function.signature)?;
        Ok(())
    }

    fn generate_function_signature(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<FunctionSignature> {
        let num_params = u.int_in_range(0..=self.config.max_num_params_in_func)?;
        let mut parameters = Vec::new();
        for _ in 0..num_params {
            let (name, _) = self
                .id_pool
                .next_identifier(IdentifierType::Var, parent_scope);

            let typ = self.type_pool.random_basic_type(u)?;
            self.type_pool.insert(&name, &typ);
            parameters.push((name, typ));
        }

        let return_type = match bool::arbitrary(u)? {
            true => Some(self.type_pool.random_basic_type(u)?),
            false => None,
        };

        Ok(FunctionSignature {
            parameters,
            return_type,
        })
    }

    fn generate_return_stmt(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        signature: &FunctionSignature,
    ) -> Result<Option<Expression>> {
        match signature.return_type {
            Some(ref typ) => {
                let ids = self.get_filtered_identifiers(
                    Some(typ),
                    Some(IdentifierType::Var),
                    Some(parent_scope),
                );
                match ids.is_empty() {
                    true => {
                        let expr = self.generate_expression_of_type(u, parent_scope, typ, true)?;
                        Ok(Some(expr))
                    },
                    false => {
                        let ident = u.choose(&ids)?.clone();
                        Ok(Some(Expression::Variable(ident)))
                    },
                }
            },
            None => Ok(None),
        }
    }

    fn generate_function_body(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<FunctionBody> {
        let len = u.int_in_range(0..=self.config.max_num_stmt_in_func)?;
        let mut stmts = Vec::new();

        for _ in 0..len {
            stmts.push(self.generate_statement(u, parent_scope)?);
        }

        Ok(FunctionBody { stmts })
    }

    fn generate_statement(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Statement> {
        match u.int_in_range(0..=1)? {
            0 => Ok(Statement::Decl(self.generate_decalration(u, parent_scope)?)),
            1 => Ok(Statement::Expr(self.generate_expression(u, parent_scope)?)),
            _ => panic!("Invalid statement type"),
        }
    }

    fn generate_decalration(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Declaration> {
        let (name, _) = self
            .id_pool
            .next_identifier(IdentifierType::Var, parent_scope);

        let typ = self.type_pool.random_basic_type(u)?;
        // let value = match bool::arbitrary(u)? {
        //     true => Some(self.generate_expression_of_type(u, parent_scope, &typ, true)?),
        //     false => None,
        // };
        // TODO: disabled declaration without value for now, need to keep track of initialization
        let value = Some(self.generate_expression_of_type(u, parent_scope, &typ, true)?);
        self.type_pool.insert(&name, &typ);
        Ok(Declaration { typ, name, value })
    }

    fn generate_expression(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Expression> {
        let callable = self.get_callable_functions(parent_scope);
        let max = if callable.is_empty() { 1 } else { 2 };
        let expr = loop {
            match u.int_in_range(0..=max)? {
                0 => {
                    break Expression::NumberLiteral(self.generate_number_literal(
                        u,
                        parent_scope,
                        None,
                    )?)
                },
                1 => {
                    let idents = self.get_filtered_identifiers(
                        None,
                        Some(IdentifierType::Var),
                        Some(parent_scope),
                    );
                    if !idents.is_empty() {
                        let ident = u.choose(&idents)?.clone();
                        break Expression::Variable(ident);
                    }
                },
                2 => {
                    let call = self.generate_function_call(u, parent_scope)?;
                    match call {
                        Some(c) => break Expression::FunctionCall(c),
                        None => panic!("No callable functions"),
                    }
                },
                _ => panic!("Invalid expression type"),
            }
        };
        Ok(expr)
    }

    fn generate_expression_of_type(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: &Type,
        allow_call: bool,
    ) -> Result<Expression> {
        // Store candidate expressions for the given type
        let mut choices: Vec<Expression> = Vec::new();

        // Directly generate a value for basic types
        if typ.is_basic_type() {
            let candidate = match typ {
                Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 => {
                    Expression::NumberLiteral(self.generate_number_literal(
                        u,
                        parent_scope,
                        Some(typ),
                    )?)
                },
                Type::Bool => Expression::Boolean(bool::arbitrary(u)?),
                _ => unimplemented!(),
            };
            choices.push(candidate);
        }

        // Access identifier with the given type
        let idents = self.get_filtered_identifiers(Some(typ), None, Some(parent_scope));

        // TODO: select from many?
        if !idents.is_empty() {
            let candidate = u.choose(&idents)?.clone();
            choices.push(Expression::Variable(candidate));
        }

        // TODO: call functions with the given type
        if allow_call {
            let callables: Vec<Function> = self
                .get_callable_functions(parent_scope)
                .into_iter()
                .filter(|f| f.signature.return_type == Some(typ.clone()))
                .collect();
            if !callables.is_empty() {
                let func = u.choose(&callables)?;
                let call = self.generate_call_to_function(u, parent_scope, func)?;
                choices.push(Expression::FunctionCall(call));
            }
        }

        Ok(u.choose(&choices)?.clone())
    }

    fn generate_function_call(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Option<FunctionCall>> {
        let callables = self.get_callable_functions(parent_scope);
        if callables.is_empty() {
            return Ok(None);
        }

        let func = u.choose(&callables)?.clone();
        Ok(Some(self.generate_call_to_function(
            u,
            parent_scope,
            &func,
        )?))
    }

    fn generate_call_to_function(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        func: &Function,
    ) -> Result<FunctionCall> {
        let mut args = Vec::new();

        for (_, typ) in func.signature.parameters.iter() {
            let expr = self.generate_expression_of_type(u, parent_scope, typ, false)?;
            args.push(expr);
        }
        Ok(FunctionCall {
            name: func.name.clone(),
            args,
        })
    }

    /// Generate a random numerical literal.
    /// If the `typ` is `None`, a random type will be chosen.
    /// If the `typ` is `Some(Type::{U8, ..., U256})`, a literal of the given type will be used.
    fn generate_number_literal(
        &mut self,
        u: &mut Unstructured,
        _parent_scope: &Scope,
        typ: Option<&Type>,
    ) -> Result<NumberLiteral> {
        let idx = match typ {
            Some(t) => match t {
                Type::U8 => 0,
                Type::U16 => 1,
                Type::U32 => 2,
                Type::U64 => 3,
                Type::U128 => 4,
                Type::U256 => 5,
                _ => panic!("Invalid number literal type"),
            },
            None => u.int_in_range(0..=5)?,
        };

        Ok(match idx {
            0 => NumberLiteral {
                value: BigUint::from(u8::arbitrary(u)?),
                typ: Type::U8,
            },
            1 => NumberLiteral {
                value: BigUint::from(u16::arbitrary(u)?),
                typ: Type::U16,
            },
            2 => NumberLiteral {
                value: BigUint::from(u32::arbitrary(u)?),
                typ: Type::U32,
            },
            3 => NumberLiteral {
                value: BigUint::from(u64::arbitrary(u)?),
                typ: Type::U64,
            },
            4 => NumberLiteral {
                value: BigUint::from(u128::arbitrary(u)?),
                typ: Type::U128,
            },
            5 => NumberLiteral {
                value: BigUint::from_bytes_be(u.bytes(32)?),
                typ: Type::U256,
            },
            _ => panic!("Invalid number literal type"),
        })
    }

    // TODO: Handle visibility check
    fn get_callable_functions(&self, scope: &Scope) -> Vec<Function> {
        let mut funcs = Vec::new();
        for m in self.modules.iter() {
            for f in m.functions.iter() {
                let parent_scope = self.id_pool.get_parent_scope_of(&f.name).unwrap();
                if is_in_scope(scope, &parent_scope) {
                    funcs.push(f.clone());
                }
            }
        }
        funcs
    }

    fn get_filtered_identifiers(
        &self,
        typ: Option<&Type>,
        ident_type: Option<IdentifierType>,
        scope: Option<&Scope>,
    ) -> Vec<Identifier> {
        // Filter based on the IdentifierType
        let all_ident = match ident_type {
            Some(t) => self.id_pool.get_identifiers_of_ident_type(t),
            None => self.id_pool.get_all_identifiers(),
        };

        // Filter based on Scope
        let ident_in_scope = match scope {
            Some(s) => self.id_pool.filter_identifier_in_scope(&all_ident, s),
            None => all_ident,
        };

        // Filter based on Type
        match typ {
            Some(t) => self
                .type_pool
                .filter_identifier_with_type(t, ident_in_scope),
            None => ident_in_scope,
        }
    }
}
