// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This is the core generation logic for MoveSmith.
//! Each MoveSmith instance can generates a single Move program consisting of
//! multiple modules and a script.
//! Each generated unit should be runnable as a transactional test.
//! The generation is deterministic. Using the same input Unstructured byte
//! sequence would lead to the same output.
//!
//! The generation for modules is divided into two phases:
//! 1. Generate the skeleton of several elements so that they can be referenced later.
//!     - Generate module names
//!     - Generate struct names and abilities
//!     - Generate function names and signatures
//! 2. Fill in the details of the generated elements.
//!     - Fill in struct fields
//!     - Fill in function bodies

use crate::{
    ast::*,
    config::Config,
    names::{Identifier, IdentifierPool, IdentifierType as IDType, Scope, ROOT_SCOPE},
    types::{Type, TypePool},
    utils::choose_idx_weighted,
};
use arbitrary::{Arbitrary, Result, Unstructured};
use num_bigint::BigUint;
use std::cell::RefCell;

/// Keeps track of the generation state.
pub struct MoveSmith {
    pub config: Config,

    // The output code
    modules: Vec<RefCell<Module>>,
    script: Option<Script>,
    runs: RefCell<Vec<Identifier>>,

    // Skeleton Information
    function_signatures: Vec<FunctionSignature>,

    // Bookkeeping
    id_pool: RefCell<IdentifierPool>,
    type_pool: RefCell<TypePool>,
    expr_depth: RefCell<usize>,
}

impl Default for MoveSmith {
    /// Create a new MoveSmith instance with default configuration.
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl MoveSmith {
    /// Create a new MoveSmith instance with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            modules: Vec::new(),
            script: None,
            runs: RefCell::new(Vec::new()),
            function_signatures: Vec::new(),
            id_pool: RefCell::new(IdentifierPool::new()),
            type_pool: RefCell::new(TypePool::new()),
            expr_depth: RefCell::new(0),
        }
    }

    /// Get the generated compile unit.
    pub fn get_compile_unit(&self) -> CompileUnit {
        let modules = self
            .modules
            .iter()
            .map(|m| m.borrow().clone())
            .collect::<Vec<Module>>();
        let runs = self.runs.borrow().clone();
        CompileUnit {
            modules,
            scripts: match &self.script {
                Some(s) => vec![s.clone()],
                None => Vec::new(),
            },
            runs,
        }
    }

    /// Generate a Move program consisting of multiple modules and a script.
    /// Consumes the given Unstructured instance to guide the generation.
    ///
    /// Script is generated after all modules are generated so that the script can call functions.
    pub fn generate(&mut self, u: &mut Unstructured) -> Result<()> {
        let num_modules = u.int_in_range(1..=self.config.max_num_modules)?;

        for _ in 0..num_modules {
            self.modules
                .push(RefCell::new(self.generate_module_skeleton(u)?));
        }

        for m in self.modules.iter() {
            self.fill_module(u, m)?;
        }

        // Disable script generation for now since intermediate states are not compared
        self.script = None;

        Ok(())
    }

    /// Generate a script that calls functions from the generated modules.
    #[allow(dead_code)]
    fn generate_script(&self, u: &mut Unstructured) -> Result<Script> {
        let mut script = Script { main: Vec::new() };

        let mut all_funcs: Vec<RefCell<Function>> = Vec::new();
        for m in self.modules.iter() {
            for f in m.borrow().functions.iter() {
                all_funcs.push(f.clone());
            }
        }

        for _ in 0..u.int_in_range(1..=self.config.max_num_calls_in_script)? {
            let func = u.choose(&all_funcs)?;
            let mut call =
                self.generate_call_to_function(u, &ROOT_SCOPE, &func.borrow().signature, false)?;
            call.name = self.id_pool.borrow().flatten_access(&call.name);
            script.main.push(call);
        }

        Ok(script)
    }

    /// Generate a module skeleton with only struct and function skeletions.
    fn generate_module_skeleton(&self, u: &mut Unstructured) -> Result<Module> {
        let hardcoded_address = Scope(Some("0xCAFE".to_string()));
        let (name, scope) = self.get_next_identifier(IDType::Module, &hardcoded_address);

        // Struct names
        let mut structs = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.max_num_structs_in_module)? {
            structs.push(RefCell::new(self.generate_struct_skeleton(u, &scope)?));
        }

        // Function signatures
        let mut functions = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.max_num_functions_in_module)? {
            functions.push(RefCell::new(self.generate_function_skeleton(u, &scope)?));
        }

        Ok(Module {
            name,
            functions,
            structs,
        })
    }

    /// Fill in the skeletons
    fn fill_module(&self, u: &mut Unstructured, module: &RefCell<Module>) -> Result<()> {
        let scope = self
            .id_pool
            .borrow()
            .get_scope_for_children(&module.borrow().name);
        // Struct fields
        for s in module.borrow().structs.iter() {
            self.fill_struct(u, s, &scope)?;
        }

        // Generate function bodies and runners
        let mut all_runners = Vec::new();
        for f in module.borrow().functions.iter() {
            self.fill_function(u, f)?;
            all_runners.extend(self.generate_runners(u, f)?);
        }

        // Insert the runners to the module and add run tasks to the whole compile unit
        // Each task is simply the flat name of the runner function
        for r in all_runners.into_iter() {
            let module_flat = self.id_pool.borrow().flatten_access(&module.borrow().name);
            let run_flat = Identifier(format!("{}::{}", module_flat.0, r.signature.name.0));
            self.runs.borrow_mut().push(run_flat);
            module.borrow_mut().functions.push(RefCell::new(r));
        }

        Ok(())
    }

    /// Generate a runner function for a callee function.
    /// The runner function does not have parameters so that
    /// it can be easily called with `//# run`.
    /// The runner function only contains one function call and have the same return type as the callee.
    // TODO: this is hacky just to have a way for comparing return results, should be improved
    fn generate_runners(
        &self,
        u: &mut Unstructured,
        callee: &RefCell<Function>,
    ) -> Result<Vec<Function>> {
        let signature = callee.borrow().signature.clone();

        let mut runners = Vec::new();
        for i in 0..self.config.num_runs_per_func {
            // Generate a call to the target function
            let call = Expression::FunctionCall(self.generate_call_to_function(
                u,
                &ROOT_SCOPE,
                &signature,
                false,
            )?);

            // Generate a body with only one statement/return expr
            let body = match signature.return_type.is_none() {
                true => Block {
                    stmts: vec![Statement::Expr(call)],
                    return_expr: None,
                },
                false => Block {
                    stmts: Vec::new(),
                    return_expr: Some(call),
                },
            };

            // Use a special name for the runner function
            // These names are not properly stored in the id_pool so they
            // should not be used elsewhere other than with `//# run`
            let runner = Function {
                signature: FunctionSignature {
                    name: Identifier(format!("{}_runner_{}", signature.name.0, i)),
                    parameters: Vec::new(),
                    return_type: signature.return_type.clone(),
                },
                visibility: Visibility { public: true },
                body: Some(body),
            };
            runners.push(runner);
        }
        Ok(runners)
    }

    // Generate a struct skeleton with name and random abilities.
    fn generate_struct_skeleton(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<StructDefinition> {
        let (name, _) = self.get_next_identifier(IDType::Struct, parent_scope);

        let mut ability_choices = vec![Ability::Store, Ability::Key];
        // TODO: Drop is added for all struct to avoid E05001 for now
        // TODO: this should be properly handled
        // TODO: Copy is added to avoid "use moved value"
        // TODO: Copy should be removed after copy/move is properly handled
        let mut abilities = vec![Ability::Drop, Ability::Copy];
        for _ in 0..u.int_in_range(0..=0)? {
            let idx = u.int_in_range(0..=(ability_choices.len() - 1))?;
            abilities.push(ability_choices.remove(idx));
        }

        self.type_pool
            .borrow_mut()
            .register_type(Type::Struct(name.clone()));
        Ok(StructDefinition {
            name,
            abilities,
            fields: Vec::new(),
        })
    }

    /// Fill in the struct fields with random types.
    fn fill_struct(
        &self,
        u: &mut Unstructured,
        st: &RefCell<StructDefinition>,
        parent_scope: &Scope,
    ) -> Result<()> {
        let struct_scope = st.borrow().name.to_scope();
        for _ in 0..u.int_in_range(0..=self.config.max_num_fields_in_struct)? {
            let (name, _) = self.get_next_identifier(IDType::Var, &struct_scope);

            let typ = loop {
                match u.int_in_range(0..=2)? {
                    // More chance to use basic types than struct types
                    0 | 1 => break self.get_random_type(u, parent_scope, true, false)?,
                    2 => {
                        let candidates = self.get_usable_struct_type(
                            st.borrow().abilities.clone(),
                            parent_scope,
                            &st.borrow().name,
                        );
                        if !candidates.is_empty() {
                            break Type::Struct(u.choose(&candidates)?.name.clone());
                        }
                    },
                    _ => panic!("Invalid type"),
                }
            };
            // Keeps track of the type of the field
            self.type_pool.borrow_mut().insert_mapping(&name, &typ);
            st.borrow_mut().fields.push((name, typ));
        }
        Ok(())
    }

    /// Return all struct definitions that:
    /// * with in the same module (TODO: allow cross module reference)
    /// * have the desired abilities
    /// * if key is in desired abilities, the struct must have store ability
    /// * does not create loop in the struct hierarchy (TODO: fix the check)
    fn get_usable_struct_type(
        &self,
        desired: Vec<Ability>,
        scope: &Scope,
        parent_struct_id: &Identifier,
    ) -> Vec<StructDefinition> {
        let ids = self.get_filtered_identifiers(None, Some(IDType::Struct), Some(scope));
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

    /// Check if the struct is reachable from another struct.
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

    /// Get the struct definition with the given identifier.
    fn get_struct_definition_with_identifier(&self, id: &Identifier) -> Option<StructDefinition> {
        for m in self.modules.iter() {
            for s in m.borrow().structs.iter() {
                if &s.borrow().name == id {
                    return Some(s.borrow().clone());
                }
            }
        }
        None
    }

    /// Generate a function skeleton with name and signature.
    fn generate_function_skeleton(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Function> {
        let (name, scope) = self.get_next_identifier(IDType::Function, parent_scope);
        let signature = self.generate_function_signature(u, &scope, name)?;

        Ok(Function {
            signature,
            visibility: Visibility { public: true },
            body: None,
        })
    }

    /// Fill in the function body and return statement.
    fn fill_function(&self, u: &mut Unstructured, function: &RefCell<Function>) -> Result<()> {
        let scope = self
            .id_pool
            .borrow()
            .get_scope_for_children(&function.borrow().signature.name);
        let signature = function.borrow().signature.clone();
        let body = self.generate_block(u, &scope, None, signature.return_type.clone())?;
        function.borrow_mut().body = Some(body);
        Ok(())
    }

    /// Generate a function signature with random number of parameters and return type.
    fn generate_function_signature(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        name: Identifier,
    ) -> Result<FunctionSignature> {
        let num_params = u.int_in_range(0..=self.config.max_num_params_in_func)?;
        let mut parameters = Vec::new();
        for _ in 0..num_params {
            let (name, _) = self.get_next_identifier(IDType::Var, parent_scope);

            // TODO: currently struct is not allowed in signature because script
            // TODO: cannot create structs
            // TODO: should remove this after visibility check is implemented
            // TODO: structs should be allowed for non-public functions
            let typ = self.get_random_type(u, parent_scope, true, false)?;
            self.type_pool.borrow_mut().insert_mapping(&name, &typ);
            parameters.push((name, typ));
        }

        let return_type = match bool::arbitrary(u)? {
            true => Some(self.get_random_type(u, parent_scope, true, true)?),
            false => None,
        };

        Ok(FunctionSignature {
            name,
            parameters,
            return_type,
        })
    }

    /// Generate an expression block
    fn generate_block(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        num_stmts: Option<usize>,
        ret_typ: Option<Type>,
    ) -> Result<Block> {
        let (_, block_scope) = self.get_next_identifier(IDType::Block, parent_scope);
        let num_stmts =
            num_stmts.unwrap_or(u.int_in_range(0..=self.config.max_num_stmts_in_block)?);
        let stmts = self.generate_statements(u, &block_scope, num_stmts)?;
        let return_expr = match ret_typ {
            Some(ref typ) => Some(self.generate_block_return(u, &block_scope, typ)?),
            None => None,
        };
        Ok(Block { stmts, return_expr })
    }

    /// Generate a return expression
    /// Prefer to return a variable in scope if possible
    fn generate_block_return(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: &Type,
    ) -> Result<Expression> {
        let ids = self.get_filtered_identifiers(Some(typ), Some(IDType::Var), Some(parent_scope));
        match ids.is_empty() {
            true => {
                let expr = self.generate_expression_of_type(u, parent_scope, typ, true, true)?;
                Ok(expr)
            },
            false => {
                let ident = u.choose(&ids)?.clone();
                Ok(Expression::Variable(ident))
            },
        }
    }

    /// Generate a list of statements.
    fn generate_statements(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        num_stmts: usize,
    ) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();
        for _ in 0..num_stmts {
            stmts.push(self.generate_statement(u, parent_scope)?);
        }
        Ok(stmts)
    }

    /// Generate a random statement.
    fn generate_statement(&self, u: &mut Unstructured, parent_scope: &Scope) -> Result<Statement> {
        match u.int_in_range(0..=1)? {
            0 => Ok(Statement::Decl(self.generate_declaration(u, parent_scope)?)),
            1 => Ok(Statement::Expr(self.generate_expression(u, parent_scope)?)),
            _ => panic!("Invalid statement type"),
        }
    }

    /// Generate an assignment to an existing variable.
    fn generate_assignment(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Option<Assignment>> {
        let idents = self.get_filtered_identifiers(None, Some(IDType::Var), Some(parent_scope));
        if idents.is_empty() {
            return Ok(None);
        }
        let ident = u.choose(&idents)?.clone();
        let typ = self.type_pool.borrow().get_type(&ident).unwrap();
        let expr = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
        Ok(Some(Assignment {
            name: ident,
            value: expr,
        }))
    }

    /// Generate a random declaration.
    fn generate_declaration(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Declaration> {
        let (name, _) = self.get_next_identifier(IDType::Var, parent_scope);

        let typ = self.get_random_type(u, parent_scope, true, true)?;
        // let value = match bool::arbitrary(u)? {
        //     true => Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?),
        //     false => None,
        // };
        // TODO: disabled declaration without value for now, need to keep track of initialization
        let value = Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?);
        // Keeps track of the type of the newly created variable
        self.type_pool.borrow_mut().insert_mapping(&name, &typ);
        Ok(Declaration { typ, name, value })
    }

    /// Generate a random expression.
    ///
    /// To avoid infinite recursion, we limit the depth of the expression tree.
    fn generate_expression(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Expression> {
        // Increment the expression depth
        *self.expr_depth.borrow_mut() += 1;

        // Reached the maximum depth, generate a dummy number literal
        if *self.expr_depth.borrow() >= self.config.max_expr_depth {
            *self.expr_depth.borrow_mut() -= 1;
            return Ok(Expression::NumberLiteral(
                self.generate_number_literal(u, None, None, None)?,
            ));
        }

        // If no function is callable, then skip generating function calls.
        let func_call_weight = match self.get_callable_functions(parent_scope).is_empty() {
            true => 0,
            false => 1,
        };

        // Check if there are any assignable variables in the current scope
        let assign_weight = match self
            .get_filtered_identifiers(None, Some(IDType::Var), Some(parent_scope))
            .is_empty()
        {
            true => 0,
            false => 1,
        };

        // Decides how often each expression type should be generated
        let weights = vec![
            1, // NumberLiteral
            1, // Variable
            // Boolean
            // StructInitialization
            1,                // Block
            func_call_weight, // FunctionCall
            assign_weight,
            1, // BinaryOperation
            1, // If-Else
        ];

        let expr = loop {
            match choose_idx_weighted(u, &weights)? {
                // Generate a number literal
                0 => {
                    break Expression::NumberLiteral(
                        self.generate_number_literal(u, None, None, None)?,
                    )
                },
                // Generate a variable access
                1 => {
                    let idents =
                        self.get_filtered_identifiers(None, Some(IDType::Var), Some(parent_scope));
                    if !idents.is_empty() {
                        let ident = u.choose(&idents)?.clone();
                        break Expression::Variable(ident);
                    }
                },
                // Generate a block
                2 => {
                    let ret_typ = match bool::arbitrary(u)? {
                        true => Some(self.get_random_type(u, parent_scope, true, true)?),
                        false => None,
                    };
                    let block = self.generate_block(u, parent_scope, None, ret_typ)?;
                    break Expression::Block(Box::new(block));
                },
                // Generate a function call
                3 => {
                    let call = self.generate_function_call(u, parent_scope)?;
                    match call {
                        Some(c) => break Expression::FunctionCall(c),
                        None => panic!("No callable functions"),
                    }
                },
                // Generate an assignment expression
                4 => {
                    let assign = self.generate_assignment(u, parent_scope)?;
                    match assign {
                        Some(a) => break Expression::Assign(Box::new(a)),
                        None => panic!("No assignable variables"),
                    }
                },
                // Generate a binary operation
                5 => {
                    break Expression::BinaryOperation(Box::new(
                        self.generate_binary_operation(u, parent_scope)?,
                    ));
                },
                // Generate an if-else expression with unit type
                6 => {
                    break Expression::IfElse(Box::new(self.generate_if(u, parent_scope, None)?));
                },
                _ => panic!("Invalid expression type"),
            }
        };

        // Decrement the expression depth
        *self.expr_depth.borrow_mut() -= 1;
        Ok(expr)
    }

    /// Generate an expression of the given type.
    /// `allow_var`: allow using variable access, this is disabled for script
    /// `allow_call`: allow using function calls
    ///
    /// Assumption: calling `generate_expression_of_type` will not cause infinite recursion
    /// since `generate_expression` is already bounded
    fn generate_expression_of_type(
        &self,
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
                Expression::NumberLiteral(self.generate_number_literal(u, Some(typ), None, None)?)
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

        // Now we have collected all candidate expressions that do not require recursion
        // We can perform the expr_depth check here
        *self.expr_depth.borrow_mut() += 1;
        if *self.expr_depth.borrow() >= self.config.max_expr_depth {
            return Ok(u.choose(&choices)?.clone());
        }

        // Call functions with the given return type
        if allow_call {
            let callables: Vec<FunctionSignature> = self
                .get_callable_functions(parent_scope)
                .into_iter()
                .filter(|f| f.return_type == Some(typ.clone()))
                .collect();
            // Currently, we generate calls to all candidate functions
            // This could consume a lot raw bytes and may interfere with mutation
            // TODO: consider just select a subset of functions to call
            if !callables.is_empty() {
                let func = u.choose(&callables)?;
                let call = self.generate_call_to_function(u, parent_scope, func, true)?;
                choices.push(Expression::FunctionCall(call));
            }
        }

        // Generate an If-Else with the given type
        let if_else = self.generate_if(u, parent_scope, Some(typ.clone()))?;
        choices.push(Expression::IfElse(Box::new(if_else)));

        // Generate a binary operation with the given type
        // Binary operations can output numerical and boolean values
        if typ.is_num_or_bool() {
            let binop = self.generate_numerical_binop(u, parent_scope, Some(typ.clone()))?;
            choices.push(Expression::BinaryOperation(Box::new(binop)));
        }

        // Additionally, boolean ops also output boolean values
        if typ.is_bool() {
            let binop = self.generate_boolean_binop(u, parent_scope)?;
            choices.push(Expression::BinaryOperation(Box::new(binop)));
        }

        // Decrement the expression depth
        *self.expr_depth.borrow_mut() -= 1;

        Ok(u.choose(&choices)?.clone())
    }

    /// Generate an If expression
    /// `typ` is the expected type of the expression.
    /// If `typ` is None, the type of the If will be unit and whether to have an
    /// else expression is randomly decided.
    ///
    /// If `typ` is not None, both If and Else will be generated with the same type.
    fn generate_if(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: Option<Type>,
    ) -> Result<IfExpr> {
        let condition =
            self.generate_expression_of_type(u, parent_scope, &Type::Bool, true, true)?;
        let body = self.generate_block(u, parent_scope, None, typ.clone())?;

        // When the If expression has a non-unit type
        // We have to generate an Else expression to match the type
        let else_expr = match (&typ, bool::arbitrary(u)?) {
            (Some(_), _) => Some(self.generate_else(u, parent_scope, typ.clone())?),
            (None, true) => Some(self.generate_else(u, parent_scope, None)?),
            (None, false) => None,
        };

        Ok(IfExpr {
            condition,
            body,
            else_expr,
        })
    }

    /// Generate an Else expression.
    /// The `typ` should be the same as the expected type of the previous If expression.
    fn generate_else(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: Option<Type>,
    ) -> Result<ElseExpr> {
        let body = self.generate_block(u, parent_scope, None, typ.clone())?;
        Ok(ElseExpr { typ, body })
    }

    // Generate a random binary operation.
    // Type is randomly selected
    fn generate_binary_operation(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<BinaryOperation> {
        match bool::arbitrary(u)? {
            true => self.generate_numerical_binop(u, parent_scope, None),
            false => self.generate_boolean_binop(u, parent_scope),
        }
    }

    /// Generate a random binary operation for numerical types
    /// Tries to reduce the chance of abort, but aborts can still happen
    /// If `typ` is provided, the generated expr will have this type
    /// `typ` can only be a basic numerical type.
    fn generate_numerical_binop(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: Option<Type>,
    ) -> Result<BinaryOperation> {
        use NumericalBinaryOperator as OP;

        // Select the operator
        let op = match &typ {
            // A desired output type is specified
            Some(typ) => {
                let ops = match (typ.is_numerical(), typ.is_bool()) {
                    // The output should be numerical
                    (true, false) => vec![
                        OP::Add,
                        OP::Sub,
                        OP::Mul,
                        OP::Mod,
                        OP::Div,
                        OP::BitAnd,
                        OP::BitOr,
                        OP::BitXor,
                        OP::Shl,
                        OP::Shr,
                    ],
                    // The output should be boolean
                    (false, true) => vec![OP::Le, OP::Ge, OP::Leq, OP::Geq, OP::Eq, OP::Neq],
                    // Numerical Binop cannot produce other types
                    (false, false) => panic!("Invalid output type for num binop"),
                    // A type cannot be both numerical and boolean
                    (true, true) => panic!("Impossible type"),
                };
                u.choose(&ops)?.clone()
            },
            // No desired type, all operators are allowed
            None => OP::arbitrary(u)?,
        };

        let typ = match &typ {
            Some(Type::U8) | Some(Type::U16) | Some(Type::U32) | Some(Type::U64)
            | Some(Type::U128) | Some(Type::U256) => typ.unwrap(),
            // To generate a boolean, we can select any numerical type
            // If a type is not provided, we also randomly select a numerical type
            Some(Type::Bool) | None => self.get_random_type(u, parent_scope, false, false)?,
            Some(_) => panic!("Invalid type"),
        };
        let (lhs, rhs) = match op {
            // Sum can overflow. Sub can underflow.
            // To reduce the chance these happend, only pick a RHS from a smaller type.
            // TODO: currently RHS can only be a number literal
            // TODO: once casting is supported, we can pick a variable with a smaller type
            OP::Add | OP::Sub => {
                let lhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                let value = match typ {
                    Type::U8 => BigUint::from(u.int_in_range(0..=127)? as u32),
                    Type::U16 => BigUint::from(u8::arbitrary(u)?),
                    Type::U32 => BigUint::from(u16::arbitrary(u)?),
                    Type::U64 => BigUint::from(u32::arbitrary(u)?),
                    Type::U128 => BigUint::from(u64::arbitrary(u)?),
                    Type::U256 => BigUint::from(u128::arbitrary(u)?),
                    _ => panic!("Invalid type"),
                };
                let rhs = Expression::NumberLiteral(NumberLiteral {
                    value,
                    typ: typ.clone(),
                });
                (lhs, rhs)
            },
            // The result can overflow, we choose u8 for RHS to be extra safe
            // TODO: can also try casting
            OP::Mul => {
                let lhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                let rhs = Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(u.int_in_range(0..=255)? as u32),
                    typ: typ.clone(),
                });
                (lhs, rhs)
            },
            // RHS cannot be 0
            OP::Mod | OP::Div => {
                let lhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                let rhs = Expression::NumberLiteral(self.generate_number_literal(
                    u,
                    Some(&typ),
                    Some(BigUint::from(1u32)),
                    None,
                )?);
                (lhs, rhs)
            },
            // RHS should be U8
            // Number of bits to shift should be less than the number of bits in LHS
            OP::Shl | OP::Shr => {
                let num_bits = match typ {
                    Type::U8 => 8,
                    Type::U16 => 16,
                    Type::U32 => 32,
                    Type::U64 => 64,
                    Type::U128 => 128,
                    Type::U256 => 256,
                    _ => panic!("Invalid type"),
                };
                let num_shift = u.int_in_range(0..=num_bits - 1)? as u32;
                let lhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                let rhs = Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(num_shift),
                    typ: Type::U8,
                });
                (lhs, rhs)
            },
            // The rest is ok as long as LHS and RHS are the same type
            _ => {
                let lhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                let rhs = self.generate_expression_of_type(u, parent_scope, &typ, true, true)?;
                (lhs, rhs)
            },
        };
        Ok(BinaryOperation {
            op: BinaryOperator::Numerical(op.clone()),
            lhs,
            rhs,
        })
    }

    // Generate a random binary operation for boolean
    fn generate_boolean_binop(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<BinaryOperation> {
        let op = BooleanBinaryOperator::arbitrary(u)?;
        let lhs = self.generate_expression_of_type(u, parent_scope, &Type::Bool, true, true)?;
        let rhs = self.generate_expression_of_type(u, parent_scope, &Type::Bool, true, true)?;
        Ok(BinaryOperation {
            op: BinaryOperator::Boolean(op),
            lhs,
            rhs,
        })
    }

    /// Generate a struct initialization expression.
    /// This is `pack` in the parser AST.
    // TODO: this is currently only used in `generate_expression_of_type`. Consider add to `generate_expression`.
    fn generate_struct_initialization(
        &self,
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

    /// Generate a random function call.
    fn generate_function_call(
        &self,
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

    /// Generate a call to the given function.
    fn generate_call_to_function(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        func: &FunctionSignature,
        allow_var: bool,
    ) -> Result<FunctionCall> {
        let mut args = Vec::new();

        for (_, typ) in func.parameters.iter() {
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
    ///
    /// `min` and `max` are used to generate a number within the given range.
    /// Both bounds are inclusive.
    fn generate_number_literal(
        &self,
        u: &mut Unstructured,
        typ: Option<&Type>,
        min: Option<BigUint>,
        max: Option<BigUint>,
    ) -> Result<NumberLiteral> {
        let typ = match typ {
            Some(t) => t.clone(),
            None => self.get_random_type(u, &ROOT_SCOPE, false, false)?,
        };

        let mut value = match &typ {
            Type::U8 => BigUint::from(u8::arbitrary(u)?),
            Type::U16 => BigUint::from(u16::arbitrary(u)?),
            Type::U32 => BigUint::from(u32::arbitrary(u)?),
            Type::U64 => BigUint::from(u64::arbitrary(u)?),
            Type::U128 => BigUint::from(u128::arbitrary(u)?),
            Type::U256 => BigUint::from_bytes_be(u.bytes(32)?),
            _ => panic!("Expecting number type"),
        };

        // Note: We are not uniformly sampling from the range [min, max].
        // Instead, all out-of-range values are clamped to the bounds.
        if let Some(min) = min {
            value = value.max(min);
        }

        if let Some(max) = max {
            value = value.min(max);
        }

        Ok(NumberLiteral { value, typ })
    }

    /// Returns one of the basic types that does not require a type argument.
    ///
    /// First choose a category of types, then choose a type from that category.
    /// Categories include:
    ///     * basic (number and boolean)
    ///     * structs (each struct definition is considered a type)
    pub fn get_random_type(
        &self,
        u: &mut Unstructured,
        scope: &Scope,
        allow_bool: bool,
        allow_struct: bool,
    ) -> Result<Type> {
        // Try to use smaller ints more often to reduce input consumption
        let bool_weight = match allow_bool {
            true => 10,
            false => 0,
        };
        let basics = vec![
            (Type::U8, 15),
            (Type::U16, 15),
            (Type::U32, 15),
            (Type::U64, 2),
            (Type::U128, 2),
            (Type::U256, 2),
            (Type::Bool, bool_weight),
        ];

        let mut categories = vec![basics];

        if allow_struct {
            let struct_ids = self.get_filtered_identifiers(None, Some(IDType::Struct), Some(scope));
            let structs = struct_ids
                .iter()
                .map(|id| (Type::Struct(id.clone()), 1))
                .collect::<Vec<(Type, u32)>>();
            if !structs.is_empty() {
                categories.push(structs);
            }
        }

        let chosen_cat = u.choose(&categories)?;

        let weights = chosen_cat.iter().map(|(_, w)| *w).collect::<Vec<u32>>();
        let choice = choose_idx_weighted(u, &weights)?;
        Ok(chosen_cat[choice].0.clone())
    }

    /// Get all callable functions in the given scope.
    // TODO: Handle visibility check
    fn get_callable_functions(&self, scope: &Scope) -> Vec<FunctionSignature> {
        let mut callable = Vec::new();
        for f in self.function_signatures.iter() {
            if self.id_pool.borrow().is_id_in_scope(&f.name, scope) {
                callable.push(f.clone());
            }
        }
        callable
    }

    /// Filter identifiers based on the given type, identifier type, and scope.
    fn get_filtered_identifiers(
        &self,
        typ: Option<&Type>,
        ident_type: Option<IDType>,
        scope: Option<&Scope>,
    ) -> Vec<Identifier> {
        // Filter based on the IDType
        let all_ident = match ident_type {
            Some(t) => self.id_pool.borrow().get_identifiers_of_ident_type(t),
            None => self.id_pool.borrow().get_all_identifiers(),
        };

        // Filter based on Scope
        let ident_in_scope = match scope {
            Some(s) => self
                .id_pool
                .borrow()
                .filter_identifier_in_scope(&all_ident, s),
            None => all_ident,
        };

        // Filter based on Type
        match typ {
            Some(t) => self
                .type_pool
                .borrow()
                .filter_identifier_with_type(t, ident_in_scope),
            None => ident_in_scope,
        }
    }

    /// Helper to get the next identifier.
    fn get_next_identifier(&self, ident_type: IDType, parent_scope: &Scope) -> (Identifier, Scope) {
        self.id_pool
            .borrow_mut()
            .next_identifier(ident_type, parent_scope)
    }
}
