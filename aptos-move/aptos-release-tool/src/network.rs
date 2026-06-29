// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::str::FromStr;
use url::Url;

/// A network to talk to, either one of the well-known ones or an arbitrary REST endpoint.
//
// TODO: unify with the identical enum in `aptos-release-builder` once the old tool is retired.
#[derive(Clone, Debug)]
pub enum NetworkSelection {
    Mainnet,
    Testnet,
    Devnet,
    RestEndpoint(String),
}

impl FromStr for NetworkSelection {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        Ok(match s {
            "mainnet" => Self::Mainnet,
            "testnet" => Self::Testnet,
            "devnet" => Self::Devnet,
            _ => Self::RestEndpoint(s.to_owned()),
        })
    }
}

impl NetworkSelection {
    pub fn to_url(&self) -> anyhow::Result<Url> {
        use NetworkSelection::*;

        let s = match self {
            Mainnet => "https://fullnode.mainnet.aptoslabs.com",
            Testnet => "https://fullnode.testnet.aptoslabs.com",
            Devnet => "https://fullnode.devnet.aptoslabs.com",
            RestEndpoint(url) => url,
        };

        Ok(Url::parse(s)?)
    }
}
