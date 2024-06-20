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
    codegen::CodeGenerator,
    config::Config,
    env::Env,
    names::{Identifier, IdentifierType as IDType, Scope, ROOT_SCOPE},
    types::{Ability, Type, TypeParameter},
    utils::choose_idx_weighted,
};
use arbitrary::{Arbitrary, Result, Unstructured};
use log::{trace, warn};
use num_bigint::BigUint;
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::BTreeSet,
};

/// Keeps track of the generation state.
pub struct MoveSmith {
    pub config: RefCell<Config>,

    // The output code
    modules: Vec<RefCell<Module>>,
    script: Option<Script>,
    runs: RefCell<Vec<Identifier>>,

    // Skeleton Information
    function_signatures: RefCell<Vec<FunctionSignature>>,

    // Bookkeeping
    env: RefCell<Env>,
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
        let env = Env::new(&config);
        Self {
            config: RefCell::new(config),
            modules: Vec::new(),
            script: None,
            runs: RefCell::new(Vec::new()),
            function_signatures: RefCell::new(Vec::new()),
            env: RefCell::new(env),
        }
    }

    fn env(&self) -> Ref<Env> {
        self.env.borrow()
    }

    fn env_mut(&self) -> RefMut<Env> {
        self.env.borrow_mut()
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
        let num_modules = u.int_in_range(1..=self.config.borrow().max_num_modules)?;

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

        for _ in 0..u.int_in_range(1..=self.config.borrow().max_num_calls_in_script)? {
            let func = u.choose(&all_funcs)?;
            let mut call = self.generate_call_to_function(
                u,
                &ROOT_SCOPE,
                &func.borrow().signature,
                None,
                false,
            )?;
            call.name = self.env().id_pool.flatten_access(&call.name);
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
        for _ in 0..u.int_in_range(1..=self.config.borrow().max_num_structs_in_module)? {
            structs.push(RefCell::new(self.generate_struct_skeleton(u, &scope)?));
        }

        // Generate a struct with all abilities to avoid having no type to choose for some type parameters
        let (struct_name, _) = self.get_next_identifier(IDType::Struct, &scope);
        self.env_mut()
            .type_pool
            .register_type(Type::Struct(struct_name.clone()));
        structs.push(RefCell::new(StructDefinition {
            name: struct_name,
            abilities: Vec::from(Ability::ALL),
            fields: Vec::new(),
        }));

        // Function signatures
        let mut functions = Vec::new();
        for _ in 0..u.int_in_range(1..=self.config.borrow().max_num_functions_in_module)? {
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
            .env()
            .id_pool
            .get_scope_for_children(&module.borrow().name);
        // Struct fields
        for s in module.borrow().structs.iter() {
            self.fill_struct(u, s, &scope)?;
        }

        // TODO: do not generate runner code for now
        // TODO: re-enable this after function call with type param is done

        // Generate function bodies and runners
        for f in module.borrow().functions.iter() {
            self.fill_function(u, f)?;
        }

        trace!("Generating runners for module: {:?}", module.borrow().name);
        // For runners, we don't want complex expressions to reduce input
        // consumption and to avoid wasting mutation
        self.env_mut().set_max_expr_depth(0);

        let mut all_runners = Vec::new();
        for f in module.borrow().functions.iter() {
            all_runners.extend(self.generate_runners(u, f)?);
        }

        // Reset the expression depth because we will also genereate other modules
        self.env_mut().reset_max_expr_depth();

        // Insert the runners to the module and add run tasks to the whole compile unit
        // Each task is simply the flat name of the runner function
        for r in all_runners.into_iter() {
            let module_flat = self.env().id_pool.flatten_access(&module.borrow().name);
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
    #[allow(dead_code)]
    fn generate_runners(
        &self,
        u: &mut Unstructured,
        callee: &RefCell<Function>,
    ) -> Result<Vec<Function>> {
        let signature = callee.borrow().signature.clone();

        let mut runners = Vec::new();
        for i in 0..self.config.borrow().num_runs_per_func {
            // Generate a call to the target function
            let call = Expression::FunctionCall(self.generate_call_to_function(
                u,
                &ROOT_SCOPE,
                &signature,
                None,
                false,
            )?);

            // If the callee returns a type parameter, we ignore the return.
            let new_ret = match &signature.return_type {
                Some(Type::TypeParameter(_)) => None,
                Some(t) => Some(t.clone()),
                None => None,
            };

            // Generate a body with only one statement/return expr
            let body = match new_ret.is_none() {
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
                    type_parameters: Vec::new(),
                    name: Identifier(format!("{}_runner_{}", signature.name.0, i)),
                    parameters: Vec::new(),
                    return_type: new_ret,
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

        self.env_mut()
            .type_pool
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
        for _ in 0..u.int_in_range(0..=self.config.borrow().max_num_fields_in_struct)? {
            let (name, _) = self.get_next_identifier(IDType::Var, &struct_scope);

            let typ = loop {
                match u.int_in_range(0..=2)? {
                    // More chance to use basic types than struct types
                    0 | 1 => {
                        break self.get_random_type(u, parent_scope, true, false, false, false)?
                    },
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
            self.env_mut().type_pool.insert_mapping(&name, &typ);
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
        let ids = self
            .env()
            .get_identifiers(None, Some(IDType::Struct), Some(scope));
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
        let signature: FunctionSignature = self.generate_function_signature(u, &scope, name)?;

        // Keep track of the function signature separately so that we don't have
        // to go through all modules to get signatures
        self.function_signatures
            .borrow_mut()
            .push(signature.clone());

        let func = Function {
            signature,
            visibility: Visibility { public: true },
            body: None,
        };
        trace!("Generated function signature: {:?}", func.inline());
        Ok(func)
    }

    /// Fill in the function body and return statement.
    fn fill_function(&self, u: &mut Unstructured, function: &RefCell<Function>) -> Result<()> {
        let scope = self
            .env()
            .id_pool
            .get_scope_for_children(&function.borrow().signature.name);
        let signature = function.borrow().signature.clone();
        trace!(
            "Creating block for the body of function: {:?}",
            signature.name
        );
        let body = self.generate_block(u, &scope, None, signature.return_type.clone())?;
        function.borrow_mut().body = Some(body);
        Ok(())
    }

    /// Generate a function signature with random number of parameters and return type.
    ///
    /// We need to make sure that if the return type is a type parameter,
    /// at least one of the parameters have this type.
    /// Otherwise, we cannot instantiate this type for return.
    fn generate_function_signature(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        name: Identifier,
    ) -> Result<FunctionSignature> {
        // First generate type parameters so that they can be used in the parameters and return type
        let mut type_parameters = Vec::new();
        for _ in 0..u.int_in_range(0..=self.config.borrow().max_num_type_params_in_func)? {
            type_parameters.push(self.generate_type_parameter(
                u,
                parent_scope,
                false,
                // TODO: unused vars are auto dropped so this prevents the error.
                // TODO: should remove this after drop is properly handled
                Some(vec![Ability::Drop]),
                Some(vec![Ability::Key]),
            )?);
        }

        let num_params = u.int_in_range(0..=self.config.borrow().max_num_params_in_func)?;
        let mut parameters = Vec::new();
        for _ in 0..num_params {
            let (name, _) = self.get_next_identifier(IDType::Var, parent_scope);

            // TODO: currently struct is not allowed in signature because script
            // TODO: cannot create structs
            // TODO: should remove this after visibility check is implemented
            // TODO: structs should be allowed for non-public functions
            let typ = self.get_random_type(u, parent_scope, true, false, true, false)?;
            self.env_mut().type_pool.insert_mapping(&name, &typ);
            parameters.push((name, typ));
        }

        // More chance to have return type than not
        // so that we can compare the the return value
        let return_type = match u.int_in_range(0..=10)? > 2 {
            true => Some(self.get_random_type(u, parent_scope, true, true, true, false)?),
            false => None,
        };

        // Check whether the return type exists in the parameters if the return
        // type is a type parameter.
        // If not in params, we insert one more parameter so that we have
        // something to return
        if let Some(ret_ty @ Type::TypeParameter(_)) = &return_type {
            if !parameters.iter().any(|(_, param_ty)| param_ty == ret_ty) {
                let (name, _) = self.get_next_identifier(IDType::Var, parent_scope);
                self.env_mut().type_pool.insert_mapping(&name, ret_ty);
                parameters.push((name, ret_ty.clone()));
            }
        }

        Ok(FunctionSignature {
            type_parameters,
            name,
            parameters,
            return_type,
        })
    }

    /// Generate a type parameter with random abilities.
    /// Albilities in `include` will always be included.
    /// Abilities in `exclude` will not be used.
    fn generate_type_parameter(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        allow_phantom: bool,
        include: Option<Vec<Ability>>,
        exclude: Option<Vec<Ability>>,
    ) -> Result<TypeParameter> {
        let (name, _) = self.get_next_identifier(IDType::TypeParameter, parent_scope);

        let is_phantom = match allow_phantom {
            true => bool::arbitrary(u)?,
            false => false,
        };

        let mut abilities = Vec::new();
        let inc = include.unwrap_or_default();
        let exc = exclude.unwrap_or_default();

        for i in [Ability::Copy, Ability::Drop, Ability::Store, Ability::Key].into_iter() {
            if exc.contains(&i) {
                continue;
            }

            if inc.contains(&i) || bool::arbitrary(u)? {
                abilities.push(i);
            }
        }

        let tp = TypeParameter {
            name: name.clone(),
            abilities,
            is_phantom,
        };

        let type_for_tp = Type::TypeParameter(tp.clone());

        // Register the type parameter so that its siblings can reference it
        self.env_mut().type_pool.register_type(type_for_tp.clone());

        // Links the type parameter to its name so that later we can
        // retrieve the type from the name
        self.env_mut()
            .type_pool
            .insert_mapping(&type_for_tp.get_name(), &type_for_tp);

        Ok(tp)
    }

    /// Generate an expression block
    fn generate_block(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        num_stmts: Option<usize>,
        ret_typ: Option<Type>,
    ) -> Result<Block> {
        trace!(
            "Generating block with parent scope: {:?}, depth: {}",
            parent_scope,
            self.env().curr_expr_depth()
        );
        let (_, block_scope) = self.get_next_identifier(IDType::Block, parent_scope);
        trace!("Created block scope: {:?}", block_scope);

        let reach_limit = self.env().will_reached_expr_depth_limit(1);
        let stmts = if reach_limit {
            warn!("Max expr depth will be reached in this block, skipping generating body");
            Vec::new()
        } else {
            let num_stmts = num_stmts
                .unwrap_or(u.int_in_range(0..=self.config.borrow().max_num_stmts_in_block)?);
            self.generate_statements(u, &block_scope, num_stmts)?
        };
        let return_expr = match ret_typ {
            Some(ref typ) => Some(self.generate_block_return(u, &block_scope, typ)?),
            None => None,
        };
        trace!("Done generating block: {:?}", block_scope);
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
        let ids = self
            .env()
            .get_identifiers(Some(typ), Some(IDType::Var), Some(parent_scope));
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
        trace!("Generating {} statements", num_stmts);
        let mut stmts = Vec::new();
        for i in 0..num_stmts {
            trace!("Generating statement #{}", i + 1);
            stmts.push(self.generate_statement(u, parent_scope)?);
            trace!("Done generating statement #{}", i + 1);
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
    ///
    /// There must be at least one variable in the scope and the type of the variable
    /// must have been decided.
    fn generate_assignment(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Option<Assignment>> {
        trace!("Generating assignment");
        let idents = self
            .env()
            .get_identifiers(None, Some(IDType::Var), Some(parent_scope));
        if idents.is_empty() {
            return Ok(None);
        }
        let ident = u.choose(&idents)?.clone();
        let typ = self.env().type_pool.get_type(&ident).unwrap();
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

        // TODO: we should not omit type parameter as we can call a function to get an object of that type
        let typ = self.get_random_type(u, parent_scope, true, true, false, false)?;
        trace!("Generating declaration of type: {:?}", typ);
        // let value = match bool::arbitrary(u)? {
        //     true => Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?),
        //     false => None,
        // };
        // TODO: disabled declaration without value for now, need to keep track of initialization
        let value = Some(self.generate_expression_of_type(u, parent_scope, &typ, true, true)?);
        // Keeps track of the type of the newly created variable
        self.env_mut().type_pool.insert_mapping(&name, &typ);
        Ok(Declaration { typ, name, value })
    }

    /// Generate a random expression.
    ///
    /// This is used only for generating statements, so some kinds of expressions are omitted.
    ///
    /// To avoid infinite recursion, we limit the depth of the expression tree.
    fn generate_expression(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Expression> {
        trace!("Generating expression from scope: {:?}", parent_scope);
        // Increment the expression depth
        // Reached the maximum depth, generate a dummy number literal
        if self.env().reached_expr_depth_limit() {
            warn!("Max expr depth reached in scope: {:?}", parent_scope);
            return Ok(Expression::NumberLiteral(
                self.generate_number_literal(u, None, None, None)?,
            ));
        }

        self.env_mut().increase_expr_depth(u);

        // If no function is callable, then skip generating function calls.
        let func_call_weight = match self.get_callable_functions(parent_scope).is_empty() {
            true => 0,
            false => 10,
        };

        // Check if there are any assignable variables in the current scope
        let assign_weight = match self
            .env()
            .get_identifiers(None, Some(IDType::Var), Some(parent_scope))
            .is_empty()
        {
            true => 0,
            false => 5,
        };

        // Decides how often each expression type should be generated
        let weights = vec![
            5,                // BinaryOperation
            5,                // If-Else
            1,                // Block
            func_call_weight, // FunctionCall
            assign_weight,    // Assignment
        ];

        let idx = choose_idx_weighted(u, &weights)?;
        trace!(
            "Chosing expression kind, idx chosen is {}, weight is {:?}",
            idx,
            weights
        );

        let expr = match idx {
            // Generate a binary operation
            0 => Expression::BinaryOperation(Box::new(self.generate_binary_operation(
                u,
                parent_scope,
                None,
            )?)),
            // Generate an if-else expression with unit type
            1 => Expression::IfElse(Box::new(self.generate_if(u, parent_scope, None)?)),
            // Generate a block
            2 => {
                let ret_typ = match bool::arbitrary(u)? {
                    true => Some(self.get_random_type(u, parent_scope, true, true, true, true)?),
                    false => None,
                };
                let block = self.generate_block(u, parent_scope, None, ret_typ)?;
                Expression::Block(Box::new(block))
            },
            // Generate a function call
            3 => {
                let call = self.generate_function_call(u, parent_scope)?;
                match call {
                    Some(c) => Expression::FunctionCall(c),
                    None => panic!("No callable functions"),
                }
            },
            // Generate an assignment expression
            4 => {
                let assign = self.generate_assignment(u, parent_scope)?;
                match assign {
                    Some(a) => Expression::Assign(Box::new(a)),
                    None => panic!("No assignable variables"),
                }
            },
            _ => panic!("Invalid expression type"),
        };

        // Decrement the expression depth
        self.env_mut().decrease_expr_depth();
        Ok(expr)
    }

    /// Concretize a type parameter or a type with type parameters.
    ///
    /// If the type cannot be concretized further (e.g. primitive,
    /// fully concretized struct, type parameters defined in current function),
    /// None will be returned.
    fn concretize_type(
        &self,
        u: &mut Unstructured,
        typ: &Type,
        parent_scope: &Scope,
        constraints: Vec<Ability>,
    ) -> Option<Type> {
        if !self.is_type_concretizable(typ, parent_scope) {
            return None;
        }

        self.env_mut().increase_type_depth(u);

        let concretized = match typ {
            Type::TypeParameter(tp) => {
                self.concretize_type_parameter(u, tp, parent_scope, constraints)
            },
            _ => panic!("{:?} cannot be concretized.", typ),
        };

        self.env_mut().decrease_type_depth();
        Some(concretized)
    }

    /// The given `tp` must be concretizable!!!
    ///
    /// Find all types in scope (including non-concrete types) that
    ///     1. Satisfy the constraints
    ///     2. Satisfy the requirement of the type parameter
    ///
    /// Randomly choose one type.
    ///
    /// If the chosen one is a non-concrete, return it.
    ///
    /// If the chosen one is a non-concrete,concretize the chosen type with
    /// the union of the required abilities of the type parameter and the original constraints.
    ///
    fn concretize_type_parameter(
        &self,
        u: &mut Unstructured,
        tp: &TypeParameter,
        parent_scope: &Scope,
        mut constraints: Vec<Ability>,
    ) -> Type {
        // TODO: better to use set... but this will never get large
        for ability in tp.abilities.iter() {
            if !constraints.contains(ability) {
                constraints.push(ability.clone());
            }
        }

        // !!! We didn't check if the choices are empty
        // !!! The assumption is that we can find a concrete type that satisfies
        // !!! the constraints of the type parameter.
        // !!! This is ensured because we insert a struct with all abilities
        // !!! to all modules
        let choices = self.get_types_with_abilities(parent_scope, &constraints, true);
        let chosen = u.choose(&choices).unwrap().clone();

        match self.is_type_concretizable(&chosen, parent_scope) {
            true => self
                .concretize_type(u, &chosen, parent_scope, constraints)
                .unwrap(),
            false => chosen,
        }
    }

    // Check whether a type can be further concretized
    // For primitive types, no
    // For structs, TODO
    // For type parameters, if it is immediately defined in the parent scope,
    // then we cannot further concretize it.
    // If it is defined else where (e.g. struct definition), we can further
    // concretize it using concrete types or local type parameters.
    fn is_type_concretizable(&self, typ: &Type, parent_scope: &Scope) -> bool {
        match typ {
            Type::TypeParameter(_) => {
                // Check if the type parameter is define in parent scope
                let tp_scope = self
                    .env()
                    .id_pool
                    .get_parent_scope_of(&typ.get_name())
                    .unwrap();
                let calling_func_scope = parent_scope.remove_hidden_scopes();
                // The type parameter can be further concretized if it's not
                // defined immediately in the parent_scope
                tp_scope != calling_func_scope
            },
            _ => false,
        }
    }

    /// Generate an expression of the given type or its subtype.
    ///
    /// `allow_var`: allow using variable access, this is disabled for script
    /// `allow_call`: allow using function calls
    fn generate_expression_of_type(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: &Type,
        allow_var: bool,
        allow_call: bool,
    ) -> Result<Expression> {
        trace!(
            "Generating expression of type {:?} in scope {:?}",
            typ,
            parent_scope
        );
        // Check whether the current type pool contains a concrete type
        // for the given type parameter.
        // If so, directly use the concrete type.
        let concrete_type = if let Type::TypeParameter(_) = typ {
            self.env().type_pool.get_concrete_type(&typ.get_name())
        } else {
            None
        };

        let typ = match &concrete_type {
            Some(concrete) => concrete,
            None => typ,
        };
        trace!("Concretized type is: {:?}", typ);

        // Store default choices that do not require recursion
        // If other options are available, will not use these
        let mut default_choices: Vec<Expression> = Vec::new();
        // Store candidate expressions for the given type
        let mut choices: Vec<Expression> = Vec::new();

        // Directly generate a value for basic types
        let some_candidate = match typ {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 => {
                Some(Expression::NumberLiteral(self.generate_number_literal(
                    u,
                    Some(typ),
                    None,
                    None,
                )?))
            },
            Type::Bool => Some(Expression::Boolean(bool::arbitrary(u)?)),
            Type::Struct(id) => Some(self.generate_struct_initialization(u, parent_scope, id)?),
            // Here we always try to concretize the type.
            // It's tricky to avoid infinite loop:
            // If the type is concretized, then it's guarenteed that the call to
            // `generate_expression_of_type` will not hit this branch and enter
            // the true branch of `if` again, so we don't need to increment the counter.
            // If the type is already fully conretized, then we do not need to generate
            // a candidate because some candidate must have been generated from
            // creating new object or from variables.
            // However, we must assert that `allow_var` is enabled.
            Type::TypeParameter(_) => {
                if let Some(concretized) = self.concretize_type(u, typ, parent_scope, vec![]) {
                    Some(self.generate_expression_of_type(
                        u,
                        parent_scope,
                        &concretized,
                        allow_var,
                        allow_call,
                    )?)
                } else {
                    // In this branch, we have a type parameter that cannot be
                    // further concretized, thus the only expression we can
                    // generate is to access a variable of this type
                    assert!(allow_var);
                    None
                }
            },
            _ => unimplemented!(),
        };

        if let Some(candidate) = some_candidate {
            if let Type::TypeParameter(_) = typ {
                choices.push(candidate.clone());
            }
            default_choices.push(candidate);
        }

        // Access identifier with the given type
        if allow_var {
            let idents =
                self.env()
                    .get_identifiers(Some(typ), Some(IDType::Var), Some(parent_scope));

            // TODO: select from many?
            if !idents.is_empty() {
                let candidate = u.choose(&idents)?.clone();
                let expr = Expression::Variable(candidate);
                default_choices.push(expr.clone());
                choices.push(expr);
            }
        }

        // Now we have collected all candidate expressions that do not require recursion
        // We can perform the expr_depth check here
        assert!(!default_choices.is_empty());
        if self.env().reached_expr_depth_limit() {
            warn!("Max expr depth reached while gen expr of type: {:?}", typ);
            return Ok(u.choose(&default_choices)?.clone());
        }
        self.env_mut().increase_expr_depth(u);

        let callables: Vec<FunctionSignature> = self
            .get_callable_functions(parent_scope)
            .into_iter()
            .filter(|f| f.return_type == Some(typ.clone()))
            .collect();

        let func_call_weight = match (allow_call, !callables.is_empty()) {
            (true, true) => 5,
            (true, false) => 0,
            (false, _) => 0,
        };

        let binop_weight = match typ.is_num_or_bool() {
            true => 5,
            false => 0,
        };

        let weights = vec![
            2,                // If-Else
            func_call_weight, // FunctionCall
            binop_weight,     // BinaryOperation
        ];

        let idx = choose_idx_weighted(u, &weights)?;
        trace!(
            "Selecting expression of type kind, idx is {}, weights: {:?}",
            idx,
            weights
        );
        match idx {
            0 => {
                // Generate an If-Else with the given type
                let if_else = self.generate_if(u, parent_scope, Some(typ.clone()))?;
                choices.push(Expression::IfElse(Box::new(if_else)));
            },
            1 => {
                assert!(!callables.is_empty());
                let func = u.choose(&callables)?;
                let call =
                    self.generate_call_to_function(u, parent_scope, func, Some(typ), true)?;
                choices.push(Expression::FunctionCall(call));
            },
            2 => {
                // Generate a binary operation with the given type
                // Binary operations can output numerical and boolean values
                assert!(typ.is_num_or_bool());
                let binop = self.generate_binary_operation(u, parent_scope, Some(typ.clone()))?;
                choices.push(Expression::BinaryOperation(Box::new(binop)));
            },
            _ => panic!("Invalid option for expression generation"),
        };

        // Decrement the expression depth
        self.env_mut().decrease_expr_depth();

        let use_choice = match choices.is_empty() {
            true => default_choices,
            false => choices,
        };
        Ok(u.choose(&use_choice)?.clone())
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
        trace!("Generating if expression of type: {:?}", typ);
        trace!("Generating condition for if expression");
        let condition =
            self.generate_expression_of_type(u, parent_scope, &Type::Bool, true, true)?;
        trace!("Generating block for if true branch");
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
        trace!("Generating block for else branch");
        let body = self.generate_block(u, parent_scope, None, typ.clone())?;
        Ok(ElseExpr { typ, body })
    }

    /// Generate a random binary operation.
    /// `typ` can specify the desired output type.
    /// `typ` can only be a basic numerical type or boolean.
    fn generate_binary_operation(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: Option<Type>,
    ) -> Result<BinaryOperation> {
        trace!("Generating binary operation");
        let chosen_typ = match typ {
            Some(t) => match t.is_num_or_bool() {
                true => t,
                false => panic!("Invalid type for binary operation"),
            },
            None => self.get_random_type(u, parent_scope, true, false, false, false)?,
        };

        if chosen_typ.is_bool() {
            let weights = vec![
                2, // num op
                3, // bool op
                5, // equality check
            ];
            match choose_idx_weighted(u, &weights)? {
                0 => self.generate_numerical_binop(u, parent_scope, Some(chosen_typ)),
                1 => self.generate_boolean_binop(u, parent_scope),
                2 => self.generate_equality_check(u, parent_scope, None),
                _ => panic!("Invalid option for binary operation"),
            }
        } else {
            self.generate_numerical_binop(u, parent_scope, Some(chosen_typ))
        }
    }

    /// Generate a random binary operation for numerical types
    /// Tries to reduce the chance of abort, but aborts can still happen
    /// If `typ` is provided, the generated expr will have this type
    /// `typ` can only be a basic numerical type or boolean.
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
                    (false, true) => vec![OP::Le, OP::Ge, OP::Leq, OP::Geq],
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
            Some(Type::Bool) | None => {
                self.get_random_type(u, parent_scope, false, false, false, false)?
            },
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

    /// Generate a random binary operation for boolean
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

    /// Generate an equality check expression.
    /// `typ` can specify the desired type for both operands.
    /// If `typ` is not provided, it will be randomly selected.
    fn generate_equality_check(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: Option<Type>,
    ) -> Result<BinaryOperation> {
        trace!(
            "Generating equality check with desired operand type: {:?}",
            typ
        );
        let op = EqualityBinaryOperator::arbitrary(u)?;
        let chosen_typ = match typ {
            Some(t) => t,
            None => self.get_random_type(u, parent_scope, true, true, true, true)?,
        };
        trace!("Chosen operand type for equality check: {:?}", chosen_typ);
        let lhs = self.generate_expression_of_type(u, parent_scope, &chosen_typ, true, true)?;
        let rhs = self.generate_expression_of_type(u, parent_scope, &chosen_typ, true, true)?;
        Ok(BinaryOperation {
            op: BinaryOperator::Equality(op),
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
            None,
            true,
        )?))
    }

    /// Generate a call to the given function.
    /// If the function returns a type parameter, the `ret_type` can specify
    /// the desired concrete type for this function.
    fn generate_call_to_function(
        &self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        func: &FunctionSignature,
        desired_ret_type: Option<&Type>,
        allow_var: bool,
    ) -> Result<FunctionCall> {
        trace!("Generating call to function: {:?}", func.name);
        let mut type_args = Vec::new();
        let mut args = Vec::new();

        for tp in func.type_parameters.iter() {
            let typ_param = Type::TypeParameter(tp.clone());

            // If this type parameter is the same as the return type, we
            // use the desired return type if provided
            let concrete_type = match desired_ret_type {
                Some(ret) => {
                    if ret == &typ_param {
                        ret.clone()
                    } else {
                        self.concretize_type(u, &typ_param, parent_scope, vec![])
                            .unwrap_or(typ_param.clone())
                    }
                },
                None => self
                    .concretize_type(u, &typ_param, parent_scope, vec![])
                    .unwrap_or(typ_param.clone()),
            };
            // Keep track of the concrete types we decided here
            self.env_mut()
                .type_pool
                .register_concrete_type(&typ_param.get_name(), &concrete_type);
            type_args.push(concrete_type);
        }

        // Generate arguments using the selected concrete types
        for (_, typ) in func.parameters.iter() {
            let expr = self.generate_expression_of_type(u, parent_scope, typ, allow_var, false)?;
            args.push(expr);
        }

        // Done using the concrete types, unregister
        for tp in func.type_parameters.iter() {
            let typ_param = Type::TypeParameter(tp.clone());
            self.env_mut()
                .type_pool
                .unregister_concrete_type(&typ_param.get_name());
        }

        trace!("Done generating call to function: {:?}", func.name);
        Ok(FunctionCall {
            name: func.name.clone(),
            type_args,
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
            None => self.get_random_type(u, &ROOT_SCOPE, false, false, false, false)?,
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
    fn get_random_type(
        &self,
        u: &mut Unstructured,
        scope: &Scope,
        allow_bool: bool,
        allow_struct: bool,
        allow_type_param: bool,
        only_instantiatable: bool,
    ) -> Result<Type> {
        let bool_weight = match allow_bool {
            true => 10,
            false => 0,
        };
        // Try to use smaller ints more often to reduce input consumption
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
        let mut category_weights = vec![1];

        // Choose struct types in scope
        // Every struct has the same weight
        if allow_struct {
            let struct_ids = self
                .env()
                .get_identifiers(None, Some(IDType::Struct), Some(scope));
            let structs = struct_ids
                .iter()
                .map(|id: &Identifier| (Type::Struct(id.clone()), 1))
                .collect::<Vec<(Type, u32)>>();
            if !structs.is_empty() {
                categories.push(structs);
                category_weights.push(5);
            }
        }

        // Choose type parameters in scope
        // Every type parameter has the same weight
        if allow_type_param {
            let mut params = self
                .env()
                .get_identifiers(None, Some(IDType::TypeParameter), Some(scope))
                .into_iter()
                .map(|id| self.env().type_pool.get_type(&id).unwrap())
                .collect::<Vec<Type>>();

            if only_instantiatable {
                params = self.filter_instantiatable_types(scope, params);
            }

            let param_cat: Vec<(Type, u32)> = params
                .into_iter()
                .map(|typ| (typ, 1))
                .collect::<Vec<(Type, u32)>>();

            if !param_cat.is_empty() {
                categories.push(param_cat);
                category_weights.push(5);
            }
        }

        let cat_idx = choose_idx_weighted(u, &category_weights)?;
        let chosen_cat = &categories[cat_idx];

        let weights = chosen_cat.iter().map(|(_, w)| *w).collect::<Vec<u32>>();
        let choice = choose_idx_weighted(u, &weights)?;
        Ok(chosen_cat[choice].0.clone())
    }

    // Filter out types that are not instantiatable
    // For each type, checks if there is an accessible variable in `scope` that has the type
    fn filter_instantiatable_types(&self, scope: &Scope, types: Vec<Type>) -> Vec<Type> {
        let instantiatables = self
            .env()
            .get_identifiers(None, Some(IDType::Var), Some(scope))
            .into_iter()
            .filter_map(|id| self.env().type_pool.get_type(&id))
            .collect::<BTreeSet<Type>>();

        types
            .into_iter()
            .filter(|typ| {
                if let Type::TypeParameter(_) = typ {
                    instantiatables.contains(typ)
                } else {
                    true
                }
            })
            .collect()
    }

    /// Get all callable functions in the given scope.
    ///
    /// If `ret_type` is specified, only functions that can return the given type
    /// will be returned.
    ///
    // TODO: Handle visibility check
    fn get_callable_functions(&self, scope: &Scope) -> Vec<FunctionSignature> {
        let mut callable = Vec::new();
        for f in self.function_signatures.borrow().iter() {
            if self.env().id_pool.is_id_in_scope(&f.name, scope) {
                callable.push(f.clone());
            }
        }
        callable
    }

    /// Finds all registered types that contains all the required abilities
    pub fn get_types_with_abilities(
        &self,
        parent_scope: &Scope,
        requires: &[Ability],
        only_instantiatable: bool,
    ) -> Vec<Type> {
        let types = self
            .env()
            .type_pool
            .get_all_types()
            .iter()
            .filter(|t| match t.is_num_or_bool() {
                true => true,
                false => {
                    let id = match t {
                        Type::Struct(id) => id,
                        Type::TypeParameter(tp) => &tp.name,
                        _ => panic!("Invalid type"),
                    };
                    self.env().id_pool.is_id_in_scope(id, parent_scope)
                },
            })
            .filter(|t| {
                let possible_abilities = self.derive_abilities_of_type(t);
                requires.iter().all(|req| possible_abilities.contains(req))
            })
            .cloned()
            .collect();
        match only_instantiatable {
            true => self.filter_instantiatable_types(parent_scope, types),
            false => types,
        }
    }

    /// Get the possible abilities of a struct type.
    /// Only give the upper bound of possible abilities.
    /// TODO: this should belong to the type.rs or somewhere else
    pub fn derive_abilities_of_type(&self, typ: &Type) -> Vec<Ability> {
        match typ {
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 | Type::Bool => {
                Vec::from(Ability::PRIMITIVES)
            },
            // TODO: currently only use the `has`
            // TODO: should properly check the type arguments for concrete struct types.
            Type::Struct(id) => {
                let st = self.get_struct_definition_with_identifier(id).unwrap();
                st.abilities.clone()
            },
            Type::TypeParameter(tp) => tp.abilities.clone(),
            _ => Vec::from(Ability::NONE),
        }
    }

    /// Helper to get the next identifier.
    fn get_next_identifier(&self, ident_type: IDType, parent_scope: &Scope) -> (Identifier, Scope) {
        self.env_mut()
            .id_pool
            .next_identifier(ident_type, parent_scope)
    }
}
