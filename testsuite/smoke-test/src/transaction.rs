// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use diem_rest_client::{
    diem_api_types::{HexEncodedBytes, ScriptPayload, TransactionPayload},
    Transaction,
};
use diem_sdk::{
    client::SignedTransaction,
    crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform},
    transaction_builder::Currency,
    types::{account_config::XUS_NAME, transaction::authenticator::AuthenticationKey},
};
use forge::{PublicUsageContext, PublicUsageTest, Result, Test};
use tokio::runtime::Runtime;

pub struct ExternalTransactionSigner;

impl Test for ExternalTransactionSigner {
    fn name(&self) -> &'static str {
        "smoke-test::external-transaction-signer"
    }
}

impl PublicUsageTest for ExternalTransactionSigner {
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(self.async_run(ctx))
    }
}

impl ExternalTransactionSigner {
    async fn async_run(&self, ctx: &mut PublicUsageContext<'_>) -> Result<()> {
        let client = ctx.rest_client();

        // generate key pair
        let private_key = Ed25519PrivateKey::generate(ctx.rng());
        let public_key = private_key.public_key();

        // create transfer parameters
        let sender_auth_key = AuthenticationKey::ed25519(&public_key);
        let sender_address = sender_auth_key.derived_address();
        ctx.create_parent_vasp_account(sender_auth_key).await?;
        ctx.fund(sender_address, 10_000_000).await?;

        let receiver = ctx.random_account();
        ctx.create_parent_vasp_account(receiver.authentication_key())
            .await?;
        ctx.fund(receiver.address(), 1_000_000).await?;

        let amount = 1_000_000;
        let test_gas_unit_price = 1;
        let test_max_gas_amount = 1_000_000;

        // prepare transfer transaction
        let test_sequence_number = client
            .get_account(sender_address)
            .await?
            .into_inner()
            .sequence_number;

        let currency_code = XUS_NAME;

        let unsigned_txn = ctx
            .transaction_factory()
            .with_diem_version(0) // Force Script not ScriptFunctions
            .peer_to_peer(Currency::XUS, receiver.address(), amount)
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
                assert_eq!(
                    user_txn.request.gas_currency_code,
                    currency_code.to_string()
                );
                assert_eq!(user_txn.request.max_gas_amount.0, test_max_gas_amount);

                if let TransactionPayload::ScriptPayload(ScriptPayload {
                    code,
                    type_arguments,
                    arguments,
                }) = user_txn.request.payload
                {
                    let expected_code = match unsigned_txn.clone().into_payload() {
                        diem_types::transaction::TransactionPayload::Script(script) => {
                            HexEncodedBytes::from(script.code().to_vec())
                        }
                        _ => bail!("unexpected transaction payload: {:?}", &unsigned_txn),
                    };
                    assert_eq!(code.bytecode, expected_code);
                    assert_eq!(
                        type_arguments
                            .into_iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<String>>(),
                        vec!["0x1::XUS::XUS"]
                    );
                    assert_eq!(
                        arguments
                            .into_iter()
                            .map(|arg| arg.as_str().unwrap().to_owned())
                            .collect::<Vec<String>>(),
                        vec![
                            receiver.address().to_hex_literal(),
                            amount.to_string(),
                            "0x".to_string(),
                            "0x".to_string()
                        ]
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
