// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};
use serde_json::Value;

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
                "description".to_owned().into_bytes(),
                "collection name".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
            ),
        );

        let collection_txn = creator.sign_with_transaction_builder(collection_builder);
        client.submit_and_wait(&collection_txn).await?;

        println!("collection created.");

        let token_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_create_token_script_script_function(
                "collection name".to_owned().into_bytes(),
                "collection description".to_owned().into_bytes(),
                "token name".to_owned().into_bytes(),
                1,
                "uri".to_owned().into_bytes(),
            ),
        );

        let token_txn = creator.sign_with_transaction_builder(token_builder);
        client.submit_and_wait(&token_txn).await?;

        let resource = client
            .get_account_resource(creator.address(), "0x1::Token::Collections")
            .await?
            .into_inner()
            .unwrap();
        let collections = &resource.data["collections"];
        let collection = client
            .get_table_item(
                table_handle(collections),
                "0x1::ASCII::String",
                "0x1::Token::Collection",
                "collection name",
            )
            .await?
            .into_inner();
        let token_data = client
            .get_table_item(
                table_handle(&collection["tokens"]),
                "0x1::ASCII::String",
                "0x1::Token::TokenData",
                "token name",
            )
            .await?
            .into_inner();
        let token_num = token_data["id"]["creation_num"].as_str().unwrap().parse()?;

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

        let transfer_builder = ctx.transaction_factory().payload(
            aptos_stdlib::encode_direct_transfer_script_function(creator.address(), token_num, 1),
        );
        let transfer_txn =
            owner.sign_multi_agent_with_transaction_builder(vec![&creator], transfer_builder);
        client.submit_and_wait(&transfer_txn).await?;

        Ok(())
    }
}

fn table_handle(table: &Value) -> u128 {
    table["handle"].as_str().unwrap().parse().unwrap()
}
