// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::{
    common::{types::MovePackageDir, utils::to_common_result},
    CliResult, Error,
};
use aptos_vm::natives::aptos_natives;
use clap::{Parser, Subcommand};
use move_cli::package::cli::{run_move_unit_tests, UnitTestResult};
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use move_unit_test::UnitTestingConfig;
use std::path::Path;

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Test(TestPackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => to_common_result(tool.execute().await),
            MoveTool::Test(tool) => to_common_result(tool.execute().await),
        }
    }
}

/// Compiles a package and returns the [`ModuleId`]s
#[derive(Parser)]
pub struct CompilePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl CompilePackage {
    pub async fn execute(&self) -> Result<Vec<String>, Error> {
        let build_config = BuildConfig {
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let compiled_package = compile_move(build_config, self.move_options.package_dir.as_path())?;
        // TODO: This can be serialized once move is updated
        let mut ids = Vec::new();
        compiled_package
            .compiled_modules()
            .iter_modules()
            .iter()
            .for_each(|module| ids.push(module.self_id().to_string()));
        Ok(ids)
    }
}

/// Run Move unit tests against a package path
#[derive(Parser)]
pub struct TestPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl TestPackage {
    pub async fn execute(&self) -> Result<&'static str, Error> {
        let config = BuildConfig {
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = run_move_unit_tests(
            self.move_options.package_dir.as_path(),
            config,
            UnitTestingConfig::default_with_bound(Some(100_000)),
            aptos_natives(),
            false,
        )
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;

        // TODO: commit back up to the move repo
        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Ok("Failure"),
        }
    }
}

/// Compiles a Move package dir, and returns the compiled modules.
fn compile_move(build_config: BuildConfig, package_dir: &Path) -> Result<CompiledPackage, Error> {
    // TODO: Add caching
    build_config
        .compile_package(package_dir, &mut Vec::new())
        .map_err(|err| Error::MoveCompiliationError(err.to_string()))
}
