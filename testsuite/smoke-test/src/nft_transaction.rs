// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_rest_client::Client as RestClient;
use diem_sdk::{
    client::SignedTransaction,
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform},
    types::transaction::authenticator::AuthenticationKey,
};
use diem_transaction_builder::experimental_stdlib;
use diem_types::nft::tokens;
use forge::{NFTPublicUsageContext, NFTPublicUsageTest, Result, Test};

pub struct NFTTransaction;

impl Test for NFTTransaction {
    fn name(&self) -> &'static str {
        "smoke-test::nft-transaction"
    }
}

#[async_trait::async_trait]
impl NFTPublicUsageTest for NFTTransaction {
    async fn run<'t>(&self, ctx: &mut NFTPublicUsageContext<'t>) -> Result<()> {
        let client = ctx.client();

        // prepare sender and receiver accounts
        let sender_private_key = Ed25519PrivateKey::generate(ctx.rng());
        let sender_public_key = sender_private_key.public_key();
        let sender_auth_key = AuthenticationKey::ed25519(&sender_public_key);
        let sender_address = sender_auth_key.derived_address();
        ctx.create_user_account(sender_auth_key).await?;
        let receiver_private_key = Ed25519PrivateKey::generate(ctx.rng());
        let receiver_public_key = receiver_private_key.public_key();
        let receiver_auth_key = AuthenticationKey::ed25519(&receiver_public_key);
        let receiver_address = receiver_auth_key.derived_address();
        ctx.create_user_account(receiver_auth_key).await?;

        // register bars user for sender
        let sender_register_txn = register_bars_user_txn(&sender_private_key, &client, ctx).await?;
        client.submit_and_wait(&sender_register_txn).await?;

        // mint 100 nft tokens to sender
        ctx.mint_bars(sender_address, 100).await?;

        // register bars user for receiver
        let receiver_register_txn =
            register_bars_user_txn(&receiver_private_key, &client, ctx).await?;
        client.submit_and_wait(&receiver_register_txn).await?;

        // transfer 42 tokens to receiver
        let transfer_amount = 42;

        // prepare transfer transaction
        let test_sequence_number = client
            .get_account(sender_address)
            .await?
            .into_inner()
            .sequence_number;

        let unsigned_txn = ctx
            .transaction_factory()
            .payload(
                experimental_stdlib::encode_transfer_token_between_galleries_script_function(
                    tokens::bars_tag(),
                    receiver_address,
                    transfer_amount,
                    sender_address,
                    2,
                ),
            )
            .sender(sender_address)
            .sequence_number(test_sequence_number)
            .build();

        assert_eq!(unsigned_txn.sender(), sender_address);

        // sign the transaction with the private key
        let signature = sender_private_key.sign(&unsigned_txn);

        // submit the transaction
        let txn = SignedTransaction::new(unsigned_txn, sender_public_key, signature);
        client.submit_and_wait(&txn).await?;
        Ok(())
    }
}

async fn register_bars_user_txn(
    private_key: &Ed25519PrivateKey,
    client: &RestClient,
    ctx: &NFTPublicUsageContext<'_>,
) -> Result<SignedTransaction> {
    let public_key = private_key.public_key();

    let sender_auth_key = AuthenticationKey::ed25519(&public_key);
    let sender_address = sender_auth_key.derived_address();

    let sequence_number = client
        .get_account(sender_address)
        .await?
        .into_inner()
        .sequence_number;

    let unsigned_txn = ctx
        .transaction_factory()
        .payload(experimental_stdlib::encode_register_bars_user_script_function())
        .sender(sender_address)
        .sequence_number(sequence_number)
        .build();

    // sign the transaction with the private key
    let signature = private_key.sign(&unsigned_txn);

    Ok(SignedTransaction::new(unsigned_txn, public_key, signature))
}
