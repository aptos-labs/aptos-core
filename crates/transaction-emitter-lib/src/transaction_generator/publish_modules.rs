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
use rand::{rngs::StdRng, seq::SliceRandom};
use std::sync::Arc;

#[allow(dead_code)]
pub struct PublishPackageGenerator {
    rng: StdRng,
    package_handler: Arc<RwLock<PackageHandler>>,
    txn_factory: TransactionFactory,
    gas_price: u64,
}

impl PublishPackageGenerator {
    pub fn new(
        rng: StdRng,
        package_handler: Arc<RwLock<PackageHandler>>,
        txn_factory: TransactionFactory,
        gas_price: u64,
    ) -> Self {
        Self {
            rng,
            package_handler,
            txn_factory,
            gas_price,
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
                    self.gas_price,
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
    rng: StdRng,
    txn_factory: TransactionFactory,
    package_handler: Arc<RwLock<PackageHandler>>,
    gas_price: u64,
}

impl PublishPackageCreator {
    pub fn new(rng: StdRng, txn_factory: TransactionFactory, gas_price: u64) -> Self {
        Self {
            rng,
            txn_factory,
            package_handler: Arc::new(RwLock::new(PackageHandler::new())),
            gas_price,
        }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for PublishPackageCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(PublishPackageGenerator::new(
            self.rng.clone(),
            self.package_handler.clone(),
            self.txn_factory.clone(),
            self.gas_price,
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
    entry_point: EntryPoints,
}

impl CallDifferentModulesGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        packages: Arc<Vec<Package>>,
        entry_point: EntryPoints,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            packages,
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
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            for _ in 0..transactions_per_account {
                let request = self
                    .packages
                    .choose(&mut self.rng)
                    .unwrap()
                    .use_specific_transaction(
                        self.entry_point,
                        account,
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
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    entry_point: EntryPoints,
}

impl CallDifferentModulesCreator {
    pub async fn new(
        mut rng: StdRng,
        txn_factory: TransactionFactory,
        accounts: &mut [LocalAccount],
        txn_executor: &dyn TransactionExecutor,
        entry_point: EntryPoints,
        num_modules: usize,
    ) -> Self {
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
            rng,
            txn_factory,
            packages: Arc::new(packages),
            entry_point,
        }
    }
}

#[async_trait]
impl TransactionGeneratorCreator for CallDifferentModulesCreator {
    async fn create_transaction_generator(&mut self) -> Box<dyn TransactionGenerator> {
        Box::new(CallDifferentModulesGenerator::new(
            self.rng.clone(),
            self.txn_factory.clone(),
            self.packages.clone(),
            self.entry_point,
        ))
    }
}
