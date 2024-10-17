// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    publishing::{module_simple::EntryPoints, publish_util::Package},
    ObjectPool, ReliableTransactionSubmitter,
};
use crate::{
    call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator},
    create_account_transaction,
    publishing::module_simple::MultiSigConfig,
    RootAccountHandle,
};
use aptos_sdk::{
    bcs,
    move_types::{ident_str, language_storage::ModuleId},
    transaction_builder::TransactionFactory,
    types::{
        transaction::{EntryFunction, SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use async_trait::async_trait;
use rand::{rngs::StdRng, Rng};
use std::{borrow::Borrow, sync::Arc};

pub struct StableCoinMinterGenerator {
    pub max_mint_amount: u64,
    pub batch_size: usize,
    pub minter_accounts: Arc<ObjectPool<LocalAccount>>,
    pub destination_accounts: Arc<ObjectPool<LocalAccount>>,
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
        Arc::new(move |_fee_payer, _package, publisher, txn_factory, rng| {
            let minter = minter_accounts.take_from_pool(1, true, rng);
            let destinations = destination_accounts.take_from_pool(batch_size, true, rng);
            if minter.is_empty() || destinations.is_empty() {
                return None;
            }
            let mint_amounts = destinations
                .iter()
                .map(|_| rng.gen_range(1, max_mint_amount))
                .collect::<Vec<_>>();
            let txn = Some(
                minter.get(0_).unwrap().sign_with_transaction_builder(
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
                ),
            );
            minter_accounts.add_to_pool(minter);
            destination_accounts.add_to_pool(destinations);
            txn
        })
    }
}
