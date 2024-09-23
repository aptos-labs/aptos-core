// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for dereferences of
//! mutable and immutable borrows that are needless and thus can be removed.
//! E.g., `*&x` can be simplified to `x`.
//!       `*&mut x` can be simplified to `x`.
//!       `*&mut y.f = 5;` can be simplified to `y.f = 5;`.

use crate::{env_pipeline::model_ast_lints::ExpressionLinter, lint_common::LintChecker};
use move_model::{
    ast::{ExpData, Operation},
    model::{GlobalEnv, NodeId},
    ty::ReferenceKind,
};

#[derive(Default)]
pub struct NeedlessDerefRef;

impl ExpressionLinter for NeedlessDerefRef {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::NeedlessDerefRef
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        if let Some((id, kind)) = Self::needless_deref_ref_pair(expr) {
            self.warning(
                env,
                &env.get_node_loc(id),
                &format!(
                    "Needless pair of `*` and {} operators: consider removing them",
                    kind
                ),
            );
        }
    }
}

impl NeedlessDerefRef {
    /// Check if `expr` is a pair of needless dereference and borrow operators.
    /// If so, return the node id of the expression and the kind of reference.
    /// Otherwise, return `None`.
    fn needless_deref_ref_pair(expr: &ExpData) -> Option<(NodeId, ReferenceKind)> {
        use ExpData::{Call, LocalVar, Mutate, Temporary};
        use Operation::{Borrow, Deref, Select, SelectVariants};
        match expr {
            Call(id, Deref, args) => {
                debug_assert!(
                    args.len() == 1,
                    "there should be exactly one argument for dereference"
                );
                let Call(_, Operation::Borrow(kind), ..) = args[0].as_ref() else {
                    return None;
                };
                Some((*id, *kind))
            },
            Mutate(id, lhs, _) => {
                let Call(_, Borrow(kind), args) = lhs.as_ref() else {
                    return None;
                };
                debug_assert!(
                    args.len() == 1,
                    "there should be exactly one argument for borrow"
                );
                // Below, we look for patterns of the form:
                //   *&mut x = value;
                //   *&mut y.f = value;
                // In all these cases, `*&mut` can safely be removed.
                if let Call(_, Select(..) | SelectVariants(..), _) | LocalVar(..) | Temporary(..) =
                    args[0].as_ref()
                {
                    Some((*id, *kind))
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}
