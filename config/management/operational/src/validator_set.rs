// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    rest_client::RestClient,
    validator_config::{fullnode_addresses, validator_addresses, DecodedValidatorConfig},
};
use diem_crypto::ed25519::Ed25519PublicKey;
use diem_management::{config::ConfigPath, error::Error, secure_backend::ValidatorBackend};
use diem_types::{
    account_address::AccountAddress, network_address::NetworkAddress, validator_info::ValidatorInfo,
};
use serde::Serialize;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ValidatorSet {
    #[structopt(flatten)]
    config: ConfigPath,
    /// JSON-RPC Endpoint (e.g. http://localhost:8080)
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(long, help = "AccountAddress to retrieve the validator set info")]
    account_address: Option<AccountAddress>,
    #[structopt(
        long,
        help = "The secure backend that contains the network address encryption keys"
    )]
    validator_backend: Option<ValidatorBackend>,
}

impl ValidatorSet {
    pub async fn execute(self) -> Result<Vec<DecryptedValidatorInfo>, Error> {
        let config = self.config.load()?.override_json_server(&self.json_server);
        let client = RestClient::new(config.json_server);
        decode_validator_set(client, self.account_address).await
    }
}

pub async fn decode_validator_set(
    client: RestClient,
    account_address: Option<AccountAddress>,
) -> Result<Vec<DecryptedValidatorInfo>, Error> {
    let set = client.validator_set(account_address).await?;

    let mut decoded_set = Vec::new();
    for info in set {
        let config = DecodedValidatorConfig::from_validator_config(info.config())
            .map_err(|e| Error::NetworkAddressDecodeError(e.to_string()))?;

        let config_resource = client.validator_config(*info.account_address()).await?;
        let name = DecodedValidatorConfig::human_name(&config_resource.human_name);

        let info = DecryptedValidatorInfo {
            name,
            account_address: *info.account_address(),
            consensus_public_key: config.consensus_public_key,
            fullnode_network_address: config.fullnode_network_address,
            validator_network_address: config.validator_network_address,
        };
        decoded_set.push(info);
    }

    Ok(decoded_set)
}

pub async fn validator_set_full_node_addresses(
    client: RestClient,
    account_address: Option<AccountAddress>,
) -> Result<Vec<(String, AccountAddress, Vec<NetworkAddress>)>, Error> {
    validator_set_addresses(client, account_address, |info| {
        fullnode_addresses(info.config())
    })
    .await
}

pub async fn validator_set_validator_addresses(
    client: RestClient,
    account_address: Option<AccountAddress>,
) -> Result<Vec<(String, AccountAddress, Vec<NetworkAddress>)>, Error> {
    validator_set_addresses(client, account_address, |info| {
        validator_addresses(info.config())
    })
    .await
}

async fn validator_set_addresses<F: Fn(ValidatorInfo) -> Result<Vec<NetworkAddress>, Error>>(
    client: RestClient,
    account_address: Option<AccountAddress>,
    address_accessor: F,
) -> Result<Vec<(String, AccountAddress, Vec<NetworkAddress>)>, Error> {
    let set = client.validator_set(account_address).await?;
    let mut decoded_set = Vec::new();
    for info in set {
        let config_resource = client.validator_config(*info.account_address()).await?;
        let name = DecodedValidatorConfig::human_name(&config_resource.human_name);
        let peer_id = *info.account_address();
        let addrs = address_accessor(info)?;
        decoded_set.push((name, peer_id, addrs));
    }

    Ok(decoded_set)
}
#[derive(Serialize)]
pub struct DecryptedValidatorInfo {
    pub name: String,
    pub account_address: AccountAddress,
    pub consensus_public_key: Ed25519PublicKey,
    pub fullnode_network_address: NetworkAddress,
    pub validator_network_address: NetworkAddress,
}
