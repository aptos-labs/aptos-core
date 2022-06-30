// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use std::convert::TryInto;
use url::Url;

const DEFAULT_LISTEN_PORT: u16 = 20121;

#[derive(Clone, Debug, Parser)]
pub struct ServerArgs {
    /// What address to listen on, e.g. localhost or 0.0.0.0
    #[clap(long, default_value = "0.0.0.0")]
    pub listen_address: String,

    /// What port to listen on.
    #[clap(long, default_value_t = DEFAULT_LISTEN_PORT)]
    pub listen_port: u16,

    /// What endpoint to run the API on. e.g. setting this to "api" will result
    /// in you calling an endpoint like http://nhc.mysite.com:20121/api/check_node
    #[clap(long, default_value = "", parse(from_str = parse_api_path))]
    pub api_path: String,
}

impl TryInto<Url> for ServerArgs {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Url> {
        let mut url = Url::parse(&format!(
            "http://{}:{}",
            self.listen_address, self.listen_port
        ))?;
        url.set_path(&self.api_path);
        Ok(url)
    }
}

fn parse_api_path(path: &str) -> String {
    let api_path = if path.starts_with('/') {
        // Strip any leading slash.
        let mut chars = path.chars();
        chars.next();
        chars.as_str()
    } else {
        path
    };
    api_path.to_owned()
}
