// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    publishing::{module_simple::EntryPoints, publish_util::Package},
    TransactionExecutor,
};
use crate::transaction_generator::{
    publishing::publish_util::PackageHandler, TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_infallible::RwLock;
use aptos_logger::{info, sample, sample::SampleRate, warn};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::{sync::Arc, time::Duration};

pub struct CallCustomModulesGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
    entry_point: EntryPoints,
}

impl CallCustomModulesGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        packages: Arc<Vec<Package>>,
        accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
        entry_point: EntryPoints,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            packages,
            accounts_pool,
            entry_point,
        }
    }
}

#[async_trait]
impl TransactionGenerator for CallCustomModulesGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let needed = accounts.len() * transactions_per_account;
        let mut requests = Vec::with_capacity(needed);

        let mut accounts_to_burn = if let Some(accounts_pool_lock) = &self.accounts_pool {
            let mut accounts_pool = accounts_pool_lock.write();
            let num_in_pool = accounts_pool.len();
            if num_in_pool < needed {
                sample!(
                    SampleRate::Duration(Duration::from_secs(10)),
                    warn!("Cannot fetch enough accounts from pool, left in pool {}, needed {}", num_in_pool, needed);
                );
                return Vec::new();
            }
            accounts_pool
                .drain((num_in_pool - needed)..)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        for account in accounts {
            for _ in 0..transactions_per_account {
                let mut next_to_burn = accounts_to_burn.pop();
                let request = self
                    .packages
                    .choose(&mut self.rng)
                    .unwrap()
                    .use_specific_transaction(
                        self.entry_point,
                        next_to_burn.as_mut().unwrap_or(account),
                        &self.txn_factory,
                        Some(&mut self.rng),
                        None,
                    );
                requests.push(request);
            }
        }
        requests
    }
}

pub struct CallCustomModulesCreator {
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
    entry_point: EntryPoints,
}

impl CallCustomModulesCreator {
    #[allow(dead_code)]
    pub async fn new(
        txn_factory: TransactionFactory,
        accounts: &mut [LocalAccount],
        txn_executor: &dyn TransactionExecutor,
        accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
        entry_point: EntryPoints,
        num_modules: usize,
    ) -> Self {
        let mut rng = StdRng::from_entropy();
        assert!(accounts.len() >= num_modules);
        let mut requests = Vec::with_capacity(accounts.len());
        let mut package_handler = PackageHandler::new();
        let mut packages = Vec::new();
        for account in accounts.iter_mut().take(num_modules) {
            let package = package_handler.pick_package(&mut rng, account);
            let txn = package.publish_transaction(account, &txn_factory);
            requests.push(txn);
            packages.push(package);
        }
        info!("Publishing {} packages", requests.len());
        txn_executor.execute_transactions(&requests).await.unwrap();
        info!("Done publishing {} packages", requests.len());

        Self {
            txn_factory,
            packages: Arc::new(packages),
            accounts_pool,
            entry_point,
        }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for CallCustomModulesCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(CallCustomModulesGenerator::new(
            StdRng::from_entropy(),
            self.txn_factory.clone(),
            self.packages.clone(),
            self.accounts_pool.clone(),
            self.entry_point,
        ))
    }
}
