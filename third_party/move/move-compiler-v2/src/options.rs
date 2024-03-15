// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experiments::{DefaultValue, EXPERIMENTS},
    external_checks::ExternalChecks,
};
use clap::Parser;
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use move_command_line_common::env::{bool_to_str, read_env_var, OVERRIDE_EXP_CACHE};
use move_compiler::{
    command_line as cli,
    shared::{
        move_compiler_warn_of_deprecation_use_env_var,
        warn_of_deprecation_use_in_aptos_libs_env_var,
    },
};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Defines options for a run of the compiler.
#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
pub struct Options {
    /// Directories where to lookup (already compiled) dependencies.
    #[clap(
        short,
        num_args = 0..
    )]
    pub dependencies: Vec<String>,

    /// Named address mapping.
    #[clap(
        short,
        num_args = 0..
    )]
    pub named_address_mapping: Vec<String>,

    /// Output directory.
    #[clap(short, long, default_value = "")]
    pub output_dir: String,

    /// The language version to use.
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,

    /// The compiler version to use.
    #[clap(long, value_parser = clap::value_parser!(CompilerVersion))]
    pub compiler_version: Option<CompilerVersion>,

    /// Do not complain about unknown attributes in Move code.
    #[clap(long, default_value = "false")]
    pub skip_attribute_checks: bool,

    /// Known attributes for this dialect of move; if empty, assumes third-party Move.
    /// Only used if skip_attribute_checks is false.
    #[clap(skip)]
    pub known_attributes: BTreeSet<String>,

    /// Whether we generate code for tests. This specifically guarantees stable output
    /// for baseline testing.
    #[clap(long)]
    pub testing: bool,

    /// Active experiments. Experiments alter default behavior of the compiler.
    /// Each element is `name[=on/off]` to enable/disable experiment `name`.
    /// See `Experiment` struct.
    #[clap(short, hide(true))]
    #[clap(
        long = "experiment",
        num_args = 0..
    )]
    pub experiments: Vec<String>,

    /// A transient cache for memoization of experiment checks.
    #[clap(skip)]
    pub experiment_cache: RefCell<BTreeMap<String, bool>>,

    /// Sources to compile (positional arg, therefore last).
    /// Each source should be a path to either (1) a Move file or (2) a directory containing Move
    /// files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`, `scripts`, and/or `tests`,
    /// depending on compilation mode).
    pub sources: Vec<String>,

    /// Dependencies to compile but not treat as a test/docgen/warning/prover target.
    /// Each source_dep should be a path to either (1) a Move file or (2) a directory containing
    /// Move files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`).
    #[clap(skip)]
    pub sources_deps: Vec<String>,

    /// Warn about use of deprecated functions, modules, etc.
    #[clap(long = cli::MOVE_COMPILER_WARN_OF_DEPRECATION_USE_FLAG,
           default_value=bool_to_str(move_compiler_warn_of_deprecation_use_env_var()))]
    pub warn_deprecated: bool,

    /// Show warnings about use of deprecated usage in the Aptos libraries,
    /// which we should generally not bother users with.
    /// Note that current value of this constant is "Wdeprecation-aptos"
    #[clap(hide(true), long = cli::WARN_OF_DEPRECATION_USE_IN_APTOS_LIBS_FLAG,
           default_value=bool_to_str(warn_of_deprecation_use_in_aptos_libs_env_var()))]
    pub warn_of_deprecation_use_in_aptos_libs: bool,

    /// Show warnings about unused functions, fields, constants, etc.
    /// Note that the current value of this constant is "Wunused"
    #[clap(long = cli::WARN_UNUSED_FLAG, default_value="false")]
    pub warn_unused: bool,

    /// Whether to compile everything, including dependencies.
    #[clap(long)]
    pub whole_program: bool,

    /// Whether to compile #[test] and #[test_only] code
    #[clap(skip)]
    pub compile_test_code: bool,

    /// Whether to compile #[verify_only] code
    #[clap(skip)]
    pub compile_verify_code: bool,

    /// External checks to be performed.
    #[clap(skip)]
    pub external_checks: Vec<Arc<dyn ExternalChecks>>,
}

impl Default for Options {
    fn default() -> Self {
        Parser::parse_from(std::iter::empty::<String>())
    }
}

impl Options {
    /// Returns the least severity of diagnosis which shall be reported.
    /// This is currently hardwired.
    pub fn report_severity(&self) -> Severity {
        Severity::Warning
    }

    /// Turns an experiment on or off, overriding command line, environment, and defaults.
    pub fn set_experiment(self, name: impl AsRef<str>, on: bool) -> Self {
        let name = name.as_ref().to_string();
        assert!(
            EXPERIMENTS.contains_key(&name),
            "experiment `{}` not declared",
            name
        );
        self.experiment_cache.borrow_mut().insert(name, on);
        self
    }

    /// Sets the language version to use.
    pub fn set_language_version(self, version: LanguageVersion) -> Self {
        Self {
            language_version: Some(version),
            ..self
        }
    }

    /// Returns true if an experiment is on.
    pub fn experiment_on(&self, name: &str) -> bool {
        self.experiment_on_recursive(name, &mut BTreeSet::new())
    }

    fn experiment_on_recursive(&self, name: &str, visited: &mut BTreeSet<String>) -> bool {
        if !visited.insert(name.to_string()) {
            panic!(
                "cyclic inheritance relation between experiments: `{} -> {}`",
                name,
                visited.iter().clone().join(",")
            )
        }
        let experiments_to_override = read_env_var(OVERRIDE_EXP_CACHE);
        let experiments = experiments_to_override
            .split(',')
            .map(|s| s.to_string())
            .collect_vec();
        if !experiments.contains(&name.to_string()) {
            if let Some(on) = self.experiment_cache.borrow().get(name).cloned() {
                return on;
            }
        }
        if let Some(exp) = EXPERIMENTS.get(&name.to_string()) {
            // First we look at experiments provided via the command line, second
            // via the env var, and last we take the configured default.
            let on = if let Some(on) = find_experiment(&self.experiments, name) {
                on
            } else if let Some(on) = find_experiment(&compiler_exp_var(), name) {
                on
            } else {
                match &exp.default {
                    DefaultValue::Given(on) => *on,
                    DefaultValue::Inherited(other_name) => {
                        self.experiment_on_recursive(other_name, visited)
                    },
                }
            };
            self.experiment_cache
                .borrow_mut()
                .insert(name.to_string(), on);
            on
        } else {
            panic!("unknown experiment `{}`", name)
        }
    }

    pub fn set_skip_attribute_checks(self, value: bool) -> Self {
        Self {
            skip_attribute_checks: value,
            ..self
        }
    }

    pub fn set_compile_test_code(self, value: bool) -> Self {
        Self {
            compile_test_code: value,
            ..self
        }
    }

    pub fn set_compile_verify_code(self, value: bool) -> Self {
        Self {
            compile_verify_code: value,
            ..self
        }
    }

    pub fn set_warn_deprecated(self, value: bool) -> Self {
        Self {
            warn_deprecated: value,
            ..self
        }
    }

    pub fn set_warn_of_deprecation_use_in_aptos_libs(self, value: bool) -> Self {
        Self {
            warn_of_deprecation_use_in_aptos_libs: value,
            ..self
        }
    }

    pub fn set_warn_unused(self, value: bool) -> Self {
        Self {
            warn_unused: value,
            ..self
        }
    }

    pub fn set_external_checks(self, value: Vec<Arc<dyn ExternalChecks>>) -> Self {
        Self {
            external_checks: value,
            ..self
        }
    }
}

/// Finds the experiment in the list of definitions. A definition
/// can be either `<exp_name>` in which case it is on, or
/// `<exp_name>=on/off`.
fn find_experiment(s: &[String], name: &str) -> Option<bool> {
    let mut result = None;
    for e in s {
        let mut parts = e.split('=');
        let exp_name = parts.next().unwrap();
        assert!(
            EXPERIMENTS.contains_key(exp_name),
            "undeclared experiment `{}`",
            exp_name
        );
        if exp_name == name {
            let on = if let Some(value) = parts.next() {
                match value.trim().to_lowercase().as_str() {
                    "on" => true,
                    "off" => false,
                    _ => panic!(
                        "invalid value for experiment `{}`: \
                        found `{}` but must be `on` or `off`",
                        name, value
                    ),
                }
            } else {
                // Default is on
                true
            };
            result = Some(on);
            // continue going to (1) check all experiments
            // in the list are actually declared (2) later
            // entries override earlier ones
        }
    }
    result
}

/// Gets the value of the env var for experiments.
fn compiler_exp_var() -> Vec<String> {
    static EXP_VAR: Lazy<Vec<String>> = Lazy::new(|| {
        for s in ["MVC_EXP", "MOVE_COMPILER_EXP"] {
            let s = read_env_var(s);
            if !s.is_empty() {
                return s.split(',').map(|s| s.to_string()).collect();
            }
        }
        vec![]
    });
    if !read_env_var(OVERRIDE_EXP_CACHE).is_empty() {
        for s in ["MVC_EXP", "MOVE_COMPILER_EXP"] {
            let s = read_env_var(s);
            if !s.is_empty() {
                return s.split(',').map(|s| s.to_string()).collect();
            }
        }
    }
    (*EXP_VAR).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_ref_cell_clone() {
        let test_key = "foo".to_owned();
        let options1 = Options::default();

        options1
            .experiment_cache
            .borrow_mut()
            .insert(test_key.clone(), false);

        let options2 = options1.clone();

        options1
            .experiment_cache
            .borrow_mut()
            .insert(test_key.clone(), true);

        let x1 = *(options1
            .experiment_cache
            .borrow()
            .get(&test_key)
            .expect("we just set foo"));
        assert!(x1);

        let x2 = *(options2
            .experiment_cache
            .borrow()
            .get(&test_key)
            .expect("we just set foo"));
        assert!(!x2);
    }
}
