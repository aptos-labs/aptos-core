// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::{transaction_builder::TransactionBuilder, types::LocalAccount};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{
    account_address::AccountAddress, account_config::aptos_root_address, chain_id::ChainId,
};
use forge::{AptosContext, AptosTest, Result, Test};

pub struct ErrorReport;

impl Test for ErrorReport {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::error-report"
    }
}

async fn submit_and_check_err<F: Fn(TransactionBuilder) -> TransactionBuilder>(
    local_account: &LocalAccount,
    ctx: &mut AptosContext<'_>,
    f: F,
    expected: &str,
) {
    let payload = ctx
        .aptos_transaction_factory()
        .payload(aptos_stdlib::encode_test_coin_claim_mint_capability())
        .sequence_number(0);
    let txn = local_account.sign_transaction(f(payload).build());
    let err = format!(
        "{:?}",
        ctx.client().submit_and_wait(&txn).await.unwrap_err()
    );
    assert!(err.contains(expected), "{}", err);
}

#[async_trait::async_trait]
impl AptosTest for ErrorReport {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let local_account = ctx.random_account();
        let address = local_account.address();
        ctx.create_user_account(local_account.public_key()).await?;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(address),
            "INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE",
        )
        .await;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(address).gas_unit_price(0),
            "GAS_UNIT_PRICE_BELOW_MIN_BOUND",
        )
        .await;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(address).chain_id(ChainId::new(100)),
            "BAD_CHAIN_ID",
        )
        .await;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(AccountAddress::random()),
            "SENDING_ACCOUNT_DOES_NOT_EXIST",
        )
        .await;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(aptos_root_address()),
            "SEQUENCE_NUMBER_TOO_OLD",
        )
        .await;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(aptos_root_address()).sequence_number(1),
            "INVALID_AUTH_KEY",
        )
        .await;
        ctx.mint(address, 100000).await?;
        submit_and_check_err(
            &local_account,
            ctx,
            |t| t.sender(address).expiration_timestamp_secs(0),
            "TRANSACTION_EXPIRED",
        )
        .await;
        Ok(())
    }
}
