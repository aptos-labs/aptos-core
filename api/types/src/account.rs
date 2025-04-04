// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{Address, HexEncodedBytes, MoveStructTag, U64};
use aptos_types::account_config::AccountResource;
use poem_openapi::Object;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Debug, str::FromStr};

/// Account data
///
/// A simplified version of the onchain Account resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: HexEncodedBytes,
    // Stateless accounts donot have `0x1::Account` resource.
    // If the `0x1::Account` resource doesn't exist, the above sequence_number is set to 0,
    // and the below state_exists is set to false.
    pub state_exists: bool,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        let authentication_key: HexEncodedBytes = ar.authentication_key().to_vec().into();
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key,
            state_exists: true,
        }
    }
}

/// An Enum for referencing an asset type, either coin or fungible asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetType {
    Coin(MoveStructTag),
    FungibleAsset(Address),
}

impl FromStr for AssetType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Address::from_str(s) {
            Ok(address) => Ok(AssetType::FungibleAsset(address)),
            Err(_) => match MoveStructTag::from_str(s) {
                Ok(tag) => Ok(AssetType::Coin(tag)),
                Err(e) => Err(e),
            },
        }
    }
}
impl Serialize for AssetType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Coin(struct_tag) => MoveStructTag::serialize(struct_tag, serializer),
            Self::FungibleAsset(addr) => Address::serialize(addr, serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AssetType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <String>::deserialize(deserializer)?;
        data.parse().map_err(D::Error::custom)
    }
}
