// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    api::{build_openapi_service, Api},
    build::BaselineConfigurationRunners,
    common::ServerArgs,
};
use crate::{
    common::{OutputArgs, OutputFormat},
    runner::SyncRunner,
};
use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;

#[derive(Clone, Debug, Parser)]
pub struct GenerateOpenapi {
    #[clap(flatten)]
    server_args: ServerArgs,

    #[clap(flatten)]
    pub output_args: OutputArgs,
}

pub async fn generate_openapi(args: GenerateOpenapi) -> Result<()> {
    let baseline_configurations = BaselineConfigurationRunners(HashMap::new());

    let api: Api<SyncRunner> = Api {
        baseline_configurations,
    };

    let api_service = build_openapi_service(api, args.server_args.clone());

    let spec = match args.output_args.format {
        OutputFormat::Json => api_service.spec(),
        OutputFormat::Yaml => api_service.spec_yaml(),
    };
    args.output_args.write(&spec)
}
