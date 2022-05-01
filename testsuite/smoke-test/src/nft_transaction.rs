// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
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

        let collection_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_token_create_unlimited_collection_script(
                collection_name.clone(),
                "description".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
            ),
        );

        let collection_txn = creator.sign_with_transaction_builder(collection_builder);
        client.submit_and_wait(&collection_txn).await?;

        println!("collection created.");

        let token_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_token_create_unlimited_token_script(
                collection_name.clone(),
                token_name.clone(),
                "collection description".to_owned().into_bytes(),
                true,
                1,
                "uri".to_owned().into_bytes(),
            ),
        );

        let token_txn = creator.sign_with_transaction_builder(token_builder);
        client.submit_and_wait(&token_txn).await?;

        let offer_builder =
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_token_transfers_offer_script(
                    owner.address(),
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    1,
                ));
        let offer_txn = creator.sign_with_transaction_builder(offer_builder);
        client.submit_and_wait(&offer_txn).await?;

        let claim_builder =
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_token_transfers_claim_script(
                    creator.address(),
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                ));

        let claim_txn = owner.sign_with_transaction_builder(claim_builder);
        client.submit_and_wait(&claim_txn).await?;

        let transfer_builder =
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_token_direct_transfer_script(
                    creator.address(),
                    collection_name.clone(),
                    token_name.clone(),
                    1,
                ));
        let transfer_txn =
            owner.sign_multi_agent_with_transaction_builder(vec![&creator], transfer_builder);
        client.submit_and_wait(&transfer_txn).await?;

        Ok(())
    }
}
