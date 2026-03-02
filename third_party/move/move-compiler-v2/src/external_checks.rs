// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains the interface for externally specified checks
//! that can be run by the Move compiler.

use legacy_move_compiler::shared::known_attributes::LintAttribute;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, StructEnv},
};
use move_stackless_bytecode::function_target::FunctionTarget;
use std::{collections::BTreeSet, fmt, sync::Arc};

/// Base URL for the linter documentation.
const LINTER_URL_BASE: &str = "https://aptos.dev/en/build/smart-contracts/linter";

/// Implement this trait to provide a collection of external checks.
pub trait ExternalChecks {
    /// Get all the expression checkers.
    fn get_exp_checkers(&self) -> Vec<Box<dyn ExpChecker>>;

    /// Get all the stackless bytecode checkers.
    fn get_stackless_bytecode_checkers(&self) -> Vec<Box<dyn StacklessBytecodeChecker>>;

    /// Get all the module-level checkers.
    fn get_module_checkers(&self) -> Vec<Box<dyn ModuleChecker>> {
        vec![]
    }
}

impl fmt::Debug for dyn ExternalChecks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let exp_checkers = self
            .get_exp_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        let stackless_bytecode_checkers = self
            .get_stackless_bytecode_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        let module_checkers = self
            .get_module_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "dyn ExternalChecks {{ exp_checkers: [{}], stackless_bytecode_checkers: [{}], module_checkers: [{}] }}",
            exp_checkers, stackless_bytecode_checkers, module_checkers
        )
    }
}

/// Implement this trait for checks that can be performed by looking at an
/// expression as we traverse the model AST.
/// Implement at least one of the `visit` methods to be a useful checker.
pub trait ExpChecker {
    /// Name of the expression checker.
    fn get_name(&self) -> String;

    /// Examine `expr` before any of its children have been visited.
    /// Potentially emit reports using `self.report()`.
    fn visit_expr_pre(&mut self, _function: &FunctionEnv, _expr: &ExpData) {}

    /// Examine `expr` after all its children have been visited.
    /// Potentially emit reports using `self.report()`.
    fn visit_expr_post(&mut self, _function: &FunctionEnv, _expr: &ExpData) {}

    /// Report the `msg` highlighting the `loc`.
    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        report(env, loc, msg, self.get_name().as_str());
    }
}

/// Implement this trait for checks that are performed on the stackless bytecode.
pub trait StacklessBytecodeChecker {
    /// Name of the stackless bytecode checker.
    fn get_name(&self) -> String;

    /// Examine the `target` and potentially emit reports via `self.report()`.
    fn check(&self, target: &FunctionTarget);

    /// Report the `msg` highlighting the `loc`.
    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        report(env, loc, msg, self.get_name().as_str());
    }
}

/// Implement this trait for checks on module-level declarations (modules, functions, constants,
/// structs). Unlike `ExpChecker` which visits expressions within function bodies, this trait
/// visits the declarations themselves.
/// Implement at least one of the `visit` methods to be a useful checker.
pub trait ModuleChecker {
    /// Name of the module checker.
    fn get_name(&self) -> String;

    /// Examine a module declaration. Called once per module.
    fn visit_module(&self, _env: &GlobalEnv, _module: &ModuleEnv) {}

    /// Examine a function declaration (not its body).
    fn visit_function(&self, _env: &GlobalEnv, _func: &FunctionEnv) {}

    /// Examine a named constant declaration.
    fn visit_named_constant(&self, _env: &GlobalEnv, _constant: &NamedConstantEnv) {}

    /// Examine a struct or enum declaration.
    fn visit_struct(&self, _env: &GlobalEnv, _struct_env: &StructEnv) {}

    /// Report the `msg` highlighting the `loc`.
    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        report(env, loc, msg, self.get_name().as_str());
    }
}

/// Get the set of known checker names from the given external checkers.
pub fn known_checker_names(external_checkers: &Vec<Arc<dyn ExternalChecks>>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for checkers in external_checkers {
        for checker in checkers.get_exp_checkers() {
            names.insert(checker.get_name());
        }
        for checker in checkers.get_stackless_bytecode_checkers() {
            names.insert(checker.get_name());
        }
        for checker in checkers.get_module_checkers() {
            names.insert(checker.get_name());
        }
    }
    names
}

/// Report the `msg` highlighting the `loc` for the `checker_name`.
fn report(env: &GlobalEnv, loc: &Loc, msg: &str, checker_name: &str) {
    env.lint_diag_with_notes(loc, msg, vec![
        format!(
        "To suppress this warning, annotate the function/module with the attribute `#[{}({})]`.",
        LintAttribute::SKIP,
        checker_name
    ),
        format!(
            "For more information, see {}#{}.",
            LINTER_URL_BASE, checker_name
        ),
    ]);
}
