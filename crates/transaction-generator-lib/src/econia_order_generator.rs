// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator}, publishing::publish_util::Package, ReliableTransactionSubmitter, RootAccountHandle};
use aptos_sdk::{
    bcs,
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag, StructTag},
    identifier::Identifier,
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use rand::{rngs::StdRng, Rng};

use std::sync::Arc;
const BASE_COIN_TYPES: [&str; 104] = ["AAC", "ABC", "ACC", "ADC", "AEC", "AFC", "AGC", "AHC", "AIC", "AJC", "AKC", "ALC", "AMC", "ANC", "AOC", "APC", "AQC", "ARC", "ASC", "ATC", "AUC", "AVC", "AWC", "AXC", "AYC", "AZC",
                                     "BAC", "BBC", "BCC", "BDC", "BEC", "BFC", "BGC", "BHC", "BIC", "BJC", "BKC", "BLC", "BMC", "BNC", "BOC", "BPC", "BQC", "BRC", "BSC", "BTC", "BUC", "BVC", "BWC", "BXC", "BYC", "BZC",
                                     "CAC", "CBC", "CCC", "CDC", "CEC", "CFC", "CGC", "CHC", "CIC", "CJC", "CKC", "CLC", "CMC", "CNC", "COC", "CPC", "CQC", "CRC", "CSC", "CTC", "CUC", "CVC", "CWC", "CXC", "CYC", "CZC",                                    
                                     "DAC", "DBC", "DCC", "DDC", "DEC", "DFC", "DGC", "DHC", "DIC", "DJC", "DKC", "DLC", "DMC", "DNC", "DOC", "DPC", "DQC", "DRC", "DSC", "DTC", "DUC", "DVC", "DWC", "DXC", "DYC", "DZC",
                                    ];
// const QUOTE_COIN_TYPES: [&str; 11] = ["QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC"];

fn base_coin_type(market_id: u64) -> &'static str {
    BASE_COIN_TYPES[(market_id-1) as usize]
}

fn quote_coin_type(_market_id: u64) -> &'static str {
    "QC"
}

/// Placeas a bid limit order.
pub fn place_bid_limit_order(
    module_id: ModuleId,
    size: u64,
    price: u64,
    market_id: u64,
    publisher: AccountAddress
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_limit_order").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name:  Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&price).unwrap(),
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

/// Placeas an ask limit order.
pub fn place_ask_limit_order(
    module_id: ModuleId,
    size: u64,
    price: u64,
    market_id: u64,
    publisher: AccountAddress
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_limit_order").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module:  Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module:  Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&price).unwrap(),
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

/// Placeas a bid market order.
pub fn place_bid_market_order(
    module_id: ModuleId,
    size: u64,
    market_id: u64,
    publisher: AccountAddress
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_market_order").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

/// Placeas an ask market order.
pub fn place_ask_market_order(
    module_id: ModuleId,
    size: u64,
    market_id: u64,
    publisher: AccountAddress
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_market_order").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

pub fn register_market(
    module_id: ModuleId,
    num_markets: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("register_multiple_markets").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&num_markets).unwrap(),
        ],
    ))
}

pub fn register_market_accounts(
    module_id: ModuleId,
    market_id: u64,
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("register_market_accounts").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

pub fn deposit_coins(
    module_id: ModuleId,
    market_id: u64,
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("deposit_coins").to_owned(),
        vec![TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id))).unwrap(),
            type_params: vec![],
        })), TypeTag::Struct(Box::new(StructTag {
            address: publisher,
            module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
            name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id))).unwrap(),
            type_params: vec![],
        }))],
        vec![
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

pub struct EconiaLimitOrderTransactionGenerator {
    num_base_orders_placed: usize,
    num_markets: Arc<u64>,
}

impl EconiaLimitOrderTransactionGenerator {
    pub fn new(
        num_markets: u64
    ) -> Self {
        Self {
            num_base_orders_placed: 0,
            num_markets: Arc::new(num_markets)
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
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let num_markets = self.num_markets.clone();
        self.num_base_orders_placed += 1;
        if self.num_base_orders_placed <= (*num_markets as usize)*10 || self.num_base_orders_placed % 2 == 0 {
            Arc::new(move |account, package, publisher, txn_factory, rng| {
                let mut requests = vec![];
                let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;

                let bid_size = rng.gen_range(4, 14);
                let ask_size = rng.gen_range(4, 14);

                let bid_price = rng.gen_range(1, 200);
                let ask_price = rng.gen_range(201, 400);

                let bid_builder = txn_factory.payload(place_bid_limit_order(package.get_module_id("txn_generator_utils"), bid_size, bid_price, market_id, publisher.address()));
                let ask_builder = txn_factory.payload(place_ask_limit_order(package.get_module_id("txn_generator_utils"), ask_size, ask_price, market_id, publisher.address()));

                requests.push(account.sign_with_transaction_builder(bid_builder));
                requests.push(account.sign_with_transaction_builder(ask_builder));
                requests
            })
        } else {
            Arc::new(move |account, package, publisher, txn_factory, rng| {
                let mut requests = vec![];
                let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;

                let bid_size = rng.gen_range(4, 14);
                let ask_size = rng.gen_range(4, 14);

                let bid_builder = txn_factory.payload(place_bid_market_order(package.get_module_id("txn_generator_utils"), bid_size, market_id, publisher.address()));
                let ask_builder = txn_factory.payload(place_ask_market_order(package.get_module_id("txn_generator_utils"), ask_size, market_id, publisher.address()));

                requests.push(account.sign_with_transaction_builder(bid_builder));
                requests.push(account.sign_with_transaction_builder(ask_builder));
                requests
            })
        }
    }
}

pub async fn register_econia_markets(
    init_txn_factory: TransactionFactory,
    packages: &mut Vec<(Package, LocalAccount)>,
    txn_executor: &dyn ReliableTransactionSubmitter,
    num_markets: u64,
) {
    assert!(num_markets > 0, "num_markets must be greater than 0");
    assert!(num_markets <= 104, "num_markets must be less than or equal to 104");        
    let mut requests = vec![];
    for (package, publisher) in packages {
        let builder = init_txn_factory.payload(register_market(package.get_module_id("txn_generator_utils"), num_markets));
        requests.push(publisher.sign_with_transaction_builder(builder));
    }
    txn_executor
        .execute_transactions(&requests)
        .await
        .unwrap();
}

pub struct EconiaRegisterMarketUserTransactionGenerator {
    num_markets: Arc<u64>,
}

impl EconiaRegisterMarketUserTransactionGenerator {
    pub fn new(
        num_markets: u64
    ) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EconiaRegisterMarketUserTransactionGenerator {
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
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let num_markets = self.num_markets.clone();
        Arc::new(move |account, package, publisher, txn_factory, _rng| {
            let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
            let builder = txn_factory.payload(register_market_accounts(package.get_module_id("txn_generator_utils"), market_id, publisher.address()));
            vec![account.sign_with_transaction_builder(builder)]
        })
    }
}



pub struct EconiaDepositCoinsTransactionGenerator {
    num_markets: Arc<u64>,
}

impl EconiaDepositCoinsTransactionGenerator {
    pub fn new(
        num_markets: u64
    ) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EconiaDepositCoinsTransactionGenerator {
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
        _root_account: &dyn RootAccountHandle,
        _txn_factory: &TransactionFactory,
        _txn_executor: &dyn ReliableTransactionSubmitter,
        _rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker> {
        let num_markets = self.num_markets.clone();
        Arc::new(move |account, package, publisher, txn_factory, _rng| {
            let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
            let builder = txn_factory.payload(deposit_coins(package.get_module_id("txn_generator_utils"), market_id, publisher.address()));
            vec![account.sign_multi_agent_with_transaction_builder(vec![publisher], builder)]
        })
    }
}
