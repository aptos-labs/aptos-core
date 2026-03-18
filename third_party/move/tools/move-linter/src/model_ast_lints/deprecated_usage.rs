// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint checks that warn on usage of items annotated with `#[deprecated]`.
//!
//! The checks are split across checker types that match how usage is tracked:
//!
//! - **ExpChecker**: function calls and closures (not tracked elsewhere)
//! - **StructChecker**: all struct usage (bodies, signatures, fields, patterns, global ops)
//!   — relies on `struct_usage_collector` having populated `StructData.users`
//! - **ConstantChecker**: all constant usage — relies on model builder tracking
//! - **FunctionChecker**: attribute value references to deprecated items
//! - **ModuleChecker**: `use` declarations importing deprecated modules/members
//!
//! Suppressed when the enclosing item or its module is itself deprecated,
//! or when `#[lint::skip(deprecated_usage)]` is present.

use legacy_move_compiler::shared::known_attributes::DeprecationAttribute;
use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, FunctionChecker, ModuleChecker, StructChecker,
};
use move_model::{
    ast::{Attribute, AttributeValue, ExpData, Operation},
    model::{
        FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, NamedConstantEnv, StructEnv, StructId,
    },
};

// ── helpers ─────────────────────────────────────────────────────────────────

fn has_deprecated_attr(env: &GlobalEnv, attrs: &[Attribute]) -> bool {
    Attribute::has(attrs, |a| {
        env.symbol_pool().string(a.name()).as_str() == DeprecationAttribute::DEPRECATED_NAME
    })
}

fn is_struct_or_module_deprecated(env: &GlobalEnv, mid: ModuleId, sid: StructId) -> bool {
    let module_env = env.get_module(mid);
    has_deprecated_attr(env, module_env.get_attributes()) || {
        let struct_env = module_env.get_struct(sid);
        has_deprecated_attr(env, struct_env.get_attributes())
    }
}

fn deprecated_struct_message(env: &GlobalEnv, mid: ModuleId, sid: StructId) -> String {
    let module_env = env.get_module(mid);
    let struct_env = module_env.get_struct(sid);
    format!(
        "Use of deprecated struct `{}::{}`.",
        module_env.get_full_name_str(),
        env.symbol_pool().string(struct_env.get_name())
    )
}

fn lint_notes(checker_name: &str) -> Vec<String> {
    use legacy_move_compiler::shared::known_attributes::LintAttribute;
    vec![
        format!(
            "To suppress this warning, annotate the function/module with the attribute `#[{}({})]`.",
            LintAttribute::SKIP,
            checker_name
        ),
        format!(
            "For more information, see https://aptos.dev/en/build/smart-contracts/linter#{}.",
            checker_name
        ),
    ]
}

/// Walk attributes looking for references to deprecated modules/members.
fn check_attributes_for_deprecated(
    env: &GlobalEnv,
    attrs: &[Attribute],
    report: &dyn Fn(&GlobalEnv, &Loc, &str),
) {
    for attr in attrs {
        match attr {
            Attribute::Apply(_, _, nested) => {
                check_attributes_for_deprecated(env, nested, report);
            },
            Attribute::Assign(_, _, AttributeValue::Name(_, Some(module_name), member_sym)) => {
                if let Some(module_env) = env.find_module(module_name) {
                    let loc = env.get_node_loc(attr.node_id());
                    let pool = env.symbol_pool();
                    let member_str = pool.string(*member_sym);
                    if member_str.as_str().is_empty() {
                        // Module-only reference
                        if has_deprecated_attr(env, module_env.get_attributes()) {
                            report(
                                env,
                                &loc,
                                &format!(
                                    "Use of deprecated module `{}`.",
                                    module_env.get_full_name_str()
                                ),
                            );
                        }
                    } else {
                        // Module::member reference
                        if has_deprecated_attr(env, module_env.get_attributes()) {
                            report(
                                env,
                                &loc,
                                &format!(
                                    "Use of deprecated module `{}`.",
                                    module_env.get_full_name_str()
                                ),
                            );
                        } else {
                            for func in module_env.get_functions() {
                                if pool.string(func.get_name()).as_str() == member_str.as_str()
                                    && has_deprecated_attr(env, func.get_attributes())
                                {
                                    report(
                                        env,
                                        &loc,
                                        &format!(
                                            "Use of deprecated function `{}::{}`.",
                                            module_env.get_full_name_str(),
                                            member_str
                                        ),
                                    );
                                }
                            }
                            for s in module_env.get_structs() {
                                if pool.string(s.get_name()).as_str() == member_str.as_str()
                                    && has_deprecated_attr(env, s.get_attributes())
                                {
                                    report(
                                        env,
                                        &loc,
                                        &deprecated_struct_message(
                                            env,
                                            s.module_env.get_id(),
                                            s.get_id(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            },
            _ => {},
        }
    }
}

// ── ExpChecker: function calls only ─────────────────────────────────────────

#[derive(Default)]
pub struct DeprecatedUsageExpChecker;

impl ExpChecker for DeprecatedUsageExpChecker {
    fn get_name(&self) -> String {
        "deprecated_usage".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();
        // Don't warn inside deprecated code
        if has_deprecated_attr(env, function.get_attributes())
            || has_deprecated_attr(env, function.module_env.get_attributes())
        {
            return;
        }
        // Only function calls and closures — struct usage is handled by StructChecker
        if let ExpData::Call(id, Operation::MoveFunction(mid, fid), _)
        | ExpData::Call(id, Operation::Closure(mid, fid, _), _) = expr
        {
            let module_env = env.get_module(*mid);
            let is_deprecated = has_deprecated_attr(env, module_env.get_attributes()) || {
                let func_env = module_env.get_function(*fid);
                has_deprecated_attr(env, func_env.get_attributes())
            };
            if is_deprecated {
                let func_env = module_env.get_function(*fid);
                let loc = env.get_node_loc(*id);
                self.report(
                    env,
                    &loc,
                    &format!(
                        "Call to deprecated function `{}::{}`.",
                        module_env.get_full_name_str(),
                        env.symbol_pool().string(func_env.get_name())
                    ),
                );
            }
        }
    }
}

// ── FunctionChecker: attribute values ───────────────────────────────────────

#[derive(Default)]
pub struct DeprecatedUsageFuncChecker;

impl FunctionChecker for DeprecatedUsageFuncChecker {
    fn get_name(&self) -> String {
        "deprecated_usage".to_string()
    }

    fn check_function(&self, func_env: &FunctionEnv) {
        let env = func_env.env();
        if has_deprecated_attr(env, func_env.get_attributes())
            || has_deprecated_attr(env, func_env.module_env.get_attributes())
        {
            return;
        }
        let checker_name = self.get_name();
        check_attributes_for_deprecated(env, func_env.get_attributes(), &|env, loc, msg| {
            env.lint_diag_with_notes(loc, msg, lint_notes(&checker_name));
        });
    }
}

// ── StructChecker: all struct usage via get_users() ─────────────────────────

#[derive(Default)]
pub struct DeprecatedUsageStructChecker;

impl StructChecker for DeprecatedUsageStructChecker {
    fn get_name(&self) -> String {
        "deprecated_usage".to_string()
    }

    fn check_struct(&self, struct_env: &StructEnv) {
        let env = struct_env.module_env.env;
        // Only interested in deprecated structs (or structs in deprecated modules).
        if !is_struct_or_module_deprecated(env, struct_env.module_env.get_id(), struct_env.get_id())
        {
            return;
        }
        let msg =
            deprecated_struct_message(env, struct_env.module_env.get_id(), struct_env.get_id());
        for user_id in struct_env.get_users() {
            self.report(env, user_id.loc(), &msg);
        }
        // Also check attribute values on the struct itself
        let checker_name = self.get_name();
        check_attributes_for_deprecated(env, struct_env.get_attributes(), &|env, loc, msg| {
            env.lint_diag_with_notes(loc, msg, lint_notes(&checker_name));
        });
    }
}

// ── ConstantChecker: deprecated constant usage ──────────────────────────────

#[derive(Default)]
pub struct DeprecatedUsageConstantChecker;

impl ConstantChecker for DeprecatedUsageConstantChecker {
    fn get_name(&self) -> String {
        "deprecated_usage".to_string()
    }

    fn check_constant(&self, const_env: &NamedConstantEnv) {
        let env = const_env.module_env.env;
        if !has_deprecated_attr(env, const_env.get_attributes())
            && !has_deprecated_attr(env, const_env.module_env.get_attributes())
        {
            return;
        }
        let module_name = const_env.module_env.get_full_name_str();
        let const_name = env.symbol_pool().string(const_env.get_name());
        let msg = format!(
            "Use of deprecated constant `{}::{}`.",
            module_name, const_name
        );
        for user_id in const_env.get_users() {
            self.report(env, user_id.loc(), &msg);
        }
    }
}

// ── ModuleChecker: use declarations ─────────────────────────────────────────

#[derive(Default)]
pub struct DeprecatedUsageModuleChecker;

impl ModuleChecker for DeprecatedUsageModuleChecker {
    fn get_name(&self) -> String {
        "deprecated_usage".to_string()
    }

    fn check_module(&self, module_env: &ModuleEnv) {
        let env = module_env.env;
        if has_deprecated_attr(env, module_env.get_attributes()) {
            return;
        }
        for use_decl in module_env.get_use_decls() {
            let Some(used_mid) = use_decl.module_id else {
                continue;
            };
            let used_module = env.get_module(used_mid);
            if has_deprecated_attr(env, used_module.get_attributes()) {
                self.report(
                    env,
                    &use_decl.loc,
                    &format!(
                        "Use of deprecated module `{}`.",
                        used_module.get_full_name_str()
                    ),
                );
                continue;
            }
            let pool = env.symbol_pool();
            for (member_loc, member_name, _) in &use_decl.members {
                let name_str = pool.string(*member_name);
                for func in used_module.get_functions() {
                    if pool.string(func.get_name()).as_str() == name_str.as_str()
                        && has_deprecated_attr(env, func.get_attributes())
                    {
                        self.report(
                            env,
                            member_loc,
                            &format!(
                                "Import of deprecated function `{}::{}`.",
                                used_module.get_full_name_str(),
                                name_str
                            ),
                        );
                    }
                }
                for s in used_module.get_structs() {
                    if pool.string(s.get_name()).as_str() == name_str.as_str()
                        && has_deprecated_attr(env, s.get_attributes())
                    {
                        self.report(
                            env,
                            member_loc,
                            &format!(
                                "Import of deprecated struct `{}::{}`.",
                                used_module.get_full_name_str(),
                                name_str
                            ),
                        );
                    }
                }
            }
        }
    }
}
