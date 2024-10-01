// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator},
    publishing::publish_util::Package,
    ReliableTransactionSubmitter, RootAccountHandle,
};
use aptos_sdk::{
    bcs,
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use async_trait::async_trait;
use move_core_types::{
    ident_str,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use rand::{rngs::StdRng, Rng};
use std::sync::Arc;
const BASE_COIN_TYPES: [&str; 104] = [
    "AAC", "ABC", "ACC", "ADC", "AEC", "AFC", "AGC", "AHC", "AIC", "AJC", "AKC", "ALC", "AMC",
    "ANC", "AOC", "APC", "AQC", "ARC", "ASC", "ATC", "AUC", "AVC", "AWC", "AXC", "AYC", "AZC",
    "BAC", "BBC", "BCC", "BDC", "BEC", "BFC", "BGC", "BHC", "BIC", "BJC", "BKC", "BLC", "BMC",
    "BNC", "BOC", "BPC", "BQC", "BRC", "BSC", "BTC", "BUC", "BVC", "BWC", "BXC", "BYC", "BZC",
    "CAC", "CBC", "CCC", "CDC", "CEC", "CFC", "CGC", "CHC", "CIC", "CJC", "CKC", "CLC", "CMC",
    "CNC", "COC", "CPC", "CQC", "CRC", "CSC", "CTC", "CUC", "CVC", "CWC", "CXC", "CYC", "CZC",
    "DAC", "DBC", "DCC", "DDC", "DEC", "DFC", "DGC", "DHC", "DIC", "DJC", "DKC", "DLC", "DMC",
    "DNC", "DOC", "DPC", "DQC", "DRC", "DSC", "DTC", "DUC", "DVC", "DWC", "DXC", "DYC", "DZC",
];
// const QUOTE_COIN_TYPES: [&str; 11] = ["QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC", "QC"];

fn base_coin_type(market_id: u64) -> &'static str {
    BASE_COIN_TYPES[(market_id - 1) as usize]
}

fn quote_coin_type(_market_id: u64) -> &'static str {
    "QC"
}

const ASK: bool = true;
const BID: bool = false;

/// Placeas a bid limit order.
pub fn place_bid_limit_order(
    module_id: ModuleId,
    size: u64,
    price: u64,
    market_id: u64,
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_limit_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
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
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_limit_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
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
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_bid_market_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
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
    publisher: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_ask_market_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
        vec![
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&market_id).unwrap(),
        ],
    ))
}

/// Placeas a market order.
pub fn place_market_order(
    module_id: ModuleId,
    publisher: AccountAddress,
    size: u64,
    market_id: u64,
    direction: bool,
    self_matching_behavior: u8,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_market_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
        vec![
            bcs::to_bytes(&market_id).unwrap(),
            bcs::to_bytes(&direction).unwrap(),
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&self_matching_behavior).unwrap(),
        ],
    ))
}

pub fn place_limit_order(
    module_id: ModuleId,
    publisher: AccountAddress,
    size: u64,
    price: u64,
    market_id: u64,
    direction: bool,
    restriction: u8,
    self_matching_behavior: u8,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_limit_order").to_owned(),
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
        vec![
            bcs::to_bytes(&market_id).unwrap(),
            bcs::to_bytes(&direction).unwrap(),
            bcs::to_bytes(&size).unwrap(),
            bcs::to_bytes(&price).unwrap(),
            bcs::to_bytes(&restriction).unwrap(),
            bcs::to_bytes(&self_matching_behavior).unwrap(),
        ],
    ))
}

pub fn place_cancel_order(module_id: ModuleId) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("place_cancel_order").to_owned(),
        vec![],
        vec![],
    ))
}

pub fn register_market(module_id: ModuleId, num_markets: u64) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        module_id,
        ident_str!("register_multiple_markets").to_owned(),
        vec![],
        vec![bcs::to_bytes(&num_markets).unwrap()],
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
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
        vec![bcs::to_bytes(&market_id).unwrap()],
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
        vec![
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(base_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
            TypeTag::Struct(Box::new(StructTag {
                address: publisher,
                module: Identifier::new(<&str as Into<Box<str>>>::into("assets")).unwrap(),
                name: Identifier::new(<&str as Into<Box<str>>>::into(quote_coin_type(market_id)))
                    .unwrap(),
                type_args: vec![],
            })),
        ],
        vec![bcs::to_bytes(&market_id).unwrap()],
    ))
}

pub struct EconiaLimitOrderTransactionGenerator {
    num_markets: Arc<u64>,
    num_prev_transactions: Arc<u64>,
}

impl EconiaLimitOrderTransactionGenerator {
    pub fn new(num_markets: u64, num_prev_transactions: u64) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
            num_prev_transactions: Arc::new(num_prev_transactions),
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
        let num_prev_transactions = self.num_prev_transactions.clone();
        Arc::new(
            move |account,
                  package,
                  publisher,
                  txn_factory,
                  rng,
                  txn_counter,
                  _prev_orders,
                  _market_maker| {
                let mut requests = vec![];
                let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
                let bid_size = rng.gen_range(4, 14);
                let ask_size = rng.gen_range(4, 14);

                if txn_counter <= (*num_prev_transactions) + (*num_markets) * 100
                    || txn_counter % 2 == 0
                {
                    let bid_price = rng.gen_range(1, 200);
                    let ask_price = rng.gen_range(201, 400);

                    let bid_builder = txn_factory.payload(place_bid_limit_order(
                        package.get_module_id("txn_generator_utils"),
                        bid_size,
                        bid_price,
                        market_id,
                        publisher.address(),
                    ));
                    let ask_builder = txn_factory.payload(place_ask_limit_order(
                        package.get_module_id("txn_generator_utils"),
                        ask_size,
                        ask_price,
                        market_id,
                        publisher.address(),
                    ));

                    requests.push(account.sign_with_transaction_builder(bid_builder));
                    requests.push(account.sign_with_transaction_builder(ask_builder));
                } else {
                    let bid_builder = txn_factory.payload(place_bid_market_order(
                        package.get_module_id("txn_generator_utils"),
                        bid_size,
                        market_id,
                        publisher.address(),
                    ));
                    let ask_builder = txn_factory.payload(place_ask_market_order(
                        package.get_module_id("txn_generator_utils"),
                        ask_size,
                        market_id,
                        publisher.address(),
                    ));

                    requests.push(account.sign_with_transaction_builder(bid_builder));
                    requests.push(account.sign_with_transaction_builder(ask_builder));
                }
                requests
            },
        )
    }
}

pub struct EconiaMarketOrderTransactionGenerator {
    num_markets: Arc<u64>,
    num_prev_transactions: Arc<u64>,
}

impl EconiaMarketOrderTransactionGenerator {
    pub fn new(num_markets: u64, num_prev_transactions: u64) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
            num_prev_transactions: Arc::new(num_prev_transactions),
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EconiaMarketOrderTransactionGenerator {
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
        let num_prev_transactions = self.num_prev_transactions.clone();
        Arc::new(
            move |account,
                  package,
                  publisher,
                  txn_factory,
                  rng,
                  txn_counter,
                  _prev_orders,
                  _market_maker| {
                let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
                if txn_counter <= (*num_prev_transactions) + (*num_markets) * 600 {
                    let bid_size = rng.gen_range(400000, 500000);
                    let ask_size = rng.gen_range(400000, 500000);
                    if rng.gen_range(0, 2) == 0 {
                        let bid_price = rng.gen_range(1000, 1500);
                        let bid_builder = txn_factory.payload(place_bid_limit_order(
                            package.get_module_id("txn_generator_utils"),
                            bid_size,
                            bid_price,
                            market_id,
                            publisher.address(),
                        ));
                        vec![account.sign_with_transaction_builder(bid_builder)]
                    } else {
                        let ask_price = rng.gen_range(1501, 2000);
                        let ask_builder = txn_factory.payload(place_ask_limit_order(
                            package.get_module_id("txn_generator_utils"),
                            ask_size,
                            ask_price,
                            market_id,
                            publisher.address(),
                        ));
                        vec![account.sign_with_transaction_builder(ask_builder)]
                    }
                } else {
                    let bid_size = rng.gen_range(4, 10);
                    let ask_size = rng.gen_range(4, 10);
                    if rng.gen_range(0, 2) == 0 {
                        let bid_builder = txn_factory.payload(place_bid_market_order(
                            package.get_module_id("txn_generator_utils"),
                            bid_size,
                            market_id,
                            publisher.address(),
                        ));
                        vec![account.sign_with_transaction_builder(bid_builder)]
                    } else {
                        let ask_builder = txn_factory.payload(place_ask_market_order(
                            package.get_module_id("txn_generator_utils"),
                            ask_size,
                            market_id,
                            publisher.address(),
                        ));
                        vec![account.sign_with_transaction_builder(ask_builder)]
                    }
                }
            },
        )
    }
}

pub struct EconiaRealOrderTransactionGenerator {}

impl EconiaRealOrderTransactionGenerator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for EconiaRealOrderTransactionGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for EconiaRealOrderTransactionGenerator {
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
        Arc::new(
            move |account,
                  package,
                  publisher,
                  txn_factory,
                  rng,
                  _txn_counter,
                  history,
                  market_maker| {
                // println!("EconiaRealOrderTransactionGenerator. account: {}, history: {:?}", account.address(), history);
                let size = rng.gen_range(4, 10000);
                if market_maker || rng.gen_range(0, 1000) < 138 {
                    // Market makers always do only limit and cancel orders
                    // Non-market makers do limit orders with 13.8% probability
                    if market_maker {
                        let num_prev_limit_orders = history
                            .iter()
                            .map(|s| if s == "place_limit_order" { 1 } else { 0 })
                            .sum::<u64>();
                        let num_prev_cancel_orders = history
                            .iter()
                            .map(|s| if s == "place_cancel_order" { 1 } else { 0 })
                            .sum::<u64>();
                        if num_prev_limit_orders > num_prev_cancel_orders
                            && rng.gen_range(1, 101) <= 98
                        {
                            // 98% probability
                            return vec![account.sign_with_transaction_builder(
                                txn_factory.payload(place_cancel_order(
                                    package.get_module_id("txn_generator_utils"),
                                )),
                            )];
                        }
                    }

                    // Limit order
                    let market_id = if rng.gen_range(1, 1000) < 740 {
                        // Market 1 with 74% probability
                        1
                    } else {
                        2
                    };
                    let rand = rng.gen_range(1, 10000);
                    let restriction = if rand < 88 {
                        // 0.88% probability
                        0
                    } else if rand < 176 {
                        // 0.88% probability
                        2
                    } else {
                        // 98.2% probability
                        3
                    };

                    let rand = rng.gen_range(1, 1000);
                    let self_matching_behavior = if rand < 8 {
                        // 0.8% probability
                        3
                    } else if rand < 295 {
                        // 2.87% probability
                        2
                    } else {
                        // 96.33% probability
                        0
                    };
                    let (direction, price) = if rng.gen_range(1, 1000) < 546 {
                        // ASK with 54.6% probability
                        (ASK, rng.gen_range(13200, 13400))
                    } else {
                        (BID, rng.gen_range(13000, 13200))
                    };
                    vec![account.sign_with_transaction_builder(txn_factory.payload(
                        place_limit_order(
                            package.get_module_id("txn_generator_utils"),
                            publisher.address(),
                            size,
                            price,
                            market_id,
                            direction,
                            restriction,
                            self_matching_behavior,
                        ),
                    ))]
                } else {
                    let market_id = if rng.gen_range(1, 1000) < 885 {
                        // Market 1 with 88.5% probability
                        1
                    } else {
                        2
                    };
                    let direction = if rng.gen_range(1, 1000) < 515 {
                        // ASK with 51.5% probability
                        ASK
                    } else {
                        BID
                    };
                    let self_matching_behavior = if rng.gen_range(1, 1000) < 184 {
                        // 18.4% probability
                        0
                    } else {
                        // 81.6% probability
                        3
                    };
                    vec![account.sign_with_transaction_builder(txn_factory.payload(
                        place_market_order(
                            package.get_module_id("txn_generator_utils"),
                            publisher.address(),
                            size,
                            market_id,
                            direction,
                            self_matching_behavior,
                        ),
                    ))]
                }
            },
        )
    }
}

pub async fn register_econia_markets(
    init_txn_factory: TransactionFactory,
    packages: &mut Vec<(Package, LocalAccount)>,
    txn_executor: &dyn ReliableTransactionSubmitter,
    num_markets: u64,
) {
    assert!(num_markets > 0, "num_markets must be greater than 0");
    assert!(
        num_markets <= 104,
        "num_markets must be less than or equal to 104"
    );
    let mut requests = vec![];
    for (package, publisher) in packages {
        let builder = init_txn_factory.payload(register_market(
            package.get_module_id("txn_generator_utils"),
            num_markets,
        ));
        requests.push(publisher.sign_with_transaction_builder(builder));
    }
    txn_executor.execute_transactions(&requests).await.unwrap();
}

pub struct EconiaRegisterMarketUserTransactionGenerator {
    num_markets: Arc<u64>,
    bucket_users_into_markets: bool,
}

impl EconiaRegisterMarketUserTransactionGenerator {
    pub fn new(num_markets: u64, bucket_users_into_markets: bool) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
            bucket_users_into_markets,
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
        if self.bucket_users_into_markets {
            Arc::new(
                move |account,
                      package,
                      publisher,
                      txn_factory,
                      _rng,
                      _txn_counter,
                      _prev_orders,
                      _market_maker| {
                    let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
                    let builder = txn_factory.payload(register_market_accounts(
                        package.get_module_id("txn_generator_utils"),
                        market_id,
                        publisher.address(),
                    ));
                    vec![account.sign_with_transaction_builder(builder)]
                },
            )
        } else {
            Arc::new(
                move |account,
                      package,
                      publisher,
                      txn_factory,
                      _rng,
                      _txn_counter,
                      _prev_orders,
                      _market_maker| {
                    let mut requests = Vec::new();
                    for market_id in 1..(*num_markets + 1) {
                        let builder = txn_factory.payload(register_market_accounts(
                            package.get_module_id("txn_generator_utils"),
                            market_id,
                            publisher.address(),
                        ));
                        requests.push(account.sign_with_transaction_builder(builder))
                    }
                    requests
                },
            )
        }
    }
}

pub struct EconiaDepositCoinsTransactionGenerator {
    num_markets: Arc<u64>,
    bucket_users_into_markets: bool,
}

impl EconiaDepositCoinsTransactionGenerator {
    pub fn new(num_markets: u64, bucket_users_into_markets: bool) -> Self {
        Self {
            num_markets: Arc::new(num_markets),
            bucket_users_into_markets,
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
        if self.bucket_users_into_markets {
            Arc::new(
                move |account,
                      package,
                      publisher,
                      txn_factory,
                      _rng,
                      _txn_counter,
                      _prev_orders,
                      _market_maker| {
                    let market_id = account.address().into_bytes()[0] as u64 % *num_markets + 1;
                    let builder = txn_factory.payload(deposit_coins(
                        package.get_module_id("txn_generator_utils"),
                        market_id,
                        publisher.address(),
                    ));
                    vec![account.sign_multi_agent_with_transaction_builder(vec![publisher], builder)]
                },
            )
        } else {
            Arc::new(
                move |account,
                      package,
                      publisher,
                      txn_factory,
                      _rng,
                      _txn_counter,
                      _prev_orders,
                      _market_maker| {
                    let mut requests = Vec::new();
                    for market_id in 1..(*num_markets + 1) {
                        let builder = txn_factory.payload(deposit_coins(
                            package.get_module_id("txn_generator_utils"),
                            market_id,
                            publisher.address(),
                        ));
                        requests.push(
                            account.sign_multi_agent_with_transaction_builder(
                                vec![publisher],
                                builder,
                            ),
                        )
                    }
                    requests
                },
            )
        }
    }
}
