// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    local_account_generator::LocalAccountGenerator, parse_seed,
    transaction_executor::RestApiReliableTransactionSubmitter,
};
use crate::{emitter::create_private_key_account_generator, EmitJobRequest};
use anyhow::{anyhow, bail, format_err, Context, Result};
use aptos_config::config::DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE;
use aptos_crypto::{ed25519::Ed25519PrivateKey, encoding_type::EncodingType};
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, AccountKey, LocalAccount},
};
use aptos_transaction_generator_lib::{
    CounterState, ReliableTransactionSubmitter, RootAccountHandle,
};
use aptos_types::account_address::AccountAddress;
use core::result::Result::{Err, Ok};
use futures::{future::try_join_all, StreamExt};
use log::{error, info};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct SourceAccountManager<'t> {
    pub source_account: Arc<LocalAccount>,
    pub txn_executor: &'t dyn ReliableTransactionSubmitter,
    pub mint_to_root: bool,
    pub prompt_before_spending: bool,
    pub txn_factory: TransactionFactory,
}

#[async_trait::async_trait]
impl RootAccountHandle for SourceAccountManager<'_> {
    async fn approve_funds(&self, amount: u64, reason: &str) {
        self.check_approve_funds(amount, reason).await.unwrap();
    }

    fn get_root_account(&self) -> Arc<LocalAccount> {
        self.source_account.clone()
    }
}

impl SourceAccountManager<'_> {
    fn source_account_address(&self) -> AccountAddress {
        self.source_account.address()
    }

    // returns true if we might want to recheck the volume, as it was auto-approved.
    async fn check_approve_funds(&self, amount: u64, reason: &str) -> Result<bool> {
        let balance = self
            .txn_executor
            .get_account_balance(self.source_account_address())
            .await?;
        Ok(if self.mint_to_root {
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

            if self.prompt_before_spending {
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
    source_account: &'t SourceAccountManager<'t>,
}

impl<'t> AccountMinter<'t> {
    pub fn new(
        source_account: &'t SourceAccountManager<'t>,
        txn_factory: TransactionFactory,
    ) -> Self {
        Self {
            source_account,
            txn_factory,
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
        seed_accounts: Vec<Arc<LocalAccount>>,
        local_accounts: Vec<Arc<LocalAccount>>,
        coins_per_account: u64,
        max_submit_batch_size: usize,
        mint_to_root: bool,
        secondary_source_account: Option<LocalAccount>,
    ) -> Result<()> {
        let num_accounts = local_accounts.len();

        info!(
            "Account creation plan created for {} accounts and {} coins per account",
            num_accounts, coins_per_account,
        );

        let expected_children_per_seed_account = num_accounts.div_ceil(seed_accounts.len());

        let coins_per_seed_account = Self::funds_needed_for_multi_transfer(
            "seed",
            expected_children_per_seed_account as u64,
            coins_per_account,
            self.txn_factory.get_max_gas_amount(),
            self.txn_factory.get_gas_unit_price(),
        );
        let coins_for_source = Self::funds_needed_for_multi_transfer(
            if mint_to_root { "root" } else { "source" },
            seed_accounts.len() as u64,
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
            let max_allowed = (3 * coins_per_account as u128)
                .checked_mul(num_accounts as u128)
                .unwrap();
            assert!(coins_for_source as u128 <= max_allowed,
                "Overhead too large to consume funds without approval - estimated total coins needed for load test ({}) are larger than expected_max_txns * expected_gas_per_txn, multiplied by 3 to account for rounding up and overheads ({})",
                coins_for_source,
                max_allowed,
            );
        }

        let new_source_account = if let Some(new_source_account) = secondary_source_account {
            self.create_new_source_account(txn_executor, coins_for_source, &new_source_account)
                .await?;
            Some(new_source_account)
        } else {
            None
        };

        let start = Instant::now();

        let request_counters = txn_executor.create_counter_state();

        // Create seed accounts with which we can create actual accounts concurrently. Adding
        // additional fund for paying gas fees later.
        self.create_and_fund_seed_accounts(
            new_source_account,
            txn_executor,
            &seed_accounts,
            coins_per_seed_account,
            max_submit_batch_size,
            &request_counters,
        )
        .await?;

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

        let approx_accounts_per_seed = num_accounts.div_ceil(seed_accounts.len());

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
            .map_err(|e| format_err!("Failed to create accounts: {:?}", e))?;

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
        seed_accounts: &[Arc<LocalAccount>],
        coins_per_seed_account: u64,
        max_submit_batch_size: usize,
        counters: &CounterState,
    ) -> Result<()> {
        info!(
            "Creating and funding seeds accounts (txn {} gas price)",
            self.txn_factory.get_gas_unit_price()
        );
        let source_account = match new_source_account {
            None => self.source_account.get_root_account().clone(),
            Some(param_account) => Arc::new(param_account),
        };

        for chunk in seed_accounts.chunks(max_submit_batch_size) {
            let txn_factory = &self.txn_factory;
            let create_requests: Vec<_> = chunk
                .iter()
                .map(|account| {
                    create_and_fund_account_request(
                        source_account.clone(),
                        coins_per_seed_account,
                        account.address(),
                        txn_factory,
                    )
                })
                .collect();
            txn_executor
                .execute_transactions_with_counter(&create_requests, counters)
                .await?;
        }

        Ok(())
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
        new_source_account: &LocalAccount,
    ) -> Result<()> {
        const NUM_TRIES: usize = 3;
        let root_account = self.source_account.get_root_account();
        let root_address = root_account.address();
        for i in 0..NUM_TRIES {
            {
                let new_sequence_number = txn_executor.query_sequence_number(root_address).await?;
                root_account.set_sequence_number(new_sequence_number);
            }

            let txn = create_and_fund_account_request(
                root_account.clone(),
                coins_for_source,
                new_source_account.address(),
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
                assert_eq!(
                    new_source_account.sequence_number(),
                    txn_executor
                        .query_sequence_number(new_source_account.address())
                        .await?,
                );
                info!(
                    "New source account created {}",
                    new_source_account.address()
                );
                return Ok(());
            }
        }
        bail!("Couldn't create new source account");
    }
}

/// Create `num_new_accounts` by transferring coins from `source_account`. Return Vec of created
/// accounts
async fn create_and_fund_new_accounts(
    source_account: Arc<LocalAccount>,
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
    for (batch_index, batch) in accounts_by_batch.into_iter().enumerate() {
        let creation_requests: Vec<_> = batch
            .iter()
            .map(|account| {
                create_and_fund_account_request(
                    source_account.clone(),
                    coins_per_new_account,
                    account.address(),
                    txn_factory,
                )
            })
            .collect();

        txn_executor
            .execute_transactions_with_counter(&creation_requests, counters)
            .await
            .with_context(|| {
                format!(
                    "Account {} couldn't mint batch {}",
                    source_address, batch_index
                )
            })?;
    }
    Ok(())
}

pub fn create_and_fund_account_request(
    creation_account: Arc<LocalAccount>,
    amount: u64,
    address: AccountAddress,
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_account_transfer(address, amount)),
    )
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

pub struct BulkAccountCreationConfig {
    max_submit_batch_size: usize,
    skip_funding_accounts: bool,
    seed: Option<[u8; 32]>,
    mint_to_root: bool,
    prompt_before_spending: bool,
    create_secondary_source_account: bool,
    expected_gas_per_transfer: u64,
    expected_gas_per_account_create: u64,
}

impl BulkAccountCreationConfig {
    pub fn new(
        max_submit_batch_size: usize,
        skip_funding_accounts: bool,
        seed: Option<&str>,
        mint_to_root: bool,
        prompt_before_spending: bool,
        create_secondary_source_account: bool,
        expected_gas_per_transfer: u64,
        expected_gas_per_account_create: u64,
    ) -> Self {
        Self {
            max_submit_batch_size,
            skip_funding_accounts,
            seed: seed.map(parse_seed),
            mint_to_root,
            prompt_before_spending,
            create_secondary_source_account,
            expected_gas_per_transfer,
            expected_gas_per_account_create,
        }
    }
}

impl From<&EmitJobRequest> for BulkAccountCreationConfig {
    fn from(req: &EmitJobRequest) -> Self {
        Self {
            max_submit_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            skip_funding_accounts: req.skip_funding_accounts,
            seed: req.account_minter_seed,
            mint_to_root: req.mint_to_root,
            prompt_before_spending: req.prompt_before_spending,
            create_secondary_source_account: req.mint_to_root
                || !req.coordination_delay_between_instances.is_zero(),
            expected_gas_per_transfer: req.get_expected_gas_per_transfer(),
            expected_gas_per_account_create: req.get_expected_gas_per_account_create(),
        }
    }
}

pub async fn bulk_create_accounts(
    coin_source_account: Arc<LocalAccount>,
    txn_executor: &RestApiReliableTransactionSubmitter,
    txn_factory: &TransactionFactory,
    account_generator: Box<dyn LocalAccountGenerator>,
    config: BulkAccountCreationConfig,
    num_accounts: usize,
    coins_per_account: u64,
) -> Result<Vec<LocalAccount>> {
    let source_account_manager = SourceAccountManager {
        source_account: coin_source_account,
        txn_executor,
        mint_to_root: config.mint_to_root,
        prompt_before_spending: config.prompt_before_spending,
        txn_factory: txn_factory.clone(),
    };

    let seed = config.seed.unwrap_or_else(|| {
        let mut rng = StdRng::from_entropy();
        rng.r#gen()
    });
    info!(
        "AccountMinter Seed (reuse accounts by passing into --account-minter-seed): {:?}",
        seed
    );

    let mut rng = StdRng::from_seed(seed);

    let secondary_source_account = if config.create_secondary_source_account {
        let new_source_account = account_generator
            .gen_local_accounts(txn_executor, 1, &mut rng)
            .await?;
        assert_eq!(1, new_source_account.len());
        new_source_account.into_iter().next()
    } else {
        None
    };

    let num_seed_accounts = (num_accounts / 50).clamp(1, (num_accounts as f32).sqrt() as usize + 1);
    let seed_accounts = create_private_key_account_generator()
        .gen_local_accounts(txn_executor, num_seed_accounts, &mut rng)
        .await?;

    let accounts = account_generator
        .gen_local_accounts(txn_executor, num_accounts, &mut rng)
        .await?;

    info!(
        "Generated and fetched re-usable accounts for seed {:?}",
        seed
    );

    let all_accounts_already_exist = accounts.iter().all(|account| account.sequence_number() > 0);
    let all_seed_accounts_already_exist = seed_accounts
        .iter()
        .all(|account| account.sequence_number() > 0);
    let all_source_accounts_exist = secondary_source_account
        .as_ref()
        .map_or(true, |v| v.sequence_number() > 0);

    info!(
        "Accounts exist: {}, seed accounts exist: {}, source account exists: {}",
        all_accounts_already_exist, all_seed_accounts_already_exist, all_source_accounts_exist,
    );

    let send_money_gas = if all_source_accounts_exist
        && all_accounts_already_exist
        && all_seed_accounts_already_exist
    {
        config.expected_gas_per_transfer
    } else {
        config.expected_gas_per_account_create
    };

    let mut account_minter = AccountMinter::new(
        &source_account_manager,
        txn_factory.clone().with_max_gas_amount(send_money_gas),
    );

    if !config.skip_funding_accounts {
        let accounts: Vec<_> = accounts.into_iter().map(Arc::new).collect();
        let seed_accounts: Vec<_> = seed_accounts.into_iter().map(Arc::new).collect();

        account_minter
            .create_and_fund_accounts(
                txn_executor,
                seed_accounts.clone(),
                accounts.clone(),
                coins_per_account,
                config.max_submit_batch_size,
                config.mint_to_root,
                secondary_source_account,
            )
            .await?;
        let accounts: Vec<_> = accounts
            .into_iter()
            .map(|a| Arc::try_unwrap(a).unwrap())
            .collect();
        info!("Accounts created and funded");
        Ok(accounts)
    } else {
        info!(
            "Account reuse plan created for {} accounts and min balance {}",
            accounts.len(),
            coins_per_account,
        );

        let balance_futures = accounts
            .iter()
            .map(|account| txn_executor.get_account_balance(account.address()));
        let balances: Vec<_> = try_join_all(balance_futures).await?;
        let underfunded = accounts
            .iter()
            .zip(balances)
            .enumerate()
            .filter(|(_idx, (_account, balance))| *balance < coins_per_account)
            .collect::<Vec<_>>();

        let first = underfunded.first();
        assert!(
            underfunded.is_empty(),
            "{} out of {} accounts are underfunded. For example Account[{}] {} has balance {} < needed_min_balance {}",
            underfunded.len(),
            accounts.len(),
            first.unwrap().0, // idx
            first.unwrap().1.0.address(), // account
            first.unwrap().1.1, // balance
            coins_per_account,
        );

        info!("Skipping funding accounts");
        Ok(accounts)
    }
}
