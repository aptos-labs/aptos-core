// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{metric_collector::ReqwestMetricCollector, runner::BlockingRunner};
use anyhow::Result;
use clap::{ArgEnum, Parser};
use std::{collections::HashMap, path::PathBuf};
use url::Url;

use super::{
    api::{build_openapi_service, Api},
    configurations_manager::{ConfigurationsManager, NodeConfigurationWrapper},
};

#[derive(ArgEnum, Clone, Debug)]
enum OutputFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug, Parser)]
pub struct GenerateOpenapi {
    /// What address to listen on.
    #[clap(long, default_value = "http://0.0.0.0")]
    pub listen_address: Url,

    /// What port to listen on.
    #[clap(long, default_value = "20121")]
    pub listen_port: u16,

    /// By default, the spec is written to stdout. If this is provided, the
    /// tool will instead write the spec to the provided path.
    #[clap(short, long)]
    output_path: Option<PathBuf>,

    /// What format to output the spec in.
    #[clap(short, long, arg_enum, default_value = "yaml")]
    format: OutputFormat,
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

    let spec = match args.format {
        OutputFormat::Json => api_service.spec(),
        OutputFormat::Yaml => api_service.spec_yaml(),
    };

    match args.output_path {
        Some(path) => std::fs::write(path, spec)?,
        None => println!("{}", spec),
    }

    Ok(())
}
