// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file contains logic for reading the node information from the input
//! file and converting it into the format check_rxn_info_hashes expects.

use crate::check::{IncompleteNetworkAddress, NodeInfo, SingleCheck, SingleCheckResult};
use anyhow::{Context, Result};
use aptos_sdk::{
    crypto::{x25519, ValidCryptoMaterialStringExt},
    types::account_address::AccountAddress,
};
use clap::Parser;
use reqwest::Url;
use serde::Deserialize;
use std::{collections::HashMap, convert::TryInto, fs::File, path::PathBuf};

#[derive(Clone, Debug, Parser)]
pub struct GetPublicFullNodes {
    #[clap(long, value_parser)]
    pub input_file: PathBuf,
}

impl GetPublicFullNodes {
    pub async fn get_node_infos(
        &self,
        account_address_allowlist: &[String],
    ) -> Result<(
        HashMap<AccountAddress, Vec<NodeInfo>>,
        HashMap<AccountAddress, Vec<SingleCheck>>,
    )> {
        // These are the valid node addresses, keyed by account address.
        let mut node_infos = HashMap::new();

        // These are results for the invalid node addresses.
        let mut invalid_node_address_results = HashMap::new();

        // Read information from the file.
        let file = File::open(&self.input_file).context("Failed to open input file")?;
        let entries: Vec<Entry> =
            serde_json::from_reader(file).context("Failed to parse input file")?;

        for entry in entries.into_iter() {
            let account_address = entry.account_address;
            if !account_address_allowlist.is_empty()
                && !account_address_allowlist.contains(&account_address.to_string())
            {
                continue;
            }
            match entry.try_into() {
                Ok(node_info) => node_infos
                    .entry(account_address)
                    .or_insert_with(Vec::new)
                    .push(node_info),
                Err(e) => invalid_node_address_results
                    .entry(account_address)
                    .or_insert_with(Vec::new)
                    .push(SingleCheck::new(
                        SingleCheckResult::IncompleteNetworkAddress(IncompleteNetworkAddress {
                            message: format!("{:#}", e),
                        }),
                        None,
                    )),
            }
        }

        Ok((node_infos, invalid_node_address_results))
    }
}

/// This struct defines the format we expect each entry in the JSON to be.
#[derive(Debug, Deserialize)]
pub struct Entry {
    /// The account address of the node operator.
    account_address: AccountAddress,

    /// The URL of the node, scheme included.
    url: String,

    /// The port the API is running on.
    api_port: u16,

    /// The noise port.
    noise_port: u16,

    #[allow(dead_code)]
    /// The port the metrics server is running on.
    pub metrics_port: u16,

    /// The public key matching the private key that the node is running with.
    public_key: String,
}

impl TryInto<NodeInfo> for Entry {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<NodeInfo> {
        Ok(NodeInfo {
            node_url: Url::parse(&self.url).context("Failed to parse URL")?,
            api_port: Some(self.api_port),
            noise_port: self.noise_port,
            public_key: Some(
                x25519::PublicKey::from_encoded_string(&self.public_key)
                    .context("Failed to parse public key")?,
            ),
        })
    }
}
