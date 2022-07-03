// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::TransactionGenerator;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};

use crate::emit::create_and_fund_account_request;
use aptos_logger::info;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct NFTMint {
    txn_factory: TransactionFactory,
    creator_account: Arc<LocalAccount>,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMint {
    pub async fn new(
        txn_factory: TransactionFactory,
        creator_account: Arc<LocalAccount>,
        collection_name: Vec<u8>,
        token_name: Vec<u8>,
    ) -> Self {
        Self {
            txn_factory,
            creator_account,
            collection_name,
            token_name,
        }
    }
}

impl TransactionGenerator for NFTMint {
    fn generate_transactions(
        &mut self,
        accounts: Vec<&mut LocalAccount>,
        _all_addresses: Arc<Vec<AccountAddress>>,
        _invalid_transaction_ratio: usize,
        _gas_price: u64,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len());
        for account in accounts {
            requests.push(create_nft_transfer_request(
                account,
                &self.creator_account,
                &self.collection_name,
                &self.token_name,
                &self.txn_factory,
            ));
        }
        requests
    }
}

pub async fn initialize_nft_collection(
    rest_client: RestClient,
    root_account: &mut LocalAccount,
    creator_account: &mut LocalAccount,
    txn_factory: &TransactionFactory,
    collection_name: &[u8],
    token_name: &[u8],
) {
    // Create and mint the owner account first
    let create_account_txn = create_and_fund_account_request(
        root_account,
        10_000_000,
        creator_account.public_key(),
        txn_factory,
    );
    rest_client
        .submit_and_wait(&create_account_txn)
        .await
        .unwrap();

    info!("create_account_txn complete");

    let collection_txn =
        create_nft_collection_request(creator_account, collection_name, txn_factory);

    rest_client.submit_and_wait(&collection_txn).await.unwrap();

    info!("collection_txn complete");

    let token_txn =
        create_nft_token_request(creator_account, collection_name, token_name, txn_factory);

    rest_client.submit_and_wait(&token_txn).await.unwrap();

    info!("token_txn complete");
}

pub fn create_nft_collection_request(
    creation_account: &mut LocalAccount,
    collection_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_stdlib::encode_token_create_unlimited_collection_script(
            collection_name.to_vec(),
            "description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
        ),
    ))
}

pub fn create_nft_token_request(
    creation_account: &mut LocalAccount,
    collection_name: &[u8],
    token_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_stdlib::encode_token_create_unlimited_token_script(
            collection_name.to_vec(),
            token_name.to_vec(),
            "collection description".to_owned().into_bytes(),
            true,
            1_000_000_000,
            "uri".to_owned().into_bytes(),
            0,
        ),
    ))
}

pub fn create_nft_transfer_request(
    owner_account: &mut LocalAccount,
    creation_account: &LocalAccount,
    collection_name: &[u8],
    token_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    owner_account.sign_multi_agent_with_transaction_builder(
        vec![creation_account],
        txn_factory.payload(aptos_stdlib::encode_token_direct_transfer_script(
            creation_account.address(),
            collection_name.to_vec(),
            token_name.to_vec(),
            1,
        )),
    )
}
