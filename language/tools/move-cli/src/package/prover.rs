// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! Support for the prover in the package system.

use crate::package::cli;
use anyhow::bail;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use colored::Colorize;
use move_package::{BuildConfig, ModelConfig};
use move_prover::run_move_prover_with_model;
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};
use tempfile::TempDir;

// =================================================================================================
// API for Rust unit tests

/// Data representing the configuration of a prover test.
pub struct ProverTest {
    path: String,
    options: Vec<String>,
    local_only: bool,
}

impl ProverTest {
    /// Creates a new prover test for the Move package at path relative to crate root.
    pub fn create(path: impl Into<String>) -> Self {
        ProverTest {
            path: path.into(),
            options: vec![],
            local_only: false,
        }
    }

    /// Set specific prover options.
    pub fn with_options(self, options: &[&str]) -> Self {
        self.with_options_owned(options.iter().map(|s| s.to_string()).collect())
    }

    /// Set specific prover options, from vector of strings.
    pub fn with_options_owned(self, options: Vec<String>) -> Self {
        Self { options, ..self }
    }

    /// Restrict this test to only run locally (not in CI)
    pub fn with_local_only(self) -> Self {
        Self {
            local_only: true,
            ..self
        }
    }

    /// Run the prover test.
    pub fn run(mut self) {
        if self.local_only && in_ci() {
            return;
        }
        let pkg_path = path_in_crate(std::mem::take(&mut self.path));
        cli::handle_package_commands(
            &pkg_path,
            move_package::BuildConfig::default(),
            &cli::PackageCommand::Prove {
                target_filter: None,
                for_test: true,
                options: Some(cli::ProverOptions::Options(std::mem::take(
                    &mut self.options,
                ))),
            },
            vec![], // prover does not need natives
        )
        .unwrap()
    }
}

fn in_ci() -> bool {
    get_env("ENV_TEST_ON_CI") == "1"
}

/// Determine path in this crate. We can't use CARGO_MANIFEST_DIR for this because
/// we need the path of the caller. However, we can assume that cargo test
/// runs in the root dir of the crate, so we can just directly use the relative path.
fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    PathBuf::from(relative.into())
}

fn get_env(var: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| String::new())
}

// =================================================================================================
// Running the prover as a package command

pub fn run_move_prover(
    mut config: BuildConfig,
    path: &Path,
    target_filter: &Option<String>,
    for_test: bool,
    options: &[String],
) -> anyhow::Result<()> {
    // Always run the prover in dev mode, so addresses get default assignments
    config.dev_mode = true;

    let mut args = vec!["package".to_string()];
    let prover_toml = Path::new(&path).join("Prover.toml");
    if prover_toml.exists() {
        args.push(format!("--config={}", prover_toml.to_string_lossy()));
    }
    args.extend(options.iter().cloned());
    let mut options = move_prover::cli::Options::create_from_args(&args)?;
    if !options.move_sources.is_empty() {
        bail!(
            "move prover options must not specify sources as those are given \
                     by the package system. Did you meant to prefix `{}` with `-t`?",
            &options.move_sources[0]
        );
    }
    if !options.move_deps.is_empty() {
        bail!(
            "move prover options must not specify dependencies as those are given \
                     by the package system"
        );
    }
    if !options.move_named_address_values.is_empty() {
        bail!(
            "move prover options must not specify named addresses as those are given \
                     by the package system"
        );
    }

    let mut message_writer = StandardStream::stdout(ColorChoice::Auto);
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    if for_test {
        options.set_quiet();
        options.setup_logging_for_test();
    } else {
        options.setup_logging();
    }
    let now = Instant::now();
    let model = config.move_model_for_package(
        path,
        ModelConfig {
            all_files_as_targets: false,
            target_filter: target_filter.clone(),
        },
    )?;
    let _temp_dir_holder = if for_test {
        // Need to ensure a distinct output.bpl file for concurrent execution. In non-test
        // mode, we actually want to use the static output.bpl for debugging purposes
        let temp_dir = TempDir::new()?;
        std::fs::create_dir_all(temp_dir.path())?;
        options.output_path = temp_dir
            .path()
            .join("output.bpl")
            .to_string_lossy()
            .to_string();
        Some(temp_dir)
    } else {
        None
    };
    let res = run_move_prover_with_model(&model, &mut error_writer, options, Some(now));
    if for_test {
        let basedir = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(String::new);
        writeln!(
            message_writer,
            "{} proving {} modules from package `{}` in {:.3}s",
            if res.is_ok() {
                "SUCCESS".bold().green()
            } else {
                "FAILURE".bold().red()
            },
            model.get_target_modules().len(),
            basedir,
            now.elapsed().as_secs_f64()
        )?;
    }
    res
}
