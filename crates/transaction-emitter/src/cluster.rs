// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{instance::Instance, query_sequence_numbers};
use anyhow::{format_err, Result};
use diem_client::Client as JsonRpcClient;
use diem_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
    Uniform,
};
use diem_sdk::{
    move_types::account_address::AccountAddress,
    types::{
        account_config::{
            diem_root_address, testnet_dd_account_address, treasury_compliance_account_address,
        },
        chain_id::ChainId,
        AccountKey, LocalAccount,
    },
};
use rand::seq::SliceRandom;
use reqwest::Client;
use std::convert::TryFrom;

const DD_KEY: &str = "dd.key";

pub struct Cluster {
    instances: Vec<Instance>,
    mint_key_pair: KeyPair<Ed25519PrivateKey, Ed25519PublicKey>,
    pub chain_id: ChainId,
}

fn clone(key: &Ed25519PrivateKey) -> Ed25519PrivateKey {
    let serialized: &[u8] = &(key.to_bytes());
    Ed25519PrivateKey::try_from(serialized).unwrap()
}

impl Cluster {
    pub fn from_host_port(
        peers: Vec<(String, u32, Option<u32>)>,
        mint_file: &str,
        chain_id: ChainId,
        vasp: bool,
    ) -> Self {
        let http_client = Client::new();
        let instances: Vec<Instance> = peers
            .into_iter()
            .map(|host_port| {
                Instance::new(
                    format!("{}:{}", &host_port.0, host_port.1), /* short_hash */
                    host_port.0,
                    host_port.1,
                    host_port.2,
                    http_client.clone(),
                )
            })
            .collect();

        let mint_key_pair = if vasp {
            dummy_key_pair()
        } else {
            KeyPair::from(generate_key::load_key(mint_file))
        };

        Self {
            instances,
            mint_key_pair,
            chain_id,
        }
    }

    fn account_key(&self) -> AccountKey {
        AccountKey::from_private_key(clone(&self.mint_key_pair.private_key))
    }

    async fn load_account_with_mint_key(
        &self,
        client: &JsonRpcClient,
        address: AccountAddress,
    ) -> Result<LocalAccount> {
        let sequence_number = query_sequence_numbers(client, &[address])
            .await
            .map_err(|e| {
                format_err!(
                    "query_sequence_numbers on {:?} for account {} failed: {}",
                    client,
                    address,
                    e
                )
            })?[0];
        Ok(LocalAccount::new(
            address,
            self.account_key(),
            sequence_number,
        ))
    }

    pub async fn load_diem_root_account(&self, client: &JsonRpcClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, diem_root_address())
            .await
    }

    pub async fn load_faucet_account(&self, client: &JsonRpcClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, testnet_dd_account_address())
            .await
    }

    pub async fn load_tc_account(&self, client: &JsonRpcClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, treasury_compliance_account_address())
            .await
    }

    pub async fn load_dd_account(&self, client: &JsonRpcClient) -> Result<LocalAccount> {
        let mint_key: Ed25519PrivateKey = generate_key::load_key(DD_KEY);
        let account_key = AccountKey::from_private_key(mint_key);
        let address = account_key.authentication_key().derived_address();
        let sequence_number = query_sequence_numbers(client, &[address])
            .await
            .map_err(|e| {
                format_err!(
                    "query_sequence_numbers on {:?} for dd account failed: {}",
                    client,
                    e
                )
            })?[0];
        Ok(LocalAccount::new(address, account_key, sequence_number))
    }

    pub fn random_instance(&self) -> Instance {
        let mut rnd = rand::thread_rng();
        self.instances
            .choose(&mut rnd)
            .expect("random_validator_instance requires non-empty validator_instances")
            .clone()
    }

    pub fn all_instances(&self) -> impl Iterator<Item = &Instance> {
        self.instances.iter()
    }
}

pub fn dummy_key_pair() -> KeyPair<Ed25519PrivateKey, Ed25519PublicKey> {
    Ed25519PrivateKey::generate_for_testing().into()
}
