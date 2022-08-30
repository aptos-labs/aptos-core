// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_sdk::types::network_address::NetworkAddress;
use std::net::{SocketAddr, ToSocketAddrs};

// This function takes a NetworkAddress and returns a string representation
// of it if it is a format we can send to NHC. Otherwise we return an error.
pub fn extract_network_address(network_address: &NetworkAddress) -> Result<String> {
    let mut socket_addrs = network_address
        .to_socket_addrs()
        .with_context(|| format!("Failed to parse network address as SocketAddr, this might imply that the domain name doesn't resolve to an IP: {}", network_address))?;
    let socket_addr = socket_addrs
        .next()
        .ok_or_else(|| anyhow::anyhow!("No socket address found"))?;
    match socket_addr {
        SocketAddr::V4(addr) => Ok(format!("http://{}", addr.ip())),
        SocketAddr::V6(addr) => Err(anyhow::anyhow!(
            "We do not not support IPv6 addresses: {}",
            addr
        )),
    }
}
