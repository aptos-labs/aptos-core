// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::token_utils::{NAME_LENGTH, URI_LENGTH};
use crate::{
    models::{
        coin_models::{
            coin_utils::COIN_ADDR,
            v2_fungible_asset_utils::{
                FungibleAssetMetadata, FungibleAssetStore, FungibleAssetSupply,
            },
        },
        default_models::{move_resources::MoveResource, v2_objects::CurrentObjectPK},
    },
    utils::util::{
        deserialize_from_string, deserialize_token_object_property_map_from_bcs_hexstring,
        standardize_address, truncate_str,
    },
};
use anyhow::{Context, Result};
use aptos_protos::transaction::v1::{Event, WriteResource};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Formatter},
};

pub const TOKEN_V2_ADDR: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000004";

/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2AggregatedDataMapping = HashMap<CurrentObjectPK, TokenV2AggregatedData>;
/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2Burned = HashSet<CurrentObjectPK>;
/// Index of the event so that we can write its inverse to the db as primary key (to avoid collisiona)
pub type EventIndex = i64;

/// This contains both metadata for fungible assets and fungible tokens
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenV2AggregatedData {
    pub aptos_collection: Option<AptosCollection>,
    pub fixed_supply: Option<FixedSupply>,
    pub fungible_asset_metadata: Option<FungibleAssetMetadata>,
    pub fungible_asset_supply: Option<FungibleAssetSupply>,
    pub fungible_asset_store: Option<FungibleAssetStore>,
    pub object: ObjectWithMetadata,
    pub property_map: Option<PropertyMapModel>,
    pub token: Option<TokenV2>,
    pub transfer_event: Option<(EventIndex, TransferEvent)>,
    pub unlimited_supply: Option<UnlimitedSupply>,
}

/// Tracks which token standard a token / collection is built upon
#[derive(Serialize)]
pub enum TokenStandard {
    V1,
    V2,
}

impl fmt::Display for TokenStandard {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let res = match self {
            TokenStandard::V1 => "v1",
            TokenStandard::V2 => "v2",
        };
        write!(f, "{}", res)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectCore {
    pub allow_ungated_transfer: bool,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub guid_creation_num: BigDecimal,
    owner: String,
}

impl ObjectCore {
    pub fn get_owner_address(&self) -> String {
        standardize_address(&self.owner)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectWithMetadata {
    pub object_core: ObjectCore,
    state_key_hash: String,
}

impl ObjectWithMetadata {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        if let V2TokenResource::ObjectCore(inner) = V2TokenResource::from_resource(
            &type_str,
            &serde_json::from_str(write_resource.data.as_str()).unwrap(),
            txn_version,
        )? {
            Ok(Some(Self {
                object_core: inner,
                state_key_hash: standardize_address(
                    hex::encode(write_resource.state_key_hash.as_slice()).as_str(),
                ),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_state_key_hash(&self) -> String {
        standardize_address(&self.state_key_hash)
    }
}

/* Section on Collection / Token */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    creator: String,
    pub description: String,
    // These are set to private because we should never get name or uri directly
    name: String,
    uri: String,
}

impl Collection {
    pub fn get_creator_address(&self) -> String {
        standardize_address(&self.creator)
    }

    pub fn get_uri_trunc(&self) -> String {
        truncate_str(&self.uri, URI_LENGTH)
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, NAME_LENGTH)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AptosCollection {
    pub mutable_description: bool,
    pub mutable_uri: bool,
}

impl AptosCollection {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::AptosCollection(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenV2 {
    collection: ResourceReference,
    pub description: String,
    // These are set to private because we should never get name or uri directly
    name: String,
    uri: String,
}

impl TokenV2 {
    pub fn get_collection_address(&self) -> String {
        self.collection.get_reference_address()
    }

    pub fn get_uri_trunc(&self) -> String {
        truncate_str(&self.uri, URI_LENGTH)
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, NAME_LENGTH)
    }

    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::TokenV2(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceReference {
    inner: String,
}

impl ResourceReference {
    pub fn get_reference_address(&self) -> String {
        standardize_address(&self.inner)
    }
}

/* Section on Supply */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedSupply {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub current_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub max_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_minted: BigDecimal,
}

impl FixedSupply {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::FixedSupply(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnlimitedSupply {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub current_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_minted: BigDecimal,
}

impl UnlimitedSupply {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::UnlimitedSupply(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

/* Section on Events */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MintEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub index: BigDecimal,
    token: String,
}

impl MintEvent {
    pub fn get_token_address(&self) -> String {
        standardize_address(&self.token)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenMutationEvent {
    pub mutated_field_name: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BurnEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub index: BigDecimal,
    token: String,
}

impl BurnEvent {
    pub fn from_event(event: &Event, txn_version: i64) -> anyhow::Result<Option<Self>> {
        if let Some(V2TokenEvent::BurnEvent(inner)) =
            V2TokenEvent::from_event(event.type_str.as_str(), &event.data, txn_version).unwrap()
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }

    pub fn get_token_address(&self) -> String {
        standardize_address(&self.token)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferEvent {
    from: String,
    to: String,
    object: String,
}

impl TransferEvent {
    pub fn from_event(event: &Event, txn_version: i64) -> anyhow::Result<Option<Self>> {
        if let Some(V2TokenEvent::TransferEvent(inner)) =
            V2TokenEvent::from_event(event.type_str.as_str(), &event.data, txn_version).unwrap()
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }

    pub fn get_from_address(&self) -> String {
        standardize_address(&self.from)
    }

    pub fn get_to_address(&self) -> String {
        standardize_address(&self.to)
    }

    pub fn get_object_address(&self) -> String {
        standardize_address(&self.object)
    }
}

/* Section on Property Maps */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyMapModel {
    #[serde(deserialize_with = "deserialize_token_object_property_map_from_bcs_hexstring")]
    pub inner: serde_json::Value,
}

impl PropertyMapModel {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::PropertyMapModel(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum V2TokenResource {
    AptosCollection(AptosCollection),
    Collection(Collection),
    FixedSupply(FixedSupply),
    ObjectCore(ObjectCore),
    UnlimitedSupply(UnlimitedSupply),
    TokenV2(TokenV2),
    PropertyMapModel(PropertyMapModel),
}

impl V2TokenResource {
    pub fn is_resource_supported(data_type: &str) -> bool {
        [
            format!("{}::object::ObjectCore", COIN_ADDR),
            format!("{}::collection::Collection", TOKEN_V2_ADDR),
            format!("{}::collection::FixedSupply", TOKEN_V2_ADDR),
            format!("{}::collection::UnlimitedSupply", TOKEN_V2_ADDR),
            format!("{}::aptos_token::AptosCollection", TOKEN_V2_ADDR),
            format!("{}::token::Token", TOKEN_V2_ADDR),
            format!("{}::property_map::PropertyMap", TOKEN_V2_ADDR),
        ]
        .contains(&data_type.to_string())
    }

    pub fn from_resource(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Self> {
        match data_type {
            x if x == format!("{}::object::ObjectCore", COIN_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::ObjectCore(inner)))
            },
            x if x == format!("{}::collection::Collection", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::Collection(inner)))
            },
            x if x == format!("{}::collection::FixedSupply", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::FixedSupply(inner)))
            },
            x if x == format!("{}::collection::UnlimitedSupply", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::UnlimitedSupply(inner)))
            },
            x if x == format!("{}::aptos_token::AptosCollection", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::AptosCollection(inner)))
            },
            x if x == format!("{}::token::Token", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::TokenV2(inner)))
            },
            x if x == format!("{}::property_map::PropertyMap", TOKEN_V2_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(Self::PropertyMapModel(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))?
        .context(format!(
            "Resource unsupported! Call is_resource_supported first. version {} type {}",
            txn_version, data_type
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum V2TokenEvent {
    MintEvent(MintEvent),
    TokenMutationEvent(TokenMutationEvent),
    BurnEvent(BurnEvent),
    TransferEvent(TransferEvent),
}

impl V2TokenEvent {
    pub fn from_event(data_type: &str, data: &str, txn_version: i64) -> Result<Option<Self>> {
        match data_type {
            "0x4::collection::MintEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::MintEvent(inner)))
            },
            "0x4::token::MutationEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::TokenMutationEvent(inner)))
            },
            "0x4::collection::BurnEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::BurnEvent(inner)))
            },
            "0x1::object::TransferEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::TransferEvent(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
