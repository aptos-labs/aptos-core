// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::EmitJobRequest;
use anyhow::{anyhow, bail, format_err, Context, Result};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
};
use aptos_logger::{error, info};
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        transaction::{authenticator::AuthenticationKey, SignedTransaction},
        AccountKey, LocalAccount,
    },
};
use aptos_transaction_generator_lib::{CounterState, ReliableTransactionSubmitter, SEND_AMOUNT};
use core::{
    cmp::min,
    result::Result::{Err, Ok},
};
use futures::StreamExt;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct AccountMinter<'t> {
    txn_factory: TransactionFactory,
    rng: StdRng,
    source_account: &'t mut LocalAccount,
}

impl<'t> AccountMinter<'t> {
    pub fn new(
        source_account: &'t mut LocalAccount,
        txn_factory: TransactionFactory,
        rng: StdRng,
    ) -> Self {
        Self {
            source_account,
            txn_factory,
            rng,
        }
    }

    /// workflow of create accounts:
    /// 1. Use given source_account as the money source
    /// 1a. Optionally, and if it is root account, mint balance to that account
    /// 2. load tc account to create seed accounts, one seed account for each endpoint
    /// 3. mint coins from faucet to new created seed accounts
    /// 4. split number of requested accounts into equally size of groups
    /// 5. each seed account take responsibility to create one size of group requested accounts and mint coins to them
    /// example:
    /// requested totally 100 new accounts with 10 endpoints
    /// will create 10 seed accounts, each seed account create 10 new accounts
    pub async fn create_accounts(
        &mut self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        req: &EmitJobRequest,
        max_submit_batch_size: usize,
        total_requested_accounts: usize,
    ) -> Result<Vec<LocalAccount>> {
        let mut accounts = vec![];
        let expected_num_seed_accounts = (total_requested_accounts / 50)
            .clamp(1, (total_requested_accounts as f32).sqrt() as usize + 1);
        let num_accounts = total_requested_accounts - accounts.len(); // Only minting extra accounts
        let coins_per_account = if let Some(val) = req.coins_per_account_override {
            val
        } else {
            (req.expected_max_txns / total_requested_accounts as u64)
                .checked_mul(SEND_AMOUNT + req.expected_gas_per_txn * req.gas_price)
                .unwrap()
                .checked_add(
                    req.max_gas_per_txn * req.gas_price
                        // for module publishing
                        + 2 * req.max_gas_per_txn * req.gas_price * req.init_gas_price_multiplier,
                )
                .unwrap() // extra coins for secure to pay none zero gas price
        };
        let txn_factory = self.txn_factory.clone();
        let expected_children_per_seed_account =
            (num_accounts + expected_num_seed_accounts - 1) / expected_num_seed_accounts;
        info!(
            "Account creation plan created for {} accounts with {} balance each.",
            num_accounts, coins_per_account
        );
        info!(
            "    because of expecting {} txns and {} gas at {} gas price for each ",
            req.expected_max_txns, req.expected_gas_per_txn, req.gas_price,
        );
        let coins_per_seed_account = (expected_children_per_seed_account as u64)
            .checked_mul(
                coins_per_account
                    + req.max_gas_per_txn * req.gas_price * req.init_gas_price_multiplier,
            )
            .unwrap_or_else(|| {
                panic!(
                    "coins_per_seed_account exceeds u64: {} * ({} + {} * {} * {})",
                    expected_children_per_seed_account,
                    coins_per_account,
                    req.max_gas_per_txn,
                    req.gas_price,
                    req.init_gas_price_multiplier
                )
            });
        info!(
            "    through {} seed accounts with {} each, each to fund {} accounts",
            expected_num_seed_accounts, coins_per_seed_account, expected_children_per_seed_account,
        );
        let coins_for_source = coins_per_seed_account
            .checked_add(req.max_gas_per_txn * req.gas_price * req.init_gas_price_multiplier)
            .unwrap_or_else(|| {
                panic!(
                    "coins_for_source exceeds u64: {} * {}",
                    coins_per_seed_account, expected_num_seed_accounts
                )
            })
            .checked_mul(expected_num_seed_accounts as u64)
            .unwrap_or_else(|| {
                panic!(
                    "coins_for_source exceeds u64: {} * {}",
                    coins_per_seed_account, expected_num_seed_accounts
                )
            });

        let balance = txn_executor
            .get_account_balance(self.source_account.address())
            .await?;
        if req.mint_to_root {
            // Check against more than coins_for_source, because we can have multiple txn emitter running simultaneously
            if balance < coins_for_source.checked_mul(100).unwrap_or(u64::MAX / 2) {
                info!(
                    "Mint account {} current balance is {}, needing {}, minting to refil it fully",
                    self.source_account.address(),
                    balance,
                    coins_for_source,
                );
                // Mint to refil the balance, to reduce number of mints
                self.mint_to_root(txn_executor, u64::MAX - balance - 1)
                    .await?;
            } else {
                info!(
                    "Mint account {} current balance is {}, needing {}. Proceeding without minting, as balance would overflow otherwise",
                    self.source_account.address(),
                    balance,
                    coins_for_source,
                );
                assert!(balance > coins_for_source);
            }
        } else {
            info!(
                "Source account {} current balance is {}, needed {} coins, or {:.3}% of its balance",
                self.source_account.address(),
                balance,
                coins_for_source,
                coins_for_source as f64 / balance as f64 * 100.0,
            );

            if balance < coins_for_source {
                return Err(anyhow!(
                    "Source ({}) doesn't have enough coins, balance {} < needed {}",
                    self.source_account.address(),
                    balance,
                    coins_for_source
                ));
            }

            if req.prompt_before_spending {
                if !prompt_yes(&format!(
                    "plan will consume in total {} balance, are you sure you want to proceed",
                    coins_for_source
                )) {
                    panic!("Aborting");
                }
            } else {
                let max_allowed = (2 * req.expected_max_txns as u128)
                    .checked_mul((req.expected_gas_per_txn * req.gas_price).into())
                    .unwrap();
                assert!(coins_for_source as u128 <= max_allowed,
                    "Estimated total coins needed for load test ({}) are larger than expected_max_txns * expected_gas_per_txn, multiplied by 2 to account for rounding up ({})",
                    coins_for_source,
                    max_allowed,
                );
            }
        }

        let new_source_account = if !req.coordination_delay_between_instances.is_zero() {
            Some(
                self.create_new_source_account(txn_executor, coins_for_source)
                    .await?,
            )
        } else {
            None
        };

        let start = Instant::now();

        let request_counters = txn_executor.create_counter_state();

        // Create seed accounts with which we can create actual accounts concurrently. Adding
        // additional fund for paying gas fees later.
        let seed_accounts = self
            .create_and_fund_seed_accounts(
                new_source_account,
                txn_executor,
                expected_num_seed_accounts,
                coins_per_seed_account,
                max_submit_batch_size,
                &request_counters,
            )
            .await?;
        let actual_num_seed_accounts = seed_accounts.len();
        let num_new_child_accounts =
            (num_accounts + actual_num_seed_accounts - 1) / actual_num_seed_accounts;
        info!(
            "Completed creating {} seed accounts in {}s, each with {} coins, request stats: {}",
            seed_accounts.len(),
            start.elapsed().as_secs(),
            coins_per_seed_account,
            request_counters.show_simple(),
        );
        info!(
            "Creating additional {} accounts with {} coins each (txn {} gas price)",
            num_accounts,
            coins_per_account,
            txn_factory.get_gas_unit_price(),
        );

        let start = Instant::now();
        let request_counters = txn_executor.create_counter_state();

        // For each seed account, create a future and transfer coins from that seed account to new accounts
        let account_futures = seed_accounts.into_iter().map(|seed_account| {
            // Spawn new threads
            create_and_fund_new_accounts(
                seed_account,
                num_new_child_accounts,
                coins_per_account,
                max_submit_batch_size,
                txn_executor,
                &txn_factory,
                StdRng::from_rng(self.rng()).unwrap(),
                &request_counters,
            )
        });

        // Each future creates 10 accounts, limit concurrency to 1000.
        let stream = futures::stream::iter(account_futures).buffer_unordered(CREATION_PARALLELISM);
        // wait for all futures to complete
        let mut created_accounts = stream
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .map_err(|e| format_err!("Failed to create accounts: {:?}", e))?
            .into_iter()
            .flatten()
            .collect();

        accounts.append(&mut created_accounts);
        assert!(
            accounts.len() >= num_accounts,
            "Something wrong in create_accounts, wanted to create {}, only have {}",
            total_requested_accounts,
            accounts.len()
        );
        info!(
            "Successfully completed creating {} accounts in {}s, request stats: {}",
            actual_num_seed_accounts * num_new_child_accounts,
            start.elapsed().as_secs(),
            request_counters.show_simple(),
        );
        Ok(accounts)
    }

    pub async fn mint_to_root(
        &mut self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        amount: u64,
    ) -> Result<()> {
        info!("Minting new coins to root");

        let txn = self
            .source_account
            .sign_with_transaction_builder(self.txn_factory.payload(
                aptos_stdlib::aptos_coin_mint(self.source_account.address(), amount),
            ));

        if let Err(e) = txn_executor.execute_transactions(&[txn]).await {
            // This cannot work simultaneously across different txn emitters,
            // so check on failure if another emitter has refilled it instead

            let balance = txn_executor
                .get_account_balance(self.source_account.address())
                .await?;
            if balance > u64::MAX / 2 {
                Ok(())
            } else {
                Err(e)
            }
        } else {
            Ok(())
        }
    }

    pub async fn create_and_fund_seed_accounts(
        &mut self,
        mut new_source_account: Option<LocalAccount>,
        txn_executor: &dyn ReliableTransactionSubmitter,
        seed_account_num: usize,
        coins_per_seed_account: u64,
        max_submit_batch_size: usize,
        counters: &CounterState,
    ) -> Result<Vec<LocalAccount>> {
        info!(
            "Creating and funding seeds accounts (txn {} gas price)",
            self.txn_factory.get_gas_unit_price()
        );
        let mut i = 0;
        let mut seed_accounts = vec![];
        while i < seed_account_num {
            let batch_size = min(max_submit_batch_size, seed_account_num - i);
            let mut rng = StdRng::from_rng(self.rng()).unwrap();
            let mut batch = gen_reusable_accounts(txn_executor, batch_size, &mut rng).await?;
            let txn_factory = &self.txn_factory;
            let create_requests: Vec<_> = batch
                .iter()
                .map(|account| {
                    create_and_fund_account_request(
                        if let Some(account) = &mut new_source_account {
                            account
                        } else {
                            self.source_account
                        },
                        coins_per_seed_account,
                        account.public_key(),
                        txn_factory,
                    )
                })
                .collect();
            txn_executor
                .execute_transactions_with_counter(&create_requests, counters)
                .await?;

            i += batch_size;
            seed_accounts.append(&mut batch);
        }

        Ok(seed_accounts)
    }

    pub async fn load_vasp_account(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        index: usize,
    ) -> Result<LocalAccount> {
        let file = "vasp".to_owned() + index.to_string().as_str() + ".key";
        let mint_key: Ed25519PrivateKey = EncodingType::BCS
            .load_key("vasp private key", Path::new(&file))
            .unwrap();
        let account_key = AccountKey::from_private_key(mint_key);
        let address = account_key.authentication_key().account_address();
        let sequence_number = txn_executor
            .query_sequence_number(address)
            .await
            .map_err(|e| {
                format_err!(
                    "query_sequence_number for account {} failed: {:?}",
                    index,
                    e
                )
            })?;
        Ok(LocalAccount::new(address, account_key, sequence_number))
    }

    pub async fn create_new_source_account(
        &mut self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        coins_for_source: u64,
    ) -> Result<LocalAccount> {
        const NUM_TRIES: usize = 3;
        for i in 0..NUM_TRIES {
            self.source_account.set_sequence_number(
                txn_executor
                    .query_sequence_number(self.source_account.address())
                    .await?,
            );

            let new_source_account = LocalAccount::generate(self.rng());
            let txn = create_and_fund_account_request(
                self.source_account,
                coins_for_source,
                new_source_account.public_key(),
                &self.txn_factory,
            );
            if let Err(e) = txn_executor.execute_transactions(&[txn]).await {
                error!(
                    "Couldn't create new source account, {:?}, try {}, retrying",
                    e, i
                );
                // random sleep to coordinate with other instances
                if i + 1 < NUM_TRIES {
                    let sleep_secs = rand::thread_rng().gen_range(0, 10);
                    tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
                }
            } else {
                new_source_account.set_sequence_number(
                    txn_executor
                        .query_sequence_number(new_source_account.address())
                        .await?,
                );
                info!(
                    "New source account created {}",
                    new_source_account.address()
                );
                return Ok(new_source_account);
            }
        }
        bail!("Couldn't create new source account");
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }
}

/// Create `num_new_accounts` by transferring coins from `source_account`. Return Vec of created
/// accounts
async fn create_and_fund_new_accounts<R>(
    mut source_account: LocalAccount,
    num_new_accounts: usize,
    coins_per_new_account: u64,
    max_num_accounts_per_batch: usize,
    txn_executor: &dyn ReliableTransactionSubmitter,
    txn_factory: &TransactionFactory,
    mut rng: R,
    counters: &CounterState,
) -> Result<Vec<LocalAccount>>
where
    R: ::rand_core::RngCore + ::rand_core::CryptoRng,
{
    let mut i = 0;
    let mut accounts = vec![];

    while i < num_new_accounts {
        let batch_size = min(max_num_accounts_per_batch, num_new_accounts - i);
        let mut batch = {
            let batch = gen_reusable_accounts(txn_executor, batch_size, &mut rng).await?;
            let creation_requests: Vec<_> = batch
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

            txn_executor
                .execute_transactions_with_counter(&creation_requests, counters)
                .await
                .with_context(|| format!("Account {} couldn't mint", source_account.address()))?;

            batch
        };

        i += batch.len();
        accounts.append(&mut batch);
    }
    Ok(accounts)
}

async fn gen_reusable_accounts<R>(
    txn_executor: &dyn ReliableTransactionSubmitter,
    num_accounts: usize,
    rng: &mut R,
) -> Result<Vec<LocalAccount>>
where
    R: rand_core::RngCore + ::rand_core::CryptoRng,
{
    let mut vasp_accounts = vec![];
    let mut i = 0;
    while i < num_accounts {
        vasp_accounts.push(gen_reusable_account(txn_executor, rng).await?);
        i += 1;
    }
    Ok(vasp_accounts)
}

async fn gen_reusable_account<R>(
    txn_executor: &dyn ReliableTransactionSubmitter,
    rng: &mut R,
) -> Result<LocalAccount>
where
    R: ::rand_core::RngCore + ::rand_core::CryptoRng,
{
    let account_key = AccountKey::generate(rng);
    let address = account_key.authentication_key().account_address();
    let sequence_number = txn_executor.query_sequence_number(address).await?;
    Ok(LocalAccount::new(address, account_key, sequence_number))
}

pub fn create_and_fund_account_request(
    creation_account: &mut LocalAccount,
    amount: u64,
    pubkey: &Ed25519PublicKey,
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    let auth_key = AuthenticationKey::ed25519(pubkey);
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_stdlib::aptos_account_transfer(auth_key.account_address(), amount),
    ))
}

const CREATION_PARALLELISM: usize = 500;

/// Copied from aptos crate, to not need to link it whole.
/// Prompts for confirmation until a yes or no is given explicitly
pub fn prompt_yes(prompt: &str) -> bool {
    let mut result: Result<bool, ()> = Err(());

    // Read input until a yes or a no is given
    while result.is_err() {
        println!("{} [yes/no] >", prompt);
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        result = match input.trim().to_lowercase().as_str() {
            "yes" | "y" => Ok(true),
            "no" | "n" => Ok(false),
            _ => Err(()),
        };
    }
    result.unwrap()
}
