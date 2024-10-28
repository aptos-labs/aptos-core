// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{publishing::publish_util::Package, ObjectPool, ReliableTransactionSubmitter};
use crate::{
    call_custom_modules::{SequenceNumberUpdateWorker, TransactionGeneratorWorker, UserModuleTransactionGenerator}, BucketedAccountPool, RootAccountHandle
};
use aptos_sdk::{
    bcs,
    move_types::{ident_str, language_storage::ModuleId, account_address::AccountAddress},
    transaction_builder::TransactionFactory,
    types::{
        transaction::{EntryFunction, SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use aptos_logger::info;
use async_trait::async_trait;
use rand::{rngs::StdRng, Rng};
use std::sync::Arc;

pub struct StableCoinConfigureControllerGenerator {}
impl StableCoinConfigureControllerGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StableCoinConfigureControllerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for StableCoinConfigureControllerGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        vec![]
    }

    async fn create_generator_fn(
        &self,
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        Arc::new(|minter_account, package, _publisher, txn_factory, _rng| {
            let txn = minter_account.sign_with_transaction_builder(txn_factory.payload(
                TransactionPayload::EntryFunction(EntryFunction::new(
                    package.get_module_id("stablecoin"),
                    ident_str!("configure_controller").to_owned(),
                    vec![],
                    vec![],
                )),
            ));
            Some(txn)
        })
    }

    async fn create_sequence_number_update_fn(
        &self,
    ) -> Option<Arc<SequenceNumberUpdateWorker>> {
        None
    }
}

pub struct StableCoinSetMinterAllowanceGenerator {}
impl StableCoinSetMinterAllowanceGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StableCoinSetMinterAllowanceGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for StableCoinSetMinterAllowanceGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        vec![]
    }

    async fn create_generator_fn(
        &self,
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        Arc::new(|minter_account, package, _publisher, txn_factory, _rng| {
            let allowance: u64 = 1000_0000_0000;
            let txn = minter_account.sign_with_transaction_builder(txn_factory.payload(
                TransactionPayload::EntryFunction(EntryFunction::new(
                    package.get_module_id("stablecoin"),
                    ident_str!("configure_minter").to_owned(),
                    vec![],
                    vec![bcs::to_bytes(&allowance).unwrap()],
                )),
            ));
            Some(txn)
        })
    }

    async fn create_sequence_number_update_fn(
        &self,
    ) -> Option<Arc<SequenceNumberUpdateWorker>> {
        None
    }
}

pub struct StableCoinMinterGenerator {
    pub max_mint_amount: u64,
    pub batch_size: usize,
    pub minter_accounts: Arc<BucketedAccountPool<AccountAddress>>,
    pub destination_accounts: Arc<ObjectPool<LocalAccount>>,
}

impl StableCoinMinterGenerator {
    pub fn new(
        max_mint_amount: u64,
        batch_size: usize,
        minter_accounts: Arc<BucketedAccountPool<AccountAddress>>,
        destination_accounts: Arc<ObjectPool<LocalAccount>>,
    ) -> Self {
        Self {
            max_mint_amount,
            batch_size,
            minter_accounts,
            destination_accounts,
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for StableCoinMinterGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        vec![]
    }

    async fn create_generator_fn(
        &self,
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let minter_accounts = self.minter_accounts.clone();
        let destination_accounts = self.destination_accounts.clone();
        let max_mint_amount = self.max_mint_amount;
        let batch_size = self.batch_size;
        Arc::new(move |account, _package, publisher, txn_factory, rng| {
            let minter = minter_accounts.take_from_pool(account.address(), 1, true, rng);
            let destinations = destination_accounts.take_from_pool(batch_size, true, rng);
            if minter.is_empty() || destinations.is_empty() {
                return None;
            }
            let mint_amounts = destinations
                .iter()
                .map(|_| rng.gen_range(1, max_mint_amount))
                .collect::<Vec<_>>();
            let txn = match batch_size {
                0 => None,
                1 => Some(minter.first().unwrap().sign_with_transaction_builder(
                    txn_factory.payload(TransactionPayload::EntryFunction(EntryFunction::new(
                        ModuleId::new(publisher.address(), ident_str!("stablecoin").to_owned()),
                        ident_str!("mint").to_owned(),
                        vec![],
                        vec![
                            bcs::to_bytes(&destinations.first().unwrap().address()).unwrap(),
                            bcs::to_bytes(&mint_amounts.first().unwrap()).unwrap(),
                        ],
                    ))),
                )),
                _ => Some(minter.first().unwrap().sign_with_transaction_builder(
                    txn_factory.payload(TransactionPayload::EntryFunction(EntryFunction::new(
                        ModuleId::new(publisher.address(), ident_str!("stablecoin").to_owned()),
                        ident_str!("batch_mint").to_owned(),
                        vec![],
                        vec![
                                bcs::to_bytes(
                                    &destinations.iter().map(|x| x.address()).collect::<Vec<_>>(),
                                )
                                .unwrap(),
                                bcs::to_bytes(&mint_amounts).unwrap(),
                            ],
                    ))),
                )),
            };
            minter_accounts.add_to_bucket(account.address(), minter);
            destination_accounts.add_to_pool(destinations);
            txn
        })
    }

    async fn create_sequence_number_update_fn(
        &self,
    ) -> Option<Arc<SequenceNumberUpdateWorker>> {
        let minter_accounts = self.minter_accounts.clone();
        Some(
            Arc::new(move |latest_fetched_sequence_numbers| {
                info!("Calling update sequence number for minter accounts {:?}", latest_fetched_sequence_numbers);
                for (minter_account, sequence_number) in latest_fetched_sequence_numbers {
                    minter_accounts.update_sequence_number(minter_account, *sequence_number);
                }
            })
        )
    }
}
