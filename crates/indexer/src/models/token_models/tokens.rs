// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    collection_datas::{CollectionData, CurrentCollectionData},
    token_claims::CurrentTokenPendingClaim,
    token_datas::{CurrentTokenData, TokenData},
    token_ownerships::{CurrentTokenOwnership, TokenOwnership},
    token_utils::{TokenResource, TokenWriteSet},
};
use crate::{
    database::PgPoolConnection, models::move_resources::MoveResource, schema::tokens,
    util::ensure_not_negative,
};
use aptos_api_types::{
    DeleteTableItem as APIDeleteTableItem, Transaction as APITransaction,
    WriteResource as APIWriteResource, WriteSetChange as APIWriteSetChange,
    WriteTableItem as APIWriteTableItem,
};
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type TableHandle = String;
type Address = String;
type TableType = String;
pub type TableHandleToOwner = HashMap<TableHandle, TableMetadataForToken>;
pub type TokenDataIdHash = String;
// PK of current_token_ownerships, i.e. token_data_id_hash + property_version + owner_address, used to dedupe
pub type CurrentTokenOwnershipPK = (TokenDataIdHash, BigDecimal, Address);
// PK of current_token_pending_claims, i.e. token_data_id_hash + property_version + to/from_address, used to dedupe
pub type CurrentTokenPendingClaimPK = (TokenDataIdHash, BigDecimal, Address, Address);
// PK of tokens table, used to dedupe tokens
pub type TokenPK = (TokenDataIdHash, BigDecimal);

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(token_data_id_hash, property_version, transaction_version))]
#[diesel(table_name = tokens)]
pub struct Token {
    pub token_data_id_hash: String,
    pub property_version: BigDecimal,
    pub transaction_version: i64,
    pub creator_address: String,
    pub collection_name: String,
    pub name: String,
    pub token_properties: serde_json::Value,
    pub inserted_at: chrono::NaiveDateTime,
    pub collection_data_id_hash: String,
}

#[derive(Debug)]
pub struct TableMetadataForToken {
    pub owner_address: Address,
    pub table_type: TableType,
}

impl Token {
    /// We can find token data from write sets in user transactions. Table items will contain metadata for collections
    /// and tokens. To find ownership, we have to look in write resource write sets for who owns those table handles
    ///
    /// We also will compute current versions of the token tables which are at a higher granularity than the transactional tables (only
    /// state at the last transaction will be tracked, hence using hashmap to dedupe)
    pub fn from_transaction(
        transaction: &APITransaction,
        conn: &mut PgPoolConnection,
    ) -> (
        Vec<Self>,
        Vec<TokenOwnership>,
        Vec<TokenData>,
        Vec<CollectionData>,
        HashMap<CurrentTokenOwnershipPK, CurrentTokenOwnership>,
        HashMap<TokenDataIdHash, CurrentTokenData>,
        HashMap<TokenDataIdHash, CurrentCollectionData>,
        HashMap<CurrentTokenPendingClaimPK, CurrentTokenPendingClaim>,
    ) {
        if let APITransaction::UserTransaction(user_txn) = transaction {
            let mut token_ownerships = vec![];
            let mut token_datas = vec![];
            let mut collection_datas = vec![];

            let mut tokens: HashMap<TokenPK, Token> = HashMap::new();
            let mut current_token_ownerships: HashMap<
                CurrentTokenOwnershipPK,
                CurrentTokenOwnership,
            > = HashMap::new();
            let mut current_token_datas: HashMap<TokenDataIdHash, CurrentTokenData> =
                HashMap::new();
            let mut current_collection_datas: HashMap<TokenDataIdHash, CurrentCollectionData> =
                HashMap::new();
            let mut current_token_claims: HashMap<
                CurrentTokenPendingClaimPK,
                CurrentTokenPendingClaim,
            > = HashMap::new();

            let txn_version = user_txn.info.version.0 as i64;
            let mut table_handle_to_owner: TableHandleToOwner = HashMap::new();
            for wsc in &user_txn.info.changes {
                if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                    let maybe_map = TableMetadataForToken::get_table_handle_to_owner(
                        write_resource,
                        txn_version,
                    )
                    .unwrap();
                    if let Some(map) = maybe_map {
                        table_handle_to_owner.extend(map);
                    }
                }
            }

            for wsc in &user_txn.info.changes {
                // Basic token and ownership data
                let (maybe_token_w_ownership, maybe_token_data, maybe_collection_data) = match wsc {
                    APIWriteSetChange::WriteTableItem(write_table_item) => (
                        Self::from_write_table_item(
                            write_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap(),
                        TokenData::from_write_table_item(write_table_item, txn_version).unwrap(),
                        CollectionData::from_write_table_item(
                            write_table_item,
                            txn_version,
                            &table_handle_to_owner,
                            conn,
                        )
                        .unwrap(),
                    ),
                    APIWriteSetChange::DeleteTableItem(delete_table_item) => (
                        Self::from_delete_table_item(
                            delete_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap(),
                        None,
                        None,
                    ),
                    _ => (None, None, None),
                };
                // More advanced token contracts
                let maybe_current_token_claim = match wsc {
                    APIWriteSetChange::WriteTableItem(write_table_item) => {
                        CurrentTokenPendingClaim::from_write_table_item(
                            write_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap()
                    }
                    APIWriteSetChange::DeleteTableItem(delete_table_item) => {
                        CurrentTokenPendingClaim::from_delete_table_item(
                            delete_table_item,
                            txn_version,
                            &table_handle_to_owner,
                        )
                        .unwrap()
                    }
                    _ => None,
                };

                if let Some((token, token_ownership, maybe_current_token_ownership)) =
                    maybe_token_w_ownership
                {
                    tokens.insert(
                        (
                            token.token_data_id_hash.clone(),
                            token.property_version.clone(),
                        ),
                        token,
                    );
                    token_ownerships.push(token_ownership);
                    if let Some(current_token_ownership) = maybe_current_token_ownership {
                        current_token_ownerships.insert(
                            (
                                current_token_ownership.token_data_id_hash.clone(),
                                current_token_ownership.property_version.clone(),
                                current_token_ownership.owner_address.clone(),
                            ),
                            current_token_ownership,
                        );
                    }
                }
                if let Some((token_data, current_token_data)) = maybe_token_data {
                    token_datas.push(token_data);
                    current_token_datas.insert(
                        current_token_data.token_data_id_hash.clone(),
                        current_token_data,
                    );
                }
                if let Some((collection_data, current_collection_data)) = maybe_collection_data {
                    collection_datas.push(collection_data);
                    current_collection_datas.insert(
                        current_collection_data.collection_data_id_hash.clone(),
                        current_collection_data,
                    );
                }
                if let Some(claim) = maybe_current_token_claim {
                    current_token_claims.insert(
                        (
                            claim.token_data_id_hash.clone(),
                            claim.property_version.clone(),
                            claim.from_address.clone(),
                            claim.to_address.clone(),
                        ),
                        claim,
                    );
                }
            }
            return (
                tokens.into_values().collect(),
                token_ownerships,
                token_datas,
                collection_datas,
                current_token_ownerships,
                current_token_datas,
                current_collection_datas,
                current_token_claims,
            );
        }
        Default::default()
    }

    /// Get token from write table item. Table items don't have address of the table so we need to look it up in the table_handle_to_owner mapping
    /// We get the mapping from resource.
    /// If the mapping is missing we'll just leave owner address as blank. This isn't great but at least helps us account for the token
    pub fn from_write_table_item(
        table_item: &APIWriteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, TokenOwnership, Option<CurrentTokenOwnership>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token = match TokenWriteSet::from_table_item_type(
            table_item_data.value_type.as_str(),
            &table_item_data.value,
            txn_version,
        )? {
            Some(TokenWriteSet::Token(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token) = maybe_token {
            let token_id = token.id;
            let token_data_id = token_id.token_data_id;
            let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
            let token_data_id_hash = token_data_id.to_hash();
            let collection_name = token_data_id.get_collection_trunc();
            let name = token_data_id.get_name_trunc();

            let token_pg = Self {
                collection_data_id_hash,
                token_data_id_hash,
                creator_address: token_data_id.creator,
                collection_name,
                name,
                property_version: token_id.property_version,
                transaction_version: txn_version,
                token_properties: token.token_properties,
                inserted_at: chrono::Utc::now().naive_utc(),
            };

            let (token_ownership, current_token_ownership) = TokenOwnership::from_token(
                &token_pg,
                ensure_not_negative(token.amount),
                table_item.handle.to_string(),
                table_handle_to_owner,
                Some(table_item_data.value_type.as_str()),
            );

            Ok(Some((token_pg, token_ownership, current_token_ownership)))
        } else {
            Ok(None)
        }
    }

    /// Get token from delete table item. The difference from write table item is that value isn't there so
    /// we'll set amount to 0 and token property to blank.
    pub fn from_delete_table_item(
        table_item: &APIDeleteTableItem,
        txn_version: i64,
        table_handle_to_owner: &TableHandleToOwner,
    ) -> anyhow::Result<Option<(Self, TokenOwnership, Option<CurrentTokenOwnership>)>> {
        let table_item_data = table_item.data.as_ref().unwrap();

        let maybe_token_id = match TokenWriteSet::from_table_item_type(
            table_item_data.key_type.as_str(),
            &table_item_data.key,
            txn_version,
        )? {
            Some(TokenWriteSet::TokenId(inner)) => Some(inner),
            _ => None,
        };

        if let Some(token_id) = maybe_token_id {
            let token_data_id = token_id.token_data_id;
            let collection_data_id_hash = token_data_id.get_collection_data_id_hash();
            let token_data_id_hash = token_data_id.to_hash();
            let collection_name = token_data_id.get_collection_trunc();
            let name = token_data_id.get_name_trunc();

            let token = Self {
                collection_data_id_hash,
                token_data_id_hash,
                creator_address: token_data_id.creator,
                collection_name,
                name,
                property_version: token_id.property_version,
                transaction_version: txn_version,
                token_properties: serde_json::Value::Null,
                inserted_at: chrono::Utc::now().naive_utc(),
            };
            let (token_ownership, current_token_ownership) = TokenOwnership::from_token(
                &token,
                BigDecimal::zero(),
                table_item.handle.to_string(),
                table_handle_to_owner,
                None,
            );
            Ok(Some((token, token_ownership, current_token_ownership)))
        } else {
            Ok(None)
        }
    }
}

impl TableMetadataForToken {
    /// Mapping from table handle to owner type, including type of the table (AKA resource type)
    fn get_table_handle_to_owner(
        write_resource: &APIWriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<TableHandleToOwner>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0,
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        let value = TableMetadataForToken {
            owner_address: resource.address,
            table_type: write_resource.data.typ.to_string(),
        };
        let table_handle: TableHandle = match TokenResource::from_resource(
            &type_str,
            resource.data.as_ref().unwrap(),
            txn_version,
        )? {
            TokenResource::CollectionResource(collection_resource) => {
                collection_resource.collection_data.handle
            }
            TokenResource::TokenStoreResource(inner) => inner.tokens.handle,
            TokenResource::PendingClaimsResource(inner) => inner.pending_claims.handle,
        };
        Ok(Some(HashMap::from([(
            Self::standardize_handle(&table_handle),
            value,
        )])))
    }

    /// Removes leading 0s after 0x in a table to standardize between resources and table items
    pub fn standardize_handle(handle: &str) -> String {
        format!("0x{}", &handle[2..].trim_start_matches('0'))
    }
}
