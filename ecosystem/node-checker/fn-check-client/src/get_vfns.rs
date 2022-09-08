// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file contains logic for reading the node information from on-chain and
//! converting it into the format expected by check.rs

use anyhow::{Context, Result};
use aptos_sdk::{
    rest_client::Client as AptosClient,
    types::{
        account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
        on_chain_config::ValidatorSet, validator_info::ValidatorInfo,
    },
};
use clap::Parser;
use log::info;
use reqwest::Url;
use std::collections::HashMap;

use crate::check::CouldNotDeserializeNetworkAddress;
use crate::check::IncompleteNetworkAddress;
use crate::check::NoVfnRegistered;
use crate::check::NodeInfo;
use crate::check::SingleCheck;
use crate::check::SingleCheckResult;
use crate::helpers::extract_network_address;

#[derive(Debug, Parser)]
pub struct GetValidatorFullNodes {
    /// Address of any node (of any type) connected to the network you want
    /// to evaluate. We use this to get the list of VFNs from on-chain.
    #[clap(long)]
    pub node_address: Url,
}

impl GetValidatorFullNodes {
    /// Get all the on chain validator info.
    async fn get_validator_infos(&self) -> Result<Vec<ValidatorInfo>> {
        let client = AptosClient::new(self.node_address.clone());
        let response = client
            .get_account_resource_bcs::<ValidatorSet>(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
            .await?;
        let active_validators = response.into_inner().active_validators;
        info!(
            "Pulled {} active validators. First: {}. Last: {}",
            active_validators.len(),
            active_validators.first().unwrap().account_address(),
            active_validators.last().unwrap().account_address()
        );
        Ok(active_validators)
    }

    /// First, retrieve the on-chain validator set. For each validator address,
    /// we go through each of their registered VFNs and confirm that we can
    /// resolve the NetworkAddress of each into a domain name / IP address
    /// and API port. If we can, they make it into the NodeInfoes. If we
    /// cannot, or they have no VFNs registered at all, they will not be
    /// included in the NodeInfoes and instead we return them in the second
    /// item of the tuple, indicating that the check failed for this reason.
    pub async fn get_node_infos(
        &self,
        account_address_allowlist: &[String],
    ) -> Result<(
        HashMap<AccountAddress, Vec<NodeInfo>>,
        HashMap<AccountAddress, Vec<SingleCheck>>,
    )> {
        let mut validator_infos = self
            .get_validator_infos()
            .await
            .with_context(|| format!("Failed to get validator info from {}", self.node_address))?;

        // These are the valid node addresses, keyed by account address.
        let mut node_infos = HashMap::new();

        // These are results for the invalid node addresses, keyed by account address.
        let mut invalid_node_address_results = HashMap::new();

        for validator_info in validator_infos.iter_mut() {
            let account_address = validator_info.account_address();
            if !account_address_allowlist.is_empty()
                && !account_address_allowlist.contains(&account_address.to_string())
            {
                continue;
            }

            let vfn_addresses = match validator_info.config().fullnode_network_addresses() {
                Ok(vfn_addresses) => vfn_addresses,
                Err(e) => {
                    invalid_node_address_results
                        .entry(*account_address)
                        .or_insert_with(Vec::new)
                        .push(SingleCheck::new(
                            SingleCheckResult::CouldNotDeserializeNetworkAddress(
                                CouldNotDeserializeNetworkAddress {
                                    message: format!("{:#}", e),
                                },
                            ),
                            None,
                        ));
                    continue;
                }
            };

            if vfn_addresses.is_empty() {
                invalid_node_address_results
                    .entry(*account_address)
                    .or_insert_with(Vec::new)
                    .push(SingleCheck::new(
                        SingleCheckResult::NoVfnRegistered(NoVfnRegistered),
                        None,
                    ));
                continue;
            }

            for vfn_address in vfn_addresses.into_iter() {
                let (node_url, noise_port) = match extract_network_address(&vfn_address) {
                    Ok(result) => result,
                    Err(e) => {
                        invalid_node_address_results
                            .entry(*account_address)
                            .or_insert_with(Vec::new)
                            .push(SingleCheck::new(
                                SingleCheckResult::IncompleteNetworkAddress(
                                    IncompleteNetworkAddress {
                                        message: format!("{:#}", e),
                                    },
                                ),
                                None,
                            ));
                        continue;
                    }
                };
                node_infos
                    .entry(*account_address)
                    .or_insert_with(Vec::new)
                    .push(NodeInfo {
                        node_url,
                        api_port: None,
                        noise_port,
                        public_key: vfn_address.find_noise_proto(),
                    });
            }
        }
        Ok((node_infos, invalid_node_address_results))
    }
}
