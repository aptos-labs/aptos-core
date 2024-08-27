// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::IncludedArtifactsArgs;
use crate::common::types::{
    CliCommand, CliError, CliTypedResult, MovePackageDir, OptimizationLevel,
};
use aptos_framework::{BuildOptions, BuiltPackage};
use async_trait::async_trait;
use clap::Parser;
use move_compiler_v2::Experiment;
use move_model::metadata::{CompilerVersion, LanguageVersion};

/// Compile the package with --lint flag to show lint warnings, in addition
/// to ordinary warnings/and or errors if found.
///
/// Ideally, we would skip code generation, but there seems to be no great
/// way to do that today, so it is equivalent to `aptos move build --lint --move-2`
/// and a few extra warning passes.
#[derive(Parser)]
pub struct LintPackage {
    // TODO: add some options to select certain lint/warning passes?
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
}

#[async_trait]
impl CliCommand<&'static str> for LintPackage {
    fn command_name(&self) -> &'static str {
        "LintPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let move_options = MovePackageDir {
            lint: true,
            optimization_level: Some(OptimizationLevel::None),
            move_2: true,
            // TODO: These should be set more automatically.
            language_version: Some(LanguageVersion::V2_0),
            compiler_version: Some(CompilerVersion::V2_0),
            bytecode_version: Some(7),
            ..self.move_options.clone()
        };
        if matches!(
            self.move_options.compiler_version,
            Some(CompilerVersion::V1)
        ) {
            return Err(CliError::CommandArgumentError(
                "`aptos move lint` is not compatible with Move Compiler V1".to_string(),
            ));
        };
        let more_experiments = vec![
            Experiment::LINT_CHECKS.to_string(),
            Experiment::SPEC_CHECK.to_string(),
            Experiment::SEQS_IN_BINOPS_CHECK.to_string(),
            Experiment::ACCESS_CHECK.to_string(),
            Experiment::STOP_AFTER_EXTENDED_CHECKS.to_string(),
        ];
        let build_options = BuildOptions {
            ..self
                .included_artifacts_args
                .included_artifacts
                .build_options_with_experiments(&move_options, more_experiments, true)?
        };
        BuiltPackage::build(self.move_options.get_package_path()?, build_options)?;
        Ok("succeeded")
    }
}