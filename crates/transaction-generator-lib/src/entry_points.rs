// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator},
    create_account_transaction,
    entry_point_trait::MultiSigConfig,
    publishing::{entry_point_trait::EntryPointTrait, publish_util::Package},
    ReliableTransactionSubmitter, RootAccountHandle,
};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::{rngs::StdRng, Rng};
use std::{borrow::Borrow, sync::Arc};

pub struct EntryPointTransactionGenerator {
    entry_points: Arc<Vec<(Box<dyn EntryPointTrait>, usize)>>,
    total_weight: usize,
}

impl EntryPointTransactionGenerator {
    pub fn new_singleton(entry_point: Box<dyn EntryPointTrait>) -> Self {
        Self::new(vec![(entry_point, 1)])
    }

    pub fn new(entry_points: Vec<(Box<dyn EntryPointTrait>, usize)>) -> Self {
        let total_weight = entry_points.iter().map(|(_, weight)| weight).sum();

        Self {
            entry_points: Arc::new(entry_points),
            total_weight,
        }
    }

    fn pick_random(
        entry_points: &[(Box<dyn EntryPointTrait>, usize)],
        total_weight: usize,
        rng: &mut StdRng,
    ) -> usize {
        let mut picked = if entry_points.len() > 1 {
            rng.gen_range(0, total_weight)
        } else {
            0
        };
        for (index, (_, weight)) in entry_points.iter().enumerate() {
            if picked < *weight {
                return index;
            }
            picked -= *weight;
        }
        unreachable!();
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EntryPointTransactionGenerator {
    fn initialize_package(
        &mut self,
        package: &Package,
        publisher: &LocalAccount,
        txn_factory: &TransactionFactory,
        rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        let mut result = vec![];

        for (entry_point, _) in self.entry_points.as_ref() {
            if let Some(initial_entry_point) = entry_point.initialize_entry_point() {
                let payload = initial_entry_point.create_payload(
                    package,
                    initial_entry_point.module_name(),
                    Some(rng),
                    Some(&publisher.address()),
                );
                result.push(publisher.sign_with_transaction_builder(txn_factory.payload(payload)))
            }
        }
        result
    }

    async fn create_generator_fn(
        &self,
        root_account: &dyn RootAccountHandle,
        txn_factory: &TransactionFactory,
        txn_executor: &dyn ReliableTransactionSubmitter,
        rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let entry_points = self.entry_points.clone();
        let total_weight = self.total_weight;

        let mut additional_signers = vec![];
        for (entry_point, _) in entry_points.as_ref() {
            additional_signers.push(match entry_point.multi_sig_additional_num() {
                MultiSigConfig::Random(num) => {
                    root_account
                        .approve_funds(
                            (num as u64)
                                * txn_factory.get_max_gas_amount()
                                * txn_factory.get_gas_unit_price(),
                            "creating random multi-sig accounts",
                        )
                        .await;

                    let new_accounts = Arc::new(
                        (0..num)
                            .map(|_| LocalAccount::generate(rng))
                            .collect::<Vec<_>>(),
                    );
                    txn_executor
                        .execute_transactions(
                            &new_accounts
                                .iter()
                                .map(|to| {
                                    create_account_transaction(
                                        root_account.get_root_account().borrow(),
                                        to.address(),
                                        txn_factory,
                                        0,
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                        .await
                        .unwrap();
                    Some(new_accounts)
                },
                _ => None,
            });
        }

        Arc::new(move |account, package, publisher, txn_factory, rng, _0| {
            let entry_point_idx = Self::pick_random(&entry_points, total_weight, rng);
            let entry_point = &entry_points[entry_point_idx].0;

            let payload = entry_point.create_payload(
                package,
                entry_point.module_name(),
                Some(rng),
                Some(&publisher.address()),
            );
            let builder = txn_factory.payload(payload);

            Some(match entry_point.multi_sig_additional_num() {
                MultiSigConfig::None => account.sign_with_transaction_builder(builder),
                MultiSigConfig::Random(_) => account.sign_multi_agent_with_transaction_builder(
                    additional_signers[entry_point_idx]
                        .as_ref()
                        .unwrap()
                        .iter()
                        .collect(),
                    builder,
                ),
                MultiSigConfig::Publisher => {
                    account.sign_multi_agent_with_transaction_builder(vec![publisher], builder)
                },
                MultiSigConfig::FeePayerPublisher => {
                    account.sign_fee_payer_with_transaction_builder(vec![], publisher, builder)
                },
            })
        })
    }
}
