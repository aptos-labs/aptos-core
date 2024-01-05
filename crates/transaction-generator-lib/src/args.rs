// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{publishing::module_simple::LoopType, EntryPoints, TransactionType};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

/// Utility class for specifying transaction type with predefined configurations through CLI
#[derive(Debug, Copy, Clone, ValueEnum, Default, Deserialize, Parser, Serialize)]
pub enum TransactionTypeArg {
    // custom
    #[default]
    CoinTransfer,
    CoinTransferWithInvalid,
    NonConflictingCoinTransfer,
    AccountGeneration,
    AccountGenerationLargePool,
    Batch100Transfer,
    PublishPackage,
    // Simple EntryPoints
    NoOp,
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
    // Complex EntryPoints
    CreateObjects10,
    CreateObjects10WithPayload10k,
    CreateObjectsConflict10WithPayload10k,
    CreateObjects100,
    CreateObjects100WithPayload10k,
    CreateObjectsConflict100WithPayload10k,
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
    TokenV2AmbassadorMint,
    VectorPictureCreate30k,
    VectorPicture30k,
    VectorPictureRead30k,
    VectorPictureCreate40,
    VectorPicture40,
    VectorPictureRead40,
    SmartTablePicture30KWith200Change,
    SmartTablePicture1MWith1KChange,
    SmartTablePicture1BWith1KChange,
}

impl TransactionTypeArg {
    pub fn materialize_default(&self) -> TransactionType {
        self.materialize(1, false)
    }

    pub fn materialize(
        &self,
        module_working_set_size: usize,
        sender_use_account_pool: bool,
    ) -> TransactionType {
        match self {
            TransactionTypeArg::CoinTransfer => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool,
            },
            TransactionTypeArg::NonConflictingCoinTransfer => {
                TransactionType::NonConflictingCoinTransfer {
                    invalid_transaction_ratio: 0,
                    sender_use_account_pool,
                }
            },
            TransactionTypeArg::CoinTransferWithInvalid => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 10,
                sender_use_account_pool,
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
            },
            TransactionTypeArg::Batch100Transfer => {
                TransactionType::BatchTransfer { batch_size: 100 }
            },
            TransactionTypeArg::AccountResource32B => TransactionType::CallCustomModules {
                entry_point: EntryPoints::BytesMakeOrChange {
                    data_length: Some(32),
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::AccountResource1KB => TransactionType::CallCustomModules {
                entry_point: EntryPoints::BytesMakeOrChange {
                    data_length: Some(1024),
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::AccountResource10KB => TransactionType::CallCustomModules {
                entry_point: EntryPoints::BytesMakeOrChange {
                    data_length: Some(10 * 1024),
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::ModifyGlobalResource => TransactionType::CallCustomModules {
                entry_point: EntryPoints::IncGlobal,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::ModifyGlobalResourceAggV2 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::IncGlobalAggV2,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::ModifyGlobalFlagAggV2 => TransactionType::CallCustomModules {
                // 100 is max, so equivalent to flag
                entry_point: EntryPoints::ModifyGlobalBoundedAggV2 { step: 100 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::ModifyGlobalBoundedAggV2 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::ModifyGlobalBoundedAggV2 { step: 10 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::NoOp => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Nop,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::NoOp2Signers => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Nop,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::NoOp5Signers => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Nop,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::Loop100k => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Loop {
                    loop_count: Some(100000),
                    loop_type: LoopType::NoOp,
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::Loop10kArithmetic => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Loop {
                    loop_count: Some(10000),
                    loop_type: LoopType::Arithmetic,
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::Loop1kBcs1k => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Loop {
                    loop_count: Some(1000),
                    loop_type: LoopType::BcsToBytes { len: 1024 },
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::CreateObjects10 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::CreateObjects {
                    num_objects: 10,
                    object_payload_size: 0,
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::CreateObjects10WithPayload10k => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::CreateObjects {
                        num_objects: 10,
                        object_payload_size: 10 * 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::CreateObjectsConflict10WithPayload10k => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::CreateObjectsConflict {
                        num_objects: 10,
                        object_payload_size: 10 * 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::CreateObjects100 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::CreateObjects {
                    num_objects: 100,
                    object_payload_size: 0,
                },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::CreateObjects100WithPayload10k => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::CreateObjects {
                        num_objects: 100,
                        object_payload_size: 10 * 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::CreateObjectsConflict100WithPayload10k => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::CreateObjectsConflict {
                        num_objects: 100,
                        object_payload_size: 10 * 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::ResourceGroupsGlobalWriteTag1KB => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::ResourceGroupsGlobalWriteTag {
                        string_length: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::ResourceGroupsGlobalWriteAndReadTag {
                        string_length: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::ResourceGroupsSenderWriteTag1KB => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::ResourceGroupsSenderWriteTag {
                        string_length: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::ResourceGroupsSenderMultiChange1KB => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::ResourceGroupsSenderMultiChange {
                        string_length: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::TokenV1NFTMintAndStoreSequential => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::TokenV1MintAndStoreNFTSequential,
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::TokenV1NFTMintAndTransferSequential => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::TokenV1MintAndTransferNFTSequential,
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::TokenV1NFTMintAndStoreParallel => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::TokenV1MintAndStoreNFTParallel,
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::TokenV1NFTMintAndTransferParallel => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::TokenV1MintAndTransferNFTParallel,
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::TokenV1FTMintAndStore => TransactionType::CallCustomModules {
                entry_point: EntryPoints::TokenV1MintAndStoreFT,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::TokenV1FTMintAndTransfer => TransactionType::CallCustomModules {
                entry_point: EntryPoints::TokenV1MintAndTransferFT,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::TokenV2AmbassadorMint => TransactionType::CallCustomModules {
                entry_point: EntryPoints::TokenV2AmbassadorMint,
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPictureCreate30k => TransactionType::CallCustomModules {
                entry_point: EntryPoints::InitializeVectorPicture { length: 30 * 1024 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPicture30k => TransactionType::CallCustomModules {
                entry_point: EntryPoints::VectorPicture { length: 30 * 1024 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPictureRead30k => TransactionType::CallCustomModules {
                entry_point: EntryPoints::VectorPictureRead { length: 30 * 1024 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPictureCreate40 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::InitializeVectorPicture { length: 40 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPicture40 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::VectorPicture { length: 40 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::VectorPictureRead40 => TransactionType::CallCustomModules {
                entry_point: EntryPoints::VectorPictureRead { length: 40 },
                num_modules: module_working_set_size,
                use_account_pool: sender_use_account_pool,
            },
            TransactionTypeArg::SmartTablePicture30KWith200Change => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::SmartTablePicture {
                        length: 30 * 1024,
                        num_points_per_txn: 200,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::SmartTablePicture1MWith1KChange => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::SmartTablePicture {
                        length: 1024 * 1024,
                        num_points_per_txn: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
            TransactionTypeArg::SmartTablePicture1BWith1KChange => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::SmartTablePicture {
                        length: 1024 * 1024 * 1024,
                        num_points_per_txn: 1024,
                    },
                    num_modules: module_working_set_size,
                    use_account_pool: sender_use_account_pool,
                }
            },
        }
    }

    pub fn args_to_transaction_mix_per_phase(
        transaction_types: &[TransactionTypeArg],
        transaction_weights: &[usize],
        transaction_phases: &[usize],
        module_working_set_size: usize,
        sender_use_account_pool: bool,
    ) -> Vec<Vec<(TransactionType, usize)>> {
        let arg_transaction_types = transaction_types
            .iter()
            .map(|t| t.materialize(module_working_set_size, sender_use_account_pool))
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
