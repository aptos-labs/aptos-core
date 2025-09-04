use anyhow::bail;
// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use velor_crypto::ed25519::Ed25519PrivateKey;
use velor_sdk::types::{
    AccountKey, EphemeralKeyPair, EphemeralPrivateKey, KeylessAccount, LocalAccount,
};
use velor_transaction_generator_lib::ReliableTransactionSubmitter;
use velor_types::{
    keyless,
    keyless::{Claims, OpenIdSig, Pepper, ZeroKnowledgeSig},
};
use async_trait::async_trait;
use futures::StreamExt;
use rand::rngs::StdRng;
use std::{
    fs::File,
    io::{self, BufRead},
};

const QUERY_PARALLELISM: usize = 300;

#[async_trait]
pub trait LocalAccountGenerator: Send + Sync {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>>;
}

pub fn create_private_key_account_generator() -> Box<dyn LocalAccountGenerator> {
    Box::new(PrivateKeyAccountGenerator)
}

pub fn create_keyless_account_generator(
    ephemeral_secret_key: Ed25519PrivateKey,
    epk_expiry_date_secs: u64,
    jwt: &str,
    proof_file_path: Option<&str>,
    keyless_config: keyless::Configuration,
) -> anyhow::Result<Box<dyn LocalAccountGenerator>> {
    let parts: Vec<&str> = jwt.split('.').collect();
    let header_bytes = base64::decode(parts[0]).unwrap();
    let jwt_header_json = String::from_utf8(header_bytes).unwrap();
    let jwt_payload_json = base64::decode_config(parts[1], base64::URL_SAFE).unwrap();
    let claims: Claims = serde_json::from_slice(&jwt_payload_json)?;
    Ok(Box::new(KeylessAccountGenerator {
        proof_file_path: proof_file_path.map(|s| s.to_string()),
        ephemeral_secret_key,
        epk_expiry_date_secs,
        iss: claims.oidc_claims.iss,
        aud: claims.oidc_claims.aud,
        uid_key: "sub".to_owned(),
        uid_val: claims.oidc_claims.sub,
        jwt_header_json,
        keyless_config,
    }))
}

pub struct PrivateKeyAccountGenerator;

#[async_trait]
impl LocalAccountGenerator for PrivateKeyAccountGenerator {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>> {
        let mut account_keys = vec![];
        let mut addresses = vec![];
        let mut i = 0;
        while i < num_accounts {
            let account_key = AccountKey::generate(rng);
            addresses.push(account_key.authentication_key().account_address());
            account_keys.push(account_key);
            i += 1;
        }
        let result_futures = addresses
            .iter()
            .map(|address| txn_executor.query_sequence_number(*address))
            .collect::<Vec<_>>();

        let seq_nums = futures::stream::iter(result_futures)
            .buffered(QUERY_PARALLELISM)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        let accounts = account_keys
            .into_iter()
            .zip(seq_nums)
            .map(|(account_key, sequence_number)| {
                LocalAccount::new(
                    account_key.authentication_key().account_address(),
                    account_key,
                    sequence_number,
                )
            })
            .collect();
        Ok(accounts)
    }
}

pub struct KeylessAccountGenerator {
    proof_file_path: Option<String>,
    ephemeral_secret_key: Ed25519PrivateKey,
    epk_expiry_date_secs: u64,
    iss: String,
    aud: String,
    uid_key: String,
    uid_val: String,
    jwt_header_json: String,
    /// We assume the on-chain keyless config won't change and cache it here.
    /// Needed by nonce generation.
    keyless_config: keyless::Configuration,
}

#[async_trait]
impl LocalAccountGenerator for KeylessAccountGenerator {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        _rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>> {
        let mut keyless_accounts = vec![];
        let mut addresses = vec![];
        let mut i = 0;
        let lines: Box<dyn Iterator<Item = Result<String, io::Error>>> = match &self.proof_file_path
        {
            None => {
                let proofs = include_str!("test_proofs_for_localnet_txn_emitter.txt");
                let lines = proofs.lines().map(|line| Ok(line.to_string()));
                Box::new(lines)
            },
            Some(path) => {
                let file = File::open(path).unwrap();
                let reader = io::BufReader::new(file);
                Box::new(reader.lines())
            },
        };

        for line in lines {
            let serialized_proof = line?;
            let zk_sig_bytes = hex::decode(serialized_proof)?;
            let zk_sig = ZeroKnowledgeSig::try_from(zk_sig_bytes.as_slice())?;

            // Cloning is disabled outside #[cfg(test)]
            let serialized: &[u8] = &(self.ephemeral_secret_key.to_bytes());
            let esk = EphemeralPrivateKey::Ed25519 {
                inner_private_key: Ed25519PrivateKey::try_from(serialized)?,
            };

            let keyless_account = KeylessAccount::new(
                &self.iss,
                &self.aud,
                &self.uid_key,
                &self.uid_val,
                &self.jwt_header_json,
                EphemeralKeyPair::new_with_keyless_config(
                    &self.keyless_config,
                    esk,
                    self.epk_expiry_date_secs,
                    vec![0; OpenIdSig::EPK_BLINDER_NUM_BYTES],
                )?,
                Pepper::from_number(i.try_into().unwrap()),
                zk_sig,
            )?;
            addresses.push(keyless_account.authentication_key().account_address());
            keyless_accounts.push(keyless_account);
            i += 1;

            if i == num_accounts {
                break;
            }
        }

        if i != num_accounts {
            bail!("not enough proofs - {num_accounts} num_accounts, {i} found")
        }

        let result_futures = addresses
            .iter()
            .map(|address| txn_executor.query_sequence_number(*address))
            .collect::<Vec<_>>();

        let seq_nums = futures::stream::iter(result_futures)
            .buffered(QUERY_PARALLELISM)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        let accounts = keyless_accounts
            .into_iter()
            .zip(seq_nums)
            .map(|(keyless_account, sequence_number)| {
                LocalAccount::new_keyless(
                    keyless_account.authentication_key().account_address(),
                    keyless_account,
                    sequence_number,
                )
            })
            .collect();
        Ok(accounts)
    }
}
