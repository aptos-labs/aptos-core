// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    publishing::{module_simple::EntryPoints, publish_util::Package},
    ReliableTransactionSubmitter,
};
use crate::{
    call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator},
    create_account_transaction,
    publishing::module_simple::MultiSigConfig,
};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::rngs::StdRng;
use std::sync::Arc;

pub struct EntryPointTransactionGenerator {
    pub entry_point: EntryPoints,
}

#[async_trait]
impl UserModuleTransactionGenerator for EntryPointTransactionGenerator {
    fn initialize_package(
        &mut self,
        package: &Package,
        publisher: &mut LocalAccount,
        txn_factory: &TransactionFactory,
        rng: &mut StdRng,
    ) -> Vec<SignedTransaction> {
        if let Some(initial_entry_point) = self.entry_point.initialize_entry_point() {
            let payload = initial_entry_point.create_payload(
                package.get_module_id(initial_entry_point.module_name()),
                Some(rng),
                Some(&publisher.address()),
            );
            vec![publisher.sign_with_transaction_builder(txn_factory.payload(payload))]
        } else {
            vec![]
        }
    }

    async fn create_generator_fn(
        &self,
        init_accounts: &mut [LocalAccount],
        txn_factory: &TransactionFactory,
        txn_executor: &dyn ReliableTransactionSubmitter,
        rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let entry_point = self.entry_point;

        let additional_signers = match entry_point.multi_sig_additional_num() {
            MultiSigConfig::Random(num) => {
                let new_accounts = Arc::new(
                    (0..num)
                        .map(|_| LocalAccount::generate(rng))
                        .collect::<Vec<_>>(),
                );
                let sender = init_accounts.get_mut(0).unwrap();
                txn_executor
                    .execute_transactions(
                        &new_accounts
                            .iter()
                            .map(|to| {
                                create_account_transaction(sender, to.address(), txn_factory, 0)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .await
                    .unwrap();
                Some(new_accounts)
            },
            _ => None,
        };

        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let payload = entry_point.create_payload(
                package.get_module_id(entry_point.module_name()),
                Some(rng),
                Some(&publisher.address()),
            );
            let builder = txn_factory.payload(payload);

            match entry_point.multi_sig_additional_num() {
                MultiSigConfig::None => account.sign_with_transaction_builder(builder),
                MultiSigConfig::Random(_) => account.sign_multi_agent_with_transaction_builder(
                    additional_signers.as_ref().unwrap().iter().collect(),
                    builder,
                ),
                MultiSigConfig::Publisher => {
                    account.sign_multi_agent_with_transaction_builder(vec![publisher], builder)
                },
            }
        })
    }
}
