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
use rand::rngs::StdRng;
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
                let request = package.use_transaction(
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
    async fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(PublishPackageGenerator::new(
            self.rng.clone(),
            self.package_handler.clone(),
            self.txn_factory.clone(),
            self.gas_price,
        ))
    }
}
