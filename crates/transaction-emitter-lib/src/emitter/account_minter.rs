// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    emitter::{MAX_TXNS, MAX_TXN_BATCH_SIZE, RETRY_POLICY, SEND_AMOUNT},
    query_sequence_numbers, EmitJobRequest,
};
use anyhow::{format_err, Result};
use aptos::common::types::EncodingType;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_logger::{debug, info};
use aptos_rest_client::{Client as RestClient, PendingTransaction, Response};
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        transaction::{
            authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
            SignedTransaction,
        },
        AccountKey, LocalAccount,
    },
};
use core::{
    cmp::min,
    result::Result::{Err, Ok},
};
use futures::future::try_join_all;
use rand::{rngs::StdRng, seq::SliceRandom};
use rand_core::SeedableRng;
use std::path::Path;

#[derive(Debug)]
pub struct AccountMinter<'t> {
    txn_factory: TransactionFactory,
    rng: StdRng,
    root_account: &'t mut LocalAccount,
}

impl<'t> AccountMinter<'t> {
    pub fn new(
        root_account: &'t mut LocalAccount,
        txn_factory: TransactionFactory,
        rng: StdRng,
    ) -> Self {
        Self {
            root_account,
            txn_factory,
            rng,
        }
    }
    /// workflow of mint accounts:
    /// 1. mint faucet account as the money source
    /// 2. load tc account to create seed accounts, one seed account for each endpoint
    /// 3. mint coins from faucet to new created seed accounts
    /// 4. split number of requested accounts into equally size of groups
    /// 5. each seed account take responsibility to create one size of group requested accounts and mint coins to them
    /// example:
    /// requested totally 100 new accounts with 10 endpoints
    /// will create 10 seed accounts, each seed account create 10 new accounts
    pub async fn mint_accounts(
        &mut self,
        req: &EmitJobRequest,
        total_requested_accounts: usize,
    ) -> Result<Vec<LocalAccount>> {
        let mut accounts = vec![];
        let expected_num_seed_accounts =
            if total_requested_accounts / req.rest_clients.len() > MAX_CHILD_VASP_NUM {
                total_requested_accounts / MAX_CHILD_VASP_NUM + 1
            } else {
                (total_requested_accounts / 50).max(1)
            };
        let num_accounts = total_requested_accounts - accounts.len(); // Only minting extra accounts
        let coins_per_account = SEND_AMOUNT * MAX_TXNS * 10; // extra coins for secure to pay none zero gas price
        let txn_factory = self.txn_factory.clone();

        // Create seed accounts with which we can create actual accounts concurrently. Adding
        // additional fund for paying gas fees later.
        let coins_per_seed_account = num_accounts as u64 * coins_per_account * 2;
        let seed_accounts = self
            .create_and_fund_seed_accounts(
                &req.rest_clients,
                expected_num_seed_accounts,
                coins_per_seed_account,
                req.vasp,
            )
            .await?;
        let actual_num_seed_accounts = seed_accounts.len();
        let num_new_child_accounts =
            (num_accounts + actual_num_seed_accounts - 1) / actual_num_seed_accounts;
        info!(
            "Completed minting {} seed accounts, each with {} coins",
            seed_accounts.len(),
            coins_per_seed_account
        );
        info!(
            "Minting additional {} accounts with {} coins each",
            num_accounts, coins_per_account
        );
        // tokio::time::sleep(Duration::from_secs(10)).await;

        let seed_rngs = gen_rng_for_reusable_account(actual_num_seed_accounts);
        // For each seed account, create a future and transfer coins from that seed account to new accounts
        let account_futures = seed_accounts
            .into_iter()
            .enumerate()
            .map(|(i, seed_account)| {
                // Spawn new threads
                let index = i % req.rest_clients.len();
                let cur_client = req.rest_clients[index].clone();
                create_and_fund_new_accounts(
                    seed_account,
                    num_new_child_accounts,
                    coins_per_account,
                    20,
                    cur_client,
                    &txn_factory,
                    req.vasp,
                    if req.vasp {
                        seed_rngs[i].clone()
                    } else {
                        StdRng::from_rng(self.rng()).unwrap()
                    },
                )
            });

        let mut minted_accounts = try_join_all(account_futures)
            .await
            .map_err(|e| format_err!("Failed to mint accounts: {}", e))?
            .into_iter()
            .flatten()
            .collect();

        accounts.append(&mut minted_accounts);
        assert!(
            accounts.len() >= num_accounts,
            "Something wrong in mint_account, wanted to mint {}, only have {}",
            total_requested_accounts,
            accounts.len()
        );
        info!("Successfully completed mint");
        Ok(accounts)
    }

    pub async fn create_and_fund_seed_accounts(
        &mut self,
        rest_clients: &[RestClient],
        seed_account_num: usize,
        coins_per_seed_account: u64,
        vasp: bool,
    ) -> Result<Vec<LocalAccount>> {
        info!("Creating and minting seeds accounts");
        let mut i = 0;
        let mut seed_accounts = vec![];
        if vasp {
            let client = self.pick_mint_client(rest_clients).clone();
            info!("Loading VASP account as seed accounts");
            let load_account_num = min(seed_account_num, MAX_VASP_ACCOUNT_NUM);
            for i in 0..load_account_num {
                let account = self.load_vasp_account(&client, i).await?;
                seed_accounts.push(account);
            }
            info!("Loaded {} VASP accounts", seed_accounts.len());
            return Ok(seed_accounts);
        }
        while i < seed_account_num {
            let client = self.pick_mint_client(rest_clients).clone();
            let batch_size = min(MAX_TXN_BATCH_SIZE, seed_account_num - i);
            let mut rng = StdRng::from_rng(self.rng()).unwrap();
            let mut batch = gen_random_accounts(batch_size, &mut rng);
            let creation_account = &mut self.root_account;
            let txn_factory = &self.txn_factory;
            let create_requests = batch
                .iter()
                .map(|account| {
                    create_and_fund_account_request(
                        creation_account,
                        coins_per_seed_account,
                        account.public_key(),
                        txn_factory,
                    )
                })
                .collect();
            execute_and_wait_transactions(&client, creation_account, create_requests).await?;
            i += batch_size;
            seed_accounts.append(&mut batch);
        }
        info!("Completed creating and funding seed accounts");

        Ok(seed_accounts)
    }

    pub async fn load_vasp_account(
        &self,
        client: &RestClient,
        index: usize,
    ) -> Result<LocalAccount> {
        let file = "vasp".to_owned() + index.to_string().as_str() + ".key";
        let mint_key: Ed25519PrivateKey = EncodingType::BCS
            .load_key("vasp private key", Path::new(&file))
            .unwrap();
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

    fn pick_mint_client<'a>(&mut self, clients: &'a [RestClient]) -> &'a RestClient {
        clients
            .choose(self.rng())
            .expect("json-rpc clients can not be empty")
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }
}

fn gen_rng_for_reusable_account(count: usize) -> Vec<StdRng> {
    // use same seed for reuse account creation and reuse
    // TODO: Investigate why we use the same seed and then consider changing
    // this so that we don't do this, since it causes conflicts between
    // runs of the emitter.
    let mut seed = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let mut rngs = vec![];
    for i in 0..count {
        seed[31] = i as u8;
        rngs.push(StdRng::from_seed(seed));
    }
    rngs
}

/// Create `num_new_accounts` by transferring coins from `source_account`. Return Vec of created
/// accounts
async fn create_and_fund_new_accounts<R>(
    mut source_account: LocalAccount,
    num_new_accounts: usize,
    coins_per_new_account: u64,
    max_num_accounts_per_batch: u64,
    client: RestClient,
    txn_factory: &TransactionFactory,
    reuse_account: bool,
    mut rng: R,
) -> Result<Vec<LocalAccount>>
where
    R: ::rand_core::RngCore + ::rand_core::CryptoRng,
{
    let mut i = 0;
    let mut accounts = vec![];
    while i < num_new_accounts {
        let batch_size = min(
            max_num_accounts_per_batch as usize,
            min(MAX_TXN_BATCH_SIZE, num_new_accounts - i),
        );
        let mut batch = if reuse_account {
            info!("Loading {} accounts if they exist", batch_size);
            gen_reusable_accounts(&client, batch_size, &mut rng).await?
        } else {
            let batch = gen_random_accounts(batch_size, &mut rng);
            let creation_requests = batch
                .as_slice()
                .iter()
                .map(|account| {
                    create_and_fund_account_request(
                        &mut source_account,
                        coins_per_new_account,
                        account.public_key(),
                        txn_factory,
                    )
                })
                .collect();
            execute_and_wait_transactions(&client, &mut source_account, creation_requests).await?;
            batch
        };

        i += batch.len();
        accounts.append(&mut batch);
    }
    Ok(accounts)
}

async fn gen_reusable_accounts<R>(
    client: &RestClient,
    num_accounts: usize,
    rng: &mut R,
) -> Result<Vec<LocalAccount>>
where
    R: rand_core::RngCore + ::rand_core::CryptoRng,
{
    let mut vasp_accounts = vec![];
    let mut i = 0;
    while i < num_accounts {
        vasp_accounts.push(gen_reusable_account(client, rng).await?);
        i += 1;
    }
    Ok(vasp_accounts)
}

async fn gen_reusable_account<R>(client: &RestClient, rng: &mut R) -> Result<LocalAccount>
where
    R: ::rand_core::RngCore + ::rand_core::CryptoRng,
{
    let account_key = AccountKey::generate(rng);
    let address = account_key.authentication_key().derived_address();
    let sequence_number = match query_sequence_numbers(client, &[address]).await {
        Ok(v) => v[0],
        Err(_) => 0,
    };
    Ok(LocalAccount::new(address, account_key, sequence_number))
}

fn gen_random_accounts<R>(num_accounts: usize, rng: &mut R) -> Vec<LocalAccount>
where
    R: ::rand_core::RngCore + ::rand_core::CryptoRng,
{
    (0..num_accounts)
        .map(|_| LocalAccount::generate(rng))
        .collect()
}

pub fn create_and_fund_account_request(
    creation_account: &mut LocalAccount,
    amount: u64,
    pubkey: &Ed25519PublicKey,
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    let preimage = AuthenticationKeyPreimage::ed25519(pubkey);
    let auth_key = AuthenticationKey::from_preimage(&preimage);
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_stdlib::encode_account_utils_create_and_fund_account(
            auth_key.derived_address(),
            amount,
        ),
    ))
}

pub async fn execute_and_wait_transactions(
    client: &RestClient,
    account: &mut LocalAccount,
    txns: Vec<SignedTransaction>,
) -> Result<()> {
    debug!(
        "[{:?}] Submitting transactions {} - {} for {}",
        client,
        account.sequence_number() - txns.len() as u64,
        account.sequence_number(),
        account.address()
    );

    let pending_txns: Vec<Response<PendingTransaction>> = try_join_all(
        txns.iter()
            .map(|t| RETRY_POLICY.retry(move || client.submit(t))),
    )
    .await?;

    try_join_all(
        pending_txns
            .iter()
            .map(|pt| RETRY_POLICY.retry(move || client.wait_for_transaction(pt.inner()))),
    )
    .await
    .map_err(|e| format_err!("Failed to wait for transactions: {}", e))?;

    debug!(
        "[{:?}] Account {} is at sequence number {} now",
        client,
        account.address(),
        account.sequence_number()
    );
    Ok(())
}

const MAX_CHILD_VASP_NUM: usize = 65536;
const MAX_VASP_ACCOUNT_NUM: usize = 16;
