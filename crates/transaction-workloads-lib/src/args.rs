// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_workloads::{LoopType, PreBuiltPackagesImpl},
    token_workflow::TokenWorkflowKind,
    EntryPoints, OrderBookState,
};
use aptos_transaction_generator_lib::{TransactionType, WorkflowProgress};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

/// Utility class for specifying transaction type with predefined configurations through CLI
#[derive(Debug, Copy, Clone, ValueEnum, Default, Deserialize, Parser, Serialize)]
pub enum TransactionTypeArg {
    // custom
    #[default]
    CoinTransfer,
    AptFaTransfer,
    CoinTransferWithInvalid,
    NonConflictingCoinTransfer,
    AccountGeneration,
    AccountGenerationLargePool,
    Batch100Transfer,
    PublishPackage,
    RepublishAndCall,
    // Simple EntryPoints
    NoOp,
    NoOpFeePayer,
    NoOp2Signers,
    NoOp5Signers,
    AccountResource32B,
    AccountResource1KB,
    AccountResource10KB,
    ModifyGlobalResource,
    Loop100k,
    Loop10kArithmetic,
    Loop1kBcs1k,
    ModifyGlobalResourceAggV2,
    ModifyGlobalFlagAggV2,
    ModifyGlobalBoundedAggV2,
    ModifyGlobalMilestoneAggV2,
    // Complex EntryPoints
    CreateObjects10,
    CreateObjects10WithPayload10k,
    CreateObjectsConflict10WithPayload10k,
    CreateObjects100,
    CreateObjects100WithPayload10k,
    CreateObjectsConflict100WithPayload10k,
    VectorTrimAppendLen3000Size1,
    VectorRemoveInsertLen3000Size1,
    ResourceGroupsGlobalWriteTag1KB,
    ResourceGroupsGlobalWriteAndReadTag1KB,
    ResourceGroupsSenderWriteTag1KB,
    ResourceGroupsSenderMultiChange1KB,
    TokenV1NFTMintAndStoreSequential,
    TokenV1NFTMintAndTransferSequential,
    TokenV1NFTMintAndStoreParallel,
    TokenV1NFTMintAndTransferParallel,
    TokenV1FTMintAndStore,
    TokenV1FTMintAndTransfer,
    // register if not registered already
    CoinInitAndMint,
    FungibleAssetMint,
    TokenV2AmbassadorMint,
    TokenV2AmbassadorMintAndBurn1M,
    LiquidityPoolSwap,
    LiquidityPoolSwapStable,
    VectorPictureCreate30k,
    VectorPicture30k,
    VectorPictureRead30k,
    VectorPictureCreate40,
    VectorPicture40,
    VectorPictureRead40,
    SmartTablePicture30KWith200Change,
    SmartTablePicture1MWith256Change,
    SmartTablePicture1BWith256Change,
    SmartTablePicture1MWith1KChangeExceedsLimit,
    DeserializeU256,
    SimpleScript,
    APTTransferWithPermissionedSigner,
    APTTransferWithMasterSigner,
    /// Basic market where sell and buy prices are in distinct ranges,
    /// and there are no matches.
    OrderBookNoMatches1Market,
    OrderBookNoMatches50Markets,
    /// Basic market where sell and buy prices are in distinct ranges,
    /// and there are no matches.
    /// Basic market, 25% of orders are in the "overlap" interval.
    /// Half of the orders are buys, half sells, and both have same size ranges.
    OrderBookBalancedMatches25Pct1Market,
    OrderBookBalancedMatches25Pct50Markets,
    /// Basic market, 80% of orders are in the "overlap" interval.
    /// Half of the orders are buys, half sells, and both have same size ranges.
    OrderBookBalancedMatches80Pct1Market,
    OrderBookBalancedMatches80Pct50Markets,
    /// Basic market, 80% of orders are in the "overlap" interval.
    /// Sells are 99 times smaller, but are 99 times more frequent than buys.
    /// That means we will match rarely, but single match will be creating ~100 positions
    OrderBookBalancedSizeSkewed80Pct1Market,
    OrderBookBalancedSizeSkewed80Pct50Markets,
    /// Dependency chain of 8 modules. Only top-level module is used.
    DependencyChain8,
    /// Dependency chain of 64 modules. Only top-level module is used.
    DependencyChain64,
    /// Dependency chain of 256 modules. Only top-level module is used.
    DependencyChain256,
    /// Dependency chain of 512 modules. Only top-level module is used.
    DependencyChain512,
    /// Dependency chain of 512 modules. First 64 modules are used.
    DependencyChain512Depth64,
    /// Dependency chain of 512 modules. First 128 modules are used.
    DependencyChain512Depth128,
    /// Dependency chain of 512 modules. First 256 modules are used.
    DependencyChain512Depth256,
    /// Dependency chain of 512 modules. All modules are used.
    DependencyChain512Depth512,
    /// Tree of dependencies (81 nodes), with all modules being used. Each node has 3 children.
    DependencyTree81With3ChildrenPerLeaf,
    /// Tree of dependencies (585 nodes), with all modules being used. Each node has 8 children.
    DependencyTree585With8ChildrenPerLeaf,
    /// Star graph with 32 leafs. Only top-level module is used.
    DependencyStar32Width0,
    /// Star graph with 512 leafs. Only top-level module is used.
    DependencyStar512Width0,
    /// Star graph with 512 leafs. Only 64 modules are used.
    DependencyStar512Width64,
    /// Star graph with 512 leafs. Only 128 modules are used.
    DependencyStar512Width128,
    /// Star graph with 512 leafs. Only 256 modules are used.
    DependencyStar512Width256,
    /// Star graph with 512 leafs. All modules are used.
    DependencyStar512Width512,
    /// Random sparse DAG on 64 nodes. Only top-level module is used.
    DependencyDag64Sparse,
    /// Random sparse DAG on 256 nodes. Only top-level module is used.
    DependencyDag256Sparse,
    /// Quasi-clique on 64 nodes. Only top-level module is used.
    DependencyDag64Dense,
    /// Quasi-clique on 256 nodes. Only top-level module is used.
    DependencyDag256Dense,
}

impl TransactionTypeArg {
    pub fn materialize_default(&self) -> TransactionType {
        self.materialize(1, false, WorkflowProgress::when_done_default())
    }

    pub fn materialize(
        &self,
        module_working_set_size: usize,
        sender_use_account_pool: bool,
        workflow_progress_type: WorkflowProgress,
    ) -> TransactionType {
        let call_custom_module = |entry_point: EntryPoints| -> TransactionType {
            TransactionType::CallCustomModules {
                entry_point: Box::new(entry_point),
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            }
        };

        match &self {
            TransactionTypeArg::CoinTransfer => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool,
                non_conflicting: false,
                use_fa_transfer: false,
            },
            TransactionTypeArg::AptFaTransfer => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool,
                non_conflicting: false,
                use_fa_transfer: true,
            },
            TransactionTypeArg::NonConflictingCoinTransfer => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool,
                non_conflicting: true,
                use_fa_transfer: false,
            },
            TransactionTypeArg::CoinTransferWithInvalid => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 10,
                sender_use_account_pool,
                non_conflicting: false,
                use_fa_transfer: false,
            },
            TransactionTypeArg::AccountGeneration => TransactionType::AccountGeneration {
                add_created_accounts_to_pool: true,
                max_account_working_set: 1_000_000,
                creation_balance: 0,
            },
            TransactionTypeArg::AccountGenerationLargePool => TransactionType::AccountGeneration {
                add_created_accounts_to_pool: true,
                max_account_working_set: 50_000_000,
                creation_balance: 200_000_000,
            },
            TransactionTypeArg::PublishPackage => TransactionType::PublishPackage {
                use_account_pool: sender_use_account_pool,
                pre_built: &PreBuiltPackagesImpl,
                package_name: "simple".to_string(),
            },
            TransactionTypeArg::Batch100Transfer => {
                TransactionType::BatchTransfer { batch_size: 100 }
            },
            TransactionTypeArg::AccountResource32B => {
                call_custom_module(EntryPoints::BytesMakeOrChange {
                    data_length: Some(32),
                })
            },
            TransactionTypeArg::AccountResource1KB => {
                call_custom_module(EntryPoints::BytesMakeOrChange {
                    data_length: Some(1024),
                })
            },
            TransactionTypeArg::AccountResource10KB => {
                call_custom_module(EntryPoints::BytesMakeOrChange {
                    data_length: Some(10 * 1024),
                })
            },
            TransactionTypeArg::ModifyGlobalResource => call_custom_module(EntryPoints::IncGlobal),
            TransactionTypeArg::ModifyGlobalResourceAggV2 => {
                call_custom_module(EntryPoints::IncGlobalAggV2)
            },
            TransactionTypeArg::ModifyGlobalFlagAggV2 => call_custom_module(
                // 100 is max, so equivalent to flag
                EntryPoints::ModifyGlobalBoundedAggV2 { step: 100 },
            ),
            TransactionTypeArg::ModifyGlobalBoundedAggV2 => {
                call_custom_module(EntryPoints::ModifyGlobalBoundedAggV2 { step: 10 })
            },
            TransactionTypeArg::ModifyGlobalMilestoneAggV2 => {
                call_custom_module(EntryPoints::IncGlobalMilestoneAggV2 {
                    milestone_every: 1000,
                })
            },
            TransactionTypeArg::RepublishAndCall => TransactionType::CallCustomModulesMix {
                entry_points: vec![
                    (Box::new(EntryPoints::Nop), 1),
                    (Box::new(EntryPoints::Republish), 1),
                ],
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::NoOp => call_custom_module(EntryPoints::Nop),
            TransactionTypeArg::NoOpFeePayer => call_custom_module(EntryPoints::NopFeePayer),
            TransactionTypeArg::NoOp2Signers => call_custom_module(EntryPoints::Nop),
            TransactionTypeArg::NoOp5Signers => call_custom_module(EntryPoints::Nop),
            TransactionTypeArg::Loop100k => call_custom_module(EntryPoints::Loop {
                loop_count: Some(100000),
                loop_type: LoopType::NoOp,
            }),
            TransactionTypeArg::Loop10kArithmetic => call_custom_module(EntryPoints::Loop {
                loop_count: Some(10000),
                loop_type: LoopType::Arithmetic,
            }),
            TransactionTypeArg::Loop1kBcs1k => call_custom_module(EntryPoints::Loop {
                loop_count: Some(1000),
                loop_type: LoopType::BcsToBytes { len: 1024 },
            }),
            TransactionTypeArg::CreateObjects10 => call_custom_module(EntryPoints::CreateObjects {
                num_objects: 10,
                object_payload_size: 0,
            }),
            TransactionTypeArg::CreateObjects10WithPayload10k => {
                call_custom_module(EntryPoints::CreateObjects {
                    num_objects: 10,
                    object_payload_size: 10 * 1024,
                })
            },
            TransactionTypeArg::CreateObjectsConflict10WithPayload10k => {
                call_custom_module(EntryPoints::CreateObjectsConflict {
                    num_objects: 10,
                    object_payload_size: 10 * 1024,
                })
            },
            TransactionTypeArg::CreateObjects100 => {
                call_custom_module(EntryPoints::CreateObjects {
                    num_objects: 100,
                    object_payload_size: 0,
                })
            },
            TransactionTypeArg::CreateObjects100WithPayload10k => {
                call_custom_module(EntryPoints::CreateObjects {
                    num_objects: 100,
                    object_payload_size: 10 * 1024,
                })
            },
            TransactionTypeArg::CreateObjectsConflict100WithPayload10k => {
                call_custom_module(EntryPoints::CreateObjectsConflict {
                    num_objects: 100,
                    object_payload_size: 10 * 1024,
                })
            },
            TransactionTypeArg::VectorTrimAppendLen3000Size1 => {
                call_custom_module(EntryPoints::VectorTrimAppend {
                    vec_len: 3000,
                    element_len: 1,
                    index: 100,
                    repeats: 1000,
                })
            },
            TransactionTypeArg::VectorRemoveInsertLen3000Size1 => {
                call_custom_module(EntryPoints::VectorRemoveInsert {
                    vec_len: 3000,
                    element_len: 1,
                    index: 100,
                    repeats: 1000,
                })
            },
            TransactionTypeArg::ResourceGroupsGlobalWriteTag1KB => {
                call_custom_module(EntryPoints::ResourceGroupsGlobalWriteTag {
                    string_length: 1024,
                })
            },
            TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB => {
                call_custom_module(EntryPoints::ResourceGroupsGlobalWriteAndReadTag {
                    string_length: 1024,
                })
            },
            TransactionTypeArg::ResourceGroupsSenderWriteTag1KB => {
                call_custom_module(EntryPoints::ResourceGroupsSenderWriteTag {
                    string_length: 1024,
                })
            },
            TransactionTypeArg::ResourceGroupsSenderMultiChange1KB => {
                call_custom_module(EntryPoints::ResourceGroupsSenderMultiChange {
                    string_length: 1024,
                })
            },
            TransactionTypeArg::TokenV1NFTMintAndStoreSequential => {
                call_custom_module(EntryPoints::TokenV1MintAndStoreNFTSequential)
            },
            TransactionTypeArg::TokenV1NFTMintAndTransferSequential => {
                call_custom_module(EntryPoints::TokenV1MintAndTransferNFTSequential)
            },
            TransactionTypeArg::TokenV1NFTMintAndStoreParallel => {
                call_custom_module(EntryPoints::TokenV1MintAndStoreNFTParallel)
            },
            TransactionTypeArg::TokenV1NFTMintAndTransferParallel => {
                call_custom_module(EntryPoints::TokenV1MintAndTransferNFTParallel)
            },
            TransactionTypeArg::TokenV1FTMintAndStore => {
                call_custom_module(EntryPoints::TokenV1MintAndStoreFT)
            },
            TransactionTypeArg::TokenV1FTMintAndTransfer => {
                call_custom_module(EntryPoints::TokenV1MintAndTransferFT)
            },
            TransactionTypeArg::CoinInitAndMint => call_custom_module(EntryPoints::CoinInitAndMint),
            TransactionTypeArg::FungibleAssetMint => {
                call_custom_module(EntryPoints::FungibleAssetMint)
            },
            TransactionTypeArg::TokenV2AmbassadorMint => {
                call_custom_module(EntryPoints::TokenV2AmbassadorMint { numbered: true })
            },
            TransactionTypeArg::TokenV2AmbassadorMintAndBurn1M => TransactionType::Workflow {
                workflow_kind: Box::new(TokenWorkflowKind::CreateMintBurn {
                    count: 10000,
                    creation_balance: 200000,
                }),
                num_modules: 1,
                use_account_pool: sender_use_account_pool,
                progress_type: workflow_progress_type,
            },
            TransactionTypeArg::LiquidityPoolSwap => {
                call_custom_module(EntryPoints::LiquidityPoolSwap { is_stable: false })
            },
            TransactionTypeArg::LiquidityPoolSwapStable => {
                call_custom_module(EntryPoints::LiquidityPoolSwap { is_stable: true })
            },
            TransactionTypeArg::VectorPictureCreate30k => {
                call_custom_module(EntryPoints::InitializeVectorPicture { length: 30 * 1024 })
            },
            TransactionTypeArg::VectorPicture30k => {
                call_custom_module(EntryPoints::VectorPicture { length: 30 * 1024 })
            },
            TransactionTypeArg::VectorPictureRead30k => {
                call_custom_module(EntryPoints::VectorPictureRead { length: 30 * 1024 })
            },
            TransactionTypeArg::VectorPictureCreate40 => {
                call_custom_module(EntryPoints::InitializeVectorPicture { length: 40 })
            },
            TransactionTypeArg::VectorPicture40 => {
                call_custom_module(EntryPoints::VectorPicture { length: 40 })
            },
            TransactionTypeArg::VectorPictureRead40 => {
                call_custom_module(EntryPoints::VectorPictureRead { length: 40 })
            },
            TransactionTypeArg::SmartTablePicture30KWith200Change => {
                call_custom_module(EntryPoints::SmartTablePicture {
                    length: 30 * 1024,
                    num_points_per_txn: 200,
                })
            },
            TransactionTypeArg::SmartTablePicture1MWith256Change => {
                call_custom_module(EntryPoints::SmartTablePicture {
                    length: 1024 * 1024,
                    num_points_per_txn: 256,
                })
            },
            TransactionTypeArg::SmartTablePicture1BWith256Change => {
                call_custom_module(EntryPoints::SmartTablePicture {
                    length: 1024 * 1024 * 1024,
                    num_points_per_txn: 256,
                })
            },
            TransactionTypeArg::SmartTablePicture1MWith1KChangeExceedsLimit => {
                call_custom_module(EntryPoints::SmartTablePicture {
                    length: 1024 * 1024,
                    num_points_per_txn: 1024,
                })
            },
            TransactionTypeArg::DeserializeU256 => call_custom_module(EntryPoints::DeserializeU256),
            TransactionTypeArg::SimpleScript => call_custom_module(EntryPoints::SimpleScript),
            TransactionTypeArg::APTTransferWithPermissionedSigner => {
                call_custom_module(EntryPoints::APTTransferWithPermissionedSigner)
            },
            TransactionTypeArg::APTTransferWithMasterSigner => {
                call_custom_module(EntryPoints::APTTransferWithMasterSigner)
            },
            TransactionTypeArg::OrderBookNoMatches1Market
            | TransactionTypeArg::OrderBookNoMatches50Markets => {
                call_custom_module(EntryPoints::OrderBook {
                    state: OrderBookState::new(),
                    num_markets: if let TransactionTypeArg::OrderBookNoMatches50Markets = self {
                        50
                    } else {
                        1
                    },
                    overlap_ratio: 0.0,
                    buy_frequency: 0.5,
                    max_sell_size: 1,
                    max_buy_size: 1,
                })
            },
            TransactionTypeArg::OrderBookBalancedMatches25Pct1Market
            | TransactionTypeArg::OrderBookBalancedMatches25Pct50Markets => {
                call_custom_module(EntryPoints::OrderBook {
                    state: OrderBookState::new(),
                    num_markets: if let TransactionTypeArg::OrderBookBalancedMatches25Pct50Markets =
                        self
                    {
                        50
                    } else {
                        1
                    },
                    overlap_ratio: 0.25,
                    buy_frequency: 0.5,
                    max_sell_size: 1,
                    max_buy_size: 1,
                })
            },
            TransactionTypeArg::OrderBookBalancedMatches80Pct1Market
            | TransactionTypeArg::OrderBookBalancedMatches80Pct50Markets => {
                call_custom_module(EntryPoints::OrderBook {
                    state: OrderBookState::new(),
                    num_markets: if let TransactionTypeArg::OrderBookBalancedMatches80Pct50Markets =
                        self
                    {
                        50
                    } else {
                        1
                    },
                    overlap_ratio: 0.8,
                    buy_frequency: 0.5,
                    max_sell_size: 1,
                    max_buy_size: 1,
                })
            },
            TransactionTypeArg::OrderBookBalancedSizeSkewed80Pct1Market
            | TransactionTypeArg::OrderBookBalancedSizeSkewed80Pct50Markets => {
                call_custom_module(EntryPoints::OrderBook {
                    state: OrderBookState::new(),
                    num_markets:
                        if let TransactionTypeArg::OrderBookBalancedSizeSkewed80Pct50Markets = self
                        {
                            50
                        } else {
                            1
                        },
                    overlap_ratio: 0.8,
                    buy_frequency: 0.01,
                    max_sell_size: 50,
                    max_buy_size: 950,
                })
            },
            TransactionTypeArg::DependencyChain8 => {
                call_custom_module(EntryPoints::DependencyChain8 { depth: 1 })
            },
            TransactionTypeArg::DependencyChain64 => {
                call_custom_module(EntryPoints::DependencyChain64 { depth: 1 })
            },
            TransactionTypeArg::DependencyChain256 => {
                call_custom_module(EntryPoints::DependencyChain256 { depth: 1 })
            },
            TransactionTypeArg::DependencyChain512 => {
                call_custom_module(EntryPoints::DependencyChain512 { depth: 1 })
            },
            TransactionTypeArg::DependencyChain512Depth64 => {
                call_custom_module(EntryPoints::DependencyChain512 { depth: 64 })
            },
            TransactionTypeArg::DependencyChain512Depth128 => {
                call_custom_module(EntryPoints::DependencyChain512 { depth: 128 })
            },
            TransactionTypeArg::DependencyChain512Depth256 => {
                call_custom_module(EntryPoints::DependencyChain512 { depth: 256 })
            },
            TransactionTypeArg::DependencyChain512Depth512 => {
                call_custom_module(EntryPoints::DependencyChain512 { depth: 512 })
            },
            TransactionTypeArg::DependencyTree81With3ChildrenPerLeaf => {
                call_custom_module(EntryPoints::DependencyTree81With3ChildrenPerLeaf)
            },
            TransactionTypeArg::DependencyTree585With8ChildrenPerLeaf => {
                call_custom_module(EntryPoints::DependencyTree585With8ChildrenPerLeaf)
            },
            TransactionTypeArg::DependencyStar32Width0 => {
                call_custom_module(EntryPoints::DependencyStar32 { width: 0 })
            },
            TransactionTypeArg::DependencyStar512Width0 => {
                call_custom_module(EntryPoints::DependencyStar512 { width: 0 })
            },
            TransactionTypeArg::DependencyStar512Width64 => {
                call_custom_module(EntryPoints::DependencyStar512 { width: 64 })
            },
            TransactionTypeArg::DependencyStar512Width128 => {
                call_custom_module(EntryPoints::DependencyStar512 { width: 128 })
            },
            TransactionTypeArg::DependencyStar512Width256 => {
                call_custom_module(EntryPoints::DependencyStar512 { width: 256 })
            },
            TransactionTypeArg::DependencyStar512Width512 => {
                call_custom_module(EntryPoints::DependencyStar512 { width: 512 })
            },
            TransactionTypeArg::DependencyDag64Sparse => {
                call_custom_module(EntryPoints::DependencyDag64Sparse)
            },
            TransactionTypeArg::DependencyDag256Sparse => {
                call_custom_module(EntryPoints::DependencyDag256Sparse)
            },
            TransactionTypeArg::DependencyDag256Dense => {
                call_custom_module(EntryPoints::DependencyDag256Dense)
            },
            TransactionTypeArg::DependencyDag64Dense => {
                call_custom_module(EntryPoints::DependencyDag64Dense)
            },
        }
    }

    pub fn args_to_transaction_mix_per_phase(
        transaction_types: &[TransactionTypeArg],
        transaction_weights: &[usize],
        transaction_phases: &[usize],
        module_working_set_size: usize,
        sender_use_account_pool: bool,
        workflow_progress_type: WorkflowProgress,
    ) -> Vec<Vec<(TransactionType, usize)>> {
        let arg_transaction_types = transaction_types
            .iter()
            .map(|t| {
                t.materialize(
                    module_working_set_size,
                    sender_use_account_pool,
                    workflow_progress_type,
                )
            })
            .collect::<Vec<_>>();

        let arg_transaction_weights = if transaction_weights.is_empty() {
            vec![1; arg_transaction_types.len()]
        } else {
            assert_eq!(
                transaction_weights.len(),
                arg_transaction_types.len(),
                "Transaction types and weights need to be the same length"
            );
            transaction_weights.to_vec()
        };
        let arg_transaction_phases = if transaction_phases.is_empty() {
            vec![0; arg_transaction_types.len()]
        } else {
            assert_eq!(
                transaction_phases.len(),
                arg_transaction_types.len(),
                "Transaction types and phases need to be the same length"
            );
            transaction_phases.to_vec()
        };

        let mut transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>> = Vec::new();
        for (transaction_type, (weight, phase)) in arg_transaction_types.into_iter().zip(
            arg_transaction_weights
                .into_iter()
                .zip(arg_transaction_phases.into_iter()),
        ) {
            assert!(
                phase <= transaction_mix_per_phase.len(),
                "cannot skip phases ({})",
                transaction_mix_per_phase.len()
            );
            if phase == transaction_mix_per_phase.len() {
                transaction_mix_per_phase.push(Vec::new());
            }
            transaction_mix_per_phase
                .get_mut(phase)
                .unwrap()
                .push((transaction_type, weight));
        }

        transaction_mix_per_phase
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct EmitWorkloadArgs {
    #[clap(
        long,
        value_enum,
        default_value = "coin-transfer",
        num_args = 1..,
        ignore_case = true
    )]
    pub transaction_type: Vec<TransactionTypeArg>,

    /// Number of copies of the modules that will be published,
    /// under separate accounts, creating independent contracts,
    /// removing contention.
    /// For example for NFT minting, setting to 1 will be equivalent
    /// to minting from single collection,
    /// setting to 20 means minting from 20 collections in parallel.
    #[clap(long)]
    pub module_working_set_size: Option<usize>,

    /// Whether to use burner accounts for the sender.
    /// For example when transaction can only be done once per account.
    /// (pool needs to be populated by account-creation transactions)
    #[clap(long)]
    pub sender_use_account_pool: Option<bool>,

    #[clap(long, num_args = 0..)]
    pub transaction_weights: Vec<usize>,

    #[clap(long, num_args = 0..)]
    pub transaction_phases: Vec<usize>,
}

impl EmitWorkloadArgs {
    pub fn args_to_transaction_mix_per_phase(&self) -> Vec<Vec<(TransactionType, usize)>> {
        TransactionTypeArg::args_to_transaction_mix_per_phase(
            &self.transaction_type,
            &self.transaction_weights,
            &self.transaction_phases,
            self.module_working_set_size.unwrap_or(1),
            self.sender_use_account_pool.unwrap_or(false),
            WorkflowProgress::when_done_default(),
        )
    }
}
