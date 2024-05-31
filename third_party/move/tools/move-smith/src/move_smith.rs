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
    pub script: Option<Script>,

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
            script: None,
            config,
            id_pool: IdentifierPool::new(),
            type_pool: TypePool::new(),
        }
    }

    pub fn get_compile_unit(&self) -> CompileUnit {
        CompileUnit {
            modules: self.modules.clone(),
            scripts: match &self.script {
                Some(s) => vec![s.clone()],
                None => Vec::new(),
            },
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

        self.generate_script(u)?;
        Ok(())
    }

    pub fn generate_script(&mut self, u: &mut Unstructured) -> Result<()> {
        let mut script = Script { main: Vec::new() };

        let all_funcs = self
            .modules
            .iter()
            .flat_map(|m| m.functions.iter().cloned())
            .collect::<Vec<Function>>();

        for _ in 0..u.int_in_range(1..=self.config.max_num_calls_in_script)? {
            let func = u.choose(&all_funcs)?;
            let mut call = self.generate_call_to_function(u, &None, func, false)?;
            call.name = self.id_pool.flatten_access(&call.name).unwrap();
            script.main.push(call);
        }

        self.script = Some(script);
        Ok(())
    }

    pub fn generate_module_skeleton(&mut self, u: &mut Unstructured) -> Result<Module> {
        let (name, scope) = self
            .id_pool
            .next_identifier(IdentifierType::Module, &Some("0xCAFE".to_string()));

        // Struct names
        let mut structs = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.max_num_structs_in_module)? {
            structs.push(self.generate_struct_skeleton(u, &scope)?);
        }

        // Function signatures
        let mut functions = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.max_num_functions_in_module)? {
            functions.push(self.generate_function_skeleton(u, &scope)?);
        }

        Ok(Module {
            name,
            functions,
            structs,
        })
    }

    pub fn fill_module(&mut self, u: &mut Unstructured, mut module: Module) -> Result<Module> {
        let scope = self.id_pool.get_scope_for_children(&module.name);
        // Struct fields
        for s in module.structs.iter_mut() {
            self.fill_struct(u, s, &scope)?;
        }

        // Function bodies
        for f in module.functions.iter_mut() {
            self.fill_function(u, f)?;
        }

        Ok(module)
    }

    fn generate_struct_skeleton(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<StructDefinition> {
        let (name, _) = self
            .id_pool
            .next_identifier(IdentifierType::Struct, parent_scope);

        let mut ability_choices = vec![Ability::Copy, Ability::Drop, Ability::Store, Ability::Key];
        let mut abilities = Vec::new();
        for _ in 0..u.int_in_range(0..=3)? {
            let idx = u.int_in_range(0..=(ability_choices.len() - 1))?;
            abilities.push(ability_choices.remove(idx));
        }

        self.type_pool.register_type(Type::Struct(name.clone()));
        Ok(StructDefinition {
            name,
            abilities,
            fields: Vec::new(),
        })
    }

    fn fill_struct(
        &mut self,
        u: &mut Unstructured,
        st: &mut StructDefinition,
        parent_scope: &Scope,
    ) -> Result<()> {
        for _ in 0..u.int_in_range(0..=self.config.max_num_fields_in_struct)? {
            let (name, _) = self
                .id_pool
                .next_identifier(IdentifierType::Var, &Some(st.name.clone()));

            let typ = loop {
                match u.int_in_range(0..=2)? {
                    0 | 1 => break self.type_pool.random_basic_type(u)?,
                    2 => {
                        let candidates = self.get_usable_struct_type(
                            st.abilities.clone(),
                            parent_scope,
                            &st.name,
                        );
                        if !candidates.is_empty() {
                            break Type::Struct(u.choose(&candidates)?.name.clone());
                        }
                    },
                    _ => panic!("Invalid type"),
                }
            };
            self.type_pool.insert_mapping(&name, &typ);
            st.fields.push((name, typ));
        }
        Ok(())
    }

    /// Return all struct definitions that:
    /// * with in the same module (TODO: allow cross module reference)
    /// * have the desired abilities
    /// * if key is in desired abilities, the struct must have store ability
    /// * does not create loop in the struct hierarchy
    fn get_usable_struct_type(
        &self,
        desired: Vec<Ability>,
        scope: &Scope,
        parent_struct_id: &Identifier,
    ) -> Vec<StructDefinition> {
        let ids = self.get_filtered_identifiers(None, Some(IdentifierType::Struct), Some(scope));
        ids.iter()
            .filter_map(|s| {
                let struct_def = self.get_struct_definition_with_identifier(s).unwrap();
                if !desired.iter().all(|a| struct_def.abilities.contains(a)) {
                    return None;
                }
                if desired.contains(&Ability::Key)
                    && !struct_def.abilities.contains(&Ability::Store)
                {
                    return None;
                }
                if self.check_struct_reachable(&struct_def.name, parent_struct_id) {
                    return None;
                }
                Some(struct_def)
            })
            .collect()
    }

    fn check_struct_reachable(&self, source: &Identifier, sink: &Identifier) -> bool {
        if source == sink {
            return true;
        }
        let source_struct = self.get_struct_definition_with_identifier(source).unwrap();
        for (_, typ) in source_struct.fields.iter() {
            let name = match typ {
                Type::Struct(id) => id,
                _ => continue,
            };
            if name == sink {
                return true;
            }
            if self.check_struct_reachable(name, sink) {
                return true;
            }
        }
        false
    }

    fn get_struct_definition_with_identifier(&self, id: &Identifier) -> Option<StructDefinition> {
        self.modules
            .iter()
            .find_map(|m| m.structs.iter().find(|s| &s.name == id).cloned())
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
            visibility: Visibility { public: true },
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
            self.type_pool.insert_mapping(&name, &typ);
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
                        let expr =
                            self.generate_expression_of_type(u, parent_scope, typ, true, true)?;
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
        //     true => Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?),
        //     false => None,
        // };
        // TODO: disabled declaration without value for now, need to keep track of initialization
        let value = Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?);
        self.type_pool.insert_mapping(&name, &typ);
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
        allow_var: bool,
        allow_call: bool,
    ) -> Result<Expression> {
        // Store candidate expressions for the given type
        let mut choices: Vec<Expression> = Vec::new();

        // Directly generate a value for basic types
        let candidate = match typ {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 => {
                Expression::NumberLiteral(self.generate_number_literal(
                    u,
                    parent_scope,
                    Some(typ),
                )?)
            },
            Type::Bool => Expression::Boolean(bool::arbitrary(u)?),
            Type::Struct(id) => self.generate_struct_initialization(u, parent_scope, id)?,
            _ => unimplemented!(),
        };
        choices.push(candidate);

        // Access identifier with the given type
        if allow_var {
            let idents = self.get_filtered_identifiers(Some(typ), None, Some(parent_scope));

            // TODO: select from many?
            if !idents.is_empty() {
                let candidate = u.choose(&idents)?.clone();
                choices.push(Expression::Variable(candidate));
            }
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
                let call = self.generate_call_to_function(u, parent_scope, func, true)?;
                choices.push(Expression::FunctionCall(call));
            }
        }

        Ok(u.choose(&choices)?.clone())
    }

    fn generate_struct_initialization(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        struct_name: &Identifier,
    ) -> Result<Expression> {
        let struct_def = self
            .get_struct_definition_with_identifier(struct_name)
            .unwrap();

        let mut fields = Vec::new();
        for (name, typ) in struct_def.fields.iter() {
            let expr = self.generate_expression_of_type(u, parent_scope, typ, true, true)?;
            fields.push((name.clone(), expr));
        }
        Ok(Expression::StructInitialization(StructInitialization {
            name: struct_name.clone(),
            fields,
        }))
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
            true,
        )?))
    }

    fn generate_call_to_function(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        func: &Function,
        allow_var: bool,
    ) -> Result<FunctionCall> {
        let mut args = Vec::new();

        for (_, typ) in func.signature.parameters.iter() {
            let expr = self.generate_expression_of_type(u, parent_scope, typ, allow_var, false)?;
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
