// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    rest_client::{Client as ApiClient, PendingTransaction},
    transaction_builder::TransactionFactory,
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        LocalAccount,
    },
};
use anyhow::{Context, Result};
use aptos_cached_packages::aptos_token_sdk_builder::EntryFunctionCall;

#[derive(Clone, Debug)]
pub struct TokenClient<'a> {
    api_client: &'a ApiClient,
}

impl<'a> TokenClient<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
    }

    async fn get_chain_id(&self) -> Result<ChainId> {
        let id = self
            .api_client
            .get_index()
            .await
            .context("Failed to get chain ID")?
            .inner()
            .chain_id;

        Ok(ChainId::new(id))
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
        // create factory
        let options = options.unwrap_or_default();
        let factory = TransactionFactory::new(self.get_chain_id().await?)
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
        royalty_options: Option<RoyaltyOptions>,
        options: Option<TransactionOptions>,
    ) -> Result<PendingTransaction> {
        // create factory
        let options = options.unwrap_or_default();
        let factory = TransactionFactory::new(self.get_chain_id().await?)
            .with_gas_unit_price(options.gas_unit_price)
            .with_max_gas_amount(options.max_gas_amount)
            .with_transaction_expiration_time(options.timeout_secs);

        // set default royalty options
        let royalty_options = match royalty_options {
            Some(opt) => opt,
            None => RoyaltyOptions {
                royalty_payee_address: account.address(),
                royalty_points_denominator: 0,
                royalty_points_numerator: 0,
            },
        };

        // create payload
        let payload = EntryFunctionCall::TokenCreateTokenScript {
            collection: collection_name.to_owned().into_bytes(),
            name: name.to_owned().into_bytes(),
            description: description.to_owned().into_bytes(),
            balance: supply,
            maximum: max_amount,
            uri: uri.to_owned().into_bytes(),
            royalty_payee_address: royalty_options.royalty_payee_address,
            royalty_points_denominator: royalty_options.royalty_points_denominator,
            royalty_points_numerator: royalty_options.royalty_points_numerator,
            mutate_setting: vec![false, false, false, false, false],
            // todo: add property support
            property_keys: vec![],
            property_values: vec![],
            property_types: vec![],
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

pub struct RoyaltyOptions {
    pub royalty_payee_address: AccountAddress,
    pub royalty_points_denominator: u64,
    pub royalty_points_numerator: u64,
}
