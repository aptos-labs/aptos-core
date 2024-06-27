// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{emitter::local_account_generator::LocalAccountGenerator, EmitJobRequest};
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
use aptos_transaction_generator_lib::{
    CounterState, ReliableTransactionSubmitter, RootAccountHandle, SEND_AMOUNT,
};
use aptos_types::account_address::AccountAddress;
use core::{
    cmp::min,
    result::Result::{Err, Ok},
};
use futures::StreamExt;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct SourceAccountManager<'t> {
    pub source_account: Arc<LocalAccount>,
    pub txn_executor: &'t dyn ReliableTransactionSubmitter,
    pub req: &'t EmitJobRequest,
    pub txn_factory: TransactionFactory,
}

#[async_trait::async_trait]
impl<'t> RootAccountHandle for SourceAccountManager<'t> {
    async fn approve_funds(&self, amount: u64, reason: &str) {
        self.check_approve_funds(amount, reason).await.unwrap();
    }

    fn get_root_account(&self) -> Arc<LocalAccount> {
        self.source_account.clone()
    }
}

impl<'t> SourceAccountManager<'t> {
    fn source_account_address(&self) -> AccountAddress {
        self.source_account.address()
    }

    // returns true if we might want to recheck the volume, as it was auto-approved.
    async fn check_approve_funds(&self, amount: u64, reason: &str) -> Result<bool> {
        let balance = self
            .txn_executor
            .get_account_balance(self.source_account_address())
            .await?;
        Ok(if self.req.mint_to_root {
            // We have a root account, so amount of funds minted is not a problem
            // We can have multiple txn emitter running simultaneously, each coming to this check at the same time.
            // So they might all pass the check, but not be able to consume funds they need. So we check more conservatively
            // here (and root acccount should have huge balance anyways)
            if balance < amount.checked_mul(100).unwrap_or(u64::MAX / 2) {
                info!(
                    "Mint account {} current balance is {}, needing {} for {}, minting to refil it fully",
                    self.source_account_address(),
                    balance,
                    amount,
                    reason,
                );
                // Mint to refil the balance, to reduce number of mints
                self.mint_to_root(self.txn_executor, u64::MAX - balance - 1)
                    .await?;
            } else {
                info!(
                    "Mint account {} current balance is {}, needing {} for {}. Proceeding without minting, as balance would overflow otherwise",
                    self.source_account_address(),
                    balance,
                    amount,
                    reason,
                );
                assert!(balance > amount);
            }
            false
        } else {
            info!(
                "Source account {} current balance is {}, needed {} coins for {}, or {:.3}% of its balance",
                self.source_account_address(),
                balance,
                amount,
                reason,
                amount as f64 / balance as f64 * 100.0,
            );

            if balance < amount {
                return Err(anyhow!(
                    "Source ({}) doesn't have enough coins, balance {} < needed {} for {}",
                    self.source_account_address(),
                    balance,
                    amount,
                    reason
                ));
            }

            if self.req.prompt_before_spending {
                if !prompt_yes(&format!(
                    "plan will consume in total {} balance for {}, are you sure you want to proceed",
                    amount,
                    reason,
                )) {
                    panic!("Aborting");
                }
                false
            } else {
                // no checks performed, caller might want to recheck the amount makes sense
                true
            }
        })
    }

    pub async fn mint_to_root(
        &self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        amount: u64,
    ) -> Result<()> {
        info!("Minting new coins to root");

        let txn = self
            .source_account
            .sign_with_transaction_builder(self.txn_factory.payload(
                aptos_stdlib::aptos_coin_mint(self.source_account_address(), amount),
            ));

        if let Err(e) = txn_executor.execute_transactions(&[txn]).await {
            // This cannot work simultaneously across different txn emitters,
            // so check on failure if another emitter has refilled it instead

            let balance = txn_executor
                .get_account_balance(self.source_account_address())
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
}

pub struct AccountMinter<'t> {
    txn_factory: TransactionFactory,
    rng: StdRng,
    source_account: &'t SourceAccountManager<'t>,
}

impl<'t> AccountMinter<'t> {
    pub fn new(
        source_account: &'t SourceAccountManager<'t>,
        txn_factory: TransactionFactory,
        rng: StdRng,
    ) -> Self {
        Self {
            source_account,
            txn_factory,
            rng,
        }
    }

    pub fn get_needed_balance_per_account(&self, req: &EmitJobRequest, num_accounts: usize) -> u64 {
        if let Some(val) = req.coins_per_account_override {
            info!("    with {} balance each because of override", val);
            val
        } else {
            // round up:
            let txnx_per_account =
                (req.expected_max_txns + num_accounts as u64 - 1) / num_accounts as u64;
            let min_balance = req.max_gas_per_txn * req.gas_price;
            let coins_per_account = txnx_per_account
                .checked_mul(SEND_AMOUNT + req.get_expected_gas_per_txn() * req.gas_price)
                .unwrap()
                .checked_add(min_balance)
                .unwrap(); // extra coins for secure to pay none zero gas price

            info!(
                "    with {} balance each because of expecting {} txns per account, with {} gas at {} gas price per txn, and min balance {}",
                coins_per_account,
                txnx_per_account,
                req.get_expected_gas_per_txn(),
                req.gas_price,
                min_balance,
            );
            coins_per_account
        }
    }

    pub fn funds_needed_for_multi_transfer(
        name: &str,
        num_destinations: u64,
        send_amount: u64,
        max_gas_per_txn: u64,
        gas_price: u64,
    ) -> u64 {
        let min_balance = max_gas_per_txn * gas_price;

        let funds_needed = num_destinations
            .checked_mul(
                // we transfer coins_per_account and rest is overhead (burnt gas)
                send_amount + max_gas_per_txn * gas_price,
            )
            .unwrap_or_else(|| {
                panic!(
                    "money_needed_for_multi_transfer checked_mul exceeds u64: {} * ({} + {} * {})",
                    num_destinations, send_amount, max_gas_per_txn, gas_price,
                )
            })
            // we need to have the minimum balance for max gas we set
            .checked_add(min_balance)
            .unwrap_or_else(|| {
                panic!(
                "money_needed_for_multi_transfer checked_add exceeds u64: {} * ({} + {} * {}) + {}",
                num_destinations,
                send_amount,
                max_gas_per_txn,
                gas_price,
                min_balance,
            )
            });

        info!(
            "    through {} accounts with {} each due to funding {} accounts with ({} balance + {} * {} gas), and min balance {}",
            name, funds_needed, num_destinations, send_amount, max_gas_per_txn, gas_price, min_balance,
        );

        funds_needed
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
    pub async fn create_and_fund_accounts(
        &mut self,
        txn_executor: &dyn ReliableTransactionSubmitter,
        req: &EmitJobRequest,
        account_generator: Box<dyn LocalAccountGenerator>,
        max_submit_batch_size: usize,
        local_accounts: Vec<Arc<LocalAccount>>,
    ) -> Result<()> {
        let num_accounts = local_accounts.len();

        info!(
            "Account creation plan created for {} accounts and {} txns:",
            num_accounts, req.expected_max_txns,
        );

        let expected_num_seed_accounts =
            (num_accounts / 50).clamp(1, (num_accounts as f32).sqrt() as usize + 1);
        let coins_per_account = self.get_needed_balance_per_account(req, num_accounts);
        let expected_children_per_seed_account =
            (num_accounts + expected_num_seed_accounts - 1) / expected_num_seed_accounts;

        let coins_per_seed_account = Self::funds_needed_for_multi_transfer(
            "seed",
            expected_children_per_seed_account as u64,
            coins_per_account,
            self.txn_factory.get_max_gas_amount(),
            self.txn_factory.get_gas_unit_price(),
        );
        let coins_for_source = Self::funds_needed_for_multi_transfer(
            if req.mint_to_root { "root" } else { "source" },
            expected_num_seed_accounts as u64,
            coins_per_seed_account,
            self.txn_factory.get_max_gas_amount(),
            self.txn_factory.get_gas_unit_price(),
        );

        if self
            .source_account
            .check_approve_funds(coins_for_source, "initial account minter")
            .await?
        {
            // recheck value makes sense for auto-approval.
            let max_allowed = (3 * req.expected_max_txns as u128)
                .checked_mul((req.get_expected_gas_per_txn() * req.gas_price).into())
                .unwrap();
            assert!(coins_for_source as u128 <= max_allowed,
                "Overhead too large to consume funds without approval - estimated total coins needed for load test ({}) are larger than expected_max_txns * expected_gas_per_txn, multiplied by 3 to account for rounding up and overheads ({})",
                coins_for_source,
                max_allowed,
            );
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
                account_generator,
                expected_num_seed_accounts,
                coins_per_seed_account,
                max_submit_batch_size,
                &request_counters,
            )
            .await?;
        let actual_num_seed_accounts = seed_accounts.len();

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
            self.txn_factory.get_gas_unit_price(),
        );

        let start = Instant::now();
        let request_counters = txn_executor.create_counter_state();

        let approx_accounts_per_seed =
            (num_accounts + actual_num_seed_accounts - 1) / actual_num_seed_accounts;

        let local_accounts_by_seed: Vec<Vec<Arc<LocalAccount>>> = local_accounts
            .chunks(approx_accounts_per_seed)
            .map(|chunk| chunk.to_vec())
            .collect();

        let txn_factory = self.txn_factory.clone();

        // For each seed account, create a future and transfer coins from that seed account to new accounts
        let account_futures = seed_accounts
            .into_iter()
            .zip(local_accounts_by_seed.into_iter())
            .map(|(seed_account, accounts)| {
                // Spawn new threads
                create_and_fund_new_accounts(
                    seed_account,
                    accounts,
                    coins_per_account,
                    max_submit_batch_size,
                    txn_executor,
                    &txn_factory,
                    &request_counters,
                )
            });

        // Each future creates 10 accounts, limit concurrency to 1000.
        let stream = futures::stream::iter(account_futures).buffer_unordered(CREATION_PARALLELISM);
        // wait for all futures to complete
        let _: Vec<_> = stream
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .map_err(|e| format_err!("Failed to create accounts: {:?}", e))?
            .into_iter()
            .collect();

        info!(
            "Successfully completed creating {} accounts in {}s, request stats: {}",
            local_accounts.len(),
            start.elapsed().as_secs(),
            request_counters.show_simple(),
        );
        Ok(())
    }

    pub async fn create_and_fund_seed_accounts(
        &mut self,
        new_source_account: Option<LocalAccount>,
        txn_executor: &dyn ReliableTransactionSubmitter,
        account_generator: Box<dyn LocalAccountGenerator>,
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
        let source_account = match new_source_account {
            None => self.source_account.get_root_account().clone(),
            Some(param_account) => Arc::new(param_account),
        };
        while i < seed_account_num {
            let batch_size = min(max_submit_batch_size, seed_account_num - i);
            let mut rng = StdRng::from_rng(self.rng()).unwrap();
            let mut batch = account_generator
                .gen_local_accounts(txn_executor, batch_size, &mut rng)
                .await?;
            let txn_factory = &self.txn_factory;
            let create_requests: Vec<_> = batch
                .iter()
                .map(|account| {
                    create_and_fund_account_request(
                        source_account.clone(),
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
        let root_account = self.source_account.get_root_account();
        let root_address = root_account.address();
        for i in 0..NUM_TRIES {
            {
                let new_sequence_number = txn_executor.query_sequence_number(root_address).await?;
                root_account.set_sequence_number(new_sequence_number);
            }

            let new_source_account = LocalAccount::generate(self.rng());
            let txn = create_and_fund_account_request(
                root_account.clone(),
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
async fn create_and_fund_new_accounts(
    source_account: LocalAccount,
    accounts: Vec<Arc<LocalAccount>>,
    coins_per_new_account: u64,
    max_num_accounts_per_batch: usize,
    txn_executor: &dyn ReliableTransactionSubmitter,
    txn_factory: &TransactionFactory,
    counters: &CounterState,
) -> Result<()> {
    let accounts_by_batch = accounts
        .chunks(max_num_accounts_per_batch)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    let source_address = source_account.address();
    let source_account = Arc::new(source_account);
    for batch in accounts_by_batch {
        let creation_requests: Vec<_> = batch
            .iter()
            .map(|account| {
                create_and_fund_account_request(
                    source_account.clone(),
                    coins_per_new_account,
                    account.public_key(),
                    txn_factory,
                )
            })
            .collect();

        txn_executor
            .execute_transactions_with_counter(&creation_requests, counters)
            .await
            .with_context(|| format!("Account {} couldn't mint", source_address))?;
    }
    Ok(())
}

pub fn create_and_fund_account_request(
    creation_account: Arc<LocalAccount>,
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
