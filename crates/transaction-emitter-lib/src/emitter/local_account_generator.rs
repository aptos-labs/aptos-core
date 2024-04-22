use aptos_crypto::ed25519::Ed25519PrivateKey;
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_sdk::types::{AccountKey, EphemeralKeyPair, KeylessAccount, LocalAccount};
use aptos_transaction_generator_lib::{AccountType, ReliableTransactionSubmitter};
use aptos_types::keyless::{
    test_utils::{get_sample_aud, get_sample_iss, get_sample_jwt_header_json, get_sample_uid_key},
    OpenIdSig, Pepper, ZeroKnowledgeSig,
};
use async_trait::async_trait;
use futures::future::try_join_all;
use rand::rngs::StdRng;

#[async_trait]
pub trait LocalAccountGenerator: Send + Sync {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>>;
}

pub fn create_account_generator(account_type: AccountType) -> Box<dyn LocalAccountGenerator> {
    match account_type {
        AccountType::Local => Box::new(PrivateKeyAccountGenerator),
        _ => {
            unimplemented!("Account type {:?} is not supported", account_type)
        },
    }
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
        let seq_nums: Vec<_> = try_join_all(result_futures).await?.into_iter().collect();

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
    serialized_proofs: Vec<String>,
    ephemeral_secret_key: Ed25519PrivateKey,
    expiry_date_secs: u64,
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
        while i < num_accounts {
            let zk_sig_bytes = hex::decode(self.serialized_proofs[i].clone())?;
            let zk_sig = ZeroKnowledgeSig::try_from(zk_sig_bytes.as_slice())?;
            // Cloning is disabled outside #[cfg(test)]
            let serialized: &[u8] = &(self.ephemeral_secret_key.to_bytes());
            let esk = Ed25519PrivateKey::try_from(serialized)?;

            let keyless_account = KeylessAccount::new(
                get_sample_iss(),
                get_sample_aud(),
                get_sample_uid_key(),
                format!("test-user-{}", i),
                get_sample_jwt_header_json(),
                EphemeralKeyPair::new(
                    esk,
                    self.expiry_date_secs,
                    vec![0; OpenIdSig::EPK_BLINDER_NUM_BYTES],
                )?,
                Pepper::from_number(0),
                zk_sig,
            )?;
            addresses.push(keyless_account.authentication_key().account_address());
            keyless_accounts.push(keyless_account);
            i += 1;
        }
        let result_futures = addresses
            .iter()
            .map(|address| txn_executor.query_sequence_number(*address))
            .collect::<Vec<_>>();
        let seq_nums: Vec<_> = try_join_all(result_futures).await?.into_iter().collect();

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
