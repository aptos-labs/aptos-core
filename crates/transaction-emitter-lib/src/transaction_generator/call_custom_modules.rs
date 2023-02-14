// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    publishing::{module_simple::EntryPoints, publish_util::Package},
    TransactionExecutor,
};
use crate::transaction_generator::{
    publishing::publish_util::PackageHandler, TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_logger::info;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::sync::Arc;

pub struct CallCustomModulesGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    entry_point: EntryPoints,
}

impl CallCustomModulesGenerator {
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
impl TransactionGenerator for CallCustomModulesGenerator {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let needed = accounts.len() * transactions_per_account;
        let mut requests = Vec::with_capacity(needed);

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

pub struct CallCustomModulesCreator {
    txn_factory: TransactionFactory,
    packages: Arc<Vec<Package>>,
    entry_point: EntryPoints,
}

impl CallCustomModulesCreator {
    pub async fn new(
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        accounts: &mut [LocalAccount],
        txn_executor: &dyn TransactionExecutor,
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
            let txn = package.publish_transaction(account, &init_txn_factory);
            requests.push(txn);
            packages.push(package);
        }
        info!("Publishing {} packages", requests.len());
        txn_executor.execute_transactions(&requests).await.unwrap();
        info!("Done publishing {} packages", requests.len());

        Self {
            txn_factory,
            packages: Arc::new(packages),
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
            self.entry_point,
        ))
    }
}
