// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{publishing::publish_util::Package, ObjectPool, ReliableTransactionSubmitter};
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

// Stage 1: Mint NFT Transaction Generator
// Only needs write access to the minted tokens pool
pub struct MintNftTransactionGenerator {
    pub minted_token_objects: Arc<RwLock<Vec<AccountAddress>>>,
}

impl MintNftTransactionGenerator {
    pub fn new(minted_token_objects: Arc<RwLock<Vec<AccountAddress>>>) -> Self {
        Self { minted_token_objects }
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
        let minted_token_objects = self.minted_token_objects.clone();

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
            let simulated_token_object = account.address(); // Simplified for now
            
            if let Ok(mut tokens) = minted_token_objects.write() {
                tokens.push(simulated_token_object);
            }

            Some(txn)
        })
    }
}

// Stage 2: Create Fee Schedule Transaction Generator
// Only needs write access to the fee schedules pool  
pub struct CreateFeeScheduleTransactionGenerator {
    pub fee_schedule_objects: Arc<RwLock<Vec<AccountAddress>>>,
}

impl CreateFeeScheduleTransactionGenerator {
    pub fn new(fee_schedule_objects: Arc<RwLock<Vec<AccountAddress>>>) -> Self {
        Self { fee_schedule_objects }
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
        let fee_schedule_objects = self.fee_schedule_objects.clone();

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
            let simulated_fee_schedule_object = account.address(); // Simplified for now
            
            if let Ok(mut schedules) = fee_schedule_objects.write() {
                schedules.push(simulated_fee_schedule_object);
            }

            Some(txn)
        })
    }
}

// Stage 3: Place Listing Transaction Generator
// Needs read access to both pools from previous stages
pub struct PlaceListingTransactionGenerator {
    pub minted_token_objects: Arc<RwLock<Vec<AccountAddress>>>,
    pub fee_schedule_objects: Arc<RwLock<Vec<AccountAddress>>>,
}

impl PlaceListingTransactionGenerator {
    pub fn new(
        minted_token_objects: Arc<RwLock<Vec<AccountAddress>>>,
        fee_schedule_objects: Arc<RwLock<Vec<AccountAddress>>>,
    ) -> Self {
        Self { 
            minted_token_objects,
            fee_schedule_objects,
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
        let minted_token_objects = self.minted_token_objects.clone();
        let fee_schedule_objects = self.fee_schedule_objects.clone();

        Arc::new(move |account, _package, publisher, txn_factory, rng| {
            // Get random objects from previous stages
            let token_object = {
                if let Ok(tokens) = minted_token_objects.read() {
                    if !tokens.is_empty() {
                        Some(tokens[rng.gen_range(0, tokens.len())])
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let fee_schedule_object = {
                if let Ok(schedules) = fee_schedule_objects.read() {
                    if !schedules.is_empty() {
                        Some(schedules[rng.gen_range(0, schedules.len())])
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let (Some(token_addr), Some(fee_schedule_addr)) = (token_object, fee_schedule_object) {
                // Create marketplace listing transaction using object addresses from previous stages
                let listing_price = rng.gen_range(100, 10000u64);
                
                let payload = TransactionPayload::EntryFunction(EntryFunction::new(
                    ModuleId::new(publisher.address(), ident_str!("marketplace").to_owned()),
                    ident_str!("place_token_listing").to_owned(),
                    vec![],
                    vec![
                        bcs::to_bytes(&token_addr).unwrap(),
                        bcs::to_bytes(&fee_schedule_addr).unwrap(),
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
