// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_json_rpc_types::views::BytesView;
use diem_sdk::{
    client::{views::TransactionDataView, BlockingClient, SignedTransaction},
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

impl NFTPublicUsageTest for NFTTransaction {
    fn run<'t>(&self, ctx: &mut NFTPublicUsageContext<'t>) -> Result<()> {
        let client = ctx.client();

        // prepare sender and receiver accounts
        let sender_private_key = Ed25519PrivateKey::generate(ctx.rng());
        let sender_public_key = sender_private_key.public_key();
        let sender_auth_key = AuthenticationKey::ed25519(&sender_public_key);
        let sender_address = sender_auth_key.derived_address();
        ctx.create_user_account(sender_auth_key)?;
        let receiver_private_key = Ed25519PrivateKey::generate(ctx.rng());
        let receiver_public_key = receiver_private_key.public_key();
        let receiver_auth_key = AuthenticationKey::ed25519(&receiver_public_key);
        let receiver_address = receiver_auth_key.derived_address();
        ctx.create_user_account(receiver_auth_key)?;

        // register bars user for sender
        let sender_register_txn = register_bars_user_txn(&sender_private_key, &client, ctx)?;
        client.submit(&sender_register_txn)?;
        client.wait_for_signed_transaction(&sender_register_txn, None, None)?;

        // mint 100 nft tokens to sender
        ctx.mint_bars(sender_address, 100)?;

        // register bars user for receiver
        let receiver_register_txn = register_bars_user_txn(&receiver_private_key, &client, ctx)?;
        client.submit(&receiver_register_txn)?;
        client.wait_for_signed_transaction(&receiver_register_txn, None, None)?;

        // transfer 42 tokens to receiver
        let transfer_amount = 42;

        // prepare transfer transaction
        let test_sequence_number = client
            .get_account(sender_address)?
            .into_inner()
            .unwrap()
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
        client.submit(&txn)?;
        client.wait_for_signed_transaction(&txn, None, None)?;

        // query the transaction and check it contains the same values as requested
        let txn = client
            .get_account_transaction(sender_address, test_sequence_number, false)?
            .into_inner()
            .unwrap();

        match txn.transaction {
            TransactionDataView::UserTransaction {
                sender,
                sequence_number,
                script,
                ..
            } => {
                assert_eq!(sender, sender_address);
                assert_eq!(sequence_number, test_sequence_number);

                assert_eq!(script.r#type, "script_function");
                assert_eq!(script.type_arguments.unwrap(), vec!["BARSToken"]);
                assert_eq!(
                    script.arguments_bcs.unwrap(),
                    vec![
                        BytesView::new(receiver_address.into_bytes()),
                        BytesView::new(transfer_amount.to_le_bytes()),
                        BytesView::new(sender_address.into_bytes()),
                        BytesView::new(2_u64.to_le_bytes())
                    ]
                );
                assert_eq!(script.module_name.unwrap(), "NFTGallery");
                assert_eq!(
                    script.function_name.unwrap(),
                    "transfer_token_between_galleries"
                );
            }
            _ => panic!("Query should get user transaction"),
        }
        Ok(())
    }
}

fn register_bars_user_txn<'t>(
    private_key: &Ed25519PrivateKey,
    client: &BlockingClient,
    ctx: &NFTPublicUsageContext<'t>,
) -> Result<SignedTransaction> {
    let public_key = private_key.public_key();

    let sender_auth_key = AuthenticationKey::ed25519(&public_key);
    let sender_address = sender_auth_key.derived_address();

    let sequence_number = client
        .get_account(sender_address)?
        .into_inner()
        .unwrap()
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
