// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Command-line types and the REST client the replay tools share.

use anyhow::Result;
use aptos_rest_client::{AptosBaseUrl, Client};
use clap::ValueEnum;
use std::str::FromStr;
use url::Url;

/// The chain to capture from. Mirrors [`AptosBaseUrl`]: a named network or a custom REST endpoint.
#[derive(Clone)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Custom(Url),
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Testnet,
            "devnet" => Network::Devnet,
            url => Network::Custom(Url::parse(url).map_err(|e| e.to_string())?),
        })
    }
}

impl From<Network> for AptosBaseUrl {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => AptosBaseUrl::Mainnet,
            Network::Testnet => AptosBaseUrl::Testnet,
            Network::Devnet => AptosBaseUrl::Devnet,
            Network::Custom(url) => AptosBaseUrl::Custom(url),
        }
    }
}

/// Which VM(s) to run. A single VM lets you profile it without the other in the same process.
#[derive(Clone, Copy, ValueEnum)]
pub enum VMSelection {
    V1,
    V2,
    Both,
}

impl VMSelection {
    pub fn runs_v1(self) -> bool {
        matches!(self, VMSelection::V1 | VMSelection::Both)
    }

    pub fn runs_v2(self) -> bool {
        matches!(self, VMSelection::V2 | VMSelection::Both)
    }
}

/// A REST client for `base_url`, authenticated with `api_key` if given.
pub fn rest_client(base_url: AptosBaseUrl, api_key: Option<String>) -> Result<Client> {
    let mut builder = Client::builder(base_url);
    if let Some(key) = api_key {
        builder = builder.api_key(&key)?;
    }
    Ok(builder.build())
}
