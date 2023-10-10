// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bcs,
    move_types::{
        identifier::Identifier,
        language_storage::{ModuleId},
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
use std::{
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct PackagePublisher<'a> {
    api_client: &'a ApiClient,
}

impl<'a> PackagePublisher<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn publish_package(
        &self,
        from_account: &mut LocalAccount,
        metadata_serialized: Vec<u8>,
        code: Vec<Vec<u8>>,    
        options: Option<PublishPackageOptions>,
    ) -> Result<PendingTransaction> {
        let options = options.unwrap_or_default();

        let chain_id = self
            .api_client
            .get_index()
            .await
            .context("Failed to get chain ID")?
            .inner()
            .chain_id;
        let transaction_builder = TransactionBuilder::new(
            TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(AccountAddress::ONE, Identifier::new("code").unwrap()),
                Identifier::new("publish_package_txn").unwrap(),
                vec![],
                vec![
                    bcs::to_bytes(&metadata_serialized).unwrap(),
                    bcs::to_bytes(&code).unwrap(),        
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
        Ok(self
            .api_client
            .submit(&signed_txn)
            .await
            .context("Failed to submit transfer transaction")?
            .into_inner())
    }

}

pub struct PublishPackageOptions {
    pub max_gas_amount: u64,

    pub gas_unit_price: u64,

    /// This is the number of seconds from now you're willing to wait for the
    /// transaction to be committed.
    pub timeout_secs: u64,

}

impl<'a> Default for PublishPackageOptions {
    fn default() -> Self {
        Self {
            max_gas_amount: 5_000,
            gas_unit_price: 100,
            timeout_secs: 10,
        }
    }
}
