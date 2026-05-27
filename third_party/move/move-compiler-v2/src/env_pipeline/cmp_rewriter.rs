// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Comparison operation rewriter
//! - The in-house Lt/Le/Gt/Ge operations only allow integer operands.
//! - We visit Lt/Le/Gt/Ge operations
//!   - If their operands are not integer primitive types,
//!     - we rewrite the operation to use the `std::cmp::compare` function,
//!     - and then interpret the result using `std::cmp::is_lt`, `std::cmp::is_le`, etc.
//!
//! Example:
//!    x <= y
//!    =======>
//!    std::cmp::compare<T>(&x, &y).std::cmp::is_le()
//!
//! Key impls
//! - `fn rewrite_cmp_operation`:
//!   - Given an ast exp representing a comparison operation, e.g., `Call(Operation::Lt, [arg1, arg2])`,
//!   - the function takes five steps:
//!     1. check if `arg1` and `arg2` can be transformed based on their types
//!     2. transform `arg1` and `arg2`, if not already references, into `&arg1` and `&arg2`
//!     3. create a call to `std::cmp::compare`: `exp1 = Call(MoveFunction(std::cmp::compare), [&arg1, &arg2])`
//!     4. create an immutable reference to the result of `std::cmp::compare`: `exp2 = Call(Borrow(Immutable), [exp1])`
//!     5. generate a final call of `is_lt / is_le / is_gt / is_ge` to interpret the result of `std::cmp::compare`: `exp3 = Call(MoveFunction(std::cmp::is_le), [exp2])`
//! - [TODO] An optimization to consider
//!   - Once we have public enum, we can direly apply `==` to the result of `std::cmp::compare` (of the `Enum Ordering` type),
//!   - which can avoid creating a reference to the result of `std::cmp::compare` and calling `is_lt / is_le / is_gt / is_ge`.
//!
//!
//! Important notes about linking `std::cmp` module:
//! - To ensure the `std::cmp` module is preserved in GlobalEnv during compilation,
//!   - code is added in `third_party/move/move-model/src/lib.rs` to keep `std::cmp` and its dependencies during the ast expansion phase
//! - To ensure implicit dependencies introduced by comparison rewriting are maintained
//!   - code is added in `third_party/move/move-compiler-v2/legacy-move-compiler/src/expansion/dependency_ordering.rs` to add dependency between every user module and `std::cmp`.
//! - If the user does not include `move-stdlib` for compilation,
//!   - a compilation error "cannot find `std::cmp` module" will be raised.
//!
//! Important notes about specs
//! - No extensions are made to comparison operations in specs.
//! - Only primitive integer types are supported as before
//!

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use move_core_types::account_address::AccountAddress;
use move_model::{
    ast::{Address, Exp, ExpData, ModuleName, Operation},
    exp_rewriter::ExpRewriterFunctions,
    model::{FunId, FunctionEnv, GlobalEnv, ModuleEnv, NodeId, QualifiedId},
    ty::*,
    well_known,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    iter::Iterator,
};

/// Main interface for the comparison operation rewriter
pub fn rewrite(env: &mut GlobalEnv) {
    // Create a shared CmpRewriter instance for processing all target functions
    let mut rewriter = CmpRewriter::new(env);
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::MoveFun(func_id) = target {
            let new_def = rewrite_target(&mut rewriter, func_id);
            if let Some(def) = new_def {
                *targets.state_mut(&target) = RewriteState::Def(def);
            }
        }
    }
    targets.write_to_env(env);
}

/// Rewrite each target function
/// - The rewriting will go through the ExpRewriterFunctions trait
/// - On a `Call` operation, the transformation will be redirected to `rewrite_call` defined later
fn rewrite_target(rewriter: &mut CmpRewriter, func_id: QualifiedId<FunId>) -> Option<Exp> {
    let func_env = rewriter.env.get_function(func_id);
    let def_opt = func_env.get_def();
    if let Some(def) = def_opt {
        let rewritten_def = rewriter.rewrite_exp(def.clone());
        if !ExpData::ptr_eq(&rewritten_def, def) {
            return Some(rewritten_def);
        }
    }
    None
}

struct CmpRewriter<'env> {
    env: &'env GlobalEnv,
    cmp_module: Option<ModuleEnv<'env>>,
    cmp_functions: BTreeMap<&'static str, FunctionEnv<'env>>,
}

/// Override `rewrite_call` from `ExpRewriterFunctions` trait
impl ExpRewriterFunctions for CmpRewriter<'_> {
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if matches!(
            oper,
            Operation::Lt | Operation::Le | Operation::Gt | Operation::Ge
        ) {
            self.rewrite_cmp_operation(call_id, oper, args)
        } else {
            None
        }
    }
}

impl<'env> CmpRewriter<'env> {
    /// Constants for the `std::cmp` module and its functions
    const COMPARE: &'static str = "compare";
    const IS_GE: &'static str = "is_ge";
    const IS_GT: &'static str = "is_gt";
    const IS_LE: &'static str = "is_le";
    const IS_LT: &'static str = "is_lt";

    fn new(env: &'env GlobalEnv) -> Self {
        let cmp_module = Self::find_cmp_module(env);
        let mut cmp_functions = BTreeMap::new();

        if let Some(module) = &cmp_module {
            for name in [
                Self::COMPARE,
                Self::IS_LT,
                Self::IS_LE,
                Self::IS_GT,
                Self::IS_GE,
            ] {
                if let Some(func) = Self::find_cmp_function(module, name) {
                    cmp_functions.insert(name, func);
                }
            }
        }

        Self {
            env,
            cmp_module,
            cmp_functions,
        }
    }

    /// Rewrite comparison operations
    ///   - Designs detailed in the beginning of this file
    fn rewrite_cmp_operation(
        &mut self,
        call_id: NodeId,
        cmp_op: &Operation,
        args: &[Exp],
    ) -> Option<Exp> {
        // Step 1: Check argument types
        if args.iter().any(|arg| self.arg_cannot_transform(arg)) {
            return None;
        }
        // Get the expected type-parameter type for the `std::cmp::compare` function
        let arg_ty = self.env.get_node_type(args[0].node_id());
        let expected_arg_ty = arg_ty.drop_reference();

        // Step 2: Transform `arg1` and `arg2` into `&arg1` and `&arg2` (do nothing if already references)
        let transformed_args: Vec<Exp> = args.iter().map(|arg| self.rewrite_cmp_arg(arg)).collect();

        // Step 3: Create an inner call to `std::cmp::compare(&arg1, &arg2)`
        let call_cmp = self.generate_call_to_compare(call_id, transformed_args, expected_arg_ty)?;

        // Step 4: Create a immutable reference to the result of `std::cmp::compare(&arg1, &arg2)`
        let immref_cmp_res = self.immborrow_compare_res(call_cmp);

        //Step 5: Generate a final call of `is_lt / is_le / is_gt / is_ge` to interpret the result of `std::cmp::compare`
        self.generate_call_to_final_res(call_id, cmp_op, vec![immref_cmp_res])
    }

    /// Generate a call to `std::cmp::compare(&arg1, &arg2)`
    fn generate_call_to_compare(
        &self,
        cmp_op_id: NodeId,
        args: Vec<Exp>,
        expected_arg_ty: Type,
    ) -> Option<Exp> {
        // Reuse the loc info of the original comparison operation
        let cmp_loc = self.env.get_node_loc(cmp_op_id);

        // Check the `std::cmp` module
        let cmp_module = match self.cmp_module.as_ref() {
            Some(module) => module,
            None => {
                self.env.error(
                    &cmp_loc,
                    "cannot find `std::cmp` module. Include `move-stdlib` for compilation.",
                );
                let invalid_id = self.env.new_node(self.env.internal_loc(), Type::Error);
                return Some(ExpData::Invalid(invalid_id).into_exp());
            },
        };
        let cmp_module_id = cmp_module.get_id();

        // Check the `std::cmp::compare` function
        let compare_function = match self.cmp_functions.get(Self::COMPARE) {
            Some(func) => func,
            None => {
                self.env.error(&cmp_loc, "cannot find `std::cmp::compare` function. Include `move-stdlib` for compilation.");
                let invalid_id = self.env.new_node(self.env.internal_loc(), Type::Error);
                return Some(ExpData::Invalid(invalid_id).into_exp());
            },
        };

        // Create a new node sharing return type with `std::cmp::compare`
        let cmp_ty = compare_function.get_result_type();
        let new_cmp_node = self.env.new_node(cmp_loc, cmp_ty.clone());

        // `std::cmp::compare` takes a type parameter,
        // which should be instantiated with the type of the actual arguments
        self.env
            .set_node_instantiation(new_cmp_node, vec![expected_arg_ty]);

        Some(
            ExpData::Call(
                new_cmp_node,
                Operation::MoveFunction(cmp_module_id, compare_function.get_id()),
                args,
            )
            .into_exp(),
        )
    }

    /// Create a new immutable reference for the result of `std::cmp::compare`
    fn immborrow_compare_res(&self, call_cmp: Exp) -> Exp {
        let call_cmp_id = call_cmp.node_id();
        let cmp_loc = self.env.get_node_loc(call_cmp_id);
        let cmp_ty = self.env.get_node_type(call_cmp_id);
        let new_ref_type = Type::Reference(ReferenceKind::Immutable, Box::new(cmp_ty));
        let new_ref_id = self.env.new_node(cmp_loc, new_ref_type);

        ExpData::Call(
            new_ref_id,
            Operation::Borrow(ReferenceKind::Immutable),
            vec![call_cmp],
        )
        .into_exp()
    }

    /// Generate a final call to interpret the result of `std::cmp::compare`
    fn generate_call_to_final_res(
        &self,
        ori_op_id: NodeId,
        cmp_op: &Operation,
        args: Vec<Exp>,
    ) -> Option<Exp> {
        // Reuse the loc info of the original comparison operation
        let final_res_loc = self.env.get_node_loc(ori_op_id);

        // Check the `std::cmp` module
        let cmp_module = match self.cmp_module.as_ref() {
            Some(module) => module,
            None => {
                self.env
                    .error(&final_res_loc, "cannot find `std::cmp` module");
                let invalid_id = self.env.new_node(self.env.internal_loc(), Type::Error);
                return Some(ExpData::Invalid(invalid_id).into_exp());
            },
        };
        let cmp_module_id = cmp_module.get_id();

        let sym = match cmp_op {
            Operation::Lt => Self::IS_LT,
            Operation::Le => Self::IS_LE,
            Operation::Gt => Self::IS_GT,
            Operation::Ge => Self::IS_GE,
            _ => return None,
        };

        // Check the `std::cmp::is_lt/is_le/is_gt/is_ge` function
        let final_res_function = match self.cmp_functions.get(sym) {
            Some(func) => func,
            None => {
                self.env.error(
                    &final_res_loc,
                    format!("cannot find `std::cmp::{}` function", sym).as_str(),
                );
                let invalid_id = self.env.new_node(self.env.internal_loc(), Type::Error);
                return Some(ExpData::Invalid(invalid_id).into_exp());
            },
        };

        let final_res_ty = Type::Primitive(PrimitiveType::Bool);
        let final_res_id = self.env.new_node(final_res_loc, final_res_ty.clone());
        Some(
            ExpData::Call(
                final_res_id,
                Operation::MoveFunction(cmp_module_id, final_res_function.get_id()),
                args,
            )
            .into_exp(),
        )
    }

    /// We cannot rewrite integer primitive types and "Num"
    /// - Integer primitive types are supported by the VM natively
    /// - "Num" is for spec only
    fn arg_cannot_transform(&mut self, arg: &Exp) -> bool {
        let arg_ty = self.env.get_node_type(arg.as_ref().node_id());
        arg_ty.is_number()
    }

    /// Obtain a reference to the argument
    fn rewrite_cmp_arg(&mut self, arg: &Exp) -> Exp {
        // Deref a reference, return the inner reference
        if let Some(arg_ref) = self.remove_deref_from_arg(arg) {
            return arg_ref;
        }
        let node_id = arg.as_ref().node_id();
        let arg_loc = self.env.get_node_loc(node_id);
        let arg_ty = self.env.get_node_type(node_id);

        match arg_ty {
            // Already a immutable reference, return it directly
            Type::Reference(ReferenceKind::Immutable, _) => arg.clone(),
            // Already a mutable reference, freeze and return it
            Type::Reference(ReferenceKind::Mutable, inner_ty) => {
                let new_ref_type = Type::Reference(ReferenceKind::Immutable, Box::new(*inner_ty));
                let new_ref_id = self.env.new_node(arg_loc, new_ref_type);
                ExpData::Call(new_ref_id, Operation::Freeze(false), vec![arg.clone()]).into_exp()
            },
            _ => {
                // Not a reference, create a new immutable reference
                let new_ref_type =
                    Type::Reference(ReferenceKind::Immutable, Box::new(arg_ty.clone()));
                let new_ref_id = self.env.new_node(arg_loc, new_ref_type);
                ExpData::Call(
                    new_ref_id,
                    Operation::Borrow(ReferenceKind::Immutable),
                    vec![arg.clone()],
                )
                .into_exp()
            },
        }
    }

    /// Optimization: if the argument is a dereference operation, we get the inner reference directly
    fn remove_deref_from_arg(&mut self, arg: &Exp) -> Option<Exp> {
        if let ExpData::Call(_, Operation::Deref, deref_args) = arg.as_ref() {
            debug_assert!(
                deref_args.len() == 1,
                "there should be exactly one argument for dereference"
            );
            let deref_arg_ty = self.env.get_node_type(deref_args[0].as_ref().node_id());
            if let Type::Reference(ReferenceKind::Immutable, _) = deref_arg_ty {
                // If the deref argument is an immutable reference, we can return it directly
                return Some(deref_args[0].clone());
            }
        }
        None
    }

    /// Find the `std::cmp` module
    fn find_cmp_module(env: &'env GlobalEnv) -> Option<ModuleEnv<'env>> {
        // Find the `std::cmp` module
        let cmp_module_name = ModuleName::new(
            Address::Numerical(AccountAddress::ONE),
            env.symbol_pool().make(well_known::CMP_MODULE),
        );
        env.find_module(&cmp_module_name)
    }

    /// Find function from the `std::cmp` module
    fn find_cmp_function(
        cmp_module: &ModuleEnv<'env>,
        func_name: &str,
    ) -> Option<FunctionEnv<'env>> {
        let compare_sym = cmp_module.symbol_pool().make(func_name);
        cmp_module.find_function(compare_sym)
    }
}
