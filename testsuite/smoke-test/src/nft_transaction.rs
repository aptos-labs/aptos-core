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

        let collection_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_create_unlimited_collection_script_script_function(
                hex::encode("description").into_bytes(),
                hex::encode("collection name").into_bytes(),
                hex::encode("uri").into_bytes(),
            ),
        );

        let collection_txn = creator.sign_with_transaction_builder(collection_builder);
        client.submit_and_wait(&collection_txn).await?;

        let token_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_create_token_script_script_function(
                hex::encode("collection name").into_bytes(),
                hex::encode("description").into_bytes(),
                hex::encode("token name").into_bytes(),
                1,
                hex::encode("uri").into_bytes(),
            ),
        );

        let token_txn = creator.sign_with_transaction_builder(token_builder);
        client.submit_and_wait(&token_txn).await?;

        let token_num = client
            .get_account_resource(creator.address(), "0x1::Token::Collections")
            .await?
            .inner()
            .as_ref()
            .unwrap()
            .data["collections"]["data"][0]["value"]["tokens"]["data"][0]["value"]["id"]
            ["creation_num"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        let offer_builder =
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_offer_script_script_function(
                    owner.address(),
                    creator.address(),
                    token_num,
                    1,
                ));

        let offer_txn = creator.sign_with_transaction_builder(offer_builder);
        client.submit_and_wait(&offer_txn).await?;

        let claim_builder =
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_claim_script_script_function(
                    creator.address(),
                    creator.address(),
                    token_num,
                ));

        let claim_txn = owner.sign_with_transaction_builder(claim_builder);
        client.submit_and_wait(&claim_txn).await?;
        Ok(())
    }
}
