// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module implements linters that check for usage of deprecated items
//! (functions, structs, modules).
//!
//! Items annotated with `#[deprecated]` should trigger a warning when used
//! from non-deprecated code. An item is considered deprecated if it has
//! `#[deprecated]` directly OR its enclosing module does.
//!
//! Warnings are suppressed when the usage occurs within a function or module
//! that is itself deprecated.
//!
//! The following checkers are provided:
//! - `DeprecatedUsage` (`ExpChecker`): checks expressions for deprecated calls,
//!   struct operations, global ops, patterns, etc.
//! - `DeprecatedUsageInSignatures` (`FunctionChecker`): checks function parameter
//!   and return types for deprecated structs.
//! - `DeprecatedUsageInFields` (`StructChecker`): checks struct field types for
//!   deprecated structs.
//! - `DeprecatedUsageOfConstants` (`ConstantChecker`): for deprecated constants,
//!   checks their users and warns at each non-deprecated usage site.

use legacy_move_compiler::shared::known_attributes::LintAttribute;
use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, FunctionChecker, StructChecker,
};
use move_model::{
    ast::{Attribute, ExpData, Operation, Pattern},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleId, NamedConstantEnv, NodeId, StructId, UserId,
    },
    ty::Type,
};

const CHECKER_NAME: &str = "deprecated_usage";

// =====================================================================
// Helper functions
// =====================================================================

fn is_deprecated_attr(env: &GlobalEnv, attr: &Attribute) -> bool {
    let s = env.symbol_pool().string(attr.name());
    s.as_str() == "deprecated"
}

fn is_deprecated(env: &GlobalEnv, attrs: &[Attribute]) -> bool {
    Attribute::has(attrs, |a| is_deprecated_attr(env, a))
}

/// Check if attributes contain `#[lint::skip(deprecated_usage)]`.
fn has_lint_skip_deprecated(env: &GlobalEnv, attrs: &[Attribute]) -> bool {
    let lint_skip = env.symbol_pool().make(LintAttribute::SKIP);
    attrs.iter().any(|attr| {
        if let Attribute::Apply(_, name, args) = attr {
            *name == lint_skip
                && args
                    .iter()
                    .any(|a| env.symbol_pool().string(a.name()).as_str() == CHECKER_NAME)
        } else {
            false
        }
    })
}

fn is_function_deprecated(env: &GlobalEnv, mid: ModuleId, fid: FunId) -> bool {
    let module_env = env.get_module(mid);
    is_deprecated(env, module_env.get_function(fid).get_attributes())
        || is_deprecated(env, module_env.get_attributes())
}

fn is_struct_deprecated(env: &GlobalEnv, mid: ModuleId, sid: StructId) -> bool {
    let module_env = env.get_module(mid);
    is_deprecated(env, module_env.get_struct(sid).get_attributes())
        || is_deprecated(env, module_env.get_attributes())
}

fn is_constant_deprecated(env: &GlobalEnv, const_env: &NamedConstantEnv) -> bool {
    is_deprecated(env, const_env.get_attributes())
        || is_deprecated(env, const_env.module_env.get_attributes())
}

/// Report if a struct is deprecated, using `report_fn`.
fn report_struct_deprecated(
    env: &GlobalEnv,
    mid: ModuleId,
    sid: StructId,
    loc: &Loc,
    report_fn: &dyn Fn(&GlobalEnv, &Loc, &str),
) {
    if is_struct_deprecated(env, mid, sid) {
        let module_env = env.get_module(mid);
        let struct_env = module_env.get_struct(sid);
        report_fn(
            env,
            loc,
            &format!(
                "use of deprecated struct `{}`",
                struct_env.get_full_name_with_address(),
            ),
        );
    }
}

/// Recursively walk a type, reporting any deprecated structs found within.
fn check_type_for_deprecated(
    env: &GlobalEnv,
    ty: &Type,
    loc: &Loc,
    report_fn: &dyn Fn(&GlobalEnv, &Loc, &str),
) {
    match ty {
        Type::Struct(mid, sid, type_args) => {
            report_struct_deprecated(env, *mid, *sid, loc, report_fn);
            for arg in type_args {
                check_type_for_deprecated(env, arg, loc, report_fn);
            }
        },
        Type::Vector(inner) => {
            check_type_for_deprecated(env, inner, loc, report_fn);
        },
        Type::Reference(_, inner) => {
            check_type_for_deprecated(env, inner, loc, report_fn);
        },
        Type::Tuple(types) => {
            for t in types {
                check_type_for_deprecated(env, t, loc, report_fn);
            }
        },
        Type::Fun(args, result, _) => {
            check_type_for_deprecated(env, args, loc, report_fn);
            check_type_for_deprecated(env, result, loc, report_fn);
        },
        // TypeDomain and ResourceDomain only appear in specifications; skip deprecation checks.
        Type::TypeDomain(..) | Type::ResourceDomain(..) => {},
        Type::Primitive(_) | Type::TypeParameter(_) | Type::Error | Type::Var(_) => {},
    }
}

// =====================================================================
// ExpChecker: checks expressions for deprecated usage
// =====================================================================

#[derive(Default)]
pub struct DeprecatedUsage {
    /// Whether the enclosing function is itself deprecated (directly or via its module).
    /// `None` means we haven't checked yet; computed lazily on the first visited expression.
    in_deprecated: Option<bool>,
}

impl DeprecatedUsage {
    fn report_deprecated_function(&self, env: &GlobalEnv, mid: ModuleId, fid: FunId, loc: &Loc) {
        if is_function_deprecated(env, mid, fid) {
            let module_env = env.get_module(mid);
            let func_env = module_env.get_function(fid);
            self.report(
                env,
                loc,
                &format!(
                    "use of deprecated function `{}`",
                    func_env.get_full_name_with_address(),
                ),
            );
        }
    }

    fn report_deprecated_struct(&self, env: &GlobalEnv, mid: ModuleId, sid: StructId, loc: &Loc) {
        if is_struct_deprecated(env, mid, sid) {
            let module_env = env.get_module(mid);
            let struct_env = module_env.get_struct(sid);
            self.report(
                env,
                loc,
                &format!(
                    "use of deprecated struct `{}`",
                    struct_env.get_full_name_with_address(),
                ),
            );
        }
    }

    fn check_pattern_deprecated(&self, env: &GlobalEnv, pattern: &Pattern) {
        pattern.visit_pre_post(&mut |entering, pat| {
            if entering && let Pattern::Struct(id, qid, _, _) = pat {
                let loc = env.get_node_loc(*id);
                self.report_deprecated_struct(env, qid.module_id, qid.id, &loc);
            }
        });
    }

    fn extract_struct_from_type_instantiation(
        env: &GlobalEnv,
        node_id: NodeId,
    ) -> Option<(ModuleId, StructId)> {
        let inst = env.get_node_instantiation(node_id);
        if let Some(Type::Struct(mid, sid, _)) = inst.first() {
            Some((*mid, *sid))
        } else {
            None
        }
    }
}

impl ExpChecker for DeprecatedUsage {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();

        let in_deprecated = *self.in_deprecated.get_or_insert_with(|| {
            is_function_deprecated(env, function.module_env.get_id(), function.get_id())
        });
        if in_deprecated {
            return;
        }

        match expr {
            ExpData::Call(id, op, _) => {
                let loc = env.get_node_loc(*id);
                match op {
                    Operation::MoveFunction(mid, fid) => {
                        self.report_deprecated_function(env, *mid, *fid, &loc);
                    },
                    Operation::Closure(mid, fid, _) => {
                        self.report_deprecated_function(env, *mid, *fid, &loc);
                    },
                    Operation::Pack(mid, sid, _) => {
                        self.report_deprecated_struct(env, *mid, *sid, &loc);
                    },
                    Operation::Select(mid, sid, _) => {
                        self.report_deprecated_struct(env, *mid, *sid, &loc);
                    },
                    Operation::SelectVariants(mid, sid, _) => {
                        self.report_deprecated_struct(env, *mid, *sid, &loc);
                    },
                    Operation::TestVariants(mid, sid, _) => {
                        self.report_deprecated_struct(env, *mid, *sid, &loc);
                    },
                    Operation::Exists(_)
                    | Operation::BorrowGlobal(_)
                    | Operation::MoveTo
                    | Operation::MoveFrom => {
                        if let Some((mid, sid)) =
                            Self::extract_struct_from_type_instantiation(env, *id)
                        {
                            self.report_deprecated_struct(env, mid, sid, &loc);
                        }
                    },
                    _ => {},
                }
            },
            ExpData::Match(_, _, arms) => {
                for arm in arms {
                    self.check_pattern_deprecated(env, &arm.pattern);
                }
            },
            ExpData::Block(_, pattern, _, _) => {
                self.check_pattern_deprecated(env, pattern);
            },
            _ => {},
        }
    }
}

// =====================================================================
// FunctionChecker: checks function param/return types for deprecated structs
// =====================================================================

#[derive(Default)]
pub struct DeprecatedUsageInSignatures;

impl FunctionChecker for DeprecatedUsageInSignatures {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func_env: &FunctionEnv) {
        let env = func_env.env();

        // Skip if the function or its module is itself deprecated.
        if is_function_deprecated(env, func_env.module_env.get_id(), func_env.get_id()) {
            return;
        }

        let report = |env: &GlobalEnv, loc: &Loc, msg: &str| {
            self.report(env, loc, msg);
        };

        // Check parameter types.
        for param in func_env.get_parameters_ref() {
            check_type_for_deprecated(env, &param.1, &param.2, &report);
        }

        // Check return type.
        let result_type = func_env.get_result_type();
        let result_loc = func_env.get_result_type_loc();
        check_type_for_deprecated(env, &result_type, &result_loc, &report);
    }
}

// =====================================================================
// StructChecker: checks struct field types for deprecated structs
// =====================================================================

#[derive(Default)]
pub struct DeprecatedUsageInFields;

impl StructChecker for DeprecatedUsageInFields {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_struct(&self, struct_env: &move_model::model::StructEnv) {
        let env = struct_env.module_env.env;

        // Skip if the struct or its module is itself deprecated.
        if is_struct_deprecated(env, struct_env.module_env.get_id(), struct_env.get_id()) {
            return;
        }

        let report = |env: &GlobalEnv, loc: &Loc, msg: &str| {
            self.report(env, loc, msg);
        };

        for field in struct_env.get_fields() {
            let field_ty = field.get_type();
            let field_loc = field.get_loc();
            check_type_for_deprecated(env, &field_ty, field_loc, &report);
        }
    }
}

// =====================================================================
// ConstantChecker: for deprecated constants, warns at each usage site
// =====================================================================

#[derive(Default)]
pub struct DeprecatedUsageOfConstants;

impl DeprecatedUsageOfConstants {
    /// Returns the location of a constant's user and whether warnings should be
    /// suppressed (because the user or its module is deprecated or has
    /// `#[lint::skip(deprecated_usage)]`).
    fn user_loc_and_should_suppress(env: &GlobalEnv, user: &UserId) -> Option<(Loc, bool)> {
        match user {
            UserId::Function(qfid) => {
                let func_env = env.get_function(*qfid);
                let suppress = is_function_deprecated(env, qfid.module_id, qfid.id)
                    || has_lint_skip_deprecated(env, func_env.get_attributes())
                    || has_lint_skip_deprecated(env, func_env.module_env.get_attributes());
                Some((func_env.get_id_loc(), suppress))
            },
            UserId::Struct(qsid) => {
                let struct_env = env.get_struct(*qsid);
                let suppress = is_struct_deprecated(env, qsid.module_id, qsid.id)
                    || has_lint_skip_deprecated(env, struct_env.get_attributes())
                    || has_lint_skip_deprecated(env, struct_env.module_env.get_attributes());
                Some((struct_env.get_loc(), suppress))
            },
            UserId::Constant(qcid) => {
                let const_env = env.get_module(qcid.module_id).into_named_constant(qcid.id);
                let suppress = is_constant_deprecated(env, &const_env)
                    || has_lint_skip_deprecated(env, const_env.get_attributes())
                    || has_lint_skip_deprecated(env, const_env.module_env.get_attributes());
                Some((const_env.get_loc(), suppress))
            },
        }
    }
}

impl ConstantChecker for DeprecatedUsageOfConstants {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    /// Note: this checker reports warnings at *user* sites, which may be in other modules.
    /// If the defining module has `#[lint::skip(deprecated_usage)]`, the lint framework will
    /// not call this method at all, which means cross-module warnings are also suppressed.
    /// This is acceptable: the module author is explicitly opting out of this lint.
    fn check_constant(&self, const_env: &NamedConstantEnv) {
        let env = const_env.module_env.env;

        if !is_constant_deprecated(env, const_env) {
            return;
        }

        let const_name = const_env.get_name().display(env.symbol_pool()).to_string();
        let module_name = const_env.module_env.get_full_name_str();

        for user in const_env.get_users() {
            if let Some((user_loc, suppress)) = Self::user_loc_and_should_suppress(env, user)
                && !suppress
            {
                let item_kind = match user {
                    UserId::Function(_) => "function",
                    UserId::Struct(_) => "struct",
                    UserId::Constant(_) => "constant",
                };
                self.report(
                    env,
                    &user_loc,
                    &format!(
                        "this {} uses deprecated constant `{}::{}`",
                        item_kind, module_name, const_name,
                    ),
                );
            }
        }
    }
}
