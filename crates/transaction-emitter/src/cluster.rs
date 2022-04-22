// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{instance::Instance, query_sequence_numbers};
use anyhow::{format_err, Result};
use aptos::common::types::EncodingType;
use aptos_crypto::{
    ed25519,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
    Uniform,
};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::{account_config::aptos_root_address, chain_id::ChainId, AccountKey, LocalAccount},
};
use rand::seq::SliceRandom;
use std::{convert::TryFrom, path::Path};

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
        let instances: Vec<Instance> = peers
            .into_iter()
            .map(|host_port| {
                Instance::new(
                    format!("{}:{}", &host_port.0, host_port.1), /* short_hash */
                    host_port.0,
                    host_port.1,
                    host_port.2,
                )
            })
            .collect();

        let mint_key_pair = if vasp {
            dummy_key_pair()
        } else {
            KeyPair::from(
                EncodingType::BCS
                    .load_key::<ed25519::Ed25519PrivateKey>("mint key pair", Path::new(mint_file))
                    .unwrap(),
            )
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
        client: &RestClient,
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

    pub async fn load_aptos_root_account(&self, client: &RestClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, aptos_root_address())
            .await
    }

    pub async fn load_faucet_account(&self, client: &RestClient) -> Result<LocalAccount> {
        self.load_account_with_mint_key(client, aptos_root_address())
            .await
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
