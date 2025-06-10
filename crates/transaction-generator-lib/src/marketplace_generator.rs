// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Custom transaction generators for marketplace workflow that support cross-stage communication

use super::{publishing::publish_util::Package, ReliableTransactionSubmitter};
use crate::{
    call_custom_modules::{TransactionGeneratorWorker, UserModuleTransactionGenerator}, 
    RootAccountHandle
};
use aptos_sdk::{
    bcs,
    move_types::{ident_str, language_storage::ModuleId, account_address::AccountAddress},
    transaction_builder::TransactionFactory,
    types::{
        transaction::{EntryFunction, SignedTransaction, TransactionPayload},
        LocalAccount,
    },
};
use async_trait::async_trait;
use rand::{rngs::StdRng, Rng};
use std::sync::{Arc, RwLock};

/// Represents a minted token with its object address
#[derive(Debug, Clone)]
pub struct MintedTokenInfo {
    pub token_address: AccountAddress,
    pub owner_address: AccountAddress,
}

/// Represents a fee schedule with its object address
#[derive(Debug, Clone)]
pub struct FeeScheduleInfo {
    pub fee_schedule_address: AccountAddress,
    pub fee_metadata_address: AccountAddress, // APT metadata object
}

/// Transaction generator for Stage 0: Mint NFT tokens and store them in the pool
pub struct MintNftTransactionGenerator {
    minted_tokens: Arc<RwLock<Vec<MintedTokenInfo>>>,
}

impl MintNftTransactionGenerator {
    pub fn new(minted_tokens: Arc<RwLock<Vec<MintedTokenInfo>>>) -> Self {
        Self { minted_tokens }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for MintNftTransactionGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &LocalAccount,
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
        let minted_tokens = self.minted_tokens.clone();

        Arc::new(move |account, _package, publisher, txn_factory, rng| {
            // Create mint NFT transaction using the provided account as signer
            let collection_name = format!("TestCollection{}", rng.gen::<u32>());
            let token_name = format!("TestToken{}", rng.gen::<u32>());
            
            let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(publisher.address(), ident_str!("create_nft").to_owned()),
                ident_str!("create_default_collection_and_mint_test_nft_to_self").to_owned(),
                vec![],
                vec![
                    bcs::to_bytes(&collection_name).unwrap(),
                    bcs::to_bytes(&token_name).unwrap(),
                    bcs::to_bytes(&account.address()).unwrap(), // Mint to the provided account
                ],
            ));

            // Use the provided account as the transaction signer (follows CreateMintBurn pattern)
            let txn = account.sign_with_transaction_builder(txn_factory.payload(payload));

            // Store the token object address for later stages
            // In a real implementation, this would be the actual token object address
            let simulated_token_info = MintedTokenInfo {
                token_address: account.address(),
                owner_address: account.address(),
            };
            
            if let Ok(mut tokens) = minted_tokens.write() {
                tokens.push(simulated_token_info);
            }

            Some(txn)
        })
    }
}

/// Transaction generator for Stage 1: Create fee schedules and store them in the pool
pub struct CreateFeeScheduleTransactionGenerator {
    fee_schedules: Arc<RwLock<Vec<FeeScheduleInfo>>>,
}

impl CreateFeeScheduleTransactionGenerator {
    pub fn new(fee_schedules: Arc<RwLock<Vec<FeeScheduleInfo>>>) -> Self {
        Self { fee_schedules }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for CreateFeeScheduleTransactionGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &LocalAccount,
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
        let fee_schedules = self.fee_schedules.clone();

        Arc::new(move |account, _package, publisher, txn_factory, _rng| {
            // Create fee schedule transaction using the provided account as signer
            let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(publisher.address(), ident_str!("fee_schedule").to_owned()),
                ident_str!("create_zero_fee_schedule").to_owned(),
                vec![],
                vec![],
            ));

            // Use the provided account as the transaction signer (follows CreateMintBurn pattern)
            let txn = account.sign_with_transaction_builder(txn_factory.payload(payload));

            // Store the fee schedule object address for later stages
            // In a real implementation, this would be the actual fee schedule object address
            let simulated_fee_schedule_info = FeeScheduleInfo {
                fee_schedule_address: account.address(),
                fee_metadata_address: account.address(),
            };
            
            if let Ok(mut schedules) = fee_schedules.write() {
                schedules.push(simulated_fee_schedule_info);
            }

            Some(txn)
        })
    }
}

/// Transaction generator for Stage 2: Place listings using tokens and fee schedules from pools
pub struct PlaceListingTransactionGenerator {
    minted_tokens: Arc<RwLock<Vec<MintedTokenInfo>>>,
    fee_schedules: Arc<RwLock<Vec<FeeScheduleInfo>>>,
}

impl PlaceListingTransactionGenerator {
    pub fn new(
        minted_tokens: Arc<RwLock<Vec<MintedTokenInfo>>>,
        fee_schedules: Arc<RwLock<Vec<FeeScheduleInfo>>>,
    ) -> Self {
        Self {
            minted_tokens,
            fee_schedules,
        }
    }
}

#[async_trait]
impl UserModuleTransactionGenerator for PlaceListingTransactionGenerator {
    fn initialize_package(
        &mut self,
        _package: &Package,
        _publisher: &LocalAccount,
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
        let minted_tokens = self.minted_tokens.clone();
        let fee_schedules = self.fee_schedules.clone();

        Arc::new(move |account, _package, publisher, txn_factory, rng| {
            // Get random objects from previous stages
            let token_info = {
                if let Ok(tokens) = minted_tokens.read() {
                    if !tokens.is_empty() {
                        Some(tokens[rng.gen_range(0, tokens.len())].clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let fee_schedule_info = {
                if let Ok(schedules) = fee_schedules.read() {
                    if !schedules.is_empty() {
                        Some(schedules[rng.gen_range(0, schedules.len())].clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let (Some(token_info), Some(fee_schedule_info)) = (token_info, fee_schedule_info) {
                // Create marketplace listing transaction using object addresses from previous stages
                let listing_price = rng.gen_range(100, 10000u64);
                
                let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(publisher.address(), ident_str!("marketplace").to_owned()),
                    ident_str!("place_token_listing").to_owned(),
                    vec![],
                    vec![
                        bcs::to_bytes(&token_info.token_address).unwrap(),
                        bcs::to_bytes(&fee_schedule_info.fee_schedule_address).unwrap(),
                        bcs::to_bytes(&fee_schedule_info.fee_metadata_address).unwrap(),
                        bcs::to_bytes(&listing_price).unwrap(),
                    ],
                ));

                // Use the provided account as the transaction signer (follows CreateMintBurn pattern)
                let txn = account.sign_with_transaction_builder(txn_factory.payload(payload));
                Some(txn)
            } else {
                // Skip transaction if no tokens or fee schedules are available yet
                None
            }
        })
    }
}
