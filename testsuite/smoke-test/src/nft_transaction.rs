// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use cached_packages::aptos_stdlib::aptos_token_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct NFTTransaction;

impl Test for NFTTransaction {
    fn name(&self) -> &'static str {
        "smoke-test::nft-transaction"
    }
}

#[async_trait::async_trait]
impl AptosTest for NFTTransaction {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();
        let mut creator = ctx.random_account();
        ctx.create_user_account(creator.public_key()).await?;
        ctx.mint(creator.address(), 10_000_000).await?;
        let mut owner = ctx.random_account();
        ctx.create_user_account(owner.public_key()).await?;
        ctx.mint(owner.address(), 10_000_000).await?;

        let collection_name = "collection name".to_owned().into_bytes();
        let token_name = "token name".to_owned().into_bytes();
        let collection_builder =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_create_collection_script(
                    collection_name.clone(),
                    "description".to_owned().into_bytes(),
                    "uri".to_owned().into_bytes(),
                    20_000_000,
                    vec![false, false, false],
                ));

        let collection_txn = creator.sign_with_transaction_builder(collection_builder);
        client.submit_and_wait(&collection_txn).await?;

        let token_builder =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_create_token_script(
                    collection_name.clone(),
                    token_name.clone(),
                    "collection description".to_owned().into_bytes(),
                    3,
                    4,
                    "uri".to_owned().into_bytes(),
                    creator.address(),
                    0,
                    0,
                    vec![false, false, false, false, true],
                    vec!["age".as_bytes().to_vec()],
                    vec!["3".as_bytes().to_vec()],
                    vec!["int".as_bytes().to_vec()],
                ));

        let token_txn = creator.sign_with_transaction_builder(token_builder);
        client.submit_and_wait(&token_txn).await?;

        let offer_builder =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_transfers_offer_script(
                    owner.address(),
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    0,
                    1,
                ));
        let offer_txn = creator.sign_with_transaction_builder(offer_builder);
        client.submit_and_wait(&offer_txn).await?;

        let claim_builder =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_transfers_claim_script(
                    creator.address(),
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    0,
                ));

        let claim_txn = owner.sign_with_transaction_builder(claim_builder);
        client.submit_and_wait(&claim_txn).await?;

        let transfer_builder =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_direct_transfer_script(
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    0,
                    1,
                ));
        let transfer_txn =
            owner.sign_multi_agent_with_transaction_builder(vec![&creator], transfer_builder);
        client.submit_and_wait(&transfer_txn).await?;

        let token_mutator =
            ctx.transaction_factory()
                .payload(aptos_token_stdlib::token_mutate_token_properties(
                    creator.address(),
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    0,
                    2,
                    vec!["age".as_bytes().to_vec()],
                    vec!["2".as_bytes().to_vec()],
                    vec!["int".as_bytes().to_vec()],
                ));
        let mutate_txn = creator.sign_with_transaction_builder(token_mutator);
        client.submit_and_wait(&mutate_txn).await?;

        Ok(())
    }
}
