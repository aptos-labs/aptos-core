// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_rest_client::{
    aptos_api_types::{ScriptFunctionPayload, TransactionPayload},
    Transaction,
};
use aptos_sdk::{
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform},
    types::transaction::{authenticator::AuthenticationKey, SignedTransaction},
};
use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct ExternalTransactionSigner;

impl Test for ExternalTransactionSigner {
    fn name(&self) -> &'static str {
        "smoke-test::external-transaction-signer"
    }
}

#[async_trait::async_trait]
impl AptosTest for ExternalTransactionSigner {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();

        // generate key pair
        let private_key = Ed25519PrivateKey::generate(ctx.rng());
        let public_key = private_key.public_key();

        // create transfer parameters
        let sender_auth_key = AuthenticationKey::ed25519(&public_key);
        let sender_address = sender_auth_key.derived_address();
        ctx.create_user_account(&public_key).await?;
        ctx.mint(sender_address, 10_000_000).await?;

        let receiver = ctx.random_account();
        ctx.create_user_account(receiver.public_key()).await?;
        ctx.mint(receiver.address(), 1_000_000).await?;

        let amount = 1_000_000;
        let test_gas_unit_price = 1;
        let test_max_gas_amount = 1_000_000;

        // prepare transfer transaction
        let test_sequence_number = client
            .get_account(sender_address)
            .await?
            .into_inner()
            .sequence_number;

        let unsigned_txn = ctx
            .transaction_factory()
            .payload(aptos_stdlib::encode_test_coin_transfer(
                receiver.address(),
                amount,
            ))
            .sender(sender_address)
            .sequence_number(test_sequence_number)
            .max_gas_amount(test_max_gas_amount)
            .gas_unit_price(test_gas_unit_price)
            .build();

        assert_eq!(unsigned_txn.sender(), sender_address);

        // sign the transaction with the private key
        let signature = private_key.sign(&unsigned_txn);

        // submit the transaction
        let txn = SignedTransaction::new(unsigned_txn.clone(), public_key, signature);
        client.submit_and_wait(&txn).await?;

        // query the transaction and check it contains the same values as requested
        let txn = client
            .get_account_transactions(sender_address, Some(test_sequence_number), Some(1))
            .await?
            .into_inner()
            .into_iter()
            .next()
            .unwrap();

        match txn {
            Transaction::UserTransaction(user_txn) => {
                assert_eq!(*user_txn.request.sender.inner(), sender_address);
                assert_eq!(user_txn.request.sequence_number.0, test_sequence_number);
                assert_eq!(user_txn.request.gas_unit_price.0, test_gas_unit_price);
                assert_eq!(user_txn.request.max_gas_amount.0, test_max_gas_amount);

                if let TransactionPayload::ScriptFunctionPayload(ScriptFunctionPayload {
                    function: _,
                    type_arguments: _,
                    arguments,
                }) = user_txn.request.payload
                {
                    assert_eq!(
                        arguments
                            .into_iter()
                            .map(|arg| arg.as_str().unwrap().to_owned())
                            .collect::<Vec<String>>(),
                        vec![receiver.address().to_hex_literal(), amount.to_string(),]
                    );
                } else {
                    bail!("unexpected transaction playload")
                }
            }
            _ => bail!("Query should get user transaction"),
        }
        Ok(())
    }
}
