// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module contains the interface for externally specified checks
//! that can be run by the Move compiler.

use legacy_move_compiler::shared::known_attributes::LintAttribute;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, Loc, NamedConstantEnv, StructEnv},
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

    /// Get all the constant checkers.
    fn get_constant_checkers(&self) -> Vec<Box<dyn ConstantChecker>>;

    /// Get all the struct checkers.
    fn get_struct_checkers(&self) -> Vec<Box<dyn StructChecker>>;

    /// Get all the function checkers.
    fn get_function_checkers(&self) -> Vec<Box<dyn FunctionChecker>>;

    /// Get the names of all known checkers across all categories.
    fn get_all_checker_names(&self) -> BTreeSet<String>;
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
        let constant_checkers = self
            .get_constant_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        let struct_checkers = self
            .get_struct_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        let function_checkers = self
            .get_function_checkers()
            .into_iter()
            .map(|c| c.get_name())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "dyn ExternalChecks {{ exp_checkers: [{}], stackless_bytecode_checkers: [{}], constant_checkers: [{}], struct_checkers: [{}], function_checkers: [{}] }}",
            exp_checkers, stackless_bytecode_checkers, constant_checkers, struct_checkers, function_checkers
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

/// Implement this trait for checks on individual constants.
pub trait ConstantChecker {
    /// Name of this checker.
    fn get_name(&self) -> String;

    /// Examine `const_env` and potentially emit reports via `self.report()`.
    fn check_constant(&self, const_env: &NamedConstantEnv);

    /// Report the `msg` highlighting the `loc`.
    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        report(env, loc, msg, self.get_name().as_str());
    }
}

/// Implement this trait for checks on individual structs/enums.
pub trait StructChecker {
    /// Name of this checker.
    fn get_name(&self) -> String;

    /// Examine `struct_env` and potentially emit reports via `self.report()`.
    fn check_struct(&self, struct_env: &StructEnv);

    /// Report the `msg` highlighting the `loc`.
    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        report(env, loc, msg, self.get_name().as_str());
    }
}

/// Implement this trait for checks on individual functions.
pub trait FunctionChecker {
    /// Name of this checker.
    fn get_name(&self) -> String;

    /// Examine `func_env` and potentially emit reports via `self.report()`.
    fn check_function(&self, func_env: &FunctionEnv);

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

/// Get the set of known checker names from the given external checkers.
pub fn known_checker_names(external_checkers: &Vec<Arc<dyn ExternalChecks>>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for checkers in external_checkers {
        names.extend(checkers.get_all_checker_names());
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
