// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use codespan_reporting::diagnostic::Severity;
use move_command_line_common::env::read_env_var;
use move_compiler::command_line as cli;
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

/// Defines options for a run of the compiler.
#[derive(Parser, Clone, Debug)]
#[clap(author, version, about)]
pub struct Options {
    /// Directories where to lookup dependencies.
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
    /// See `Experiment` struct.
    #[clap(short)]
    #[clap(
        long = "experiment",
        num_args = 0..
    )]
    pub experiments: Vec<String>,
    /// A transient cache for memoization of experiment checks.
    #[clap(skip)]
    pub experiment_cache: RefCell<BTreeMap<String, bool>>,
    /// Sources to compile (positional arg, therefore last)
    pub sources: Vec<String>,
    /// Show warnings about unused functions, fields, constants, etc.
    /// Note that the current value of this constant is "Wunused"
    #[clap(long = cli::WARN_UNUSED_FLAG, default_value="false")]
    pub warn_unused: bool,
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

    /// Returns true if an experiment is on.
    pub fn experiment_on(&self, name: &str) -> bool {
        let on_opt = self.experiment_cache.borrow().get(name).cloned();
        if let Some(on) = on_opt {
            on
        } else {
            let on = self.experiments.iter().any(|s| s == name)
                || compiler_exp_var().iter().any(|s| s == name);
            self.experiment_cache
                .borrow_mut()
                .insert(name.to_string(), on);
            on
        }
    }
}

fn compiler_exp_var() -> Vec<String> {
    static EXP_VAR: Lazy<Vec<String>> = Lazy::new(|| {
        let s = read_env_var("MOVE_COMPILER_EXP");
        s.split(',').map(|s| s.to_string()).collect()
    });
    (*EXP_VAR).clone()
}
