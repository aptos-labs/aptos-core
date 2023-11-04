// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::build_schema_str;
use crate::{
    common::types::{CliCommand, CliError, CliTypedResult, MovePackageDir},
    move_tool::IncludedArtifactsArgs,
};
use aptos_move_graphql_schema::BuilderOptions;
use async_graphql_reverse::{output_schema, parse_schema_file, Phase, RendererConfig};
use async_trait::async_trait;
use clap::Parser;
use std::path::PathBuf;

/// Generate a GraphQL schema based on the ABI of the compiled modules
///
/// aptos move generate rust
///
#[derive(Parser)]
pub struct GenerateRust {
    /// If provided the tool will use this schema. If not, it will build the schema
    /// in a temporary location and use that.
    #[clap(long, value_parser)]
    schema: Option<PathBuf>,

    /// Where to output the generated code.
    #[clap(long)]
    generate_to: PathBuf,

    #[clap(flatten)]
    included_artifacts_args: IncludedArtifactsArgs,

    #[clap(flatten)]
    move_options: MovePackageDir,

    #[clap(flatten)]
    builder_options: BuilderOptions,
}

#[async_trait]
impl CliCommand<String> for GenerateRust {
    fn command_name(&self) -> &'static str {
        "GenerateRust"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let schema_path = match self.schema.clone() {
            Some(path) => {
                println!("Using schema at path: {}", path.display());
                path
            },
            None => {
                println!("No schema path provided, generating schema in temporary location...");
                let schema = build_schema_str(
                    &self.included_artifacts_args,
                    &self.move_options,
                    self.builder_options.clone(),
                )?;
                let schema_path = std::env::temp_dir().join("schema.graphql");

                // Write the schema to the file.
                std::fs::write(&schema_path, schema).map_err(|e| {
                    CliError::UnexpectedError(format!("Failed to write schema: {:#}", e))
                })?;
                println!("Schema written to {}", schema_path.display());
                schema_path
            },
        };

        let renderer_config = self.build_renderer_config();

        let structued_schema = parse_schema_file(&schema_path.to_string_lossy(), &renderer_config)?;

        // Despite the name, this does not output a schema, it outputs based on one.
        output_schema(
            &self.generate_to.to_string_lossy(),
            structued_schema,
            renderer_config,
        )?;

        Ok(format!("Code generated in {}", self.generate_to.display()))
    }
}

impl GenerateRust {
    /// Build the config for use by async-graphql-reverse.
    fn build_renderer_config(&self) -> RendererConfig {
        RendererConfig {
            phases: vec![Phase::Objects],
            no_object_impl: true,
            no_dependency_imports: true,
            header: "use serde::{Deserialize, Serialize};use aptos_move_graphql_scalars::*;"
                .to_string(),
            additional_attributes: Some("Serialize, Deserialize".to_string()),
            resolver_type: Some("field".to_string()),
            ..Default::default()
        }
    }
}
