use anyhow::bail;
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::CryptoMaterialError;
use aptos_sdk::types::{
    extract_claims_from_jwt, extract_header_json_from_jwt, AccountKey, EphemeralKeyPair,
    EphemeralPrivateKey, FederatedKeylessAccount, KeylessAccount, LocalAccount,
};
use aptos_transaction_generator_lib::ReliableTransactionSubmitter;
use aptos_types::{
    account_address::AccountAddress,
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
    jwk_addr: Option<AccountAddress>,
) -> anyhow::Result<Box<dyn LocalAccountGenerator>> {
    let jwt_header_json = extract_header_json_from_jwt(jwt)?;
    let claims: Claims = extract_claims_from_jwt(jwt)?;
    let generator = BaseKeylessAccountGenerator {
        proof_file_path: proof_file_path.map(|s| s.to_string()),
        ephemeral_secret_key,
        epk_expiry_date_secs,
        iss: claims.oidc_claims.iss,
        aud: claims.oidc_claims.aud,
        uid_key: "sub".to_owned(),
        uid_val: claims.oidc_claims.sub,
        jwt_header_json,
    };
    if let Some(jwk_addr) = jwk_addr {
        Ok(Box::new(FederatedKeylessAccountGenerator {
            generator,
            jwk_addr,
        }))
    } else {
        Ok(Box::new(KeylessAccountGenerator { generator }))
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
    generator: BaseKeylessAccountGenerator,
}

#[async_trait]
impl LocalAccountGenerator for KeylessAccountGenerator {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        _rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>> {
        self.generator
            .generate_accounts(
                txn_executor,
                num_accounts,
                |i, zk_sig, ephemeral_key_pair| {
                    KeylessAccount::new(
                        &self.generator.iss,
                        &self.generator.aud,
                        &self.generator.uid_key,
                        &self.generator.uid_val,
                        &self.generator.jwt_header_json,
                        ephemeral_key_pair,
                        Pepper::from_number(i.try_into().unwrap()),
                        zk_sig,
                    )
                    .map(|account| {
                        LocalAccount::new_keyless(
                            account.authentication_key().account_address(),
                            account,
                            0,
                        )
                    })
                },
            )
            .await
    }
}

pub struct FederatedKeylessAccountGenerator {
    generator: BaseKeylessAccountGenerator,
    jwk_addr: AccountAddress,
}

#[async_trait]
impl LocalAccountGenerator for FederatedKeylessAccountGenerator {
    async fn gen_local_accounts(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        _rng: &mut StdRng,
    ) -> anyhow::Result<Vec<LocalAccount>> {
        self.generator
            .generate_accounts(
                txn_executor,
                num_accounts,
                |i, zk_sig, ephemeral_key_pair| {
                    FederatedKeylessAccount::new(
                        &self.generator.iss,
                        &self.generator.aud,
                        &self.generator.uid_key,
                        &self.generator.uid_val,
                        &self.generator.jwt_header_json,
                        ephemeral_key_pair,
                        Pepper::from_number(i.try_into().unwrap()),
                        zk_sig,
                        self.jwk_addr,
                    )
                    .map(|account| {
                        LocalAccount::new_federated_keyless(
                            account.authentication_key().account_address(),
                            account,
                            0,
                        )
                    })
                },
            )
            .await
    }
}

pub struct BaseKeylessAccountGenerator {
    proof_file_path: Option<String>,
    ephemeral_secret_key: Ed25519PrivateKey,
    epk_expiry_date_secs: u64,
    iss: String,
    aud: String,
    uid_key: String,
    uid_val: String,
    jwt_header_json: String,
}

impl BaseKeylessAccountGenerator {
    fn load_lines(&self) -> Box<dyn Iterator<Item = Result<String, io::Error>>> {
        match &self.proof_file_path {
            None => {
                let proofs = include_str!("test_proofs_for_localnet_txn_emitter.txt");
                Box::new(proofs.lines().map(|line| Ok(line.to_string())))
            },
            Some(path) => {
                let file = File::open(path).unwrap();
                let reader = io::BufReader::new(file);
                Box::new(reader.lines())
            },
        }
    }

    fn create_ephemeral_key(&self) -> Result<EphemeralPrivateKey, CryptoMaterialError> {
        let serialized: &[u8] = &(self.ephemeral_secret_key.to_bytes());
        Ok(EphemeralPrivateKey::Ed25519 {
            inner_private_key: Ed25519PrivateKey::try_from(serialized)?,
        })
    }

    async fn set_sequence_numbers(
        txn_executor: &dyn ReliableTransactionSubmitter,
        accounts: Vec<LocalAccount>,
    ) -> anyhow::Result<Vec<LocalAccount>> {
        let result_futures = accounts
            .iter()
            .map(|account| txn_executor.query_sequence_number(account.address()))
            .collect::<Vec<_>>();

        let seq_nums = futures::stream::iter(result_futures)
            .buffered(QUERY_PARALLELISM)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(accounts
            .into_iter()
            .zip(seq_nums)
            .map(|(account, sequence_number)| {
                account.set_sequence_number(sequence_number);
                account
            })
            .collect())
    }

    async fn generate_accounts<F>(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_accounts: usize,
        create_account: F,
    ) -> anyhow::Result<Vec<LocalAccount>>
    where
        F: Fn(usize, ZeroKnowledgeSig, EphemeralKeyPair) -> anyhow::Result<LocalAccount>,
    {
        let mut accounts = vec![];
        let mut i = 0;
        let lines = self.load_lines();

        for line in lines {
            let serialized_proof = line?;
            let zk_sig_bytes = hex::decode(serialized_proof)?;
            let zk_sig = ZeroKnowledgeSig::try_from(zk_sig_bytes.as_slice())?;

            let ephemeral_key_pair = EphemeralKeyPair::new(
                self.create_ephemeral_key()?,
                self.epk_expiry_date_secs,
                vec![0; OpenIdSig::EPK_BLINDER_NUM_BYTES],
            )?;

            let account = create_account(i, zk_sig, ephemeral_key_pair)?;
            accounts.push(account);
            i += 1;

            if i == num_accounts {
                break;
            }
        }

        if i != num_accounts {
            bail!("not enough proofs - {num_accounts} num_accounts, {i} found")
        }

        BaseKeylessAccountGenerator::set_sequence_numbers(txn_executor, accounts).await
    }
}
