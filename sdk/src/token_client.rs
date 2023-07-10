// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bcs,
    move_types::{identifier::Identifier, language_storage::ModuleId},
    rest_client::{Client as ApiClient, PendingTransaction},
    transaction_builder::{TransactionBuilder, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{EntryFunction, TransactionPayload},
        LocalAccount,
    },
};
use anyhow::{Context, Result};
use aptos_cached_packages::aptos_token_sdk_builder::EntryFunctionCall;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct TokenClient<'a> {
    api_client: &'a ApiClient,
}

impl<'a> TokenClient<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
    }

    pub async fn create_collection(
        &self,
        account: &mut LocalAccount,
        name: &str,
        description: &str,
        uri: &str,
        max_amount: u64,
        options: Option<TransactionOptions>,
    ) -> Result<PendingTransaction> {
        let options = options.unwrap_or_default();

        // get chain id
        let chain_id = self
            .api_client
            .get_index()
            .await
            .context("Failed to get chain ID")?
            .inner()
            .chain_id;

        // create factory
        let factory = TransactionFactory::new(ChainId::new(chain_id))
            .with_gas_unit_price(options.gas_unit_price)
            .with_max_gas_amount(options.max_gas_amount)
            .with_transaction_expiration_time(options.timeout_secs);

        // create payload
        let payload = EntryFunctionCall::TokenCreateCollectionScript {
            name: name.to_owned().into_bytes(),
            description: description.to_owned().into_bytes(),
            uri: uri.to_owned().into_bytes(),
            maximum: max_amount,
            mutate_setting: vec![false, false, false],
        }
        .encode();

        // create transaction
        let builder = factory
            .payload(payload)
            .sender(account.address())
            .sequence_number(account.sequence_number());
        let signed_txn = account.sign_with_transaction_builder(builder);

        // submit and return
        Ok(self
            .api_client
            .submit(&signed_txn)
            .await
            .context("Failed to submit transfer transaction")?
            .into_inner())
    }

    pub async fn create_token(
        &self,
        account: &mut LocalAccount,
        collection_name: &str,
        name: &str,
        description: &str,
        supply: u64,
        uri: &str,
        max_amount: u64,
        royalty_payee_address: &AccountAddress,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        property_keys: Box<Vec<&str>>,
        property_values: Box<Vec<&str>>,
        property_types: Box<Vec<&str>>,
        options: Option<TransactionOptions>,
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
                ModuleId::new(
                    AccountAddress::from_hex_literal("0x3").unwrap(),
                    Identifier::new("token").unwrap(),
                ),
                Identifier::new("create_token_script").unwrap(),
                vec![],
                vec![
                    bcs::to_bytes(&collection_name).unwrap(),
                    bcs::to_bytes(&name).unwrap(),
                    bcs::to_bytes(&description).unwrap(),
                    bcs::to_bytes(&supply).unwrap(),
                    bcs::to_bytes(&max_amount).unwrap(),
                    bcs::to_bytes(&uri).unwrap(),
                    bcs::to_bytes(&royalty_payee_address).unwrap(),
                    bcs::to_bytes(&royalty_points_denominator).unwrap(),
                    bcs::to_bytes(&royalty_points_numerator).unwrap(),
                    bcs::to_bytes(&vec![false, false, false, false, false]).unwrap(),
                    bcs::to_bytes(property_keys.as_ref()).unwrap(),
                    // TODO: this is wrong
                    bcs::to_bytes(property_values.as_ref()).unwrap(),
                    bcs::to_bytes(property_types.as_ref()).unwrap(),
                ],
            )),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + options.timeout_secs,
            ChainId::new(chain_id),
        )
        .sender(account.address())
        .sequence_number(account.sequence_number())
        .max_gas_amount(options.max_gas_amount)
        .gas_unit_price(options.gas_unit_price);
        let signed_txn = account.sign_with_transaction_builder(transaction_builder);
        Ok(self
            .api_client
            .submit(&signed_txn)
            .await
            .context("Failed to submit transfer transaction")?
            .into_inner())
    }
}

pub struct TransactionOptions {
    pub max_gas_amount: u64,

    pub gas_unit_price: u64,

    /// This is the number of seconds from now you're willing to wait for the
    /// transaction to be committed.
    pub timeout_secs: u64,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            max_gas_amount: 5_000,
            gas_unit_price: 100,
            timeout_secs: 10,
        }
    }
}
