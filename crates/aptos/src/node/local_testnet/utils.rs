// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use reqwest::Url;
use std::net::SocketAddr;

pub fn socket_addr_to_url(socket_addr: &SocketAddr, scheme: &str) -> Result<Url> {
    let host = match socket_addr {
        SocketAddr::V4(v4) => format!("{}", v4.ip()),
        SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
    };
    let full_url = format!("{}://{}:{}", scheme, host, socket_addr.port());
    Ok(Url::parse(&full_url)?)
}
