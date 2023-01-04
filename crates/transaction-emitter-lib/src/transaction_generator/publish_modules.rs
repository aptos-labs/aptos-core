// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::{
    publishing::publish_util::PackageHandler, TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_infallible::RwLock;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::{prelude::StdRng, Rng, seq::SliceRandom};
use rand_core::{OsRng, SeedableRng};
use std::sync::Arc;

#[allow(dead_code)]
pub struct PublishPackageGenerator {
    rng: StdRng,
    package_handler: Arc<RwLock<PackageHandler>>,
    txn_factory: TransactionFactory,
}

impl PublishPackageGenerator {
    pub fn new(
        rng: StdRng,
        package_handler: Arc<RwLock<PackageHandler>>,
        txn_factory: TransactionFactory,
    ) -> Self {
        Self {
            rng,
            package_handler,
            txn_factory,
        }
    }
}

#[async_trait]
impl TransactionGenerator for PublishPackageGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            // First publish the module and then use it
            let package = self
                .package_handler
                .write()
                .pick_package(&mut self.rng, account);
            let txn = package.publish_transaction(account, &self.txn_factory);
            requests.push(txn);
            // use module published
            // for _ in 1..transactions_per_account - 1 {
            for _ in 1..transactions_per_account {
                let request = package.use_random_transaction(
                    &mut self.rng,
                    account,
                    &self.txn_factory,
                );
                requests.push(request);
            }
            // republish
            // let package = self
            //     .package_handler
            //     .write()
            //     .pick_package(&mut self.rng, account);
            // let txn = package.publish_transaction(account, &self.txn_factory);
            // requests.push(txn);
        }
        requests
    }
}

pub struct PublishPackageCreator {
    txn_factory: TransactionFactory,
    package_handler: Arc<RwLock<PackageHandler>>,
}

impl PublishPackageCreator {
    pub fn new(txn_factory: TransactionFactory) -> Self {
        Self {
            txn_factory,
            package_handler: Arc::new(RwLock::new(PackageHandler::new())),
        }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for PublishPackageCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(PublishPackageGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            self.package_handler.clone(),
            self.txn_factory.clone(),
        ))
    }
}

// ================= CallDifferentModules ===========

use super::{
    publishing::{module_simple::EntryPoints, publish_util::Package},
    TransactionExecutor,
};
use aptos_logger::info;

#[allow(dead_code)]
pub struct CallDifferentModulesGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
    entry_point: EntryPoints,
}

impl CallDifferentModulesGenerator {
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
impl TransactionGenerator for CallDifferentModulesGenerator {
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
            accounts_pool.drain((num_in_pool - needed)..).collect::<Vec<_>>()
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

pub struct CallDifferentModulesCreator {
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
    entry_point: EntryPoints,
}

impl CallDifferentModulesCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        accounts: &mut [LocalAccount],
        txn_executor: &dyn TransactionExecutor,
        accounts_pool: Option<Arc<RwLock<Vec<LocalAccount>>>>,
        entry_point: EntryPoints,
        num_modules: usize,
    ) -> Self {
        let mut rng = StdRng::from_seed(OsRng.gen());
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
        txn_executor.execute_transactions(&requests).await;
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
impl TransactionGeneratorCreator for CallDifferentModulesCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(CallDifferentModulesGenerator::new(
            StdRng::from_seed(OsRng.gen()),
            self.txn_factory.clone(),
            self.packages.clone(),
            self.accounts_pool.clone(),
            self.entry_point,
        ))
    }
}
