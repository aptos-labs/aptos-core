// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Workflows for the DeFi benchmark packages under
//! `third_party/move/mono-move/testsuite/benches-e2e`.
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
        account_address::AccountAddress,
        ident_str,
        identifier::Identifier,
        int256::U256,
        language_storage::{ModuleId, StructTag, TypeTag},
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
    entry_point_trait::{get_payload, get_payload_ty},
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

/// Gas ceiling on every account-signed transaction. The prologue makes the
/// sender cover `max_gas_amount * gas_unit_price` up front, so the SDK default
/// of 20,000,000 units would lock 20 APT of every account's balance against a
/// transaction that spends a fraction of it. Two million still leaves several
/// times the headroom the widest fan-out in the suite needs.
const MAX_GAS_UNITS: u64 = 2_000_000;

/// Balance each benchmark account is created with, bounded from both sides.
/// The floor: the prologue reserves [`MAX_GAS_UNITS`] at 100 octas a unit,
/// 2 APT that is never spent, and a 50-block run burns around 5 more on
/// storage, since a new state slot costs 400,000 octas and `airdrop_fanout`
/// creates 75 of them per distribution. The ceiling: the harness funds these
/// five to a sender out of 100 APT, so past 20 the fifth transfer is short.
const ACCOUNT_CREATION_BALANCE: u64 = 10_0000_0000;

#[derive(Debug, Copy, Clone)]
pub enum BenchWorkflowKind {
    /// Lending market. Reserves are configured by the publisher, each account
    /// then supplies collateral and takes one variable-rate borrow, and the
    /// mix samples supply / withdraw / borrow / repay / collateral toggle /
    /// flash loan.
    LendingMarket {
        num_accounts: usize,
        num_reserves: usize,
        collaterals_per_account: usize,
        num_txns: usize,
    },
    /// Order book over a bit-packed AVL queue. The publisher registers one
    /// market and seeds it, each account opens a funded market account, and
    /// the mix samples resting placements / matching / cancels / traversal.
    ClobAvl {
        num_accounts: usize,
        seed_orders_per_side: usize,
        index_limit: usize,
        num_txns: usize,
    },
    /// Concentrated liquidity pool. The publisher creates the pair and seeds
    /// positions, each account opens one of its own, and the mix samples
    /// swaps / rebalances / fee settlement.
    ClmmSwap {
        num_accounts: usize,
        seed_positions: usize,
        swap_size: u64,
        tick_density: u32,
        num_txns: usize,
    },
    /// StableSwap pools in the Curve shape. The publisher creates a two-coin
    /// and a three-coin pool and seeds both, each account opens a funded LP
    /// position, and the mix samples swaps / imbalanced deposits / single-coin
    /// withdrawals / a storage-free solve / amplification ramps.
    Stableswap {
        num_accounts: usize,
        amp_2coin: u64,
        amp_3coin: u64,
        fee_bps: u64,
        imbalance_bp: u64,
        swap_size: u64,
        math_only_pools: usize,
        ramp_duration_secs: u64,
        num_txns: usize,
    },
    /// Cross-chain message relay. The publisher opens and configures the
    /// channels, each account claims one and prepays the executor, and the mix
    /// samples sends / deliveries / attestations / state reads / nonce skips /
    /// config writes.
    BridgeRelay {
        num_accounts: usize,
        num_channels: usize,
        msgs_per_txn: usize,
        payload_len: usize,
        verifiers_per_msg: usize,
        read_depth: usize,
        num_txns: usize,
    },
    /// Airdrop fan-out. The publisher shards a claim registry and seeds warm
    /// recipients, each account onboards with a funded balance and a slot of
    /// its own, and the mix samples batch distribution to recipients that
    /// already have a slot and to ones that do not, fungible-asset payouts,
    /// the write over-approximation probe, claims, and read-only sweeps.
    AirdropFanout {
        num_accounts: usize,
        n_shards: usize,
        recipients_per_txn: usize,
        warm_recipients: usize,
        payload_len: usize,
        write_every: u64,
        fresh_ratio: u32,
        num_txns: usize,
    },
    /// Oracle feed updates. The publisher installs an authority set and the
    /// feeds, each account claims a contiguous feed window, and the mix samples
    /// ed25519 updates / secp256k1 updates / quorum acceptance, against
    /// verify-only and write-only controls.
    OracleBatch {
        num_accounts: usize,
        num_feeds: usize,
        feeds_per_txn: usize,
        num_signers: usize,
        quorum_k: usize,
        decode_depth: usize,
        num_txns: usize,
    },
    /// Collateralized debt positions in a sorted doubly linked list, in the
    /// shape Liquity and Thala use. The publisher seeds the list and the
    /// stability pool, each account opens one vault, and the mix samples walks
    /// / adjustments / liquidation sweeps / auction steps. Every hop along the
    /// list reads the next vault's address out of the current vault, so a walk
    /// is a chain of sequentially dependent resource reads.
    CdpLiquidation {
        num_accounts: usize,
        list_length: usize,
        walk_limit: usize,
        liquidations_per_txn: usize,
        hint_stride: usize,
        mcr_bps: u64,
        price_step: u64,
        auction_lots: usize,
        num_txns: usize,
    },
    /// DEX aggregator. The publisher creates eight assets and seeds four pool
    /// backends, each account is funded in every asset, and the mix samples one
    /// to five hop routes whose venues and legs arrive as type arguments. The
    /// five-hop route sits on the verifier's thirty-two argument ceiling.
    DexAggregator {
        num_accounts: usize,
        pools_per_backend: usize,
        amount_in: u64,
        split_bps: u64,
        tick_crossings: u64,
        book_depth: u64,
        num_txns: usize,
    },
    /// Token v2 mint and marketplace. The publisher creates the collections and
    /// seeds them with listed tokens, each account mints an inventory and lists
    /// half of it, and the mix samples buys / listings / cancels / token and
    /// collection offers / mints / index reads.
    NftMintMarket {
        num_accounts: usize,
        num_collections: usize,
        tokens_per_collection: usize,
        tokens_per_account: usize,
        tokens_read_per_txn: usize,
        price: u64,
        commission_bps: u64,
        royalty_bps: u64,
        mint_batch: usize,
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
            BenchWorkflowKind::LendingMarket {
                num_accounts,
                num_reserves,
                collaterals_per_account,
                num_txns,
            } => {
                let config = lending_market::Config::new(*num_reserves, *collaterals_per_account);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(lending_market::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        lending_market::mix_worker(config),
                    )),
                ];
                (
                    lending_market::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::ClobAvl {
                num_accounts,
                seed_orders_per_side,
                index_limit,
                num_txns,
            } => {
                let config = clob_avl::Config::new(*seed_orders_per_side, *index_limit);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(clob_avl::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        clob_avl::mix_worker(config),
                    )),
                ];
                (clob_avl::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
            BenchWorkflowKind::ClmmSwap {
                num_accounts,
                seed_positions,
                swap_size,
                tick_density,
                num_txns,
            } => {
                let config = clmm_swap::Config::new(*seed_positions, *swap_size, *tick_density);
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(clmm_swap::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        clmm_swap::mix_worker(config),
                    )),
                ];
                (clmm_swap::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
            BenchWorkflowKind::Stableswap {
                num_accounts,
                amp_2coin,
                amp_3coin,
                fee_bps,
                imbalance_bp,
                swap_size,
                math_only_pools,
                ramp_duration_secs,
                num_txns,
            } => {
                let config = stableswap::Config::new(
                    *amp_2coin,
                    *amp_3coin,
                    *fee_bps,
                    *imbalance_bp,
                    *swap_size,
                    *math_only_pools,
                    *ramp_duration_secs,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(stableswap::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        stableswap::mix_worker(config),
                    )),
                ];
                (stableswap::PACKAGE_NAME, *num_accounts, *num_txns, workers)
            },
            BenchWorkflowKind::BridgeRelay {
                num_accounts,
                num_channels,
                msgs_per_txn,
                payload_len,
                verifiers_per_msg,
                read_depth,
                num_txns,
            } => {
                let config = bridge_relay::Config::new(
                    *num_channels,
                    *msgs_per_txn,
                    *payload_len,
                    *verifiers_per_msg,
                    *read_depth,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(bridge_relay::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        bridge_relay::mix_worker(config),
                    )),
                ];
                (
                    bridge_relay::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::AirdropFanout {
                num_accounts,
                n_shards,
                recipients_per_txn,
                warm_recipients,
                payload_len,
                write_every,
                fresh_ratio,
                num_txns,
            } => {
                let config = airdrop_fanout::Config::new(
                    *n_shards,
                    *recipients_per_txn,
                    *warm_recipients,
                    *payload_len,
                    *write_every,
                    *fresh_ratio,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(airdrop_fanout::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        airdrop_fanout::mix_worker(config),
                    )),
                ];
                (
                    airdrop_fanout::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::OracleBatch {
                num_accounts,
                num_feeds,
                feeds_per_txn,
                num_signers,
                quorum_k,
                decode_depth,
                num_txns,
            } => {
                let config = oracle_batch::Config::new(
                    *num_feeds,
                    *feeds_per_txn,
                    *num_signers,
                    *quorum_k,
                    *decode_depth,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(oracle_batch::Onboard(config.clone())),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        oracle_batch::mix_worker(config),
                    )),
                ];
                (
                    oracle_batch::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::CdpLiquidation {
                num_accounts,
                list_length,
                walk_limit,
                liquidations_per_txn,
                hint_stride,
                mcr_bps,
                price_step,
                auction_lots,
                num_txns,
            } => {
                let config = cdp_liquidation::Config::new(
                    *list_length,
                    *walk_limit,
                    *liquidations_per_txn,
                    *hint_stride,
                    *mcr_bps,
                    *price_step,
                    *auction_lots,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(cdp_liquidation::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        cdp_liquidation::mix_worker(config),
                    )),
                ];
                (
                    cdp_liquidation::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::DexAggregator {
                num_accounts,
                pools_per_backend,
                amount_in,
                split_bps,
                tick_crossings,
                book_depth,
                num_txns,
            } => {
                let config = dex_aggregator::Config::new(
                    *pools_per_backend,
                    *amount_in,
                    *split_bps,
                    *tick_crossings,
                    *book_depth,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(dex_aggregator::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        dex_aggregator::mix_worker(config),
                    )),
                ];
                (
                    dex_aggregator::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
            },
            BenchWorkflowKind::NftMintMarket {
                num_accounts,
                num_collections,
                tokens_per_collection,
                tokens_per_account,
                tokens_read_per_txn,
                price,
                commission_bps,
                royalty_bps,
                mint_batch,
                num_txns,
            } => {
                let config = nft_mint_market::Config::new(
                    *num_collections,
                    *tokens_per_collection,
                    *tokens_per_account,
                    *tokens_read_per_txn,
                    *price,
                    *commission_bps,
                    *royalty_bps,
                    *mint_batch,
                );
                let workers: Vec<Box<dyn UserModuleTransactionGenerator>> = vec![
                    Box::new(nft_mint_market::Onboard(config)),
                    Box::new(PlainUserModuleTransactionGenerator::new(
                        nft_mint_market::mix_worker(config),
                    )),
                ];
                (
                    nft_mint_market::PACKAGE_NAME,
                    *num_accounts,
                    *num_txns,
                    workers,
                )
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

mod lending_market {
    use super::*;

    pub const PACKAGE_NAME: &str = "lending_market";

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
                "lending_market supports at most {} reserves",
                SYMBOLS.len()
            );
            // The mix clears one collateral slot and later restores it. The
            // account has to stay solvent while that slot is off, so a second
            // collateral has to carry the debt in the meantime.
            assert!(
                collaterals_per_account >= 2 && collaterals_per_account < num_reserves,
                "each account needs two collaterals and one reserve left to borrow from"
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

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("lending_assets")
    }

    fn pool(package: &Package) -> ModuleId {
        package.get_module_id("lending_pool")
    }

    fn logic(package: &Package) -> ModuleId {
        package.get_module_id("lending_logic")
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

            for symbol in SYMBOLS.iter().take(config.num_reserves) {
                payloads.push(get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(symbol).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ));
            }

            for (i, &(ltv, threshold, bonus, price)) in
                PARAMS.iter().enumerate().take(config.num_reserves)
            {
                let asset = config.asset(publisher.address(), i);
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
    ///
    /// Every entry but the collateral toggle goes through a `bench_*` wrapper
    /// that clamps its amount to what the account and the reserve can actually
    /// take, so no transaction in the mix aborts however long it runs or
    /// however large `STEP` is set.
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
                MixKind::Withdraw => (ident_str!("bench_withdraw"), vec![
                    collateral,
                    u256_arg(STEP),
                    bcs::to_bytes(&account.address()).unwrap(),
                ]),
                MixKind::Borrow => (ident_str!("bench_borrow"), vec![
                    debt,
                    u256_arg(STEP),
                    bcs::to_bytes(&VARIABLE).unwrap(),
                ]),
                MixKind::Repay => (ident_str!("bench_repay"), vec![
                    debt,
                    u256_arg(STEP),
                    bcs::to_bytes(&VARIABLE).unwrap(),
                ]),
                // Always the account's last collateral, never a random one: the
                // two directions then cancel out instead of leaving a different
                // slot off on every pass. The flag alternates with the sequence
                // number, so both of them run. Clearing it is what puts
                // `validate_hf_and_ltv` on the measured path; the collaterals
                // left standing keep the health factor above six.
                MixKind::SetCollateral => (ident_str!("set_user_use_reserve_as_collateral"), vec![
                    bcs::to_bytes(collaterals.last().unwrap()).unwrap(),
                    bcs::to_bytes(&(account.sequence_number() % 2 == 0)).unwrap(),
                ]),
                MixKind::FlashLoan => (ident_str!("bench_flash_loan"), vec![
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

mod clob_avl {
    use super::*;

    pub const PACKAGE_NAME: &str = "clob_avl";

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

    /// Ticks a side spans, starting `SPREAD` away from the mid. Prices are
    /// drawn uniformly over the band however many orders a call places, so a
    /// band eight times the resting depth leaves most orders on a price level
    /// of their own: 512 orders over 4097 ticks come out at 478 distinct
    /// levels, a tree nine deep. A band the size of the order count would pile
    /// them onto a handful of levels and leave no pointer chase to measure.
    const PRICE_BAND: u64 = 4096;

    /// `seed_book` draws every price from that one band, however many orders
    /// a call places, and refuses to price a bid below zero.
    const _: () = assert!(BASE_PRICE > SPREAD + PRICE_BAND + 1);

    /// Orders one replenish adds to a side that is under its depth target.
    const REPLENISH_PER_SIDE: u64 = 4;

    /// Largest market order, in lots. Seeded orders average 4.5 lots, so the
    /// mean market order of 16.5 walks about four resting orders off the head.
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

    /// Orders one AVL queue holds, and there is one queue per side.
    const AVL_NODES_MAX: u64 = 16_383;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub seed_orders_per_side: u64,
        pub index_limit: u64,
    }

    impl Config {
        pub fn new(seed_orders_per_side: usize, index_limit: usize) -> Self {
            let seed_orders_per_side = seed_orders_per_side as u64;
            // The seeded depth is also the depth the mix holds the book at,
            // and a side that grew past its queue would abort.
            assert!(
                seed_orders_per_side < AVL_NODES_MAX,
                "seeded depth would overflow the AVL queue"
            );
            Self {
                seed_orders_per_side,
                index_limit: index_limit as u64,
            }
        }
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("clob_assets")
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
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(BASE_SYMBOL).unwrap(),
                        bcs::to_bytes(&BASE_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    assets(package),
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
                        bcs::to_bytes(&PRICE_BAND).unwrap(),
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

            let (func, args) = match MIX[dist.sample(rng)].0 {
                // Tops up both sides, each gated on its own depth and centred
                // on the mid the book was seeded around rather than on a side,
                // so a replenished order rests instead of crossing. A side
                // under its target gains 0.20 * 4 * 4.5 = 3.6 lots a
                // transaction against the 0.30 * 0.5 * 16.5 = 2.5 lots market
                // orders take off it, and the gate closes at the target, so
                // depth settles at `seed_orders_per_side` for a run of any
                // length. The surplus is what stops the book draining and the
                // gate is what stops it growing until the AVL queue runs out
                // of nodes.
                MixKind::Replenish => (ident_str!("bench_replenish"), vec![
                    bcs::to_bytes(&MARKET_ID).unwrap(),
                    bcs::to_bytes(&REPLENISH_PER_SIDE).unwrap(),
                    bcs::to_bytes(&config.seed_orders_per_side).unwrap(),
                    bcs::to_bytes(&BASE_PRICE).unwrap(),
                    bcs::to_bytes(&SPREAD).unwrap(),
                    bcs::to_bytes(&PRICE_BAND).unwrap(),
                    bcs::to_bytes(&rng.gen_range(0u64, u64::MAX)).unwrap(),
                ]),
                MixKind::PlaceAndCancel => {
                    // Inside the band the book occupies, so the insert and the
                    // remove land among the resting levels rather than off the
                    // end of the tree, and on the resting side of the spread,
                    // so the placement always rests and the cancel always runs
                    // the AVL remove path.
                    let jitter = rng.gen_range(0, PRICE_BAND + 1);
                    let price = if side == BID {
                        BASE_PRICE - SPREAD - jitter
                    } else {
                        BASE_PRICE + SPREAD + jitter
                    };
                    (ident_str!("bench_place_and_cancel"), vec![
                        bcs::to_bytes(&MARKET_ID).unwrap(),
                        bcs::to_bytes(&side).unwrap(),
                        bcs::to_bytes(&price).unwrap(),
                        bcs::to_bytes(&rng.gen_range(1u64, 9)).unwrap(),
                    ])
                },
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

mod clmm_swap {
    use super::*;

    pub const PACKAGE_NAME: &str = "clmm_swap";

    /// The assets, chained: consecutive symbols share a pool, so a route can
    /// walk the chain in either direction and every hop has somewhere to go.
    const ASSET_SYMBOLS: [&[u8]; 4] = [b"CMA", b"CMB", b"CMC", b"CMD"];
    const NUM_POOLS: usize = ASSET_SYMBOLS.len() - 1;
    const DECIMALS: u8 = 8;

    /// 0.3% fee against the package's million-unit denominator, and Uniswap
    /// V3's 0.3% tier spacing.
    const FEE_RATE: u64 = 3000;
    const TICK_SPACING: u32 = 60;

    /// Q64.96 one, which is the square root price at tick zero.
    const Q96: u128 = 1 << 96;

    /// The publisher's backstop position, as wide as the package's fixed-point
    /// math takes at this liquidity. Must match `clmm_pool::bench_backstop_tick`,
    /// which the package's tests hold to.
    const BACKSTOP_TICK: i32 = 150_000;
    const BACKSTOP_LIQUIDITY: u128 = 1_000_000_000_000_000;

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
        pub tick_density: u32,
    }

    impl Config {
        pub fn new(seed_positions: usize, swap_size: u64, tick_density: u32) -> Self {
            assert!(swap_size > 0, "a zero-sized swap aborts");
            assert!(tick_density > 0, "a zero density seeds no boundaries");
            Self {
                seed_positions: seed_positions as u64,
                swap_size,
                tick_density,
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

        /// The pool an account keeps its position in, derived the same way for
        /// the same reason.
        fn pool_of(&self, account: &LocalAccount) -> usize {
            slot_for(account, NUM_POOLS)
        }
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("clmm_assets")
    }

    fn pool(package: &Package) -> ModuleId {
        package.get_module_id("clmm_pool")
    }

    /// Address of the pool over assets `index` and `index + 1`. The package
    /// derives it as a named object seeded with the pair and the
    /// configuration.
    fn pool_address(publisher: AccountAddress, index: usize) -> AccountAddress {
        let token_a = create_object_address(publisher, ASSET_SYMBOLS[index]);
        let token_b = create_object_address(publisher, ASSET_SYMBOLS[index + 1]);
        let mut seed = b"clmm_swap_pool".to_vec();
        seed.extend_from_slice(&bcs::to_bytes(&token_a).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&token_b).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&FEE_RATE).unwrap());
        seed.extend_from_slice(&bcs::to_bytes(&TICK_SPACING).unwrap());
        create_object_address(publisher, &seed)
    }

    /// Stage 1: create and seed the pools from the publisher, then give each
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
            let publisher_address = publisher.address();
            let token =
                |index: usize| create_object_address(publisher_address, ASSET_SYMBOLS[index]);

            let mut payloads = vec![];
            for (index, symbol) in ASSET_SYMBOLS.iter().enumerate() {
                payloads.push(get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(*symbol).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    assets(package),
                    ident_str!("mint").to_owned(),
                    vec![
                        bcs::to_bytes(&token(index)).unwrap(),
                        bcs::to_bytes(&publisher_address).unwrap(),
                        bcs::to_bytes(&PUBLISHER_FUNDING).unwrap(),
                    ],
                ));
            }

            // Pools are created in chain order, which is the order the package
            // registers them in and therefore the order a path indexes them.
            for index in 0..NUM_POOLS {
                let pool_id = pool_address(publisher_address, index);
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("create_pool").to_owned(),
                    vec![
                        bcs::to_bytes(&token(index)).unwrap(),
                        bcs::to_bytes(&token(index + 1)).unwrap(),
                        bcs::to_bytes(&FEE_RATE).unwrap(),
                        bcs::to_bytes(&TICK_SPACING).unwrap(),
                        bcs::to_bytes(&Q96).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("mint").to_owned(),
                    vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&-BACKSTOP_TICK).unwrap(),
                        bcs::to_bytes(&BACKSTOP_TICK).unwrap(),
                        bcs::to_bytes(&BACKSTOP_LIQUIDITY).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("seed_positions").to_owned(),
                    vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&config.seed_positions).unwrap(),
                        bcs::to_bytes(&config.tick_density).unwrap(),
                        bcs::to_bytes(&(42 + index as u64)).unwrap(),
                    ],
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
            Arc::new(move |account, package, publisher, txn_factory, _rng| {
                let (lower, upper) = config.range_of(account);
                let pool_id = pool_address(publisher.address(), config.pool_of(account));
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(pool(package), ident_str!("bench_onboard").to_owned(), vec![
                        bcs::to_bytes(&pool_id).unwrap(),
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
        MultiHop,
        Rebalance,
        Poke,
        Collect,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Swaps
    /// dominate, since crossing ticks is the read set this package exists to
    /// stress.
    const MIX: [(MixKind, u32); 5] = [
        (MixKind::Swap, 40),
        (MixKind::MultiHop, 15),
        (MixKind::Rebalance, 20),
        (MixKind::Poke, 15),
        (MixKind::Collect, 10),
    ];

    /// A contiguous run of two or three pools, walked in either direction.
    ///
    /// Only a run chains, since a pool shares a token with its neighbours and
    /// nothing else. Walking it both ways keeps a long mix from pushing every
    /// pool's price the same way.
    fn route_path(rng: &mut StdRng) -> Vec<u64> {
        let hops = rng.gen_range(2, NUM_POOLS as u64 + 1);
        let start = rng.gen_range(0, NUM_POOLS as u64 + 1 - hops);
        let run = (start..start + hops).collect::<Vec<u64>>();
        if rng.gen_bool(0.5) {
            run.into_iter().rev().collect::<Vec<u64>>()
        } else {
            run
        }
    }

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let home = pool_address(publisher.address(), config.pool_of(account));
            let pool_id = bcs::to_bytes(&home).unwrap();
            let (lower, upper) = config.range_of(account);
            let lower = bcs::to_bytes(&lower).unwrap();
            let upper = bcs::to_bytes(&upper).unwrap();

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Swap => (ident_str!("bench_swap_in"), vec![
                    pool_id,
                    bcs::to_bytes(&rng.gen_bool(0.5)).unwrap(),
                    bcs::to_bytes(&rng.gen_range(1, config.swap_size + 1)).unwrap(),
                ]),
                MixKind::MultiHop => (ident_str!("bench_swap_multi_hop"), vec![
                    bcs::to_bytes(&route_path(rng)).unwrap(),
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
mod stableswap {
    use super::*;

    pub const PACKAGE_NAME: &str = "stableswap";

    /// Every coin the package needs, with its decimals. Pools draw from this
    /// set by index and overlap, so a coin's reserve is moved by traffic on
    /// more than one pool. Mixed decimals are what put the rate scaling in
    /// `xp_mem` on the hot path.
    const SYMBOLS: [(&[u8], u8); 5] = [
        (b"SS0", 6),
        (b"SS1", 6),
        (b"SS2", 6),
        (b"SS3", 8),
        (b"SS4", 8),
    ];

    struct PoolSpec {
        /// Coins the pool holds, as indices into `SYMBOLS`. The first is the
        /// pool's lead coin, which every deposit into it is sized against.
        coins: &'static [usize],
        /// Share of the lead coin's amount every other coin gets on a deposit
        /// the package makes, in basis points.
        deposit_skew_bp: u64,
        /// Run at a fraction of the configured amplification. A near-balanced
        /// pool at a low `A` costs Newton rounds that a lopsided one at a high
        /// `A` does not, so the ladder needs both.
        soft_amp: bool,
    }

    /// The pools the publisher builds, in the order it builds them. Ids are
    /// handed out from one, so an entry's position fixes its id.
    ///
    /// Two run at the configured amplification and two at a hundredth of it,
    /// and their target compositions span two orders of magnitude. Distance
    /// from balance and `A` are the two things that set a Newton trip count, so
    /// spanning both is what spreads the counts across the mix instead of
    /// pinning them at one value.
    static POOLS: [PoolSpec; 4] = [
        PoolSpec {
            coins: &[0, 1],
            deposit_skew_bp: 10_000,
            soft_amp: false,
        },
        PoolSpec {
            coins: &[2, 3, 4],
            deposit_skew_bp: 1_000,
            soft_amp: false,
        },
        PoolSpec {
            coins: &[1, 3],
            deposit_skew_bp: 300,
            soft_amp: true,
        },
        PoolSpec {
            coins: &[0, 2, 4],
            deposit_skew_bp: 100,
            soft_amp: true,
        },
    ];

    /// Divisor and floor a softened pool's amplification is taken at. The floor
    /// keeps `A` above the point where the invariant stops having a solution
    /// the Newton loop reaches.
    const SOFT_AMP_DIV: u64 = 100;
    const MIN_SOFT_AMP: u64 = 2;

    /// Whole tokens of the lead coin the publisher seeds a pool with; every
    /// other coin gets the pool's skew share of that. Deliberately small: a
    /// swap has to be a percent-scale move on a reserve for the package's 5%
    /// per-swap cap to bind and for the solve to start somewhere new each time.
    /// At a million the mix's swaps are rounding error and every solve costs
    /// the same three rounds.
    const SEED_UNITS: u64 = 20_000;

    /// Whole tokens an onboarding transaction faucets the account in every
    /// coin, and how much of that it deposits into each pool. Funding covers a
    /// long run of swaps before the package has to top the account up; the
    /// deposit is what gives it LP to withdraw against.
    const ONBOARD_FUND_UNITS: u64 = 100_000;
    const ONBOARD_DEPOSIT_UNITS: u64 = 20;

    /// Whole tokens an imbalanced deposit leads with.
    ///
    /// This and `ONBOARD_DEPOSIT_UNITS` are small enough that the skew share of
    /// the two narrowest pools truncates to zero whole tokens, making those
    /// deposits single-sided. That is the intent, not an accident of rounding:
    /// it is what keeps pools 3 and 4 lopsided against their own liquidity
    /// traffic. Raising either constant past 10000 / `deposit_skew_bp` pulls
    /// them back toward balance and flattens the trip counts.
    const ADD_UNITS: u64 = 10;

    /// Share of its LP a single-coin withdrawal burns, in basis points. Large
    /// enough that the withdrawal moves the pool, small enough that an account
    /// keeps a position across many of them.
    const REMOVE_BPS: u64 = 2_000;

    /// Amplification the storage-free math path solves against. Held apart
    /// from the pools' own so the two do not move together.
    const MATH_ONLY_A: u64 = 200;

    /// Distinct assignments of coin pair and withdrawal coin. Accounts spread
    /// over these, so concurrent transactions push a pool in different
    /// directions and their Newton loops take different numbers of rounds. Six
    /// is the least common multiple of the two pool widths' ordered-pair
    /// counts, so every pair is somebody's.
    const ACCOUNT_SLOTS: usize = 6;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub amp_2coin: u64,
        pub amp_3coin: u64,
        pub fee_bps: u64,
        pub imbalance_bp: u64,
        pub swap_size: u64,
        pub math_only_pools: u64,
        pub ramp_duration_secs: u64,
    }

    impl Config {
        pub fn new(
            amp_2coin: u64,
            amp_3coin: u64,
            fee_bps: u64,
            imbalance_bp: u64,
            swap_size: u64,
            math_only_pools: usize,
            ramp_duration_secs: u64,
        ) -> Self {
            // The package clamps all of these, so a value outside the range is
            // not an abort. It is a silently different benchmark, which is
            // worse.
            assert!(
                amp_2coin > 0 && amp_2coin <= 1_000_000,
                "amplification outside the range the package keeps"
            );
            assert!(
                amp_3coin > 0 && amp_3coin <= 1_000_000,
                "amplification outside the range the package keeps"
            );
            assert!(fee_bps <= 100, "fee above the package's ceiling");
            assert!(imbalance_bp <= 10_000, "skew above a balanced deposit");
            assert!(swap_size > 0, "a zero-sized swap moves no reserve");
            assert!(
                math_only_pools > 0 && math_only_pools <= 64,
                "synthetic pool count outside the package's bound"
            );
            assert!(
                ramp_duration_secs > 0 && ramp_duration_secs <= 31_536_000,
                "ramp duration outside the package's bound"
            );
            Self {
                amp_2coin,
                amp_3coin,
                fee_bps,
                imbalance_bp,
                swap_size,
                math_only_pools: math_only_pools as u64,
                ramp_duration_secs,
            }
        }
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("ss_assets")
    }

    fn pool(package: &Package) -> ModuleId {
        package.get_module_id("ss_pool")
    }

    fn lp(package: &Package) -> ModuleId {
        package.get_module_id("ss_lp")
    }

    fn math(package: &Package) -> ModuleId {
        package.get_module_id("ss_math")
    }

    fn amp(package: &Package) -> ModuleId {
        package.get_module_id("ss_amp")
    }

    fn pool_id_of(index: usize) -> u64 {
        index as u64 + 1
    }

    /// The amplification a pool is created at.
    fn amp_for(spec: &PoolSpec, config: Config) -> u64 {
        let base = if spec.coins.len() == 2 {
            config.amp_2coin
        } else {
            config.amp_3coin
        };
        if spec.soft_amp {
            (base / SOFT_AMP_DIV).max(MIN_SOFT_AMP)
        } else {
            base
        }
    }

    /// A uniformly chosen pool holding `n_coins` coins, with its id. Picking
    /// per transaction rather than per account keeps any one pool from being
    /// driven in a single direction for a whole run, which drains a coin and
    /// puts the invariant somewhere the Newton loop takes its full bound to
    /// leave.
    fn pool_of_width(rng: &mut StdRng, n_coins: usize) -> (u64, &'static PoolSpec) {
        let matching = POOLS
            .iter()
            .filter(|spec| spec.coins.len() == n_coins)
            .count();
        let mut pick = rng.gen_range(0, matching);
        for (index, spec) in POOLS.iter().enumerate() {
            if spec.coins.len() == n_coins {
                if pick == 0 {
                    return (pool_id_of(index), spec);
                }
                pick -= 1;
            }
        }
        unreachable!("POOLS holds a pool of every width the mix asks for")
    }

    fn any_pool(rng: &mut StdRng) -> u64 {
        pool_id_of(rng.gen_range(0, POOLS.len()))
    }

    /// The ordered coin pair an account swaps, out of a pool with `n_coins`.
    /// Both stages recompute it from the address, so they agree without any
    /// shared state.
    fn pair_for(account: &LocalAccount, n_coins: u64) -> (u64, u64) {
        let slot = slot_for(account, ACCOUNT_SLOTS) as u64;
        // Every ordered pair, so no coin is only ever a swap destination. A
        // coin that is never a source is one the pool only ever pays out, and
        // it drains to the package's spread floor and stays there.
        let p = slot % (n_coins * (n_coins - 1));
        let i = p / (n_coins - 1);
        // The offset stays in one to `n_coins - 1`, so the two indices are
        // always different and the package never has to repair them.
        let j = (i + 1 + p % (n_coins - 1)) % n_coins;
        (i, j)
    }

    /// Stage 1: create the assets and every pool from the publisher, then give
    /// each account one transaction that funds it and opens an LP position.
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
            let mut payloads = SYMBOLS
                .iter()
                .map(|(symbol, decimals)| {
                    get_payload(
                        assets(package),
                        ident_str!("create_asset_entry").to_owned(),
                        vec![
                            bcs::to_bytes(symbol).unwrap(),
                            bcs::to_bytes(decimals).unwrap(),
                        ],
                    )
                })
                .collect::<Vec<TransactionPayload>>();

            payloads.push(get_payload(
                pool(package),
                ident_str!("initialize").to_owned(),
                vec![],
            ));

            for (index, spec) in POOLS.iter().enumerate() {
                let symbols = spec
                    .coins
                    .iter()
                    .map(|k| SYMBOLS[*k].0.to_vec())
                    .collect::<Vec<Vec<u8>>>();
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("create_pool").to_owned(),
                    vec![
                        bcs::to_bytes(&pool_id_of(index)).unwrap(),
                        bcs::to_bytes(&symbols).unwrap(),
                        bcs::to_bytes(&amp_for(spec, config)).unwrap(),
                        bcs::to_bytes(&config.fee_bps).unwrap(),
                        bcs::to_bytes(&spec.deposit_skew_bp).unwrap(),
                    ],
                ));
            }

            // One pool per transaction. Seeding several in one would mint,
            // move, and solve the invariant for all their coins at once, which
            // runs past the per-transaction execution limit.
            for index in 0..POOLS.len() {
                payloads.push(get_payload(
                    pool(package),
                    ident_str!("seed_liquidity").to_owned(),
                    vec![
                        bcs::to_bytes(&pool_id_of(index)).unwrap(),
                        bcs::to_bytes(&SEED_UNITS).unwrap(),
                    ],
                ));
            }

            // Leave the three-coin pool's amplification entry mid-ramp from the
            // start, so what the mix rewrites is a ramp rather than the flat
            // one `create_pool` registers.
            payloads.push(get_payload(
                amp(package),
                ident_str!("set_ramp").to_owned(),
                vec![
                    bcs::to_bytes(&pool_id_of(1)).unwrap(),
                    bcs::to_bytes(&(config.amp_3coin / 2)).unwrap(),
                    bcs::to_bytes(&config.ramp_duration_secs).unwrap(),
                ],
            ));

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
                    get_payload(pool(package), ident_str!("bench_onboard").to_owned(), vec![
                        bcs::to_bytes(&ONBOARD_FUND_UNITS).unwrap(),
                        bcs::to_bytes(&ONBOARD_DEPOSIT_UNITS).unwrap(),
                    ]),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Swap2,
        Swap3,
        AddImbalanced,
        RemoveOneCoin,
        MathOnly,
        Ramp,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Swaps
    /// dominate, as they do on a real stable pool, and the two liquidity flows
    /// are the ones that solve the invariant twice.
    ///
    /// `Ramp` does not move the amplification: the harness freezes the block
    /// timestamp, so `ramp_to` sets `t0 = now` and the interpolation returns
    /// the value already in place. It keeps a small share because it is the
    /// only branch that writes the amplification entry every swap on that pool
    /// reads, which is a write to a hot slot rather than a Newton solve.
    const MIX: [(MixKind, u32); 6] = [
        (MixKind::Swap2, 40),
        (MixKind::Swap3, 25),
        (MixKind::AddImbalanced, 12),
        (MixKind::RemoveOneCoin, 8),
        (MixKind::MathOnly, 12),
        (MixKind::Ramp, 3),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let slot = slot_for(account, ACCOUNT_SLOTS) as u64;
            // Sizes are jittered around the configured one so consecutive
            // swaps leave the pool at a different distance from balance.
            let size = config.swap_size / 2 + rng.gen_range(0, config.swap_size);

            let (module, func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Swap2 => {
                    let (pool_id, spec) = pool_of_width(rng, 2);
                    let (i, j) = pair_for(account, spec.coins.len() as u64);
                    (pool(package), ident_str!("bench_swap"), vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&i).unwrap(),
                        bcs::to_bytes(&j).unwrap(),
                        bcs::to_bytes(&size).unwrap(),
                    ])
                },
                MixKind::Swap3 => {
                    let (pool_id, spec) = pool_of_width(rng, 3);
                    let (i, j) = pair_for(account, spec.coins.len() as u64);
                    (pool(package), ident_str!("bench_swap"), vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&i).unwrap(),
                        bcs::to_bytes(&j).unwrap(),
                        bcs::to_bytes(&size).unwrap(),
                    ])
                },
                MixKind::AddImbalanced => {
                    let pool_id = any_pool(rng);
                    (lp(package), ident_str!("bench_add_imbalanced"), vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&ADD_UNITS).unwrap(),
                        bcs::to_bytes(&config.imbalance_bp).unwrap(),
                    ])
                },
                MixKind::RemoveOneCoin => {
                    let pool_id = any_pool(rng);
                    (lp(package), ident_str!("bench_remove_one"), vec![
                        bcs::to_bytes(&pool_id).unwrap(),
                        bcs::to_bytes(&slot).unwrap(),
                        bcs::to_bytes(&REMOVE_BPS).unwrap(),
                    ])
                },
                MixKind::MathOnly => (math(package), ident_str!("bench_math_only"), vec![
                    bcs::to_bytes(&config.math_only_pools).unwrap(),
                    bcs::to_bytes(&MATH_ONLY_A).unwrap(),
                    bcs::to_bytes(&rng.gen_range(0u64, u64::MAX)).unwrap(),
                ]),
                MixKind::Ramp => {
                    let index = rng.gen_range(0, POOLS.len());
                    let base = amp_for(&POOLS[index], config);
                    // The harness advances the block timestamp by a microsecond
                    // per block, so an interpolated `A` never leaves the value
                    // it starts from and the target below does not take effect.
                    // The write is the point.
                    let target = if rng.gen_bool(0.5) {
                        base / 2
                    } else {
                        base * 2
                    };
                    (amp(package), ident_str!("bench_ramp"), vec![
                        bcs::to_bytes(&pool_id_of(index)).unwrap(),
                        bcs::to_bytes(&target).unwrap(),
                        bcs::to_bytes(&config.ramp_duration_secs).unwrap(),
                    ])
                },
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(module, func.to_owned(), args),
            ))
        })
    }
}

mod bridge_relay {
    use super::*;

    pub const PACKAGE_NAME: &str = "bridge_relay";

    /// Endpoint ids either side of the bridge. Both are LayerZero V2 mainnet
    /// ids, so a channel reads the way a real one does.
    const SRC_EID: u32 = 30101;
    const DST_EID: u32 = 30108;

    /// Channels one `register_oapps` transaction opens. Opening the whole set
    /// at once runs past the per-transaction execution limit.
    const REGISTER_CHUNK: u64 = 32;

    /// Verifiers a channel may hold, mirroring the cap the package clamps to.
    const MAX_VERIFIERS: u64 = 16;

    /// Messages a single `bench_verify` or `bench_skip` may name, mirroring
    /// the package's own clamp.
    const MAX_MSGS: u64 = 64;

    /// Widest message body. A relay carries application payloads, not blobs,
    /// and 4 KiB is already past what a bridged call needs.
    const MAX_PAYLOAD_LEN: usize = 4096;

    /// Channels the setup will open. Each one costs a `set_verifiers` and a
    /// `fund` transaction, both signed by the publisher and so serialized on
    /// its sequence number; 4096 channels is already 8193 setup transactions.
    const MAX_CHANNELS: usize = 4096;

    /// Bytes the chain accepts for one signed transaction, matching the gas
    /// schedule's `txn.max_transaction_size_in_bytes`.
    const MAX_TXN_BYTES: usize = 65536;

    /// What a signed mix transaction carries besides the message bodies: the
    /// 32-byte sender, the sequence number and the three u64 gas and expiry
    /// fields and the chain id at 33, the Ed25519 authenticator at 99, the
    /// entry-function header at 67 (payload tag, 32-byte module address,
    /// `relay_endpoint` at 15, the longest entry name at 17, an empty type
    /// argument list, and the argument count), the channel argument at 9, and
    /// 4 for the two BCS length prefixes ahead of the message vector. That is
    /// 244; the remaining 5 cover the versioned payload format, which wraps
    /// the same entry function in five more enum tags.
    const TXN_ENVELOPE_BYTES: usize = 249;

    /// Prepaid executor balance per channel, and the top-up each account adds
    /// when it onboards. Sends debit it per byte, so it has to outlast a run.
    const EXECUTOR_FUNDING: u128 = 1_000_000_000_000_000;
    const ONBOARD_PREPAY: u128 = 1_000_000_000;

    /// Executor rate `bench_set_config` writes back. One unit a byte keeps the
    /// debit proportional to the payload without draining the prepayment.
    const FEE_PER_BYTE: u64 = 1;

    /// Message library versions the mix alternates between. Both are
    /// registered shapes, so switching is a write and never a rejection.
    const MSGLIB_VERSIONS: [u8; 2] = [1, 2];

    /// Messages a single skip takes off the queue. Skips are rare, and a large
    /// one would starve the deliveries that follow it.
    const SKIP_PER_TXN: u64 = 1;

    /// Bytes BCS spends on the ULEB128 length prefix ahead of `len` bytes.
    const fn uleb_len(len: usize) -> usize {
        let mut rest = len;
        let mut bytes = 1;
        while rest >= 0x80 {
            rest >>= 7;
            bytes += 1;
        }
        bytes
    }

    /// Bytes one transaction's messages occupy once encoded: each body plus
    /// its own length prefix.
    const fn message_bytes(msgs_per_txn: usize, payload_len: usize) -> usize {
        msgs_per_txn * (payload_len + uleb_len(payload_len))
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub num_channels: usize,
        pub msgs_per_txn: usize,
        pub payload_len: usize,
        pub verifiers_per_msg: usize,
        pub read_depth: usize,
    }

    impl Config {
        pub fn new(
            num_channels: usize,
            msgs_per_txn: usize,
            payload_len: usize,
            verifiers_per_msg: usize,
            read_depth: usize,
        ) -> Self {
            assert!(
                num_channels > 0 && num_channels <= MAX_CHANNELS,
                "bridge_relay supports 1 to {MAX_CHANNELS} channels"
            );
            assert!(
                msgs_per_txn > 0 && msgs_per_txn as u64 <= MAX_MSGS,
                "a transaction carries 1 to {} messages",
                MAX_MSGS
            );
            assert!(
                payload_len <= MAX_PAYLOAD_LEN,
                "payloads are at most {MAX_PAYLOAD_LEN} bytes"
            );
            // The two knobs above each pass at their own ceiling and still
            // multiply out past what the chain will accept, and an oversized
            // transaction is discarded rather than aborted, which the harness
            // does not tolerate.
            let signed = TXN_ENVELOPE_BYTES + message_bytes(msgs_per_txn, payload_len);
            assert!(
                signed <= MAX_TXN_BYTES,
                "{msgs_per_txn} messages of {payload_len} bytes do not fit a \
                 transaction: {signed} bytes against a {MAX_TXN_BYTES} byte limit"
            );
            assert!(
                verifiers_per_msg > 0 && verifiers_per_msg as u64 <= MAX_VERIFIERS,
                "a channel holds 1 to {} verifiers",
                MAX_VERIFIERS
            );
            assert!(
                read_depth > 0 && read_depth as u64 <= MAX_MSGS,
                "a read walks 1 to {} messages",
                MAX_MSGS
            );
            Self {
                num_channels,
                msgs_per_txn,
                payload_len,
                verifiers_per_msg,
                read_depth,
            }
        }

        /// Payload bytes for one transaction, BCS-encoded as the
        /// `vector<vector<u8>>` the entry functions take. They are built here
        /// rather than in Move so that a measured transaction spends its time
        /// on argument decode and the table write.
        fn payloads(&self, rng: &mut StdRng) -> Vec<u8> {
            let messages = (0..self.msgs_per_txn)
                .map(|_| {
                    (0..self.payload_len)
                        .map(|_| rng.gen_range(0u8, u8::MAX))
                        .collect::<Vec<u8>>()
                })
                .collect::<Vec<Vec<u8>>>();
            bcs::to_bytes(&messages).unwrap()
        }
    }

    fn endpoint(package: &Package) -> ModuleId {
        package.get_module_id("relay_endpoint")
    }

    fn dvn(package: &Package) -> ModuleId {
        package.get_module_id("relay_dvn")
    }

    fn executor(package: &Package) -> ModuleId {
        package.get_module_id("relay_executor")
    }

    /// Stage 1: open and configure every channel from the publisher, then give
    /// each account one transaction that claims its channel and prepays the
    /// executor.
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
            let num_channels = config.num_channels as u64;
            let mut payloads = vec![get_payload(
                endpoint(package),
                ident_str!("initialize").to_owned(),
                vec![
                    bcs::to_bytes(&num_channels).unwrap(),
                    bcs::to_bytes(&SRC_EID).unwrap(),
                    bcs::to_bytes(&DST_EID).unwrap(),
                ],
            )];

            let mut start = 0;
            while start < num_channels {
                let count = REGISTER_CHUNK.min(num_channels - start);
                payloads.push(get_payload(
                    endpoint(package),
                    ident_str!("register_oapps").to_owned(),
                    vec![
                        bcs::to_bytes(&start).unwrap(),
                        bcs::to_bytes(&count).unwrap(),
                    ],
                ));
                start += count;
            }

            for channel in 0..num_channels {
                payloads.push(get_payload(
                    dvn(package),
                    ident_str!("set_verifiers").to_owned(),
                    vec![
                        bcs::to_bytes(&channel).unwrap(),
                        bcs::to_bytes(&(config.verifiers_per_msg as u64)).unwrap(),
                    ],
                ));
            }

            for channel in 0..num_channels {
                payloads.push(get_payload(
                    executor(package),
                    ident_str!("fund").to_owned(),
                    vec![
                        bcs::to_bytes(&channel).unwrap(),
                        bcs::to_bytes(&EXECUTOR_FUNDING).unwrap(),
                    ],
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
                let channel = slot_for(account, config.num_channels) as u64;
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        endpoint(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&channel).unwrap(),
                            bcs::to_bytes(&ONBOARD_PREPAY).unwrap(),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    enum MixKind {
        Send,
        Deliver,
        Verify,
        ReadState,
        Skip,
        SetConfig,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Sends and
    /// deliveries are one to one, since every cross-chain message is sent once
    /// and received once. Verification sits just under them because one call
    /// attests a whole batch against every configured verifier, and the
    /// messages a skip takes off the queue are never attested at all.
    const MIX: [(MixKind, u32); 6] = [
        (MixKind::Send, 30),
        (MixKind::Deliver, 30),
        (MixKind::Verify, 22),
        (MixKind::ReadState, 10),
        (MixKind::Skip, 4),
        (MixKind::SetConfig, 4),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let channel = bcs::to_bytes(&(slot_for(account, config.num_channels) as u64)).unwrap();

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Send => (ident_str!("bench_send"), vec![
                    channel,
                    config.payloads(rng),
                ]),
                MixKind::Deliver => (ident_str!("bench_deliver"), vec![
                    channel,
                    config.payloads(rng),
                ]),
                MixKind::Verify => (ident_str!("bench_verify"), vec![
                    channel,
                    bcs::to_bytes(&(config.msgs_per_txn as u64)).unwrap(),
                    bcs::to_bytes(&(config.verifiers_per_msg as u64)).unwrap(),
                ]),
                MixKind::ReadState => (ident_str!("bench_read_state"), vec![
                    channel,
                    bcs::to_bytes(&(config.read_depth as u64)).unwrap(),
                ]),
                MixKind::Skip => (ident_str!("bench_skip"), vec![
                    channel,
                    bcs::to_bytes(&SKIP_PER_TXN).unwrap(),
                ]),
                MixKind::SetConfig => (ident_str!("bench_set_config"), vec![
                    channel,
                    bcs::to_bytes(&(config.verifiers_per_msg as u64)).unwrap(),
                    bcs::to_bytes(&MSGLIB_VERSIONS[rng.gen_range(0, MSGLIB_VERSIONS.len())])
                        .unwrap(),
                    bcs::to_bytes(&FEE_PER_BYTE).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(endpoint(package), func.to_owned(), args),
            ))
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use aptos_sdk::types::chain_id::ChainId;
        use rand::SeedableRng;

        /// Size of the widest mix transaction `config` can produce, measured
        /// the way the chain measures it. The module and entry names are the
        /// longest the package has, so this covers every branch.
        fn widest_signed_txn_bytes(config: Config) -> usize {
            let mut rng = StdRng::seed_from_u64(0);
            let account = LocalAccount::generate(&mut rng);
            let txn_factory = TransactionFactory::new(ChainId::test());
            let payload = get_payload(
                ModuleId::new(AccountAddress::ONE, ident_str!("relay_endpoint").to_owned()),
                ident_str!("bench_read_state").to_owned(),
                vec![bcs::to_bytes(&0u64).unwrap(), config.payloads(&mut rng)],
            );
            bcs::to_bytes(&sign_capped(&account, &txn_factory, payload))
                .unwrap()
                .len()
        }

        #[test]
        fn the_widest_accepted_config_fits_a_transaction() {
            for msgs_per_txn in 1..=MAX_MSGS as usize {
                let payload_len = (1..=MAX_PAYLOAD_LEN)
                    .rev()
                    .find(|len| {
                        TXN_ENVELOPE_BYTES + message_bytes(msgs_per_txn, *len) <= MAX_TXN_BYTES
                    })
                    .expect("one message has to fit at every width");
                let bytes =
                    widest_signed_txn_bytes(Config::new(1, msgs_per_txn, payload_len, 1, 1));
                assert!(
                    bytes <= MAX_TXN_BYTES,
                    "{msgs_per_txn} messages of {payload_len} bytes sign to {bytes} bytes, \
                     past the {MAX_TXN_BYTES} byte limit"
                );
            }
        }

        #[test]
        #[should_panic(expected = "do not fit a transaction")]
        fn both_knobs_at_their_own_ceiling_are_rejected() {
            Config::new(1, MAX_MSGS as usize, MAX_PAYLOAD_LEN, 1, 1);
        }

        /// Batches a channel's queue has to hold for a delivery to find a full
        /// one waiting. The queue is a random walk reflected at empty, so its
        /// mean depth has to sit several batches clear of that boundary;
        /// below this the Deliver branch, nearly a third of the mix, starts
        /// writing fewer payloads than it was handed and the run stops
        /// measuring writes.
        const QUEUE_DEPTH_IN_BATCHES: f64 = 4.0;

        fn weight_of(kind: MixKind) -> f64 {
            MIX.iter()
                .find(|(k, _)| *k == kind)
                .expect("every mix kind carries a weight")
                .1 as f64
        }

        /// The Send and Deliver weights, the Skip weight, and `SKIP_PER_TXN`
        /// together fix how deep a channel's queue sits, and nothing else in
        /// the package checks them against each other. At the shipped weights
        /// 100 transactions put 30/100 * 8 * 100 = 240 messages on a channel
        /// and take 30/100 * 8 * 100 + 4/100 * 1 * 100 = 244 off, a drift of
        /// -0.04 a transaction against a spread of about 38, which settles the
        /// queue at 480 messages, or 60 batches.
        #[test]
        fn the_mix_drains_the_queue_slowly_enough_to_keep_deliveries_full() {
            let total = MIX.iter().map(|(_, weight)| *weight).sum::<u32>() as f64;
            let send = weight_of(MixKind::Send) / total;
            let deliver = weight_of(MixKind::Deliver) / total;
            let skip = weight_of(MixKind::Skip) / total;

            for msgs_per_txn in 1..=MAX_MSGS {
                let batch = msgs_per_txn as f64;
                let skipped = SKIP_PER_TXN as f64;
                // Messages one transaction puts on a channel's queue: a send
                // adds a batch, a delivery takes one off, a skip takes
                // `SKIP_PER_TXN` off, and the rest of the mix leaves it alone.
                let produced = send * batch;
                let consumed = deliver * batch + skip * skipped;
                assert!(
                    consumed > produced,
                    "the mix puts {produced} messages a transaction on a channel and \
                     takes {consumed} off, so the queue grows for the length of a run"
                );

                let drift = consumed - produced;
                let spread =
                    send * batch * batch + deliver * batch * batch + skip * skipped * skipped
                        - drift * drift;
                // Mean depth of a random walk with this drift and spread,
                // reflected at an empty queue.
                let depth = spread / (2.0 * drift);
                assert!(
                    depth >= QUEUE_DEPTH_IN_BATCHES * batch,
                    "with batches of {batch} the queue settles at {depth} messages, \
                     under the {QUEUE_DEPTH_IN_BATCHES} batches a delivery needs to \
                     find a full one waiting"
                );
            }
        }
    }
}

mod airdrop_fanout {
    use super::*;
    use aptos_sdk::crypto::hash::HashValue;

    pub const PACKAGE_NAME: &str = "airdrop_fanout";

    /// Symbol of the airdropped asset. The package derives its address from
    /// this, so both sides have to agree on it.
    const SYMBOL: &[u8] = b"ADROP";
    const DECIMALS: u8 = 8;

    /// Recipients a single `seed_recipients` transaction creates. A whole
    /// shard at once runs past the per-transaction execution limit.
    const SEED_CHUNK: usize = 64;

    /// Widest fan-out the package will walk in one transaction. Passing more
    /// is not an error, they are simply ignored.
    const MAX_RECIPIENTS: usize = 256;

    /// Balance a seeded slot starts with, and so what a claim pays out.
    const SEED_AMOUNT: u64 = 1_000_000;

    /// Moved per recipient by a distribution or a fungible-asset fan-out.
    /// Small enough that a long run cannot overflow a slot.
    const DISTRIBUTE_AMOUNT: u64 = 1_000;

    /// Faucetted at onboarding, so the first fan-outs run without topping up.
    const ONBOARD_FUNDING: u64 = 1_000_000_000_000;

    /// Fills the memo. Any byte works; a recognisable one makes a dumped
    /// write set readable.
    const MEMO_BYTE: u8 = 0xAD;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub n_shards: usize,
        pub recipients_per_txn: usize,
        pub warm_recipients: usize,
        pub payload_len: usize,
        pub write_every: u64,
        pub fresh_ratio: u32,
    }

    impl Config {
        pub fn new(
            n_shards: usize,
            recipients_per_txn: usize,
            warm_recipients: usize,
            payload_len: usize,
            write_every: u64,
            fresh_ratio: u32,
        ) -> Self {
            assert!(n_shards > 0, "the registry needs at least one shard");
            assert!(
                recipients_per_txn > 0 && recipients_per_txn <= MAX_RECIPIENTS,
                "the package walks at most {MAX_RECIPIENTS} recipients per transaction"
            );
            assert!(
                warm_recipients >= recipients_per_txn,
                "a warm batch is drawn from the seeded slots, so there have to be at least \
                 as many of those as a batch is wide"
            );
            assert!(
                write_every > 0,
                "a zero stride writes every slot, which the distribute branch already measures"
            );
            assert!(fresh_ratio <= 100, "fresh_ratio is a percentage");
            Self {
                n_shards,
                recipients_per_txn,
                warm_recipients,
                payload_len,
                write_every,
                fresh_ratio,
            }
        }

        /// Recipients that already hold a slot, starting at `start` and
        /// wrapping. Two accounts on one shard draw from the same pool, so
        /// their batches overlap and produce modification writes.
        fn warm_batch(&self, shard: usize, start: usize) -> Vec<AccountAddress> {
            (0..self.recipients_per_txn)
                .map(|k| warm_recipient(shard, (start + k) % self.warm_recipients))
                .collect::<Vec<AccountAddress>>()
        }

        /// A batch that is `fresh_ratio` percent addresses nothing has paid
        /// before, which is what a campaign's first pass over a cohort looks
        /// like. The rest are warm, so the branch is not purely creations.
        fn fresh_batch(
            &self,
            account: &LocalAccount,
            shard: usize,
            start: usize,
            counter: u64,
        ) -> Vec<AccountAddress> {
            let fresh = self.recipients_per_txn * self.fresh_ratio as usize / 100;
            (0..self.recipients_per_txn)
                .map(|k| {
                    if k < fresh {
                        fresh_recipient(account, counter.wrapping_add(k as u64))
                    } else {
                        warm_recipient(shard, (start + k) % self.warm_recipients)
                    }
                })
                .collect::<Vec<AccountAddress>>()
        }

        /// The batch a distribute branch sends: `batch` with its last
        /// recipient replaced by the caller. Every benchmark account is a
        /// campaign participant as well as a distributor, and this is the only
        /// thing in the mix that credits an address a benchmark account can
        /// sign for, so without it no claim past the onboarding one would find
        /// a balance.
        fn with_own_slot(
            &self,
            account: &LocalAccount,
            mut batch: Vec<AccountAddress>,
        ) -> Vec<AccountAddress> {
            if let Some(last) = batch.last_mut() {
                *last = account.address();
            }
            batch
        }

        fn memo(&self) -> Vec<u8> {
            vec![MEMO_BYTE; self.payload_len]
        }
    }

    /// Warm recipient `index` of `shard`. Both the seeding stage and the mix
    /// recompute these from nothing but the two indices, so they agree with no
    /// shared state.
    fn warm_recipient(shard: usize, index: usize) -> AccountAddress {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[16..24].copy_from_slice(&(shard as u64 + 1).to_be_bytes());
        bytes[24..32].copy_from_slice(&(index as u64).to_be_bytes());
        AccountAddress::new(bytes)
    }

    /// An address nothing has paid before, so crediting it creates a slot
    /// rather than modifying one.
    fn fresh_recipient(account: &LocalAccount, counter: u64) -> AccountAddress {
        let mut input = bcs::to_bytes(&account.address()).unwrap();
        input.extend_from_slice(&counter.to_le_bytes());
        AccountAddress::from_bytes(HashValue::sha3_256_of(&input).to_vec()).unwrap()
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("ad_assets")
    }

    fn distributor(package: &Package) -> ModuleId {
        package.get_module_id("ad_distributor")
    }

    /// Stage 1: create the asset and the shards from the publisher, seed the
    /// warm recipients, then give each account one transaction that funds it
    /// and opens a slot of its own.
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
            let mut payloads = vec![
                get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(SYMBOL).unwrap(),
                        bcs::to_bytes(&DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    distributor(package),
                    ident_str!("initialize").to_owned(),
                    vec![bcs::to_bytes(&(config.n_shards as u64)).unwrap()],
                ),
            ];

            for shard in 0..config.n_shards {
                let mut seeded = 0;
                while seeded < config.warm_recipients {
                    let end = (seeded + SEED_CHUNK).min(config.warm_recipients);
                    let recipients = (seeded..end)
                        .map(|index| warm_recipient(shard, index))
                        .collect::<Vec<AccountAddress>>();
                    payloads.push(get_payload(
                        distributor(package),
                        ident_str!("seed_recipients").to_owned(),
                        vec![
                            bcs::to_bytes(&(shard as u64)).unwrap(),
                            bcs::to_bytes(&recipients).unwrap(),
                            bcs::to_bytes(&SEED_AMOUNT).unwrap(),
                        ],
                    ));
                    seeded = end;
                }
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
                let shard = slot_for(account, config.n_shards);
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        distributor(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&(shard as u64)).unwrap(),
                            bcs::to_bytes(&ONBOARD_FUNDING).unwrap(),
                            bcs::to_bytes(&SEED_AMOUNT).unwrap(),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        DistributeUpdate,
        DistributeFresh,
        FaFanout,
        TouchNoop,
        Claim,
        SweepRead,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. Distribution
    /// dominates because a campaign is mostly batch payouts, and the split
    /// between the update and fresh branches is what separates modification
    /// writes from creation writes. Claims sit far below the 56 weight of the
    /// two distribute branches that credit the caller, so 56/64 of them find a
    /// credited slot and pay out.
    const MIX: [(MixKind, u32); 6] = [
        (MixKind::DistributeUpdate, 34),
        (MixKind::DistributeFresh, 22),
        (MixKind::FaFanout, 18),
        (MixKind::TouchNoop, 14),
        (MixKind::Claim, 8),
        (MixKind::SweepRead, 4),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let shard = slot_for(account, config.n_shards);
            let start = rng.gen_range(0, config.warm_recipients);
            let shard_arg = bcs::to_bytes(&(shard as u64)).unwrap();

            let (func, args) = match MIX[dist.sample(rng)].0 {
                // Both distribution branches call one entry point: which
                // addresses the batch holds is the only difference between
                // modifying a slot and creating one.
                MixKind::DistributeUpdate => {
                    let batch = config.with_own_slot(account, config.warm_batch(shard, start));
                    (ident_str!("bench_distribute"), vec![
                        shard_arg,
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&DISTRIBUTE_AMOUNT).unwrap(),
                        bcs::to_bytes(&config.memo()).unwrap(),
                    ])
                },
                MixKind::DistributeFresh => {
                    // Drawing the counter from the worker's own rng is what
                    // keeps the addresses unseen without any state shared
                    // between transactions.
                    let counter = rng.gen_range(0u64, u64::MAX);
                    let batch = config
                        .with_own_slot(account, config.fresh_batch(account, shard, start, counter));
                    (ident_str!("bench_distribute"), vec![
                        shard_arg,
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&DISTRIBUTE_AMOUNT).unwrap(),
                        bcs::to_bytes(&config.memo()).unwrap(),
                    ])
                },
                MixKind::FaFanout => (ident_str!("bench_fa_fanout"), vec![
                    shard_arg,
                    bcs::to_bytes(&config.warm_batch(shard, start)).unwrap(),
                    bcs::to_bytes(&DISTRIBUTE_AMOUNT).unwrap(),
                ]),
                MixKind::TouchNoop => (ident_str!("bench_touch"), vec![
                    shard_arg,
                    bcs::to_bytes(&config.warm_batch(shard, start)).unwrap(),
                    bcs::to_bytes(&config.write_every).unwrap(),
                ]),
                MixKind::Claim => (ident_str!("bench_claim"), vec![shard_arg]),
                MixKind::SweepRead => (ident_str!("bench_sweep"), vec![
                    shard_arg,
                    bcs::to_bytes(&config.warm_batch(shard, start)).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(distributor(package), func.to_owned(), args),
            ))
        })
    }
}

mod oracle_batch {
    use super::*;
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
        secp256k1_ecdsa,
        traits::{signing_message, PrivateKey, Signature, SigningKey, Uniform},
    };
    use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
    use rand::SeedableRng;
    use serde::{Deserialize, Serialize};

    pub const PACKAGE_NAME: &str = "oracle_batch";

    /// Wire width of one report. The Move decoder reads the same layout by
    /// hand, so the two sides have to agree byte for byte.
    const RECORD_LEN: usize = 48;

    /// Reports in one batch. More than this and a single update runs past the
    /// per-transaction execution limit.
    const MAX_FEEDS_PER_TXN: u64 = 32;

    /// Feeds one `create_feeds` transaction appends. The whole feed set at once
    /// runs past the same limit.
    const SEED_BATCH: u64 = 64;

    /// Prices are far below the registry's cap, so a decoded report lands
    /// unchanged and a run stays comparable across VMs.
    const BASE_PRICE: u128 = 100_000_000;
    const PRICE_STEP: u128 = 7;
    const BASE_TS: u64 = 1_700_000_000;

    /// Seed of the key material and the report pool. Fixed, so two runs sign
    /// and verify exactly the same bytes.
    const POOL_SEED: [u8; 32] = [7u8; 32];

    /// What the authorities sign. `SigningKey::sign` prepends a 32-byte domain
    /// separator and BCS-encodes the value, and the workload hands the result
    /// to Move verbatim, so on chain nothing has to be reassembled before the
    /// signature can be checked.
    #[derive(Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
    struct ReportBatch(Vec<u8>);

    /// One report batch and every authority's signature over it. Signing costs
    /// tens of microseconds, so the mix draws from a precomputed pool rather
    /// than signing per transaction.
    struct SignedBatch {
        /// The signed bytes, not the bare records.
        message: Vec<u8>,
        ed: Vec<Vec<u8>>,
        secp: Vec<Vec<u8>>,
    }

    #[derive(Clone)]
    pub struct Config {
        pub num_feeds: u64,
        pub feeds_per_txn: u64,
        pub num_signers: u64,
        pub quorum_k: u64,
        /// Share of a quorum's signatures that are ed25519; the rest are
        /// secp256k1.
        pub scheme_ratio: f64,
        pub decode_depth: u64,
        /// Authority keys as `orc_queue::initialize` wants them: 32-byte
        /// ed25519 keys, then raw 64-byte secp256k1 keys.
        pubkeys: Vec<Vec<u8>>,
        pool: Arc<Vec<SignedBatch>>,
    }

    impl Config {
        pub fn new(
            num_feeds: usize,
            feeds_per_txn: usize,
            num_signers: usize,
            quorum_k: usize,
            decode_depth: usize,
        ) -> Self {
            let feeds_per_txn = (feeds_per_txn as u64).clamp(1, MAX_FEEDS_PER_TXN);
            let num_feeds = (num_feeds as u64).max(feeds_per_txn);
            let num_signers = (num_signers as u64).max(1);
            let quorum_k = (quorum_k as u64).clamp(1, num_signers);
            let sig_pool_size = DEFAULT_SIG_POOL_SIZE;

            let mut rng = StdRng::from_seed(POOL_SEED);
            let ed_keys = (0..num_signers)
                .map(|_| Ed25519PrivateKey::generate(&mut rng))
                .collect::<Vec<Ed25519PrivateKey>>();
            let secp_keys = (0..num_signers)
                .map(|_| secp256k1_ecdsa::PrivateKey::generate(&mut rng))
                .collect::<Vec<secp256k1_ecdsa::PrivateKey>>();

            let mut pubkeys = vec![];
            for key in ed_keys.iter() {
                let bytes = Ed25519PublicKey::from(key).to_bytes().to_vec();
                assert_eq!(bytes.len(), 32, "ed25519 public key is not 32 bytes");
                pubkeys.push(bytes);
            }
            for key in secp_keys.iter() {
                // The wrapper keeps the uncompressed SEC1 prefix, which the
                // Move side never sees.
                let bytes = key.public_key().to_bytes();
                assert_eq!(bytes.len(), 65, "secp256k1 public key is not 65 bytes");
                pubkeys.push(bytes[1..].to_vec());
            }

            let pool = (0..sig_pool_size)
                .map(|slot| {
                    let batch = ReportBatch(batch_records(slot, feeds_per_txn));
                    let message = signing_message(&batch).expect("batch does not serialize");
                    let ed = ed_keys
                        .iter()
                        .map(|key| {
                            let signature = key.sign(&batch).expect("ed25519 signing failed");
                            // A pool that does not verify would leave the
                            // scalar multiplication unreached and the benchmark
                            // measuring nothing, so fail here instead.
                            signature
                                .verify_arbitrary_msg(&message, &Ed25519PublicKey::from(key))
                                .expect("precomputed ed25519 signature does not verify");
                            signature.to_bytes().to_vec()
                        })
                        .collect::<Vec<Vec<u8>>>();
                    // Signing sha3-256s the message and drops the recovery id.
                    // The Move side hashes the same bytes and guesses id zero,
                    // so what it recovers is counted rather than matched.
                    let secp = secp_keys
                        .iter()
                        .map(|key| {
                            let signature = key.sign(&batch).expect("secp256k1 signing failed");
                            let bytes = signature.to_bytes().to_vec();
                            assert_eq!(bytes.len(), 64, "secp256k1 signature is not 64 bytes");
                            bytes
                        })
                        .collect::<Vec<Vec<u8>>>();
                    SignedBatch { message, ed, secp }
                })
                .collect::<Vec<SignedBatch>>();

            Self {
                num_feeds,
                feeds_per_txn,
                num_signers,
                quorum_k,
                scheme_ratio: DEFAULT_SCHEME_RATIO,
                decode_depth: decode_depth as u64,
                pubkeys,
                pool: Arc::new(pool),
            }
        }

        /// Feed windows the accounts are spread over.
        fn num_windows(&self) -> usize {
            (self.num_feeds / self.feeds_per_txn) as usize
        }

        /// Signatures of each scheme a quorum call carries.
        fn quorum_split(&self) -> (usize, usize) {
            let ed = ((self.quorum_k as f64) * self.scheme_ratio).round() as u64;
            let ed = ed.clamp(1, self.quorum_k);
            (ed as usize, (self.quorum_k - ed) as usize)
        }
    }

    const DEFAULT_SIG_POOL_SIZE: usize = 64;
    const DEFAULT_SCHEME_RATIO: f64 = 0.6;

    fn record(feed: u64, price: u128, conf: u64, ts: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity(RECORD_LEN);
        out.extend_from_slice(&feed.to_be_bytes());
        out.extend_from_slice(&price.to_be_bytes());
        out.extend_from_slice(&conf.to_be_bytes());
        out.extend_from_slice(&ts.to_be_bytes());
        out.extend_from_slice(&[0u8; 8]);
        out
    }

    /// A report's feed id is an offset into the caller's window, not an
    /// absolute id, so one pool entry serves every account without two of them
    /// ever writing the same feed.
    fn batch_records(slot: usize, feeds_per_txn: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity(feeds_per_txn as usize * RECORD_LEN);
        for i in 0..feeds_per_txn {
            out.extend_from_slice(&record(
                i,
                BASE_PRICE + (slot as u128 * 1000 + i as u128) * PRICE_STEP,
                10 + i,
                BASE_TS + slot as u64 * 60 + i,
            ));
        }
        out
    }

    fn queue(package: &Package) -> ModuleId {
        package.get_module_id("orc_queue")
    }

    fn registry(package: &Package) -> ModuleId {
        package.get_module_id("orc_registry")
    }

    fn aggregator(package: &Package) -> ModuleId {
        package.get_module_id("orc_aggregator")
    }

    /// Stage 1: install the authority set and the feeds from the publisher,
    /// then give each account one transaction that claims a feed window.
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
            let config = &self.0;
            let mut payloads = vec![get_payload(
                queue(package),
                ident_str!("initialize").to_owned(),
                vec![
                    bcs::to_bytes(&config.pubkeys).unwrap(),
                    bcs::to_bytes(&config.quorum_k).unwrap(),
                ],
            )];

            let mut seeded = 0;
            while seeded < config.num_feeds {
                let batch = SEED_BATCH.min(config.num_feeds - seeded);
                payloads.push(get_payload(
                    registry(package),
                    ident_str!("create_feeds").to_owned(),
                    vec![bcs::to_bytes(&batch).unwrap()],
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
            let config = self.0.clone();
            Arc::new(move |account, package, _publisher, txn_factory, _rng| {
                let feed_base =
                    slot_for(account, config.num_windows()) as u64 * config.feeds_per_txn;
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        aggregator(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![bcs::to_bytes(&feed_base).unwrap()],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        UpdateEd25519,
        UpdateSecp256k1,
        VerifyOnly,
        WriteOnly,
        Quorum,
        ReadAggregate,
    }

    /// Stage 2: the steady-state mix, with relative frequencies.
    const MIX: [(MixKind, u32); 6] = [
        (MixKind::UpdateEd25519, 34),
        (MixKind::UpdateSecp256k1, 22),
        (MixKind::VerifyOnly, 14),
        (MixKind::WriteOnly, 14),
        (MixKind::Quorum, 8),
        (MixKind::ReadAggregate, 8),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        let (ed_quorum, secp_quorum) = config.quorum_split();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let feed_base = slot_for(account, config.num_windows()) as u64 * config.feeds_per_txn;
            let primary = slot_for(account, config.num_signers as usize);
            let batch = &config.pool[rng.gen_range(0, config.pool.len())];

            let (func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::UpdateEd25519 => (ident_str!("bench_update_ed25519"), vec![
                    bcs::to_bytes(&feed_base).unwrap(),
                    bcs::to_bytes(&batch.message).unwrap(),
                    bcs::to_bytes(&vec![batch.ed[primary].clone()]).unwrap(),
                    bcs::to_bytes(&vec![primary as u64]).unwrap(),
                    bcs::to_bytes(&config.decode_depth).unwrap(),
                ]),
                MixKind::UpdateSecp256k1 => (ident_str!("bench_update_secp256k1"), vec![
                    bcs::to_bytes(&feed_base).unwrap(),
                    bcs::to_bytes(&batch.message).unwrap(),
                    bcs::to_bytes(&vec![batch.secp[primary].clone()]).unwrap(),
                    bcs::to_bytes(&config.decode_depth).unwrap(),
                ]),
                MixKind::VerifyOnly => (ident_str!("bench_verify_only"), vec![
                    bcs::to_bytes(&batch.message).unwrap(),
                    bcs::to_bytes(&vec![batch.ed[primary].clone()]).unwrap(),
                    bcs::to_bytes(&vec![primary as u64]).unwrap(),
                ]),
                MixKind::WriteOnly => (ident_str!("bench_write_only"), vec![
                    bcs::to_bytes(&feed_base).unwrap(),
                    bcs::to_bytes(&batch.message).unwrap(),
                    bcs::to_bytes(&config.decode_depth).unwrap(),
                ]),
                MixKind::Quorum => {
                    // The quorum's signers start at the account's primary and
                    // wrap, so two accounts on the same window still present
                    // different authorities.
                    let idxs = (0..ed_quorum)
                        .map(|i| ((primary + i) % config.num_signers as usize) as u64)
                        .collect::<Vec<u64>>();
                    let ed = idxs
                        .iter()
                        .map(|i| batch.ed[*i as usize].clone())
                        .collect::<Vec<Vec<u8>>>();
                    let secp = (0..secp_quorum)
                        .map(|i| batch.secp[(primary + i) % config.num_signers as usize].clone())
                        .collect::<Vec<Vec<u8>>>();
                    (ident_str!("bench_quorum"), vec![
                        bcs::to_bytes(&feed_base).unwrap(),
                        bcs::to_bytes(&batch.message).unwrap(),
                        bcs::to_bytes(&ed).unwrap(),
                        bcs::to_bytes(&idxs).unwrap(),
                        bcs::to_bytes(&secp).unwrap(),
                        bcs::to_bytes(&config.decode_depth).unwrap(),
                    ])
                },
                MixKind::ReadAggregate => (ident_str!("bench_read_aggregate"), vec![
                    bcs::to_bytes(&feed_base).unwrap(),
                    bcs::to_bytes(&config.feeds_per_txn).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(aggregator(package), func.to_owned(), args),
            ))
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use aptos_crypto::ed25519::Ed25519Signature;

        #[test]
        fn signature_pool_verifies() {
            let config = Config::new(512, 16, 8, 3, 4);
            assert_eq!(config.pool.len(), DEFAULT_SIG_POOL_SIZE);
            assert_eq!(config.pubkeys.len(), 2 * config.num_signers as usize);
            assert_eq!(config.num_windows(), 32);

            let batch = &config.pool[0];
            // A 32-byte domain separator and a BCS length sit ahead of the
            // records.
            assert_eq!(
                batch.message.len(),
                34 + config.feeds_per_txn as usize * RECORD_LEN
            );
            for signer in 0..config.num_signers as usize {
                let pubkey = Ed25519PublicKey::try_from(config.pubkeys[signer].as_slice())
                    .expect("authority key is not a valid ed25519 public key");
                let signature = Ed25519Signature::try_from(batch.ed[signer].as_slice())
                    .expect("pool entry is not a valid ed25519 signature");
                signature
                    .verify_arbitrary_msg(&batch.message, &pubkey)
                    .expect("pool entry does not verify against its authority");
                assert_eq!(
                    config.pubkeys[config.num_signers as usize + signer].len(),
                    64
                );
                assert_eq!(batch.secp[signer].len(), 64);
            }

            // A pool entry is bound to its own bytes, so it cannot be replayed
            // against another batch.
            let other = &config.pool[1];
            let pubkey = Ed25519PublicKey::try_from(config.pubkeys[0].as_slice()).unwrap();
            let signature = Ed25519Signature::try_from(batch.ed[0].as_slice()).unwrap();
            assert!(signature
                .verify_arbitrary_msg(&other.message, &pubkey)
                .is_err());
        }

        #[test]
        fn quorum_split_covers_both_schemes() {
            let config = Config::new(512, 16, 8, 3, 4);
            let (ed, secp) = config.quorum_split();
            assert_eq!(ed + secp, config.quorum_k as usize);
            assert!(ed >= 1);
        }
    }
}

mod cdp_liquidation {
    use super::*;

    pub const PACKAGE_NAME: &str = "cdp_liquidation";

    const COLL_SYMBOL: &[u8] = b"CDPC";
    const STABLE_SYMBOL: &[u8] = b"CDPS";
    const COLL_DECIMALS: u8 = 8;
    const STABLE_DECIMALS: u8 = 6;

    /// Seed of a vault's named object, before the index is appended.
    const VAULT_SEED: &[u8] = b"cdp_vault";

    /// Price the feed starts at. The package sizes every vault against its own
    /// reference price rather than the spot price, so this only picks where in
    /// the band the sawtooth begins.
    const START_PRICE: u64 = 2000;

    /// Band the package's sawtooth sweeps, and the mid-band price every ratio
    /// target in the package is measured against. Mirrors `cdp_oracle`.
    const PRICE_MIN: u64 = 1000;
    const PRICE_SPAN: u64 = 2000;
    const PRICE_MAX: u64 = PRICE_MIN + PRICE_SPAN - 1;
    const REFERENCE_PRICE: u64 = PRICE_MIN + PRICE_SPAN / 2;

    /// Ratios the package seeds its filler vaults over, in basis points against
    /// the reference price. Mirrors `cdp_sorted`. Vault `i` is seeded at
    /// `SEED_ICR_MIN_BPS + i * SEED_ICR_STEP_BPS`, so the list length decides
    /// how healthy the healthiest filler vault is.
    const SEED_ICR_MIN_BPS: u64 = 11500;
    const SEED_ICR_STEP_BPS: u64 = 5;

    /// Vaults one `seed_vaults` transaction creates. A whole list at once runs
    /// past the per-transaction execution limit.
    const SEED_BATCH: u64 = 32;

    /// Staked by the publisher, so the pool can absorb a long run of
    /// liquidations before it stops cancelling debt.
    const POOL_FUNDING: u64 = 1_000_000_000_000;

    /// Position an account is onboarded with. Well above the minimum, so an
    /// account's own vault survives the price band and the mix keeps a stable
    /// number of live positions.
    const ONBOARD_COLL: u64 = 150;
    const ONBOARD_ICR_BPS: u64 = 25_000;

    /// Largest move one adjust makes on either leg. Small against the
    /// onboarded position, so a vault drifts through the list over many
    /// transactions rather than jumping end to end.
    const ADJUST_COLL: u64 = 4;
    const ADJUST_DEBT: u64 = 20_000;

    /// Staked per deposit transaction.
    const DEPOSIT_AMOUNT: u64 = 1_000_000;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub list_length: u64,
        pub walk_limit: u64,
        pub liquidations_per_txn: u64,
        pub hint_stride: u64,
        pub mcr_bps: u64,
        pub price_step: u64,
        pub auction_lots: u64,
    }

    impl Config {
        pub fn new(
            list_length: usize,
            walk_limit: usize,
            liquidations_per_txn: usize,
            hint_stride: usize,
            mcr_bps: u64,
            price_step: u64,
            auction_lots: usize,
        ) -> Self {
            assert!(hint_stride > 0, "hint stride must divide the list");
            assert!(
                list_length >= hint_stride,
                "list is too short to hold one hint slot"
            );
            assert!(walk_limit > 0, "a zero walk limit measures nothing");
            assert!(
                liquidations_per_txn > 0,
                "a zero liquidation batch measures nothing"
            );
            // The publisher's filler vaults are seeded from this ratio upward,
            // so a minimum at or above it makes every one of them liquidatable
            // before the mix has moved the price at all.
            assert!(
                mcr_bps > 0 && mcr_bps < SEED_ICR_MIN_BPS,
                "minimum ratio leaves the seeded list unhealthy"
            );
            assert!(price_step > 0, "a zero price step never creates candidates");
            assert!(
                (PRICE_MIN..=PRICE_MAX).contains(&START_PRICE),
                "the feed clamps a start price outside its own band"
            );
            // Both ends of the liquidation loop have to meet inside the price
            // band. The healthiest seeded vault must fall below the minimum
            // ratio somewhere in the band, or the top of the list is safe
            // forever and the sweep runs out of work; since the band is fixed,
            // that is a ceiling on how far the seeded ratios run and so on the
            // list length. The riskiest one must climb above the minimum
            // somewhere in the band, or every sweep liquidates and the branch
            // never measures an empty one.
            let top_icr_bps = SEED_ICR_MIN_BPS + (list_length as u64 - 1) * SEED_ICR_STEP_BPS;
            assert!(
                PRICE_MIN * top_icr_bps / REFERENCE_PRICE < mcr_bps,
                "list is long enough that its healthiest vault never liquidates"
            );
            assert!(
                PRICE_MAX * SEED_ICR_MIN_BPS / REFERENCE_PRICE >= mcr_bps,
                "the riskiest seeded vault is a candidate across the whole band"
            );
            Self {
                list_length: list_length as u64,
                walk_limit: walk_limit as u64,
                liquidations_per_txn: liquidations_per_txn as u64,
                hint_stride: hint_stride as u64,
                mcr_bps,
                price_step,
                auction_lots: auction_lots as u64,
            }
        }

        /// The node an account starts its walks from. Both stages recompute it
        /// from the address, so start points spread over the list without any
        /// shared cursor.
        fn hint_for(&self, account: &LocalAccount, publisher: AccountAddress) -> AccountAddress {
            let slot = slot_for(account, (self.list_length / self.hint_stride) as usize) as u64;
            vault_address(publisher, slot * self.hint_stride)
        }
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("cdp_assets")
    }

    fn vault(package: &Package) -> ModuleId {
        package.get_module_id("cdp_vault")
    }

    fn sorted(package: &Package) -> ModuleId {
        package.get_module_id("cdp_sorted")
    }

    fn oracle(package: &Package) -> ModuleId {
        package.get_module_id("cdp_oracle")
    }

    fn stability(package: &Package) -> ModuleId {
        package.get_module_id("cdp_stability")
    }

    fn auction(package: &Package) -> ModuleId {
        package.get_module_id("cdp_auction")
    }

    /// Address of `owner`'s vault number `index`. The package derives it as a
    /// named object seeded with the index.
    fn vault_address(owner: AccountAddress, index: u64) -> AccountAddress {
        let mut seed = VAULT_SEED.to_vec();
        seed.extend_from_slice(&bcs::to_bytes(&index).unwrap());
        create_object_address(owner, &seed)
    }

    /// Stage 1: configure the protocol and seed the list from the publisher,
    /// then give each account one transaction that opens a funded vault.
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
            let mut payloads = vec![
                get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(COLL_SYMBOL).unwrap(),
                        bcs::to_bytes(&COLL_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(STABLE_SYMBOL).unwrap(),
                        bcs::to_bytes(&STABLE_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(vault(package), ident_str!("initialize").to_owned(), vec![
                    bcs::to_bytes(&config.mcr_bps).unwrap(),
                ]),
                get_payload(oracle(package), ident_str!("set_price").to_owned(), vec![
                    bcs::to_bytes(&START_PRICE).unwrap(),
                ]),
            ];

            // The list has to reach its full length before any account
            // onboards, otherwise the first accounts walk a short list.
            let mut seeded = 0;
            while seeded < config.list_length {
                let batch = SEED_BATCH.min(config.list_length - seeded);
                payloads.push(get_payload(
                    sorted(package),
                    ident_str!("seed_vaults").to_owned(),
                    vec![
                        bcs::to_bytes(&seeded).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                    ],
                ));
                seeded += batch;
            }

            payloads.push(get_payload(
                stability(package),
                ident_str!("fund").to_owned(),
                vec![bcs::to_bytes(&POOL_FUNDING).unwrap()],
            ));
            payloads.push(get_payload(
                auction(package),
                ident_str!("initialize").to_owned(),
                vec![bcs::to_bytes(&config.auction_lots).unwrap()],
            ));

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
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        sorted(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&ONBOARD_COLL).unwrap(),
                            bcs::to_bytes(&ONBOARD_ICR_BPS).unwrap(),
                            bcs::to_bytes(&config.hint_for(account, publisher.address())).unwrap(),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Walk,
        AdjustReinsert,
        LiquidateBatch,
        StabilityDeposit,
        PriceTick,
        AuctionStep,
        OpenClose,
    }

    /// Stage 2: the steady-state mix, with relative frequencies. The walk and
    /// the liquidation sweep dominate, since a chain of dependent resource
    /// reads is what this package exists to stress.
    const MIX: [(MixKind, u32); 7] = [
        (MixKind::Walk, 26),
        (MixKind::AdjustReinsert, 24),
        (MixKind::LiquidateBatch, 18),
        (MixKind::StabilityDeposit, 12),
        (MixKind::PriceTick, 8),
        (MixKind::AuctionStep, 8),
        (MixKind::OpenClose, 4),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, publisher, txn_factory, rng| {
            let hint = bcs::to_bytes(&config.hint_for(account, publisher.address())).unwrap();

            let (module, func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Walk => (sorted(package), ident_str!("bench_walk"), vec![
                    hint,
                    bcs::to_bytes(&config.walk_limit).unwrap(),
                ]),
                MixKind::AdjustReinsert => (sorted(package), ident_str!("bench_adjust"), vec![
                    bcs::to_bytes(&rng.gen_range(1u64, ADJUST_COLL + 1)).unwrap(),
                    bcs::to_bytes(&rng.gen_range(1u64, ADJUST_DEBT + 1)).unwrap(),
                    bcs::to_bytes(&rng.gen_bool(0.5)).unwrap(),
                    hint,
                ]),
                MixKind::LiquidateBatch => {
                    (stability(package), ident_str!("bench_liquidate"), vec![
                        bcs::to_bytes(&config.liquidations_per_txn).unwrap(),
                        bcs::to_bytes(&config.walk_limit).unwrap(),
                    ])
                },
                MixKind::StabilityDeposit => {
                    (stability(package), ident_str!("bench_deposit"), vec![
                        bcs::to_bytes(&DEPOSIT_AMOUNT).unwrap(),
                    ])
                },
                MixKind::PriceTick => (oracle(package), ident_str!("bench_price_tick"), vec![
                    bcs::to_bytes(&config.price_step).unwrap(),
                ]),
                MixKind::AuctionStep => (auction(package), ident_str!("bench_auction_step"), vec![
                    bcs::to_bytes(&config.auction_lots).unwrap(),
                ]),
                MixKind::OpenClose => (sorted(package), ident_str!("bench_close_reopen"), vec![
                    hint,
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(module, func.to_owned(), args),
            ))
        })
    }
}

mod dex_aggregator {
    use super::*;

    pub const PACKAGE_NAME: &str = "dex_aggregator";

    /// The eight benchmark assets, in the order `map_markers` binds them to
    /// `A0`..`A7`.
    const ASSET_SYMBOLS: [&[u8]; 8] = [
        b"DX0", b"DX1", b"DX2", b"DX3", b"DX4", b"DX5", b"DX6", b"DX7",
    ];
    const ASSET_DECIMALS: u8 = 6;

    /// Backend markers, in the order `dexr_backend::tag` returns them.
    const BACKENDS: [&str; 4] = ["Cpmm", "Stable", "Clmm", "Book"];

    /// Asset markers. One per benchmark asset.
    const ASSETS: [&str; 8] = ["A0", "A1", "A2", "A3", "A4", "A5", "A6", "A7"];

    /// Mode markers. Bound to nothing: they widen the instantiation and jitter
    /// which pool a hop lands on.
    const MODES: [&str; 24] = [
        "M0", "M1", "M2", "M3", "M4", "M5", "M6", "M7", "M8", "M9", "M10", "M11", "M12", "M13",
        "M14", "M15", "M16", "M17", "M18", "M19", "M20", "M21", "M22", "M23",
    ];

    /// Reserve every seeded pool starts on, per side.
    const POOL_RESERVE: u64 = 1_000_000_000_000;

    const FEE_BPS: u64 = 30;
    const STABLE_FEE_BPS: u64 = 6;
    const AMP: u64 = 100;

    /// Pools one `seed_pools` transaction creates. A whole backend at once
    /// runs past the per-transaction execution limit, and each pool creates
    /// two funded vault objects.
    const SEED_BATCH: u64 = 16;

    /// Ticks and levels a seeded venue holds, as a multiple of how many one
    /// swap is meant to cross. The slack keeps a pool from walking off its own
    /// range on every trade.
    const RANGE_SLACK: u64 = 4;

    /// Minted into every asset at onboarding. A hop tops an account back up
    /// when it runs short, so this only has to cover the common case.
    const ONBOARD_AMOUNT: u64 = 1_000_000_000_000_000;

    /// Minted into both sides of a pool by a maintenance transaction.
    const REBALANCE_AMOUNT: u64 = 1_000_000_000;

    /// Widest pool offset a route's mode markers plus its hops can add.
    /// `bench_route5` sets it: twenty-one mode markers worth at most three
    /// each sum to thirty-one over the worst starting slot, and its fifth hop
    /// adds four more. A backend with fewer pools than this wraps every route
    /// onto the same few.
    const MAX_POOL_SPAN: usize = 35;

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub pools_per_backend: u64,
        pub amount_in: u64,
        /// Share of a split route's first leg that goes to the first venue.
        pub split_bps: u64,
        /// Initialized ticks one concentrated liquidity hop crosses.
        pub tick_crossings: u64,
        /// Book levels one order book hop clears.
        pub book_depth: u64,
    }

    impl Config {
        pub fn new(
            pools_per_backend: usize,
            amount_in: u64,
            split_bps: u64,
            tick_crossings: u64,
            book_depth: u64,
        ) -> Self {
            let pools_per_backend = pools_per_backend as u64;
            assert!(
                pools_per_backend as usize > MAX_POOL_SPAN,
                "every route would wrap onto the same pools"
            );
            // Past a twentieth of the reserve a hop clamps, and the mix would
            // measure the clamp instead of the trade.
            assert!(
                amount_in > 0 && amount_in <= POOL_RESERVE / 20,
                "amount_in outside what a seeded pool accepts"
            );
            // A split that gives one venue everything leaves the other leg at
            // zero, and the second half of the route measures nothing.
            assert!(
                split_bps > 0 && split_bps < 10_000,
                "split_bps has to leave both venues a leg"
            );
            // A hop sized to cross fewer ticks than it has pays for a walk it
            // never takes; one that crosses more than the seeded range walks
            // off the end and refills on every trade.
            assert!(
                tick_crossings > 0 && tick_crossings <= amount_in,
                "tick_crossings outside what one hop can walk"
            );
            assert!(
                book_depth > 0 && book_depth <= amount_in,
                "book_depth outside what one hop can clear"
            );
            Self {
                pools_per_backend,
                amount_in,
                split_bps,
                tick_crossings,
                book_depth,
            }
        }

        /// Liquidity per tick, sized so one hop crosses `tick_crossings` of
        /// them.
        fn tick_liquidity(&self) -> u64 {
            (self.amount_in / self.tick_crossings).max(1)
        }

        /// Resting size per level, sized so one hop clears `book_depth` of
        /// them.
        fn level_size(&self) -> u64 {
            (self.amount_in / self.book_depth).max(1)
        }

        fn n_ticks(&self) -> u64 {
            self.tick_crossings * RANGE_SLACK
        }

        fn depth(&self) -> u64 {
            self.book_depth * RANGE_SLACK
        }
    }

    fn assets_module(package: &Package) -> ModuleId {
        package.get_module_id("dexr_assets")
    }

    fn markers(package: &Package) -> ModuleId {
        package.get_module_id("dexr_markers")
    }

    fn router(package: &Package) -> ModuleId {
        package.get_module_id("dexr_router")
    }

    fn cpmm(package: &Package) -> ModuleId {
        package.get_module_id("dexr_pool_cpmm")
    }

    fn stable(package: &Package) -> ModuleId {
        package.get_module_id("dexr_pool_stable")
    }

    fn clmm(package: &Package) -> ModuleId {
        package.get_module_id("dexr_pool_clmm")
    }

    fn book(package: &Package) -> ModuleId {
        package.get_module_id("dexr_pool_book")
    }

    /// One marker type argument. The address comes off the published package,
    /// since a run may publish under any account.
    fn marker(package: &Package, name: &str) -> TypeTag {
        let module = markers(package);
        TypeTag::Struct(Box::new(StructTag {
            address: *module.address(),
            module: module.name().to_owned(),
            name: Identifier::new(name).unwrap(),
            type_args: vec![],
        }))
    }

    /// Address byte each axis takes its slot from. `slot_for` reduces the same
    /// eight-byte tail for every modulus, which ties a smaller slot to a larger
    /// one: a backend slot taken that way is always the asset slot mod four.
    /// Reading a byte per axis leaves the three independent, so the mix reaches
    /// the whole four-by-eight-by-twenty-four space of instantiations.
    const BACKEND_BYTE: usize = 30;
    const ASSET_BYTE: usize = 31;
    const MODE_BYTE: usize = 29;

    fn byte_slot(account: &LocalAccount, byte: usize, modulus: usize) -> usize {
        account.address().into_bytes()[byte] as usize % modulus
    }

    /// Backend markers for a route's hops. Rotating from a per-account start
    /// keeps two accounts off the same venue for the same hop.
    fn backend_tys(package: &Package, start: usize, count: usize) -> Vec<TypeTag> {
        (0..count)
            .map(|i| marker(package, BACKENDS[(start + i) % BACKENDS.len()]))
            .collect::<Vec<TypeTag>>()
    }

    fn asset_tys(package: &Package, start: usize, count: usize) -> Vec<TypeTag> {
        (0..count)
            .map(|i| marker(package, ASSETS[(start + i) % ASSETS.len()]))
            .collect::<Vec<TypeTag>>()
    }

    fn mode_tys(package: &Package, start: usize, count: usize) -> Vec<TypeTag> {
        (0..count)
            .map(|i| marker(package, MODES[(start + i) % MODES.len()]))
            .collect::<Vec<TypeTag>>()
    }

    /// Stage 1: create the assets, stand up the four backends and seed them
    /// from the publisher, then give each account one transaction that funds
    /// it in every asset.
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
            let assets = ASSET_SYMBOLS
                .iter()
                .map(|symbol| create_object_address(publisher.address(), symbol))
                .collect::<Vec<AccountAddress>>();
            let x = assets[0];
            let y = assets[1];

            let mut payloads = ASSET_SYMBOLS
                .iter()
                .map(|symbol| {
                    get_payload(
                        assets_module(package),
                        ident_str!("create_asset_entry").to_owned(),
                        vec![
                            bcs::to_bytes(symbol).unwrap(),
                            bcs::to_bytes(&ASSET_DECIMALS).unwrap(),
                        ],
                    )
                })
                .collect::<Vec<TransactionPayload>>();

            payloads.push(get_payload(
                router(package),
                ident_str!("initialize").to_owned(),
                vec![],
            ));

            // One pool per backend family first, so pool zero of every backend
            // exists before any route runs.
            payloads.push(get_payload(
                cpmm(package),
                ident_str!("create_pool").to_owned(),
                vec![
                    bcs::to_bytes(&x).unwrap(),
                    bcs::to_bytes(&y).unwrap(),
                    bcs::to_bytes(&POOL_RESERVE).unwrap(),
                    bcs::to_bytes(&FEE_BPS).unwrap(),
                ],
            ));
            payloads.push(get_payload(
                stable(package),
                ident_str!("create_pool").to_owned(),
                vec![
                    bcs::to_bytes(&x).unwrap(),
                    bcs::to_bytes(&y).unwrap(),
                    bcs::to_bytes(&POOL_RESERVE).unwrap(),
                    bcs::to_bytes(&STABLE_FEE_BPS).unwrap(),
                    bcs::to_bytes(&AMP).unwrap(),
                ],
            ));
            payloads.push(get_payload(
                clmm(package),
                ident_str!("create_pool").to_owned(),
                vec![
                    bcs::to_bytes(&x).unwrap(),
                    bcs::to_bytes(&y).unwrap(),
                    bcs::to_bytes(&POOL_RESERVE).unwrap(),
                    bcs::to_bytes(&FEE_BPS).unwrap(),
                    bcs::to_bytes(&config.n_ticks()).unwrap(),
                    bcs::to_bytes(&config.tick_liquidity()).unwrap(),
                    bcs::to_bytes(&7u64).unwrap(),
                ],
            ));
            payloads.push(get_payload(
                book(package),
                ident_str!("create_pool").to_owned(),
                vec![
                    bcs::to_bytes(&x).unwrap(),
                    bcs::to_bytes(&y).unwrap(),
                    bcs::to_bytes(&POOL_RESERVE).unwrap(),
                    bcs::to_bytes(&FEE_BPS).unwrap(),
                    bcs::to_bytes(&config.depth()).unwrap(),
                    bcs::to_bytes(&config.level_size()).unwrap(),
                    bcs::to_bytes(&11u64).unwrap(),
                ],
            ));

            let mut seeded = 1;
            while seeded < config.pools_per_backend {
                let batch = SEED_BATCH.min(config.pools_per_backend - seeded);
                payloads.push(get_payload(
                    cpmm(package),
                    ident_str!("seed_pools").to_owned(),
                    vec![
                        bcs::to_bytes(&assets).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&POOL_RESERVE).unwrap(),
                        bcs::to_bytes(&FEE_BPS).unwrap(),
                        bcs::to_bytes(&(1 + seeded)).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    stable(package),
                    ident_str!("seed_pools").to_owned(),
                    vec![
                        bcs::to_bytes(&assets).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&POOL_RESERVE).unwrap(),
                        bcs::to_bytes(&STABLE_FEE_BPS).unwrap(),
                        bcs::to_bytes(&AMP).unwrap(),
                        bcs::to_bytes(&(2 + seeded)).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    clmm(package),
                    ident_str!("seed_pools").to_owned(),
                    vec![
                        bcs::to_bytes(&assets).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&POOL_RESERVE).unwrap(),
                        bcs::to_bytes(&FEE_BPS).unwrap(),
                        bcs::to_bytes(&config.n_ticks()).unwrap(),
                        bcs::to_bytes(&config.tick_liquidity()).unwrap(),
                        bcs::to_bytes(&(3 + seeded)).unwrap(),
                    ],
                ));
                payloads.push(get_payload(
                    book(package),
                    ident_str!("seed_pools").to_owned(),
                    vec![
                        bcs::to_bytes(&assets).unwrap(),
                        bcs::to_bytes(&batch).unwrap(),
                        bcs::to_bytes(&POOL_RESERVE).unwrap(),
                        bcs::to_bytes(&FEE_BPS).unwrap(),
                        bcs::to_bytes(&config.depth()).unwrap(),
                        bcs::to_bytes(&config.level_size()).unwrap(),
                        bcs::to_bytes(&(4 + seeded)).unwrap(),
                    ],
                ));
                seeded += batch;
            }

            payloads.push(get_payload(
                router(package),
                ident_str!("map_markers").to_owned(),
                vec![bcs::to_bytes(&assets).unwrap()],
            ));

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
                        router(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![bcs::to_bytes(&ONBOARD_AMOUNT).unwrap()],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Route1,
        Route2,
        Route3,
        Route5,
        Split,
        Quote,
        Rebalance,
    }

    /// Stage 2: the steady-state mix, with relative frequencies.
    const MIX: [(MixKind, u32); 7] = [
        (MixKind::Route1, 22),
        (MixKind::Route2, 22),
        (MixKind::Route3, 18),
        (MixKind::Route5, 16),
        (MixKind::Split, 10),
        (MixKind::Quote, 8),
        (MixKind::Rebalance, 4),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            // Recomputed from the address rather than stored, so the mix needs
            // no state shared across workers.
            let backend_slot = byte_slot(account, BACKEND_BYTE, BACKENDS.len());
            let asset_slot = byte_slot(account, ASSET_BYTE, ASSETS.len());
            let mode_slot = byte_slot(account, MODE_BYTE, MODES.len());
            let pool_id = rng.gen_range(0u64, config.pools_per_backend);

            let pool_arg = bcs::to_bytes(&pool_id).unwrap();
            let amount_arg = bcs::to_bytes(&config.amount_in).unwrap();

            let (func, ty_args, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Route1 => (
                    ident_str!("bench_route1"),
                    [
                        backend_tys(package, backend_slot, 1),
                        asset_tys(package, asset_slot, 2),
                        mode_tys(package, mode_slot, 1),
                    ]
                    .concat(),
                    vec![pool_arg, amount_arg],
                ),
                MixKind::Route2 => (
                    ident_str!("bench_route2"),
                    [
                        backend_tys(package, backend_slot, 2),
                        asset_tys(package, asset_slot, 3),
                        mode_tys(package, mode_slot, 3),
                    ]
                    .concat(),
                    vec![pool_arg, amount_arg],
                ),
                MixKind::Route3 => (
                    ident_str!("bench_route3"),
                    [
                        backend_tys(package, backend_slot, 3),
                        asset_tys(package, asset_slot, 4),
                        mode_tys(package, mode_slot, 9),
                    ]
                    .concat(),
                    vec![pool_arg, amount_arg],
                ),
                MixKind::Route5 => (
                    ident_str!("bench_route5"),
                    [
                        backend_tys(package, backend_slot, 5),
                        asset_tys(package, asset_slot, 6),
                        mode_tys(package, mode_slot, 21),
                    ]
                    .concat(),
                    vec![pool_arg, amount_arg],
                ),
                MixKind::Split => (
                    ident_str!("bench_split"),
                    [
                        backend_tys(package, backend_slot, 4),
                        asset_tys(package, asset_slot, 3),
                        mode_tys(package, mode_slot, 9),
                    ]
                    .concat(),
                    vec![
                        pool_arg,
                        amount_arg,
                        bcs::to_bytes(&config.split_bps).unwrap(),
                    ],
                ),
                MixKind::Quote => (
                    ident_str!("bench_quote"),
                    [
                        backend_tys(package, backend_slot, 3),
                        asset_tys(package, asset_slot, 4),
                        mode_tys(package, mode_slot, 9),
                    ]
                    .concat(),
                    vec![pool_arg, amount_arg],
                ),
                MixKind::Rebalance => (ident_str!("bench_rebalance"), vec![], vec![
                    bcs::to_bytes(&(backend_slot as u8)).unwrap(),
                    pool_arg,
                    bcs::to_bytes(&REBALANCE_AMOUNT).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload_ty(router(package), func.to_owned(), ty_args, args),
            ))
        })
    }
}

mod nft_mint_market {
    use super::*;

    pub const PACKAGE_NAME: &str = "nft_mint_market";

    /// Symbol of the payment asset. The package derives its metadata address
    /// from this, so it has to match `nftm_assets::PAYMENT_SYMBOL`.
    const PAYMENT_SYMBOL: &[u8] = b"NFTM";
    const PAYMENT_DECIMALS: u8 = 8;

    /// Tokens or listings one setup transaction handles. A whole collection at
    /// once runs past the per-transaction execution limit.
    const SEED_BATCH: u64 = 32;

    /// Headroom the collection's fixed supply gets over what the publisher
    /// seeds. Mints in the mix come out of it, and a cap that a long run
    /// reaches turns every later mint into a no-op.
    const SUPPLY_HEADROOM: u64 = 1024;

    /// Inventory entries an account keeps, mirroring
    /// `nftm_token::INVENTORY_CAP`. Hints into it are drawn from this range.
    const INVENTORY_CAP: u64 = 8;

    /// Listings the book holds at once, mirroring
    /// `nftm_listing::LISTING_RING_CAP`. A hint into the book is drawn from
    /// this range, so a buy can land on any standing ask rather than only on
    /// the first few thousand slots.
    const LISTING_RING_CAP: u64 = 4096;

    /// Payment balance an account is onboarded with. Entry points that spend
    /// top an account back up, so this only has to cover the first few
    /// transactions.
    const ONBOARD_PAYMENT_PRICES: u64 = 256;

    /// URI the publisher rewrites each collection to once setup is done, which
    /// exercises the mutator ref every mint of the collection reads through.
    const COLLECTION_URI: &[u8] = b"https://bench.invalid/collection/live";

    #[derive(Debug, Copy, Clone)]
    pub struct Config {
        pub num_collections: u64,
        pub tokens_per_collection: u64,
        pub tokens_per_account: u64,
        pub tokens_read_per_txn: u64,
        pub price: u64,
        pub commission_bps: u64,
        pub royalty_bps: u64,
        pub mint_batch: u64,
    }

    impl Config {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            num_collections: usize,
            tokens_per_collection: usize,
            tokens_per_account: usize,
            tokens_read_per_txn: usize,
            price: u64,
            commission_bps: u64,
            royalty_bps: u64,
            mint_batch: usize,
        ) -> Self {
            assert!(
                num_collections > 0,
                "the mix needs a collection to draw from"
            );
            assert!(
                tokens_per_collection > 0,
                "a derived token index has to land on a seeded token"
            );
            assert!(
                tokens_per_account > 0,
                "an account with no tokens can neither list nor accept an offer"
            );
            assert!(
                tokens_read_per_txn > 0,
                "a read that touches nothing measures nothing"
            );
            assert!(
                price > 0,
                "a zero price makes every fee split trivially zero"
            );
            // `nftm_fees::init_schedule` refuses a schedule that would take
            // more than the whole price.
            assert!(
                commission_bps + royalty_bps <= 10_000,
                "commission and royalty together exceed the price"
            );
            assert!(mint_batch > 0, "a zero-sized mint does nothing");
            Self {
                num_collections: num_collections as u64,
                tokens_per_collection: tokens_per_collection as u64,
                tokens_per_account: tokens_per_account as u64,
                tokens_read_per_txn: tokens_read_per_txn as u64,
                price,
                commission_bps,
                royalty_bps,
                mint_batch: mint_batch as u64,
            }
        }

        fn supply_cap(&self) -> u64 {
            self.tokens_per_collection * SUPPLY_HEADROOM
        }

        fn onboard_payment(&self) -> u64 {
            self.price * ONBOARD_PAYMENT_PRICES
        }
    }

    fn assets(package: &Package) -> ModuleId {
        package.get_module_id("nftm_assets")
    }

    fn collection(package: &Package) -> ModuleId {
        package.get_module_id("nftm_collection")
    }

    fn fees(package: &Package) -> ModuleId {
        package.get_module_id("nftm_fees")
    }

    fn listing(package: &Package) -> ModuleId {
        package.get_module_id("nftm_listing")
    }

    fn offer(package: &Package) -> ModuleId {
        package.get_module_id("nftm_offer")
    }

    fn token(package: &Package) -> ModuleId {
        package.get_module_id("nftm_token")
    }

    /// Stage 1: create the payment asset, the collections and the fee schedule
    /// from the publisher and seed every collection with listed tokens, then
    /// give each account one transaction that funds it, mints it an inventory
    /// and lists half of it.
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
            let mut payloads = vec![
                get_payload(
                    assets(package),
                    ident_str!("create_asset_entry").to_owned(),
                    vec![
                        bcs::to_bytes(PAYMENT_SYMBOL).unwrap(),
                        bcs::to_bytes(&PAYMENT_DECIMALS).unwrap(),
                    ],
                ),
                get_payload(
                    collection(package),
                    ident_str!("initialize").to_owned(),
                    vec![],
                ),
            ];
            for index in 0..config.num_collections {
                payloads.push(get_payload(
                    collection(package),
                    ident_str!("create_collection").to_owned(),
                    vec![
                        bcs::to_bytes(&index).unwrap(),
                        bcs::to_bytes(&config.supply_cap()).unwrap(),
                        bcs::to_bytes(&config.royalty_bps).unwrap(),
                    ],
                ));
            }
            payloads.push(get_payload(
                fees(package),
                ident_str!("init_schedule").to_owned(),
                vec![
                    bcs::to_bytes(&config.commission_bps).unwrap(),
                    bcs::to_bytes(&config.royalty_bps).unwrap(),
                ],
            ));

            for index in 0..config.num_collections {
                let mut seeded = 0;
                while seeded < config.tokens_per_collection {
                    let batch = SEED_BATCH.min(config.tokens_per_collection - seeded);
                    payloads.push(get_payload(
                        token(package),
                        ident_str!("seed_tokens").to_owned(),
                        vec![
                            bcs::to_bytes(&index).unwrap(),
                            bcs::to_bytes(&batch).unwrap(),
                        ],
                    ));
                    seeded += batch;
                }
            }
            for index in 0..config.num_collections {
                let mut listed = 0;
                while listed < config.tokens_per_collection {
                    let batch = SEED_BATCH.min(config.tokens_per_collection - listed);
                    payloads.push(get_payload(
                        listing(package),
                        ident_str!("seed_listings").to_owned(),
                        vec![
                            bcs::to_bytes(&index).unwrap(),
                            bcs::to_bytes(&listed).unwrap(),
                            bcs::to_bytes(&batch).unwrap(),
                            bcs::to_bytes(&config.price).unwrap(),
                        ],
                    ));
                    listed += batch;
                }
            }
            for index in 0..config.num_collections {
                payloads.push(get_payload(
                    collection(package),
                    ident_str!("set_collection_uri").to_owned(),
                    vec![
                        bcs::to_bytes(&index).unwrap(),
                        bcs::to_bytes(COLLECTION_URI).unwrap(),
                    ],
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
                let index = slot_for(account, config.num_collections as usize) as u64;
                Some(sign_capped(
                    account,
                    txn_factory,
                    get_payload(
                        listing(package),
                        ident_str!("bench_onboard").to_owned(),
                        vec![
                            bcs::to_bytes(&index).unwrap(),
                            bcs::to_bytes(&config.tokens_per_account).unwrap(),
                            bcs::to_bytes(&config.onboard_payment()).unwrap(),
                            bcs::to_bytes(&config.price).unwrap(),
                        ],
                    ),
                ))
            })
        }
    }

    #[derive(Debug, Copy, Clone)]
    enum MixKind {
        Buy,
        List,
        TokenOffer,
        AcceptOffer,
        CancelListing,
        Mint,
        CollectionOffer,
        ReadIndex,
    }

    /// Stage 2: the steady-state mix, with relative frequencies.
    ///
    /// The branches that add a listing outweigh the ones that take it away,
    /// twenty plus ten against ten plus ten, so the book fills to the cap the
    /// package holds it at instead of draining.
    const MIX: [(MixKind, u32); 8] = [
        (MixKind::Buy, 26),
        (MixKind::List, 20),
        (MixKind::TokenOffer, 12),
        (MixKind::AcceptOffer, 10),
        (MixKind::CancelListing, 10),
        (MixKind::Mint, 10),
        (MixKind::CollectionOffer, 6),
        (MixKind::ReadIndex, 6),
    ];

    pub fn mix_worker(config: Config) -> Arc<TransactionGeneratorWorker> {
        let dist = WeightedIndex::new(MIX.iter().map(|(_, weight)| *weight)).unwrap();
        Arc::new(move |account, package, _publisher, txn_factory, rng| {
            let index = slot_for(account, config.num_collections as usize) as u64;
            // Each account starts from its own base and walks forward, so
            // accounts spread over the collection rather than all landing on
            // the same token. An index past what the publisher seeded wraps
            // back into it inside the package.
            let base = slot_for(account, config.tokens_per_collection as usize) as u64;
            let token_index = base + rng.gen_range(0, config.tokens_per_collection);
            let inventory_hint = rng.gen_range(0, INVENTORY_CAP);
            let listing_hint = rng.gen_range(0, LISTING_RING_CAP);
            let offer_hint = rng.gen_range(0, LISTING_RING_CAP);
            // A collection offer is keyed by bidder and collection both, so
            // drawing the collection per transaction rather than per account
            // opens the book up from one row per account to all of them.
            let offer_collection = rng.gen_range(0, config.num_collections);

            let (module, func, args) = match MIX[dist.sample(rng)].0 {
                MixKind::Buy => (listing(package), ident_str!("bench_buy"), vec![
                    bcs::to_bytes(&index).unwrap(),
                    bcs::to_bytes(&token_index).unwrap(),
                    bcs::to_bytes(&listing_hint).unwrap(),
                ]),
                MixKind::List => (listing(package), ident_str!("bench_list"), vec![
                    bcs::to_bytes(&index).unwrap(),
                    bcs::to_bytes(&token_index).unwrap(),
                    bcs::to_bytes(&inventory_hint).unwrap(),
                    bcs::to_bytes(&config.price).unwrap(),
                ]),
                MixKind::TokenOffer => (offer(package), ident_str!("bench_token_offer"), vec![
                    bcs::to_bytes(&index).unwrap(),
                    bcs::to_bytes(&token_index).unwrap(),
                    bcs::to_bytes(&config.price).unwrap(),
                ]),
                MixKind::AcceptOffer => (offer(package), ident_str!("bench_accept_offer"), vec![
                    bcs::to_bytes(&inventory_hint).unwrap(),
                    bcs::to_bytes(&offer_hint).unwrap(),
                ]),
                MixKind::CancelListing => {
                    (listing(package), ident_str!("bench_cancel_listing"), vec![
                        bcs::to_bytes(&inventory_hint).unwrap(),
                    ])
                },
                MixKind::Mint => (listing(package), ident_str!("bench_mint_and_list"), vec![
                    bcs::to_bytes(&index).unwrap(),
                    bcs::to_bytes(&config.mint_batch).unwrap(),
                    bcs::to_bytes(&config.price).unwrap(),
                ]),
                MixKind::CollectionOffer => {
                    (offer(package), ident_str!("bench_collection_offer"), vec![
                        bcs::to_bytes(&offer_collection).unwrap(),
                        bcs::to_bytes(&config.price).unwrap(),
                    ])
                },
                MixKind::ReadIndex => (token(package), ident_str!("bench_read_index"), vec![
                    bcs::to_bytes(&index).unwrap(),
                    bcs::to_bytes(&token_index).unwrap(),
                    bcs::to_bytes(&config.tokens_read_per_txn).unwrap(),
                ]),
            };

            Some(sign_capped(
                account,
                txn_factory,
                get_payload(module, func.to_owned(), args),
            ))
        })
    }
}
