// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use aptos_config::network_id::NetworkId;
use aptos_types::{chain_id::ChainId, network_address::NetworkAddress};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Parser, Serialize)]
pub struct NodeAddressArgs {
    /// `NetworkAddress` of remote server interface
    #[clap(long, value_parser = validate_address)]
    pub address: NetworkAddress,
    /// `ChainId` of remote server
    #[clap(long)]
    pub chain_id: ChainId,
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct HandshakeArgs {
    /// `NetworkId` of remote server interface
    #[clap(long, default_value = "public")]
    pub network_id: NetworkId,
    /// Optional number of seconds to timeout attempting to connect to endpoint
    #[clap(long, default_value_t = 5)]
    pub timeout_seconds: u64,
    /// Skip handshake for network checking
    #[clap(long)]
    pub no_handshake: bool,
}

#[derive(Clone, Debug, Deserialize, Parser, Serialize)]
pub struct CheckEndpointArgs {
    #[clap(flatten)]
    pub node_address_args: NodeAddressArgs,

    #[clap(flatten)]
    pub handshake_args: HandshakeArgs,
}

fn validate_address(address: &str) -> Result<NetworkAddress> {
    let address = NetworkAddress::from_str(address)
        .with_context(|| format!("Invalid address: {}", address))?;
    if !address.is_aptosnet_addr() {
        bail!("Address must have IP / DNS, TCP, noise key, and handshake")
    }
    Ok(address)
}
