// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use url::Url;

#[derive(Clone, Debug, Parser)]
pub struct ServerArgs {
    /// What address to listen on.
    #[clap(long, default_value = "http://0.0.0.0")]
    pub listen_address: Url,

    /// What port to listen on.
    #[clap(long, default_value = "20121")]
    pub listen_port: u16,

    /// What endpoint to run the API on. e.g. setting this to "api" will result
    /// in you calling an endpoint like http://nhc.mysite.com:20121/api/check_node
    #[clap(long, default_value = "")]
    pub api_endpoint: String,
}
