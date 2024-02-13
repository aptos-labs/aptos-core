// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator}, econia_order_generator, publishing::publish_util::Package, ObjectPool, ReliableTransactionSubmitter};
use aptos_sdk::{
    bcs,
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use move_core_types::{
    ident_str,
    language_storage::ModuleId,
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use rand::{rngs::StdRng, Rng};

use std::sync::Arc;
// use aptos_infallible::RwLock;

/// Placeas a bid limit order.
pub fn place_bid_limit_order(
    module_id: ModuleId,
    size: u64,
    price: u64
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_limit_order").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&price).unwrap()
        ],
    ))
}

/// Placeas an ask limit order.
pub fn place_ask_limit_order(
    module_id: ModuleId,
    size: u64,
    price: u64
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_limit_order").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&price).unwrap()
        ],
    ))
}

/// Placeas a bid market order.
pub fn place_bid_market_order(
    module_id: ModuleId,
    size: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_market_order").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&size).unwrap(),
        ],
    ))
}

/// Placeas an ask market order.
pub fn place_ask_market_order(
    module_id: ModuleId,
    size: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_market_order").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&size).unwrap(),
        ],
    ))
}

pub struct EconiaLimitOrderTransactionGenerator {
    to_setup: Arc<ObjectPool<LocalAccount>>,
    done: Arc<ObjectPool<LocalAccount>>,
    num_base_orders_placed: usize,
}

impl EconiaLimitOrderTransactionGenerator {
    pub fn new(
        to_setup: Arc<ObjectPool<LocalAccount>>,
        done: Arc<ObjectPool<LocalAccount>>,
    ) -> Self {
        Self {
            to_setup,
            done,
            num_base_orders_placed: 0,
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EconiaLimitOrderTransactionGenerator {
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
        &mut self,
        _root_account: &mut LocalAccount,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let to_setup = self.to_setup.clone();
        let done = self.done.clone();
        self.num_base_orders_placed += 1;
        if self.num_base_orders_placed <= 100 || self.num_base_orders_placed % 2 == 0 {
            Arc::new(move |account, package, publisher, txn_factory, rng| {
                let batch = to_setup.take_from_pool(1, true, rng);
                if batch.is_empty() {
                    return vec![];
                }
                done.add_to_pool(batch);
                let bid_size = rng.gen_range(2, 10);
                let ask_size = rng.gen_range(2, 10);

                let bid_price = rng.gen_range(1, 200);
                let ask_price = rng.gen_range(201, 400);

                let bid_builder = txn_factory.payload(place_bid_limit_order(package.get_module_id("txn_generator_utils"), bid_size, bid_price));
                let ask_builder = txn_factory.payload(place_ask_limit_order(package.get_module_id("txn_generator_utils"), ask_size, ask_price));
                vec![
                    account.sign_with_transaction_builder(bid_builder),
                    account.sign_with_transaction_builder(ask_builder)
                ]
            })
        } else {
            Arc::new(move |account, package, publisher, txn_factory, rng| {
                let batch = to_setup.take_from_pool(1, true, rng);
                if batch.is_empty() {
                    return vec![];
                }
                done.add_to_pool(batch);

                let bid_size = rng.gen_range(2, 10);
                let ask_size = rng.gen_range(2, 10);

                let bid_builder = txn_factory.payload(place_bid_market_order(package.get_module_id("txn_generator_utils"), bid_size));
                let ask_builder = txn_factory.payload(place_ask_market_order(package.get_module_id("txn_generator_utils"), ask_size));
                vec![
                    account.sign_with_transaction_builder(bid_builder),
                    account.sign_with_transaction_builder(ask_builder)
                ]
            })
        }
    }
}
