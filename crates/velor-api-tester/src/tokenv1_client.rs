// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// TODO: this should be part of the SDK

use anyhow::{anyhow, Context, Result};
use velor_api_types::U64;
use velor_cached_packages::velor_token_sdk_builder::EntryFunctionCall;
use velor_sdk::{
    rest_client::{Client as ApiClient, PendingTransaction},
    transaction_builder::TransactionFactory,
    types::LocalAccount,
};
use velor_types::{
    account_address::AccountAddress, chain_id::ChainId, transaction::TransactionPayload,
};
use serde::{Deserialize, Serialize};

/// Gets chain ID for use in submitting transactions.
async fn get_chain_id(client: &ApiClient) -> Result<ChainId> {
    let id = client
        .get_index()
        .await
        .context("Failed to get chain ID")?
        .inner()
        .chain_id;

    Ok(ChainId::new(id))
}

/// Helper function to take care of a transaction after creating the payload.
pub async fn build_and_submit_transaction(
    client: &ApiClient,
    account: &mut LocalAccount,
    payload: TransactionPayload,
    options: TransactionOptions,
) -> Result<PendingTransaction> {
    // create factory
    let factory = TransactionFactory::new(get_chain_id(client).await?)
        .with_gas_unit_price(options.gas_unit_price)
        .with_max_gas_amount(options.max_gas_amount)
        .with_transaction_expiration_time(options.timeout_secs);

    // create transaction
    let builder = factory
        .payload(payload)
        .sender(account.address())
        .sequence_number(account.sequence_number());

    // sign transaction
    let signed_txn = account.sign_with_transaction_builder(builder);

    // submit and return
    Ok(client
        .submit(&signed_txn)
        .await
        .context("Failed to submit transaction")?
        .into_inner())
}

#[derive(Clone, Debug)]
pub struct TokenClient<'a> {
    api_client: &'a ApiClient,
}

impl<'a> TokenClient<'a> {
    pub fn new(api_client: &'a ApiClient) -> Self {
        Self { api_client }
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

    /// Helper function to get the handle address of tokens for 0x3::token::TokenStore resources.
    async fn get_tokens_handle(&self, address: AccountAddress) -> Option<String> {
        if let Ok(response) = self
            .api_client
            .get_account_resource(address, "0x3::token::TokenStore")
            .await
        {
            Some(
                response
                    .into_inner()?
                    .data
                    .get("tokens")?
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
        // create payload
        let payload = EntryFunctionCall::TokenCreateCollectionScript {
            name: name.to_owned().into_bytes(),
            description: description.to_owned().into_bytes(),
            uri: uri.to_owned().into_bytes(),
            maximum: max_amount,
            mutate_setting: vec![false, false, false],
        }
        .encode();

        // create and submit transaction
        build_and_submit_transaction(
            self.api_client,
            account,
            payload,
            options.unwrap_or_default(),
        )
        .await
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

        // create and submit transaction
        build_and_submit_transaction(
            self.api_client,
            account,
            payload,
            options.unwrap_or_default(),
        )
        .await
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

        Ok(serde_json::from_value(value)?)
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

        Ok(serde_json::from_value(value)?)
    }

    /// Retrieves the information for a given token.
    pub async fn get_token(
        &self,
        account: AccountAddress,
        creator: AccountAddress,
        collection_name: &str,
        token_name: &str,
    ) -> Result<Token> {
        // get handle for tokens
        let handle = match self.get_tokens_handle(account).await {
            Some(s) => AccountAddress::from_hex_literal(&s)?,
            None => return Err(anyhow!("Couldn't retrieve handle for tokens")),
        };

        // construct key for table lookup
        let token_id = TokenId {
            token_data_id: TokenDataId {
                creator: creator.to_hex_literal(),
                collection: collection_name.to_string(),
                name: token_name.to_string(),
            },
            property_version: U64(0),
        };

        // get table item with the handle
        let value = self
            .api_client
            .get_table_item(handle, "0x3::token::TokenId", "0x3::token::Token", token_id)
            .await?
            .into_inner();

        Ok(serde_json::from_value(value)?)
    }

    /// Transfers specified amount of tokens from account to receiver.
    pub async fn offer_token(
        &self,
        account: &mut LocalAccount,
        receiver: AccountAddress,
        creator: AccountAddress,
        collection_name: &str,
        name: &str,
        amount: u64,
        property_version: Option<u64>,
        options: Option<TransactionOptions>,
    ) -> Result<PendingTransaction> {
        // create payload
        let payload = EntryFunctionCall::TokenTransfersOfferScript {
            receiver,
            creator,
            collection: collection_name.to_owned().into_bytes(),
            name: name.to_owned().into_bytes(),
            property_version: property_version.unwrap_or(0),
            amount,
        }
        .encode();

        // create and submit transaction
        build_and_submit_transaction(
            self.api_client,
            account,
            payload,
            options.unwrap_or_default(),
        )
        .await
    }

    pub async fn claim_token(
        &self,
        account: &mut LocalAccount,
        sender: AccountAddress,
        creator: AccountAddress,
        collection_name: &str,
        name: &str,
        property_version: Option<u64>,
        options: Option<TransactionOptions>,
    ) -> Result<PendingTransaction> {
        // create payload
        let payload = EntryFunctionCall::TokenTransfersClaimScript {
            sender,
            creator,
            collection: collection_name.to_owned().into_bytes(),
            name: name.to_owned().into_bytes(),
            property_version: property_version.unwrap_or(0),
        }
        .encode();

        // create and submit transaction
        build_and_submit_transaction(
            self.api_client,
            account,
            payload,
            options.unwrap_or_default(),
        )
        .await
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

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct CollectionData {
    pub name: String,
    pub description: String,
    pub uri: String,
    pub maximum: U64,
    pub mutability_config: CollectionMutabilityConfig,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct CollectionMutabilityConfig {
    pub description: bool,
    pub maximum: bool,
    pub uri: bool,
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

#[derive(Debug, Deserialize)]
pub struct Token {
    // id: TokenId,
    pub amount: U64,
    // todo: add property support
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenId {
    token_data_id: TokenDataId,
    property_version: U64,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenDataId {
    creator: String,
    collection: String,
    name: String,
}
