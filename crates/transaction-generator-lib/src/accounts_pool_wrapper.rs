// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ObjectPool, TransactionGenerator, TransactionGeneratorCreator};
use velor_sdk::types::{transaction::SignedTransaction, LocalAccount};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

/// Wrapper that allows inner transaction generator to have unique accounts
/// for all transactions (instead of having 5-20 transactions per account, as default)
/// This is achieved via using accounts from the pool that account creatin can fill,
/// and burning (removing accounts from the pool) them - basically using them only once.
/// (we cannot use more as sequence number is not updated on failure)
pub struct AccountsPoolWrapperGenerator {
    rng: StdRng,
    generator: Box<dyn TransactionGenerator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
}

impl AccountsPoolWrapperGenerator {
    pub fn new(
        rng: StdRng,
        generator: Box<dyn TransactionGenerator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    ) -> Self {
        Self {
            rng,
            generator,
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGenerator for AccountsPoolWrapperGenerator {
    fn generate_transactions(
        &mut self,
        _account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let accounts_to_use =
            self.source_accounts_pool
                .take_from_pool(num_to_create, true, &mut self.rng);
        if accounts_to_use.is_empty() {
            return Vec::new();
        }
        let txns = accounts_to_use
            .iter()
            .flat_map(|account| self.generator.generate_transactions(account, 1))
            .collect();
        if let Some(destination_accounts_pool) = &self.destination_accounts_pool {
            destination_accounts_pool.add_to_pool(accounts_to_use);
        }
        txns
    }
}

pub struct AccountsPoolWrapperCreator {
    creator: Box<dyn TransactionGeneratorCreator>,
    source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
    destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
}

impl AccountsPoolWrapperCreator {
    pub fn new(
        creator: Box<dyn TransactionGeneratorCreator>,
        source_accounts_pool: Arc<ObjectPool<LocalAccount>>,
        destination_accounts_pool: Option<Arc<ObjectPool<LocalAccount>>>,
    ) -> Self {
        Self {
            creator,
            source_accounts_pool,
            destination_accounts_pool,
        }
    }
}

impl TransactionGeneratorCreator for AccountsPoolWrapperCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(AccountsPoolWrapperGenerator::new(
            StdRng::from_entropy(),
            self.creator.create_transaction_generator(),
            self.source_accounts_pool.clone(),
            self.destination_accounts_pool.clone(),
        ))
    }
}
