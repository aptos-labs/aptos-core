// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{EntryPoints, TransactionType};
use clap::{ArgEnum, Parser};
use serde::{Deserialize, Serialize};

/// Utility class for specifying transaction type with predefined configurations through CLI
#[derive(Debug, Copy, Clone, ArgEnum, Deserialize, Parser, Serialize)]
pub enum TransactionTypeArg {
    CoinTransfer,
    AccountGeneration,
    AccountGenerationLargePool,
    NftMintAndTransfer,
    PublishPackage,
    CustomFunctionLargeModuleWorkingSet,
    CreateNewResource,
    NoOp,
}

impl Default for TransactionTypeArg {
    fn default() -> Self {
        TransactionTypeArg::CoinTransfer
    }
}

impl TransactionTypeArg {
    pub fn materialize(&self) -> TransactionType {
        match self {
            TransactionTypeArg::CoinTransfer => TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
            },
            TransactionTypeArg::AccountGeneration => TransactionType::default_account_generation(),
            TransactionTypeArg::AccountGenerationLargePool => TransactionType::AccountGeneration {
                add_created_accounts_to_pool: true,
                max_account_working_set: 50_000_000,
                creation_balance: 200_000_000,
            },
            TransactionTypeArg::NftMintAndTransfer => TransactionType::NftMintAndTransfer,
            TransactionTypeArg::PublishPackage => TransactionType::PublishPackage {
                use_account_pool: false,
            },
            TransactionTypeArg::CustomFunctionLargeModuleWorkingSet => {
                TransactionType::CallCustomModules {
                    entry_point: EntryPoints::Nop,
                    num_modules: 1000,
                    use_account_pool: false,
                }
            },
            TransactionTypeArg::CreateNewResource => TransactionType::CallCustomModules {
                entry_point: EntryPoints::BytesMakeOrChange {
                    data_length: Some(32),
                },
                num_modules: 1,
                use_account_pool: true,
            },
            TransactionTypeArg::NoOp => TransactionType::CallCustomModules {
                entry_point: EntryPoints::Nop,
                num_modules: 1,
                use_account_pool: false,
            },
        }
    }
}
