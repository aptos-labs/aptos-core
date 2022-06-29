// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use clap::Parser;
use std::convert::TryFrom;
use url::Url;

const DEFAULT_LISTEN_PORT: u16 = 20121;

#[derive(Clone, Debug, Parser)]
pub struct ServerArgs {
    /// What address to listen on. If port is not given, it is assumed to
    /// be 20121.
    #[clap(long, default_value = "http://0.0.0.0:20121", parse(try_from_str = parse_listen_address))]
    pub listen_address: Url,

    /// What endpoint to run the API on. e.g. setting this to "api" will result
    /// in you calling an endpoint like http://nhc.mysite.com:20121/api/check_node
    #[clap(long, default_value = "", parse(from_str = parse_api_path))]
    pub api_path: String,
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

fn parse_listen_address(listen_address: &str) -> Result<Url> {
    let mut url = Url::try_from(listen_address).map_err(|e| {
        format_err!(
            "Failed to parse listen address, try adding a scheme, e.g. http://: {}",
            e
        )
    })?;
    if url.port().is_none() {
        url.set_port(Some(DEFAULT_LISTEN_PORT))
            .map_err(|_| anyhow::anyhow!("Failed to set port to default"))?;
    }
    Ok(url)
}
