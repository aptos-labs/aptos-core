// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{publishing::publish_util::Package, ReliableTransactionSubmitter};
use crate::{
    create_account_transaction,
    publishing::{entry_point_trait::PreBuiltPackages, publish_util::PackageHandler},
    RootAccountHandle, TransactionGenerator, TransactionGeneratorCreator,
};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use log::{error, info};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::{borrow::Borrow, sync::Arc};

// Fn + Send + Sync, as it will be called from multiple threads simultaneously
// if you need any coordination, use Arc<RwLock<X>> fields
pub type TransactionGeneratorWorker = dyn Fn(
        &LocalAccount,
        &Package,
        &LocalAccount,
        &TransactionFactory,
        &mut StdRng,
    ) -> Option<SignedTransaction>
    + Send
    + Sync;

#[async_trait]
pub trait UserModuleTransactionGenerator: Sync + Send {
    /// Called for each instance of the module we publish,
    /// if any additional transactions are needed to setup the package.
    /// For example, if we need to create an NFT collection, or otherwise
    /// call directly additional initialization of the module.
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        Vec::new()
    }

    /// Create TransactionGeneratorWorker function, which will be called
    /// to generate transactions to submit.
    /// TransactionGeneratorWorker will be called from multiple threads simultaneously.
    /// if you need any coordination, use Arc<RwLock<X>> fields
    /// If you need to send any additional initialization transactions
    /// (like creating and funding additional accounts), you can do so by using provided txn_executor
    async fn create_generator_fn(
        &self,
        root_account: &dyn RootAccountHandle,
        txn_factory: &TransactionFactory,
        txn_executor: &dyn ReliableTransactionSubmitter,
        rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker>;
}

pub struct PlainUserModuleTransactionGenerator {
    generator_worker: Arc<TransactionGeneratorWorker>,
}

impl PlainUserModuleTransactionGenerator {
    pub fn new(generator_worker: Arc<TransactionGeneratorWorker>) -> Self {
        Self { generator_worker }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for PlainUserModuleTransactionGenerator {
    async fn create_generator_fn(
        &self,
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        self.generator_worker.clone()
    }
}

pub struct CustomModulesDelegationGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<(Package, LocalAccount)>>,
    txn_generator: Arc<TransactionGeneratorWorker>,
}

impl CustomModulesDelegationGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        packages: Arc<Vec<(Package, LocalAccount)>>,
        txn_generator: Arc<TransactionGeneratorWorker>,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            packages,
            txn_generator,
        }
    }
}

impl TransactionGenerator for CustomModulesDelegationGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);

        for _ in 0..num_to_create {
            let (package, publisher) = self.packages.choose(&mut self.rng).unwrap();
            let request = (self.txn_generator)(
                account,
                package,
                publisher,
                &self.txn_factory,
                &mut self.rng,
            );
            if let Some(request) = request {
                requests.push(request);
            }
        }
        requests
    }
}

pub struct CustomModulesDelegationGeneratorCreator {
    txn_factory: TransactionFactory,
    packages: Arc<Vec<(Package, LocalAccount)>>,
    txn_generator: Arc<TransactionGeneratorWorker>,
}

impl CustomModulesDelegationGeneratorCreator {
    #[allow(dead_code)]
    pub fn new_raw(
        txn_factory: TransactionFactory,
        packages: Arc<Vec<(Package, LocalAccount)>>,
        txn_generator: Arc<TransactionGeneratorWorker>,
    ) -> Self {
        Self {
            txn_factory,
            packages,
            txn_generator,
        }
    }

    pub async fn new(
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        pre_built: &'static dyn PreBuiltPackages,
        package_name: &str,
        workload: &mut dyn UserModuleTransactionGenerator,
    ) -> Self {
        let packages = Self::publish_package(
            init_txn_factory.clone(),
            root_account,
            txn_executor,
            num_modules,
            pre_built,
            package_name,
            None,
        )
        .await;
        let worker = Self::create_worker(
            init_txn_factory,
            root_account,
            txn_executor,
            &packages,
            workload,
        )
        .await;
        Self {
            txn_factory,
            packages: Arc::new(packages),
            txn_generator: worker,
        }
    }

    pub async fn create_worker(
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        packages: &[(Package, LocalAccount)],
        workload: &mut dyn UserModuleTransactionGenerator,
    ) -> Arc<TransactionGeneratorWorker> {
        let mut rng = StdRng::from_entropy();
        let mut requests_initialize = Vec::with_capacity(packages.len());

        for (package, publisher) in packages.iter() {
            requests_initialize.append(&mut workload.initialize_package(
                package,
                publisher,
                &init_txn_factory,
                &mut rng,
            ));
        }

        if !requests_initialize.is_empty() {
            info!(
                "Initializing workload with {} transactions",
                requests_initialize.len()
            );
            txn_executor
                .execute_transactions(&requests_initialize)
                .await
                .unwrap();
        }

        info!("Done preparing workload for {} packages", packages.len());

        workload
            .create_generator_fn(root_account, &init_txn_factory, txn_executor, &mut rng)
            .await
    }

    pub async fn publish_package(
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        pre_built: &'static dyn PreBuiltPackages,
        package_name: &str,
        publisher_balance: Option<u64>,
    ) -> Vec<(Package, LocalAccount)> {
        let mut rng = StdRng::from_entropy();
        let mut requests_create = Vec::with_capacity(num_modules);
        let mut accounts = Vec::new();

        let publisher_balance = publisher_balance.unwrap_or(
            4 * init_txn_factory.get_gas_unit_price() * init_txn_factory.get_max_gas_amount(),
        );
        let total_funds = (num_modules as u64) * publisher_balance;
        root_account
            .approve_funds(total_funds, "funding publishers")
            .await;

        for _i in 0..num_modules {
            let publisher = LocalAccount::generate(&mut rng);
            let publisher_address = publisher.address();
            requests_create.push(create_account_transaction(
                root_account.get_root_account().borrow(),
                publisher_address,
                &init_txn_factory,
                publisher_balance,
            ));

            accounts.push(publisher);
        }

        info!("Creating {} publisher accounts", requests_create.len());
        // all publishers are created from root account, split it up.
        for req_chunk in requests_create.chunks(100) {
            txn_executor
                .execute_transactions(req_chunk)
                .await
                .inspect_err(|err| {
                    error!(
                        "Failed to execute creation of publisher accounts: {:#}",
                        err
                    )
                })
                .unwrap();
        }

        let packages = Self::publish_package_to_accounts(init_txn_factory, txn_executor, pre_built, package_name, &accounts).await;

        packages.into_iter().zip(accounts.into_iter()).collect()
    }

    pub async fn publish_package_to_accounts(
        init_txn_factory: TransactionFactory,
        txn_executor: &dyn ReliableTransactionSubmitter,
        pre_built: &'static dyn PreBuiltPackages,
        package_name: &str,
        accounts: &[LocalAccount],
    ) -> Vec<Package> {
        let mut rng = StdRng::from_entropy();
        let mut requests_publish = Vec::with_capacity(accounts.len());
        let mut package_handler = PackageHandler::new(pre_built, package_name);
        let mut packages = Vec::new();

        for publisher in accounts {
            let package = package_handler.pick_package(&mut rng, publisher.address());
            for payload in package.publish_transaction_payload(&init_txn_factory.get_chain_id()) {
                requests_publish.push(
                    publisher.sign_with_transaction_builder(init_txn_factory.payload(payload)),
                );
            }

            packages.push(package);
        }

        info!(
            "Publishing {} copies of package {}",
            requests_publish.len(),
            package_name
        );
        txn_executor
            .execute_transactions(&requests_publish)
            .await
            .inspect_err(|err| error!("Failed to publish test package {}: {:#}", package_name, err))
            .unwrap();

        info!("Done publishing {} packages", packages.len());

        packages
    }
}

impl TransactionGeneratorCreator for CustomModulesDelegationGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(CustomModulesDelegationGenerator::new(
            StdRng::from_entropy(),
            self.txn_factory.clone(),
            self.packages.clone(),
            self.txn_generator.clone(),
        ))
    }
}
