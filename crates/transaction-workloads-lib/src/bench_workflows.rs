// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Workflows for the DeFi benchmark packages under
//! `third_party/move/mono-move/benchmarks`.
//!
//! Each of these packages needs more setup than an [`crate::EntryPoints`]
//! variant can express: admin-signed protocol configuration, then one
//! onboarding transaction per account, then a steady-state mix. That maps onto
//! a three-stage [`WorkflowKind`] — account creation, onboarding, then the mix
//! looping on the same pool.
//!
//! The mix samples an entry function per transaction from a seeded RNG, so a
//! single registration produces a varied but reproducible transaction stream.

use crate::prebuilt_packages::PreBuiltPackagesImpl;
use aptos_sdk::{
    bcs,
    move_types::{
        account_address::AccountAddress, ident_str, int256::U256, language_storage::ModuleId,
    },
    transaction_builder::TransactionFactory,
    types::{
        account_address::create_object_address,
        transaction::{SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use aptos_transaction_generator_lib::{
    call_custom_modules::{
        CustomModulesDelegationGeneratorCreator, PlainUserModuleTransactionGenerator,
        TransactionGeneratorWorker, UserModuleTransactionGenerator,
    },
    entry_point_trait::get_payload,
    publishing::publish_util::Package,
    workflow_delegator::{StageTracking, WorkflowKind, WorkflowTxnGeneratorCreator},
    ReliableTransactionSubmitter, RootAccountHandle,
};
use async_trait::async_trait;
use rand::{distributions::WeightedIndex, prelude::Distribution, rngs::StdRng, Rng};
use std::sync::Arc;

/// Enough to publish the package and run the whole admin-signed setup, which
/// seeds a book or a pool and so costs far more than a steady-state
/// transaction.
const PUBLISHER_BALANCE: u64 = 1000_0000_0000;

/// Gas ceiling on every account-signed transaction. The prologue requires the
/// sender to cover `max_gas_amount * gas_unit_price` up front, so the default
/// ceiling would put the floor on [`ACCOUNT_CREATION_BALANCE`] at 20 APT.
/// Funding thousands of accounts at that rate drains the accounts the harness
/// draws from, which start with 100 APT each.
const MAX_GAS_UNITS: u64 = 2_000_000;

/// Balance each benchmark account is created with: the prologue's 2 APT floor
/// plus room for the gas a long run actually burns.
const ACCOUNT_CREATION_BALANCE: u64 = 5_0000_0000;

#[derive(Debug, Copy, Clone)]
pub enum BenchWorkflowKind {
    /// Lending market. Reserves are configured by the publisher, each account
    /// then supplies collateral and takes one variable-rate borrow, and the
    /// mix samples supply / withdraw / borrow / repay / collateral toggle /
    /// flash loan.
    Aave {
        num_accounts: usize,
        num_reserves: usize,
        collaterals_per_account: usize,
        num_txns: usize,
    },
    /// Order book over a bit-packed AVL queue. The publisher registers one
    /// market and seeds it, each account opens a funded market account, and
    /// the mix samples resting placements / matching / cancels / traversal.
    Clob {
        num_accounts: usize,
        seed_orders_per_side: usize,
        index_limit: usize,
        num_txns: usize,
    },
    /// Concentrated liquidity pool. The publisher creates the pair and seeds
    /// positions, each account opens one of its own, and the mix samples
    /// swaps / rebalances / fee settlement.
    Clmm {
        num_accounts: usize,
        seed_positions: usize,
        swap_size: u64,
        num_txns: usize,
    },
}

#[async_trait]
impl WorkflowKind for BenchWorkflowKind {
    async fn construct_workflow(
        &self,
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &dyn RootAccountHandle,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        stage_tracking: StageTracking,
    ) -> WorkflowTxnGeneratorCreator {
        let (package_name, num_accounts, num_txns, workers) = match self {
            BenchWorkflowKind::Aave {
                num_accounts,
                num_reserves,
                collaterals_per_account,
                num_txns,
            } => {
                let config = aave::Config::new(*num_reserves, *collaterals_per_account);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(aave::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(aave::mix_worker(
                        config,
                    ))),
                ];
                (aave::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
            BenchWorkflowKind::Clob {
                num_accounts,
                seed_orders_per_side,
                index_limit,
                num_txns,
            } => {
                let config = clob::Config::new(*seed_orders_per_side, *index_limit);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(clob::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(clob::mix_worker(
                        config,
                    ))),
                ];
                (clob::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
            BenchWorkflowKind::Clmm {
                num_accounts,
                seed_positions,
                swap_size,
                num_txns,
            } => {
                let config = clmm::Config::new(*seed_positions, *swap_size);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(clmm::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(clmm::mix_worker(
                        config,
                    ))),
                ];
                (clmm::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
        };

        let packages = Arc::new(
            CustomModulesDelegationGeneratorCreator::publish_package(
                init_txn_factory.clone(),
                root_account,
                txn_executor,
                num_modules,
                &PreBuiltPackagesImpl,
                package_name,
                Some(PUBLISHER_BALANCE),
            )
            .await,
        );

        WorkflowTxnGeneratorCreator::new_staged_with_account_pool(
            num_accounts,
            ACCOUNT_CREATION_BALANCE,
            workers,
            Some(num_txns),
            packages,
            txn_factory,
            init_txn_factory,
            root_account,
            txn_executor,
            stage_tracking,
        )
        .await
    }
}

/// Spreads accounts over the reserve space without any shared state: both the
/// onboarding stage and the mix recompute the same slot from the address, so
/// they agree on which reserves an account holds.
fn slot_for(account: &LocalAccount, modulus: usize) -> usize {
    let bytes = account.address().into_bytes();
    let tail = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
    (tail % modulus as u64) as usize
}

fn sign_capped(
    account: &LocalAccount,
    txn_factory: &TransactionFactory,
    payload: TransactionPayload,
) -> SignedTransaction {
    account.sign_with_transaction_builder(
        txn_factory
            .clone()
            .with_max_gas_amount(MAX_GAS_UNITS)
            .payload(payload),
    )
}

fn u256_arg(value: u64) -> Vec<u8> {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&value.to_le_bytes());
    bcs::to_bytes(&U256::from_le_bytes(bytes)).unwrap()
}

mod aave {
    use super::*;

    pub const PACKAGE_NAME: &str = "bench_aave";

    /// Aave's variable interest rate mode. The package has no stable mode.
    const VARIABLE: u8 = 2;

    /// Reserve symbols, and with them the cap on how many reserves a run can
    /// configure. All are six-decimal.
    const SYMBOLS: [&[u8]; 8] = [
        b"BA0", b"BA1", b"BA2", b"BA3", b"BA4", b"BA5", b"BA6", b"BA7",
    ];
    const DECIMALS: u8 = 6;

    /// Per-reserve `(ltv, liquidation_threshold, liquidation_bonus, price)` in
    /// basis points except the price. Deliberately all different, so the
    /// health-factor loop reads a distinct configuration per collateral rather
    /// than the same one repeatedly.
    const PARAMS: [(u64, u64, u64, u64); 8] = [
        (8000, 8500, 10500, 100_000_000),
        (7500, 8000, 11000, 200_000_000),
        (7000, 7500, 11000, 50_000_000),
        (8250, 8600, 10400, 150_000_000),
        (6500, 7000, 11500, 300_000_000),
        (7700, 8200, 10800, 80_000_000),
        (8100, 8400, 10600, 120_000_000),
        (6000, 6500, 12000, 40_000_000),
    ];

    /// Rate strategy, shared by every reserve: 80% optimal usage, no base rate,
    /// 4% slope below the kink and 75% above it.
    const OPTIMAL_USAGE_BPS: u64 = 8000;
    const BASE_BORROW_RATE_BPS: u64 = 0;
    const SLOPE1_BPS: u64 = 400;
    const SLOPE2_BPS: u64 = 7500;
    const RESERVE_FACTOR_BPS: u64 = 1000;
    const PROTOCOL_FEE_BPS: u64 = 0;

    /// Liquidity the publisher puts into every reserve, so that borrows do not
    /// depend on which accounts have been onboarded yet.
    const BOOTSTRAP_SUPPLY: u64 = 1_000_000_000_000;

    /// Per-collateral supply and the single borrow each account starts with.
    const ONBOARD_SUPPLY: u64 = 1_000_000_000;
    const ONBOARD_BORROW: u64 = 100_000_000;

    /// Amount each steady-state transaction moves. Small relative to the
    /// onboarded position, so no account drifts into an unhealthy state or
    /// runs out of balance however long the mix runs.
    const STEP: u64 = 10_000;

    /// Flash loan size and its premium, in basis points.
    const FLASH_AMOUNT: u64 = 1_000_000;
    const FLASH_PREMIUM_BPS: u64 = 9;

    /// Iterations of the in-package receiver the flash loan runs before
    /// repaying.
    const FLASH_RECEIVER_OPS: u64 = 8;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub num_reserves: usize,
        pub collaterals_per_account: usize,
    }

    impl Config {
        pub fn new(num_reserves: usize, collaterals_per_account: usize) -> Self {
            assert!(
                num_reserves <= SYMBOLS.len(),
                "bench_aave supports at most {} reserves",
                SYMBOLS.len()
            );
            assert!(
                collaterals_per_account > 0 && collaterals_per_account < num_reserves,
                "each account needs at least one collateral and one reserve left to borrow from"
            );
            Self {
                num_reserves,
                collaterals_per_account,
            }
        }

        /// Address of the `i`th reserve's underlying asset. The package derives
        /// it as a named object of the publisher seeded with the symbol.
        fn asset(&self, publisher: AccountAddress, i: usize) -> AccountAddress {
            create_object_address(publisher, SYMBOLS[i])
        }

        /// The reserves an account holds: `collaterals_per_account` collaterals
        /// starting at its slot, then one debt reserve after those.
        fn reserves_of(
            &self,
            account: &LocalAccount,
            publisher: AccountAddress,
        ) -> (Vec<AccountAddress>, AccountAddress) {
            let start = slot_for(account, self.num_reserves);
            let collaterals = (0..self.collaterals_per_account)
                .map(|k| self.asset(publisher, (start + k) % self.num_reserves))
                .collect::<Vec<AccountAddress>>();
            let debt = self.asset(
                publisher,
                (start + self.collaterals_per_account) % self.num_reserves,
            );
            (collaterals, debt)
        }
    }

    fn mock_fa(package: &Package) -> ModuleId {
        package.get_module_id("aave_mock_fa")
    }

    fn pool(package: &Package) -> ModuleId {
        package.get_module_id("aave_pool")
    }

    fn logic(package: &Package) -> ModuleId {
        package.get_module_id("aave_logic")
    }

    /// Stage 1: configure the protocol from the publisher, then hand each
    /// account a single transaction that leaves it supplying and borrowing.
    pub struct Onboard(pub Config);

    #[async_trait]
    impl UserModuleTransactionGenerator for Onboard {
        fn initialize_package(
            &mut self,
            package: &Package,
            publisher: &LocalAccount,
            txn_factory: &TransactionFactory,
            _rng: &mut StdRng,
        ) -> Vec<SignedTransaction> {
            let config = self.0;
            let mut payloads = vec![get_payload(
                pool(package),
                ident_str!("initialize").to_owned(),
                vec![],
            )];

            for i in 0..config.num_reserves {
                payloads.push(get_payload(
                    mock_fa(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(SYMBOLS[i]).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ));
            }

            for i in 0..config.num_reserves {
                let asset = config.asset(publisher.address(), i);
                let (ltv, threshold, bonus, price) = PARAMS[i];
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("admin_add_reserve").to_owned(),
                    vec![
                        bcs::to_bytes(&asset).unwrap(),
                        u256_arg(ltv),
                        u256_arg(threshold),
                        u256_arg(bonus),
                        u256_arg(RESERVE_FACTOR_BPS),
                        u256_arg(PROTOCOL_FEE_BPS),
                        u256_arg(OPTIMAL_USAGE_BPS),
                        u256_arg(BASE_BORROW_RATE_BPS),
                        u256_arg(SLOPE1_BPS),
                        u256_arg(SLOPE2_BPS),
                    ],
                ));
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("oracle_set_price").to_owned(),
                    vec![bcs::to_bytes(&asset).unwrap(), u256_arg(price)],
                ));
                payloads.push(get_payload(
                    logic(package),
                    ident_str!("bench_supply").to_owned(),
                    vec![bcs::to_bytes(&asset).unwrap(), u256_arg(BOOTSTRAP_SUPPLY)],
                ));
            }

            payloads
                .into_iter()
                .map(|payload| {
                    publisher.sign_with_transaction_builder(txn_factory.payload(payload))
                })
                .collect::<Vec<SignedTransaction>>()
        }

        async fn create_generator_fn(
            &self,
            _root_account: &dyn RootAccountHandle,
            _txn_factory: &TransactionFactory,
            _txn_executor: &dyn ReliableTransactionSubmitter,
            _rng: &mut StdRng,
        ) -> Arc<TransactionGeneratorWorker> {
            let config = self.0;
            Arc::new(move |account, package, _publisher, txn_factory, _rng| {
                let start = slot_for(account, config.num_reserves);
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        logic(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&(start as u64)).unwrap(),
                            bcs::to_bytes(&(config.collaterals_per_account as u64)).unwrap(),
                            u256_arg(ONBOARD_SUPPLY),
                            u256_arg(ONBOARD_BORROW),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Supply,
        Withdraw,
        Borrow,
        Repay,
        SetCollateral,
        FlashLoan,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Supply
    /// outweighs withdraw and repay outweighs nothing, so positions grow
    /// slowly rather than draining.
    const MIX: [(MixKind, u32); 6] = [
        (MixKind::Supply, 25),
        (MixKind::Withdraw, 20),
        (MixKind::Borrow, 15),
        (MixKind::Repay, 15),
        (MixKind::SetCollateral, 5),
        (MixKind::FlashLoan, 20),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let (collaterals, debt) = config.reserves_of(account, publisher.address());
            let collateral =
                bcs::to_bytes(&collaterals[rng.gen_range(0, collaterals.len())]).unwrap();
            let debt = bcs::to_bytes(&debt).unwrap();

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Supply => (ident_str!("bench_supply"), vec![collateral, u256_arg(STEP)]),
                MixKind::Withdraw => (ident_str!("withdraw"), vec![
                    collateral,
                    u256_arg(STEP),
                    bcs::to_bytes(&account.address()).unwrap(),
                ]),
                MixKind::Borrow => (ident_str!("borrow"), vec![
                    debt,
                    u256_arg(STEP),
                    bcs::to_bytes(&VARIABLE).unwrap(),
                ]),
                MixKind::Repay => (ident_str!("repay"), vec![
                    debt,
                    u256_arg(STEP),
                    bcs::to_bytes(&VARIABLE).unwrap(),
                ]),
                MixKind::SetCollateral => (ident_str!("set_user_use_reserve_as_collateral"), vec![
                    collateral,
                    bcs::to_bytes(&true).unwrap(),
                ]),
                MixKind::FlashLoan => (ident_str!("flash_loan_simple"), vec![
                    collateral,
                    u256_arg(FLASH_AMOUNT),
                    u256_arg(FLASH_PREMIUM_BPS),
                    bcs::to_bytes(&FLASH_RECEIVER_OPS).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(logic(package), func.to_owned(), args),
            ))
        })
    }
}

mod clob {
    use super::*;

    pub const PACKAGE_NAME: &str = "bench_clob";

    const BID: bool = true;

    const BASE_SYMBOL: &[u8] = b"CBASE";
    const QUOTE_SYMBOL: &[u8] = b"CQUOTE";
    const BASE_DECIMALS: u8 = 8;
    const QUOTE_DECIMALS: u8 = 6;

    /// One lot of base and one tick of quote, so an order's collateral is
    /// exactly its size and its size times its price.
    const LOT_SIZE: u64 = 1;
    const TICK_SIZE: u64 = 1;

    /// The market registry numbers markets from one, and the publisher only
    /// ever registers this one.
    const MARKET_ID: u64 = 1;

    /// Mid price the book is seeded around, and the gap the seeded sides leave
    /// on either side of it.
    const BASE_PRICE: u64 = 1_000_000;
    const SPREAD: u64 = 100;

    /// How far a placement may move the price it picks off the mid. Wide
    /// enough that the tree keeps growing new levels rather than piling onto
    /// the same handful.
    const PRICE_JITTER: u64 = 20_000;

    /// Orders a single steady-state placement adds per side, and the largest
    /// one it may be.
    const REPLENISH_PER_SIDE: u64 = 2;

    /// Market orders are drawn from a wider size range than resting orders, so
    /// the book has more removal capacity than it has arrivals. Depth then
    /// settles where market orders start running out of book, instead of
    /// growing until the AVL queue runs out of nodes.
    const MARKET_ORDER_MAX_SIZE: u64 = 32;

    /// Deposited at onboarding and topped back up during the mix.
    const ONBOARD_BASE: u64 = 10_000_000_000;
    const ONBOARD_QUOTE: u64 = 10_000_000_000_000;
    const TOPUP_BASE: u64 = 100_000_000;
    const TOPUP_QUOTE: u64 = 100_000_000_000;

    /// The publisher owns the seeded book, so it needs to cover all of it.
    const PUBLISHER_BASE: u64 = 1_000_000_000_000;
    const PUBLISHER_QUOTE: u64 = 1_000_000_000_000_000_000;

    /// Orders one `seed_book` transaction places per side. A whole book at once
    /// runs past the per-transaction execution limit.
    const SEED_BATCH_PER_SIDE: u64 = 32;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub seed_orders_per_side: u64,
        pub index_limit: u64,
    }

    impl Config {
        pub fn new(seed_orders_per_side: usize, index_limit: usize) -> Self {
            let seed_orders_per_side = seed_orders_per_side as u64;
            // `seed_book` jitters each order over four ticks and refuses to
            // price one below zero.
            assert!(
                BASE_PRICE > SPREAD + seed_orders_per_side * 4 + 1,
                "seeded bids would run below zero"
            );
            Self {
                seed_orders_per_side,
                index_limit: index_limit as u64,
            }
        }
    }

    fn mock_fa(package: &Package) -> ModuleId {
        package.get_module_id("clob_mock_fa")
    }

    fn market(package: &Package) -> ModuleId {
        package.get_module_id("clob_market")
    }

    /// Stage 1: register and seed the market from the publisher, then give
    /// each account one transaction that opens a funded market account.
    pub struct Onboard(pub Config);

    #[async_trait]
    impl UserModuleTransactionGenerator for Onboard {
        fn initialize_package(
            &mut self,
            package: &Package,
            publisher: &LocalAccount,
            txn_factory: &TransactionFactory,
            _rng: &mut StdRng,
        ) -> Vec<SignedTransaction> {
            let config = self.0;
            let base = create_object_address(publisher.address(), BASE_SYMBOL);
            let quote = create_object_address(publisher.address(), QUOTE_SYMBOL);
            let mut payloads = vec![
                get_payload(
                    mock_fa(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(BASE_SYMBOL).unwrap(),
                        bcs::to_bytes(&BASE_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    mock_fa(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(QUOTE_SYMBOL).unwrap(),
                        bcs::to_bytes(&QUOTE_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    market(package),
                    ident_str!("register_market").to_owned(),
                    vec![
                        bcs::to_bytes(&base).unwrap(),
                        bcs::to_bytes(&quote).unwrap(),
                        bcs::to_bytes(&LOT_SIZE).unwrap(),
                        bcs::to_bytes(&TICK_SIZE).unwrap(),
                    ],
                ),
                get_payload(
                    market(package),
                    ident_str!("bench_onboard").to_owned(),
                    vec![
                        bcs::to_bytes(&MARKET_ID).unwrap(),
                        bcs::to_bytes(&PUBLISHER_BASE).unwrap(),
                        bcs::to_bytes(&PUBLISHER_QUOTE).unwrap(),
                    ],
                ),
            ];

            let mut seeded = 0;
            while seeded < config.seed_orders_per_side {
                let batch = SEED_BATCH_PER_SIDE.min(config.seed_orders_per_side - seeded);
                payloads.push(get_payload(
                    market(package),
                    ident_str!("seed_book").to_owned(),
                    vec![
                        bcs::to_bytes(&MARKET_ID).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&BASE_PRICE).unwrap(),
                        bcs::to_bytes(&SPREAD).unwrap(),
                        bcs::to_bytes(&(42 + seeded)).unwrap(),
                    ],
                ));
                seeded += batch;
            }

            payloads
                .into_iter()
                .map(|payload| {
                    publisher.sign_with_transaction_builder(txn_factory.payload(payload))
                })
                .collect::<Vec<SignedTransaction>>()
        }

        async fn create_generator_fn(
            &self,
            _root_account: &dyn RootAccountHandle,
            _txn_factory: &TransactionFactory,
            _txn_executor: &dyn ReliableTransactionSubmitter,
            _rng: &mut StdRng,
        ) -> Arc<TransactionGeneratorWorker> {
            Arc::new(move |account, package, _publisher, txn_factory, _rng| {
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        market(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&MARKET_ID).unwrap(),
                            bcs::to_bytes(&ONBOARD_BASE).unwrap(),
                            bcs::to_bytes(&ONBOARD_QUOTE).unwrap(),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Replenish,
        PlaceAndCancel,
        MarketOrder,
        Index,
        Topup,
    }

    /// Stage 2: the steady-state mix, with relative frequencies.
    const MIX: [(MixKind, u32); 5] = [
        (MixKind::Replenish, 20),
        (MixKind::PlaceAndCancel, 25),
        (MixKind::MarketOrder, 30),
        (MixKind::Index, 15),
        (MixKind::Topup, 10),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let side = rng.gen_bool(0.5);
            let jitter = rng.gen_range(0, PRICE_JITTER);
            // Bids sit below the mid and asks above it, so a placement rests
            // rather than crossing whatever the book currently looks like.
            let price = if side == BID {
                BASE_PRICE - SPREAD - jitter
            } else {
                BASE_PRICE + SPREAD + jitter
            };

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Replenish => (ident_str!("seed_book"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&REPLENISH_PER_SIDE).unwrap(),
                    bcs::to_bytes(&REPLENISH_PER_SIDE).unwrap(),
                    bcs::to_bytes(&price).unwrap(),
                    bcs::to_bytes(&SPREAD).unwrap(),
                    bcs::to_bytes(&rng.gen_range(0u64, u64::MAX)).unwrap(),
                ]),
                MixKind::PlaceAndCancel => (ident_str!("bench_place_and_cancel"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&side).unwrap(),
                    bcs::to_bytes(&price).unwrap(),
                    bcs::to_bytes(&rng.gen_range(1u64, 9)).unwrap(),
                ]),
                MixKind::MarketOrder => (ident_str!("place_market_order"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&side).unwrap(),
                    bcs::to_bytes(&rng.gen_range(1u64, MARKET_ORDER_MAX_SIZE + 1)).unwrap(),
                ]),
                MixKind::Index => (ident_str!("bench_index"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&side).unwrap(),
                    bcs::to_bytes(&config.index_limit).unwrap(),
                ]),
                MixKind::Topup => (ident_str!("bench_topup"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&TOPUP_BASE).unwrap(),
                    bcs::to_bytes(&TOPUP_QUOTE).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(market(package), func.to_owned(), args),
            ))
        })
    }
}

mod clmm {
    use super::*;

    pub const PACKAGE_NAME: &str = "bench_clmm";

    const TOKEN_A_SYMBOL: &[u8] = b"CMA";
    const TOKEN_B_SYMBOL: &[u8] = b"CMB";
    const DECIMALS: u8 = 8;

    /// 0.3% fee against the package's million-unit denominator, and Uniswap
    /// V3's 0.3% tier spacing.
    const FEE_RATE: u64 = 3000;
    const TICK_SPACING: u32 = 60;

    /// Q64.96 one, which is the square root price at tick zero.
    const Q96: u128 = 1 << 96;

    /// The publisher's backstop position. It spans far more than the mix can
    /// move the price, so no swap ever finds an empty book.
    const BACKSTOP_TICK: i32 = 60_000;
    const BACKSTOP_LIQUIDITY: u128 = 1_000_000_000_000_000;

    /// Width of each position `seed_positions` opens, in ticks.
    const SEED_WIDTH_TICKS: u32 = 600;

    /// Half-width of the range an account is onboarded with, in tick spacings.
    /// Ranges are jittered across accounts so the tick table stays sparse.
    const RANGE_SPACINGS: i32 = 20;
    const RANGE_SLOTS: usize = 32;

    const ONBOARD_LIQUIDITY: u128 = 100_000_000_000;
    const REBALANCE_LIQUIDITY: u128 = 10_000_000_000;

    /// Funded per transaction that pays into the pool, so a long run never
    /// exhausts an account.
    const FUND_AMOUNT: u64 = 1_000_000_000_000;
    const PUBLISHER_FUNDING: u64 = 1_000_000_000_000_000_000;

    /// Fees are collected in full every time, so the cap only has to exceed
    /// what one position can accrue between collections.
    const COLLECT_MAX: u64 = u64::MAX;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub seed_positions: u64,
        pub swap_size: u64,
    }

    impl Config {
        pub fn new(seed_positions: usize, swap_size: u64) -> Self {
            assert!(swap_size > 0, "a zero-sized swap aborts");
            Self {
                seed_positions: seed_positions as u64,
                swap_size,
            }
        }

        /// The tick range an account holds. Both the onboarding stage and the
        /// mix recompute it from the address, so they agree without any
        /// shared state.
        fn range_of(&self, account: &LocalAccount) -> (i32, i32) {
            let slot = slot_for(account, RANGE_SLOTS) as i32;
            let center = (slot - RANGE_SLOTS as i32 / 2) * TICK_SPACING as i32;
            let half = RANGE_SPACINGS * TICK_SPACING as i32;
            (center - half, center + half)
        }
    }

    fn mock_fa(package: &Package) -> ModuleId {
        package.get_module_id("clmm_mock_fa")
    }

    fn pool(package: &Package) -> ModuleId {
        package.get_module_id("clmm_pool")
    }

    /// Address of the pool the publisher creates. The package derives it as a
    /// named object seeded with the pair and the configuration.
    fn pool_address(publisher: AccountAddress) -> AccountAddress {
        let token_a = create_object_address(publisher, TOKEN_A_SYMBOL);
        let token_b = create_object_address(publisher, TOKEN_B_SYMBOL);
        let mut seed = b"bench_clmm_pool".to_vec();
        seed.extend_from_slice(&bcs::to_bytes(&token_a).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&token_b).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&FEE_RATE).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&TICK_SPACING).unwrap());
        create_object_address(publisher, &seed)
    }

    /// Stage 1: create and seed the pool from the publisher, then give each
    /// account one transaction that opens a funded position.
    pub struct Onboard(pub Config);

    #[async_trait]
    impl UserModuleTransactionGenerator for Onboard {
        fn initialize_package(
            &mut self,
            package: &Package,
            publisher: &LocalAccount,
            txn_factory: &TransactionFactory,
            _rng: &mut StdRng,
        ) -> Vec<SignedTransaction> {
            let config = self.0;
            let token_a = create_object_address(publisher.address(), TOKEN_A_SYMBOL);
            let token_b = create_object_address(publisher.address(), TOKEN_B_SYMBOL);
            let pool_id = pool_address(publisher.address());
            let payloads = vec![
                get_payload(
                    mock_fa(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(TOKEN_A_SYMBOL).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    mock_fa(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(TOKEN_B_SYMBOL).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ),
                get_payload(mock_fa(package), ident_str!("mint").to_owned(), vec![
                    bcs::to_bytes(&token_a).unwrap(),
                    bcs::to_bytes(&publisher.address()).unwrap(),
                    bcs::to_bytes(&PUBLISHER_FUNDING).unwrap(),
                ]),
                get_payload(mock_fa(package), ident_str!("mint").to_owned(), vec![
                    bcs::to_bytes(&token_b).unwrap(),
                    bcs::to_bytes(&publisher.address()).unwrap(),
                    bcs::to_bytes(&PUBLISHER_FUNDING).unwrap(),
                ]),
                get_payload(pool(package), ident_str!("create_pool").to_owned(), vec![
                    bcs::to_bytes(&token_a).unwrap(),
                    bcs::to_bytes(&token_b).unwrap(),
                    bcs::to_bytes(&FEE_RATE).unwrap(),
                    bcs::to_bytes(&TICK_SPACING).unwrap(),
                    bcs::to_bytes(&Q96).unwrap(),
                ]),
                get_payload(pool(package), ident_str!("mint").to_owned(), vec![
                    bcs::to_bytes(&pool_id).unwrap(),
                    bcs::to_bytes(&-BACKSTOP_TICK).unwrap(),
                    bcs::to_bytes(&BACKSTOP_TICK).unwrap(),
                    bcs::to_bytes(&BACKSTOP_LIQUIDITY).unwrap(),
                ]),
                get_payload(
                    pool(package),
                    ident_str!("seed_positions").to_owned(),
                    vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&config.seed_positions).unwrap(),
                        bcs::to_bytes(&SEED_WIDTH_TICKS).unwrap(),
                        bcs::to_bytes(&42u64).unwrap(),
                    ],
                ),
            ];

            payloads
                .into_iter()
                .map(|payload| {
                    publisher.sign_with_transaction_builder(txn_factory.payload(payload))
                })
                .collect::<Vec<SignedTransaction>>()
        }

        async fn create_generator_fn(
            &self,
            _root_account: &dyn RootAccountHandle,
            _txn_factory: &TransactionFactory,
            _txn_executor: &dyn ReliableTransactionSubmitter,
            _rng: &mut StdRng,
        ) -> Arc<TransactionGeneratorWorker> {
            let config = self.0;
            Arc::new(move |account, package, publisher, txn_factory, _rng| {
                let (lower, upper) = config.range_of(account);
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(pool(package), ident_str!("bench_onboard").to_owned(), vec![
                        bcs::to_bytes(&pool_address(publisher.address())).unwrap(),
                        bcs::to_bytes(&lower).unwrap(),
                        bcs::to_bytes(&upper).unwrap(),
                        bcs::to_bytes(&ONBOARD_LIQUIDITY).unwrap(),
                        bcs::to_bytes(&FUND_AMOUNT).unwrap(),
                    ]),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Swap,
        Rebalance,
        Poke,
        Collect,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Swaps
    /// dominate, since crossing ticks is the read set this package exists to
    /// stress.
    const MIX: [(MixKind, u32); 4] = [
        (MixKind::Swap, 55),
        (MixKind::Rebalance, 20),
        (MixKind::Poke, 15),
        (MixKind::Collect, 10),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let pool_id = bcs::to_bytes(&pool_address(publisher.address())).unwrap();
            let (lower, upper) = config.range_of(account);
            let lower = bcs::to_bytes(&lower).unwrap();
            let upper = bcs::to_bytes(&upper).unwrap();

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Swap => (ident_str!("bench_swap_in"), vec![
                    pool_id,
                    bcs::to_bytes(&rng.gen_bool(0.5)).unwrap(),
                    bcs::to_bytes(&rng.gen_range(1, config.swap_size + 1)).unwrap(),
                ]),
                MixKind::Rebalance => (ident_str!("bench_rebalance"), vec![
                    pool_id,
                    lower,
                    upper,
                    bcs::to_bytes(&REBALANCE_LIQUIDITY).unwrap(),
                    bcs::to_bytes(&FUND_AMOUNT).unwrap(),
                ]),
                MixKind::Poke => (ident_str!("poke"), vec![pool_id, lower, upper]),
                MixKind::Collect => (ident_str!("collect"), vec![
                    pool_id,
                    lower,
                    upper,
                    bcs::to_bytes(&COLLECT_MAX).unwrap(),
                    bcs::to_bytes(&COLLECT_MAX).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(pool(package), func.to_owned(), args),
            ))
        })
    }
}
