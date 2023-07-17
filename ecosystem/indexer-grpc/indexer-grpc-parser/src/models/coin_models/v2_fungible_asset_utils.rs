// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::coin_utils::COIN_ADDR;
use crate::{
    models::{
        default_models::move_resources::MoveResource,
        token_models::{token_utils::URI_LENGTH, v2_token_utils::ResourceReference},
    },
    utils::util::{deserialize_from_string, truncate_str},
};
use anyhow::{Context, Result};
use aptos_protos::transaction::v1::WriteResource;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

const FUNGIBLE_ASSET_LENGTH: usize = 32;
const FUNGIBLE_ASSET_SYMBOL: usize = 10;

/* Section on fungible assets resources */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FungibleAssetMetadata {
    name: String,
    symbol: String,
    pub decimals: i32,
    icon_uri: String,
    project_uri: String,
}

impl FungibleAssetMetadata {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2FungibleAssetResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2FungibleAssetResource::FungibleAssetMetadata(inner) =
            V2FungibleAssetResource::from_resource(
                &type_str,
                resource.data.as_ref().unwrap(),
                txn_version,
            )?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }

    pub fn get_name(&self) -> String {
        truncate_str(&self.name, FUNGIBLE_ASSET_LENGTH)
    }

    pub fn get_symbol(&self) -> String {
        truncate_str(&self.name, FUNGIBLE_ASSET_SYMBOL)
    }

    pub fn get_icon_uri(&self) -> String {
        truncate_str(&self.icon_uri, URI_LENGTH)
    }

    pub fn get_project_uri(&self) -> String {
        truncate_str(&self.project_uri, URI_LENGTH)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FungibleAssetStore {
    pub metadata: ResourceReference,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub balance: BigDecimal,
    pub frozen: bool,
}

impl FungibleAssetStore {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2FungibleAssetResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2FungibleAssetResource::FungibleAssetStore(inner) =
            V2FungibleAssetResource::from_resource(
                &type_str,
                resource.data.as_ref().unwrap(),
                txn_version,
            )?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FungibleAssetSupply {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub current: BigDecimal,
    pub maximum: OptionalBigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OptionalBigDecimal {
    vec: Vec<BigDecimalWrapper>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BigDecimalWrapper(#[serde(deserialize_with = "deserialize_from_string")] pub BigDecimal);

impl FungibleAssetSupply {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str: String = MoveResource::get_outer_type_from_resource(write_resource);
        if !V2FungibleAssetResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2FungibleAssetResource::FungibleAssetSupply(inner) =
            V2FungibleAssetResource::from_resource(
                &type_str,
                resource.data.as_ref().unwrap(),
                txn_version,
            )?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }

    pub fn get_maximum(&self) -> Option<BigDecimal> {
        self.maximum.vec.first().map(|x| x.0.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DepositEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithdrawEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub amount: BigDecimal,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum V2FungibleAssetResource {
    FungibleAssetMetadata(FungibleAssetMetadata),
    FungibleAssetStore(FungibleAssetStore),
    FungibleAssetSupply(FungibleAssetSupply),
}

impl V2FungibleAssetResource {
    pub fn is_resource_supported(data_type: &str) -> bool {
        [
            format!("{}::fungible_asset::Supply", COIN_ADDR),
            format!("{}::fungible_asset::Metadata", COIN_ADDR),
            format!("{}::fungible_asset::FungibleStore", COIN_ADDR),
        ]
        .contains(&data_type.to_string())
    }

    pub fn from_resource(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Self> {
        match data_type {
            x if x == format!("{}::fungible_asset::Supply", COIN_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(Self::FungibleAssetSupply(inner)))
            },
            x if x == format!("{}::fungible_asset::Metadata", COIN_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(Self::FungibleAssetMetadata(inner)))
            },
            x if x == format!("{}::fungible_asset::FungibleStore", COIN_ADDR) => {
                serde_json::from_value(data.clone())
                    .map(|inner| Some(Self::FungibleAssetStore(inner)))
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

pub enum FungibleAssetEvent {
    DepositEvent(DepositEvent),
    WithdrawEvent(WithdrawEvent),
}

impl FungibleAssetEvent {
    pub fn from_event(data_type: &str, data: &str, txn_version: i64) -> Result<Option<Self>> {
        match data_type {
            "0x1::fungible_asset::DepositEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::DepositEvent(inner)))
            },
            "0x1::fungible_asset::WithdrawEvent" => {
                serde_json::from_str(data).map(|inner| Some(Self::WithdrawEvent(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fungible_asset_supply_null() {
        let test = r#"{"current": "0", "maximum": {"vec": []}}"#;
        let test: serde_json::Value = serde_json::from_str(test).unwrap();
        let supply = serde_json::from_value(test)
            .map(V2FungibleAssetResource::FungibleAssetSupply)
            .unwrap();
        if let V2FungibleAssetResource::FungibleAssetSupply(supply) = supply {
            assert_eq!(supply.current, BigDecimal::from(0));
            assert_eq!(supply.get_maximum(), None);
        } else {
            panic!("Wrong type")
        }
    }

    #[test]
    fn test_fungible_asset_supply_nonnull() {
        let test = r#"{"current": "100", "maximum": {"vec": ["5000"]}}"#;
        let test: serde_json::Value = serde_json::from_str(test).unwrap();
        let supply = serde_json::from_value(test)
            .map(V2FungibleAssetResource::FungibleAssetSupply)
            .unwrap();
        if let V2FungibleAssetResource::FungibleAssetSupply(supply) = supply {
            assert_eq!(supply.current, BigDecimal::from(100));
            assert_eq!(supply.get_maximum(), Some(BigDecimal::from(5000)));
        } else {
            panic!("Wrong type")
        }
    }
}
