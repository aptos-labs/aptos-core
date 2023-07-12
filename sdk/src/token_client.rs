// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    rest_client::{Client as ApiClient, PendingTransaction},
    transaction_builder::TransactionFactory,
    types::{account_address::AccountAddress, chain_id::ChainId, LocalAccount},
};
use anyhow::{anyhow, Context, Result};
use aptos_api_types::U64;
use aptos_cached_packages::aptos_token_sdk_builder::EntryFunctionCall;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TokenClient<'a> {
    api_client: &'a ApiClient,
}

impl<'a> TokenClient<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
    }

    /// Gets chain ID for use in submitting transactions.
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

    /// Helper function to get the handle address of collection_data for 0x3::token::Collections
    /// resources.
    async fn get_collection_data_handle(&self, address: AccountAddress) -> Option<String> {
        if let Ok(response) = self
            .api_client
            .get_account_resource(address, "0x3::token::Collections")
            .await
        {
            Some(
                response
                    .into_inner()?
                    .data
                    .get("collection_data")?
                    .get("handle")?
                    .as_str()?
                    .to_owned(),
            )
        } else {
            None
        }
    }

    /// Helper function to get the handle address of token_data for 0x3::token::Collections
    /// resources.
    async fn get_token_data_handle(&self, address: AccountAddress) -> Option<String> {
        if let Ok(response) = self
            .api_client
            .get_account_resource(address, "0x3::token::Collections")
            .await
        {
            Some(
                response
                    .into_inner()?
                    .data
                    .get("token_data")?
                    .get("handle")?
                    .as_str()?
                    .to_owned(),
            )
        } else {
            None
        }
    }

    /// Creates a collection with the given fields.
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

    /// Creates a token with the given fields. Does not support property keys.
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
                payee_address: account.address(),
                royalty_points_denominator: U64(0),
                royalty_points_numerator: U64(0),
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
            royalty_payee_address: royalty_options.payee_address,
            royalty_points_denominator: royalty_options.royalty_points_denominator.0,
            royalty_points_numerator: royalty_options.royalty_points_numerator.0,
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

    /// Retrieves collection metadata from the API.
    pub async fn get_collection_data(
        &self,
        creator: AccountAddress,
        collection_name: &str,
    ) -> Result<CollectionData> {
        // get handle for collection_data
        let handle = match self.get_collection_data_handle(creator).await {
            Some(s) => AccountAddress::from_hex_literal(&s)?,
            None => return Err(anyhow!("Couldn't retrieve handle for collections data")),
        };

        // get table item with the handle
        let value = self
            .api_client
            .get_table_item(
                handle,
                "0x1::string::String",
                "0x3::token::CollectionData",
                collection_name,
            )
            .await?
            .into_inner();

        // reconstruct from strings
        let response: CollectionData = serde_json::from_value(value)?;
        Ok(response)
    }

    /// Retrieves token metadata from the API.
    pub async fn get_token_data(
        &self,
        creator: AccountAddress,
        collection_name: &str,
        token_name: &str,
    ) -> Result<TokenData> {
        // get handle for token_data
        let handle = match self.get_token_data_handle(creator).await {
            Some(s) => AccountAddress::from_hex_literal(&s)?,
            None => return Err(anyhow!("Couldn't retrieve handle for token data")),
        };

        // construct key for table lookup
        let token_data_id = TokenDataId {
            creator: creator.to_hex_literal(),
            collection: collection_name.to_string(),
            name: token_name.to_string(),
        };

        // get table item with the handle
        let value = self
            .api_client
            .get_table_item(
                handle,
                "0x3::token::TokenDataId",
                "0x3::token::TokenData",
                token_data_id,
            )
            .await?
            .into_inner();

        // reconstruct from strings
        let response: TokenData = serde_json::from_value(value.clone())?;
        Ok(response)
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

#[derive(Debug, PartialEq, Deserialize)]
pub struct CollectionData {
    pub name: String,
    pub description: String,
    pub uri: String,
    pub maximum: U64,
    pub mutability_config: CollectionMutabilityConfig,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct CollectionMutabilityConfig {
    pub description: bool,
    pub maximum: bool,
    pub uri: bool,
}

#[derive(Serialize)]
struct TokenDataId {
    creator: String,
    collection: String,
    name: String,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct TokenData {
    pub name: String,
    pub description: String,
    pub uri: String,
    pub maximum: U64,
    pub supply: U64,
    pub royalty: RoyaltyOptions,
    pub mutability_config: TokenMutabilityConfig,
    pub largest_property_version: U64,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct RoyaltyOptions {
    pub payee_address: AccountAddress,
    pub royalty_points_denominator: U64,
    pub royalty_points_numerator: U64,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct TokenMutabilityConfig {
    pub description: bool,
    pub maximum: bool,
    pub properties: bool,
    pub royalty: bool,
    pub uri: bool,
}
