// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::build_schema_str;
use crate::{
    common::types::{CliCommand, CliError, CliTypedResult, MovePackageDir},
    move_tool::IncludedArtifactsArgs,
};
use aptos_move_graphql_schema::BuilderOptions;
use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;

/// Generate a GraphQL schema based on the ABI of the compiled modules.
///
/// aptos move generate schema
///
#[derive(Parser)]
pub struct GenerateSchema {
    /// Where to write the schema file. If not given the schema will be written to the
    /// package directory.
    #[clap(long, value_parser)]
    schema_path: Option<PathBuf>,

    /// The filename of the schema.
    #[clap(long, default_value = "schema.graphql")]
    schema_fname: String,

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageDir,

    #[clap(flatten)]
    builder_options: BuilderOptions,
}

#[async_trait]
impl CliCommand<String> for GenerateSchema {
    fn command_name(&self) -> &'static str {
        "GenerateSchema"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let package_dir = self.move_options.get_package_path()?;

        let sdl = build_schema_str(
            &self.included_artifacts_args,
            &self.move_options,
            self.builder_options.clone(),
        )?;

        let schema_path = self
            .schema_path
            .unwrap_or(package_dir)
            .join(&self.schema_fname);

        // Write the schema to the file.
        std::fs::write(&schema_path, sdl)
            .map_err(|e| CliError::UnexpectedError(format!("Failed to write schema: {:#}", e)))?;

        Ok(format!("Schema written to {}", schema_path.display()))
    }
}
