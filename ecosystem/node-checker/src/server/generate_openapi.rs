// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::common::ServerArgs;
use crate::{
    common_args::{OutputArgs, OutputFormat},
    metric_collector::ReqwestMetricCollector,
    runner::BlockingRunner,
};
use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;

use super::{
    api::{build_openapi_service, Api},
    configurations_manager::{ConfigurationsManager, NodeConfigurationWrapper},
};

#[derive(Clone, Debug, Parser)]
pub struct GenerateOpenapi {
    #[clap(flatten)]
    server_args: ServerArgs,

    #[clap(flatten)]
    pub output_args: OutputArgs,
}

pub async fn generate_openapi(args: GenerateOpenapi) -> Result<()> {
    let configurations: HashMap<
        _,
        NodeConfigurationWrapper<BlockingRunner<ReqwestMetricCollector>>,
    > = HashMap::new();

    let api: Api<ReqwestMetricCollector, _> = Api {
        configurations_manager: ConfigurationsManager { configurations },
        preconfigured_test_node: None,
        allow_preconfigured_test_node_only: false,
    };

    let api_service = build_openapi_service(api, args.server_args.clone());

    let spec = match args.output_args.format {
        OutputFormat::Json => api_service.spec(),
        OutputFormat::Yaml => api_service.spec_yaml(),
    };
    args.output_args.write(&spec)
}
