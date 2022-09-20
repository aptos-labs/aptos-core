// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::transaction_generator::{TransactionGenerator, TransactionGeneratorCreator};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib::aptos_token_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};

use crate::emitter::{account_minter::create_and_fund_account_request, RETRY_POLICY};
use aptos_logger::{info, warn};
use rand::rngs::StdRng;
use std::sync::Arc;

pub struct NFTMint {
    txn_factory: TransactionFactory,
    creator_account: Arc<LocalAccount>,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMint {
    pub fn new(
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
        transactions_per_account: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(accounts.len() * transactions_per_account);
        for account in accounts {
            for _ in 0..transactions_per_account {
                requests.push(create_nft_transfer_request(
                    account,
                    &self.creator_account,
                    &self.collection_name,
                    &self.token_name,
                    &self.txn_factory,
                ));
            }
        }
        requests
    }
}

async fn submit_retry_and_wait(rest_client: &RestClient, txn: &SignedTransaction) {
    let submit_result = RETRY_POLICY
        .retry(move || rest_client.submit_bcs(txn))
        .await;
    if let Err(e) = submit_result {
        warn!("Failed submitting transaction {:?} with {:?}", txn, e);
    }
    // if submission timeouts, it might still get committed:
    RETRY_POLICY
        .retry(move || rest_client.wait_for_signed_transaction_bcs(txn))
        .await
        .unwrap();
}

pub async fn initialize_nft_collection(
    rest_client: RestClient,
    root_account: &mut LocalAccount,
    creator_account: &mut LocalAccount,
    txn_factory: &TransactionFactory,
    collection_name: &[u8],
    token_name: &[u8],
) {
    // resync root account sequence number
    match rest_client.get_account(root_account.address()).await {
        Ok(result) => {
            let account = result.into_inner();
            if root_account.sequence_number() < account.sequence_number {
                warn!(
                    "Root account sequence number got out of sync: remotely {}, locally {}",
                    account.sequence_number,
                    root_account.sequence_number_mut()
                );
                *root_account.sequence_number_mut() = account.sequence_number;
            }
        }
        Err(e) => warn!(
            "[{}] Couldn't check account sequence number due to {:?}",
            rest_client.path_prefix_string(),
            e
        ),
    }

    // Create and mint the owner account first
    let create_account_txn = create_and_fund_account_request(
        root_account,
        10_000_000,
        creator_account.public_key(),
        txn_factory,
    );

    submit_retry_and_wait(&rest_client, &create_account_txn).await;

    info!("create_account_txn complete");

    let collection_txn =
        create_nft_collection_request(creator_account, collection_name, txn_factory);

    submit_retry_and_wait(&rest_client, &collection_txn).await;

    info!("collection_txn complete");

    let token_txn =
        create_nft_token_request(creator_account, collection_name, token_name, txn_factory);

    submit_retry_and_wait(&rest_client, &token_txn).await;

    info!("token_txn complete");
}

pub fn create_nft_collection_request(
    creation_account: &mut LocalAccount,
    collection_name: &[u8],
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    creation_account.sign_with_transaction_builder(txn_factory.payload(
        aptos_token_stdlib::token_create_collection_script(
            collection_name.to_vec(),
            "description".to_owned().into_bytes(),
            "uri".to_owned().into_bytes(),
            u64::MAX,
            vec![false, false, false],
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
        aptos_token_stdlib::token_create_token_script(
            collection_name.to_vec(),
            token_name.to_vec(),
            "collection description".to_owned().into_bytes(),
            1_000_000_000,
            u64::MAX,
            "uri".to_owned().into_bytes(),
            creation_account.address(),
            0,
            0,
            vec![false, false, false, false, false],
            vec![Vec::new()],
            vec![Vec::new()],
            vec![Vec::new()],
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
        txn_factory.payload(aptos_token_stdlib::token_direct_transfer_script(
            creation_account.address(),
            collection_name.to_vec(),
            token_name.to_vec(),
            0,
            1,
        )),
    )
}

pub struct NFTMintGeneratorCreator {
    txn_factory: TransactionFactory,
    creator_account: Arc<LocalAccount>,
    collection_name: Vec<u8>,
    token_name: Vec<u8>,
}

impl NFTMintGeneratorCreator {
    pub async fn new(
        mut rng: StdRng,
        txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        rest_client: RestClient,
    ) -> Self {
        let mut creator_account = LocalAccount::generate(&mut rng);
        let collection_name = "collection name".to_owned().into_bytes();
        let token_name = "token name".to_owned().into_bytes();
        initialize_nft_collection(
            rest_client,
            root_account,
            &mut creator_account,
            &txn_factory,
            &collection_name,
            &token_name,
        )
        .await;
        Self {
            txn_factory,
            creator_account: Arc::new(creator_account),
            collection_name,
            token_name,
        }
    }
}

impl TransactionGeneratorCreator for NFTMintGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(NFTMint::new(
            self.txn_factory.clone(),
            self.creator_account.clone(),
            self.collection_name.clone(),
            self.token_name.clone(),
        ))
    }
}
