// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::token_utils::{NAME_LENGTH, URI_LENGTH};
use crate::{
    models::{
        coin_models::v2_fungible_asset_utils::{
            FungibleAssetMetadata, FungibleAssetStore, FungibleAssetSupply,
        },
        move_resources::MoveResource,
        v2_objects::CurrentObjectPK,
    },
    util::{
        deserialize_token_object_property_map_from_bcs_hexstring, standardize_address, truncate_str,
    },
};
use anyhow::{Context, Result};
use velor_api_types::{deserialize_from_string, Event, WriteResource};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Formatter},
};

/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2AggregatedDataMapping = HashMap<CurrentObjectPK, TokenV2AggregatedData>;
/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2Burned = HashSet<CurrentObjectPK>;
/// Index of the event so that we can write its inverse to the db as primary key (to avoid collisiona)
pub type EventIndex = i64;

/// This contains both metadata for fungible assets and fungible tokens
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenV2AggregatedData {
    pub velor_collection: Option<VelorCollection>,
    pub fixed_supply: Option<FixedSupply>,
    pub fungible_asset_metadata: Option<FungibleAssetMetadata>,
    pub fungible_asset_supply: Option<FungibleAssetSupply>,
    pub fungible_asset_store: Option<FungibleAssetStore>,
    pub object: ObjectWithMetadata,
    pub property_map: Option<PropertyMap>,
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
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        if let V2TokenResource::ObjectCore(inner) = V2TokenResource::from_resource(
            &type_str,
            &serde_json::to_value(&write_resource.data.data).unwrap(),
            txn_version,
        )? {
            Ok(Some(Self {
                object_core: inner,
                state_key_hash: standardize_address(write_resource.state_key_hash.as_str()),
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
pub struct VelorCollection {
    pub mutable_description: bool,
    pub mutable_uri: bool,
}

impl VelorCollection {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::VelorCollection(inner) =
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
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
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
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
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
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
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
        let event_type = event.typ.to_string();
        if let Some(V2TokenEvent::BurnEvent(inner)) =
            V2TokenEvent::from_event(event_type.as_str(), &event.data, txn_version).unwrap()
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
        let event_type = event.typ.to_string();
        if let Some(V2TokenEvent::TransferEvent(inner)) =
            V2TokenEvent::from_event(event_type.as_str(), &event.data, txn_version).unwrap()
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
pub struct PropertyMap {
    #[serde(deserialize_with = "deserialize_token_object_property_map_from_bcs_hexstring")]
    pub inner: serde_json::Value,
}

impl PropertyMap {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::PropertyMap(inner) =
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
    VelorCollection(VelorCollection),
    Collection(Collection),
    FixedSupply(FixedSupply),
    ObjectCore(ObjectCore),
    UnlimitedSupply(UnlimitedSupply),
    TokenV2(TokenV2),
    PropertyMap(PropertyMap),
}

impl V2TokenResource {
    pub fn is_resource_supported(data_type: &str) -> bool {
        matches!(
            data_type,
            "0x1::object::ObjectCore"
                | "0x4::collection::Collection"
                | "0x4::collection::FixedSupply"
                | "0x4::collection::UnlimitedSupply"
                | "0x4::velor_token::VelorCollection"
                | "0x4::token::Token"
                | "0x4::property_map::PropertyMap"
        )
    }

    pub fn from_resource(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Self> {
        match data_type {
            "0x1::object::ObjectCore" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::ObjectCore(inner)))
            },
            "0x4::collection::Collection" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::Collection(inner)))
            },
            "0x4::collection::FixedSupply" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::FixedSupply(inner)))
            },
            "0x4::collection::UnlimitedSupply" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::UnlimitedSupply(inner)))
            },
            "0x4::velor_token::VelorCollection" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::VelorCollection(inner)))
            },
            "0x4::token::Token" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::TokenV2(inner)))
            },
            "0x4::property_map::PropertyMap" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::PropertyMap(inner)))
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
    pub fn from_event(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Option<Self>> {
        match data_type {
            "0x4::collection::MintEvent" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::MintEvent(inner)))
            },
            "0x4::token::MutationEvent" => serde_json::from_value(data.clone())
                .map(|inner| Some(Self::TokenMutationEvent(inner))),
            "0x4::collection::BurnEvent" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::BurnEvent(inner)))
            },
            "0x1::object::TransferEvent" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::TransferEvent(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
