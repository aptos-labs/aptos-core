// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Comparison operation rewriter
//! - Eq/Neq operation on values consumes them (which may require making copies)
//!   - On vector and struct, copy can be expensive
//!   - We insert references before vector/struct values to avoid copying
//! - We visit Eq/Neq operations and check if the arguments are values.
//!   - If they are, we insert an immutable borrow before them to get their references.
//!
//! Key structs/impls
//! - function `rewrite` is the entry point for this rewriting pass, which
//!   iterates over target functions and rewrites Eq/Neq inside
//!
//! - struct `CmpRewriter` implements the `ExpRewriterFunctions` trait
//!   - It overrides the `rewrite_call` method to handle Eq/Neq operations
//!    represented as a Call expression.
//!   - It also has a helper method `rewrite_cmp_arg` to wrap an argument with a reference.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use move_model::{
    ast::{Exp, ExpData, Operation},
    exp_rewriter::ExpRewriterFunctions,
    model::{FunId, GlobalEnv, NodeId, QualifiedId},
    ty::{ReferenceKind, Type},
};
use std::{collections::BTreeSet, iter::Iterator};

pub fn rewrite(env: &mut GlobalEnv) {
    // Get all to-be-compiled targets
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let todo: BTreeSet<_> = targets.keys().collect();
    for target in todo {
        if let RewriteTarget::MoveFun(func_id) = target {
            let new_def: Option<Exp> = rewrite_target(env, func_id);
            // Record rewritten functions
            if let Some(def) = new_def {
                *targets.state_mut(&target) = RewriteState::Def(def);
            }
        }
    }
    // Write back the rewritten functions
    targets.write_to_env(env);
}

fn rewrite_target(env: &GlobalEnv, func_id: QualifiedId<FunId>) -> Option<Exp> {
    let mut rewriter = CmpRewriter::new(env);
    let func_env = env.get_function(func_id);
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
}

impl<'env> CmpRewriter<'env> {
    fn new(env: &'env GlobalEnv) -> Self {
        Self { env }
    }

    fn rewrite_cmp_arg(&mut self, arg: &Exp) -> Exp {
        // Optimization: if the arg is Deref(Borrow(Immutable)), we return the inner reference directly
        if let Some(arg_ref) = self.remove_deref_from_arg(arg) {
            return arg_ref;
        }
        // Create a new expression to replace the argument
        let arg_loc = self.env.get_node_loc(arg.as_ref().node_id());
        let arg_ty = self.env.get_node_type(arg.as_ref().node_id());
        let new_ref_type = Type::Reference(ReferenceKind::Immutable, Box::new(arg_ty.clone()));
        let new_ref_id = self.env.new_node(arg_loc, new_ref_type);
        ExpData::Call(
            new_ref_id,
            Operation::Borrow(ReferenceKind::Immutable),
            vec![arg.clone()],
        )
        .into_exp()
    }

    // We only transform struct/vector arguments
    // Other types are omitted as copying them can be cheaper than creating a reference
    fn can_arg_transform(&mut self, arg: &Exp) -> bool {
        let arg_ty = self.env.get_node_type(arg.as_ref().node_id());
        matches!(arg_ty, Type::Vector(_)) || matches!(arg_ty, Type::Struct(_, _, _))
    }

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
}

impl ExpRewriterFunctions for CmpRewriter<'_> {
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        match oper {
            Operation::Eq | Operation::Neq => {
                // Check if all arguments are tranformable
                let transformable = args.iter().all(|exp| self.can_arg_transform(exp));
                if transformable {
                    let transformed_args =
                        args.iter().map(|exp| self.rewrite_cmp_arg(exp)).collect();
                    // Create a new call expression with the transformed arguments
                    return Some(ExpData::Call(call_id, oper.clone(), transformed_args).into_exp());
                }
                None
            },
            _ => None,
        }
    }
}
