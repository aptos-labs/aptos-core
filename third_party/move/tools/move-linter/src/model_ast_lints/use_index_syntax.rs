// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Detects cases where concise index notation syntax can be used for vectors and global storage.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv, Loc, NodeId, SurfaceSyntax},
    well_known::{VECTOR_BORROW, VECTOR_BORROW_MUT},
};
use std::collections::BTreeSet;

const LINT_NAME: &str = "use_index_syntax";
const VECTOR_INDEX_NOTATION_URL: &str =
    "https://aptos.dev/build/smart-contracts/book/vector#index-notation-for-vectors";
const GLOBAL_STORAGE_INDEX_NOTATION_URL: &str =
    "https://aptos.dev/build/smart-contracts/book/global-storage-operators#index-notation-for-storage-operators";

#[derive(Default)]
pub struct UseIndexSyntax {
    /// NodeIds already reported, to avoid duplicate warnings.
    reported_in_context: BTreeSet<NodeId>,
}

#[derive(Clone, Copy)]
enum VerboseBorrowKind {
    Vector,
    Global,
}

/// If the expression is a verbose call to `vector::borrow` or `vector::borrow_mut`
/// (not from index notation desugaring), returns the call's NodeId.
fn is_verbose_vector_borrow(env: &GlobalEnv, expr: &ExpData) -> Option<NodeId> {
    let ExpData::Call(id, Operation::MoveFunction(mid, fid), _) = expr else {
        return None;
    };
    if env.has_surface_syntax(*id, SurfaceSyntax::IndexNotation) {
        return None;
    }
    let func_env = env.get_module(*mid).into_function(*fid);
    if func_env.is_well_known(VECTOR_BORROW) || func_env.is_well_known(VECTOR_BORROW_MUT) {
        Some(*id)
    } else {
        None
    }
}

/// If the expression is a verbose `borrow_global` or `borrow_global_mut` call
/// (not from index notation desugaring), returns the call's NodeId.
fn is_verbose_global_borrow(env: &GlobalEnv, expr: &ExpData) -> Option<NodeId> {
    let ExpData::Call(id, Operation::BorrowGlobal(_), _) = expr else {
        return None;
    };
    if env.has_surface_syntax(*id, SurfaceSyntax::IndexNotation) {
        return None;
    }
    Some(*id)
}

/// Checks if an expression is a verbose borrow (vector or global) that could use index syntax.
fn is_verbose_borrow(env: &GlobalEnv, expr: &ExpData) -> Option<(NodeId, VerboseBorrowKind)> {
    is_verbose_vector_borrow(env, expr)
        .map(|id| (id, VerboseBorrowKind::Vector))
        .or_else(|| is_verbose_global_borrow(env, expr).map(|id| (id, VerboseBorrowKind::Global)))
}

fn report_index_syntax(env: &GlobalEnv, loc: &Loc, kind: VerboseBorrowKind) {
    let (description, url) = match kind {
        VerboseBorrowKind::Vector => ("vector", VECTOR_INDEX_NOTATION_URL),
        VerboseBorrowKind::Global => ("global storage", GLOBAL_STORAGE_INDEX_NOTATION_URL),
    };
    env.lint_diag_with_notes(
        loc,
        &format!("concise {description} index notation syntax can be used here instead."),
        vec![
            format!(
                "Concise {description} index notation syntax is described here: {url}.",
            ),
            format!(
                "To suppress this warning, annotate the function/module with the attribute `#[lint::skip({})]`.",
                LINT_NAME,
            ),
        ],
    );
}

impl ExpChecker for UseIndexSyntax {
    fn get_name(&self) -> String {
        LINT_NAME.to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();

        // Pattern 1: `*vector::borrow(v, i)` or `*borrow_global<T>(addr)`
        if let ExpData::Call(deref_id, Operation::Deref, args) = expr {
            if let Some(inner) = args.first() {
                if let Some((inner_id, kind)) = is_verbose_borrow(env, inner.as_ref()) {
                    report_index_syntax(env, &env.get_node_loc(*deref_id), kind);
                    self.reported_in_context.insert(inner_id);
                    return;
                }
            }
        }

        // Pattern 2: `vector::borrow(v, i).field` or `borrow_global<T>(addr).field`
        if let ExpData::Call(select_id, Operation::Select(_, _, _), args) = expr {
            if self.reported_in_context.contains(select_id) {
                return;
            }
            if let Some(inner) = args.first() {
                if let Some((inner_id, kind)) = is_verbose_borrow(env, inner.as_ref()) {
                    report_index_syntax(env, &env.get_node_loc(*select_id), kind);
                    self.reported_in_context.insert(inner_id);
                    return;
                }
            }
        }

        // Pattern 3: `*vector::borrow_mut(v, i) = x` or `borrow_global_mut<T>(addr).field = x`
        if let ExpData::Mutate(mutate_id, lhs, _) = expr {
            if let Some((inner_id, kind)) = is_verbose_borrow(env, lhs.as_ref()) {
                report_index_syntax(env, &env.get_node_loc(*mutate_id), kind);
                self.reported_in_context.insert(inner_id);
                return;
            }
            if let ExpData::Call(select_id, Operation::Select(_, _, _), select_args) = lhs.as_ref()
            {
                if let Some(inner) = select_args.first() {
                    if let Some((inner_id, kind)) = is_verbose_borrow(env, inner.as_ref()) {
                        report_index_syntax(env, &env.get_node_loc(*mutate_id), kind);
                        self.reported_in_context.insert(*select_id);
                        self.reported_in_context.insert(inner_id);
                        return;
                    }
                }
            }
        }

        // Pattern 4: `vector::borrow(v, i)` or `borrow_global<T>(addr)` (bare, not wrapped)
        if let Some((id, kind)) = is_verbose_borrow(env, expr) {
            if self.reported_in_context.contains(&id) {
                return;
            }
            report_index_syntax(env, &env.get_node_loc(id), kind);
        }
    }
}
