// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bcs,
    move_types::{
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
    },
    rest_client::{Client as ApiClient, PendingTransaction},
    transaction_builder::TransactionBuilder,
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{EntryFunction, TransactionPayload},
        LocalAccount,
    },
};
use anyhow::{Context, Result};
use velor_types::transaction::SignedTransaction;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct CoinClient<'a> {
    api_client: &'a ApiClient,
}

impl<'a> CoinClient<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn transfer(
        &self,
        from_account: &mut LocalAccount,
        to_account: AccountAddress,
        amount: u64,
        options: Option<TransferOptions<'_>>,
    ) -> Result<PendingTransaction> {
        let signed_txn = self
            .get_signed_transfer_txn(from_account, to_account, amount, options)
            .await?;
        Ok(self
            .api_client
            .submit(&signed_txn)
            .await
            .context("Failed to submit transfer transaction")?
            .into_inner())
        // <:!:section_1
    }

    pub async fn get_signed_transfer_txn(
        &self,
        from_account: &mut LocalAccount,
        to_account: AccountAddress,
        amount: u64,
        options: Option<TransferOptions<'_>>,
    ) -> Result<SignedTransaction> {
        let options = options.unwrap_or_default();

        // :!:>section_1
        let chain_id = self
            .api_client
            .get_index()
            .await
            .context("Failed to get chain ID")?
            .inner()
            .chain_id;
        let transaction_builder = TransactionBuilder::new(
            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(
                    AccountAddress::ONE,
                    Identifier::new("velor_account").unwrap(),
                ),
                Identifier::new("transfer_coins").unwrap(),
                vec![TypeTag::from_str(options.coin_type).unwrap()],
                vec![
                    bcs::to_bytes(&to_account).unwrap(),
                    bcs::to_bytes(&amount).unwrap(),
                ],
            )),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + options.timeout_secs,
            ChainId::new(chain_id),
        )
        .sender(from_account.address())
        .sequence_number(from_account.sequence_number())
        .max_gas_amount(options.max_gas_amount)
        .gas_unit_price(options.gas_unit_price);
        let signed_txn = from_account.sign_with_transaction_builder(transaction_builder);
        Ok(signed_txn)
    }

    pub async fn get_account_balance(&self, account: &AccountAddress) -> Result<u64> {
        let response = self
            .api_client
            .view_apt_account_balance(*account)
            .await
            .context("Failed to get account balance")?;
        Ok(response.into_inner())
    }
}

pub struct TransferOptions<'a> {
    pub max_gas_amount: u64,

    pub gas_unit_price: u64,

    /// This is the number of seconds from now you're willing to wait for the
    /// transaction to be committed.
    pub timeout_secs: u64,

    /// This is the coin type to transfer.
    pub coin_type: &'a str,
}

impl Default for TransferOptions<'_> {
    fn default() -> Self {
        Self {
            max_gas_amount: 5_000,
            gas_unit_price: 100,
            timeout_secs: 10,
            coin_type: "0x1::velor_coin::VelorCoin",
        }
    }
}
