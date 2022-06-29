// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common_args::{OutputArgs, OutputFormat},
    metric_collector::ReqwestMetricCollector,
    runner::BlockingRunner,
};
use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use url::Url;

use super::{
    api::{build_openapi_service, Api},
    configurations_manager::{ConfigurationsManager, NodeConfigurationWrapper},
};

#[derive(Clone, Debug, Parser)]
pub struct GenerateOpenapi {
    /// What address to listen on.
    #[clap(long, default_value = "http://0.0.0.0")]
    pub listen_address: Url,

    /// What port to listen on.
    #[clap(long, default_value = "20121")]
    pub listen_port: u16,

    #[clap(flatten)]
    pub output_args: OutputArgs,
}

pub async fn generate_openapi(args: GenerateOpenapi) -> Result<()> {
    let configurations: HashMap<
        _,
        NodeConfigurationWrapper<ReqwestMetricCollector, BlockingRunner<ReqwestMetricCollector>>,
    > = HashMap::new();

    let api = Api {
        configurations_manager: ConfigurationsManager { configurations },
        target_metric_collector: None,
        allow_preconfigured_test_node_only: false,
    };

    let api_service =
        build_openapi_service(api, args.listen_address.clone(), args.listen_port, None);

    let spec = match args.output_args.format {
        OutputFormat::Json => api_service.spec(),
        OutputFormat::Yaml => api_service.spec_yaml(),
    };
    args.output_args.write(&spec)
}
