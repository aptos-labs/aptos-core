// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod crypto_kv_storage;
mod crypto_storage;
mod error;
mod in_memory;
mod kv_storage;
mod namespaced;
mod on_disk;
mod policy;
mod storage;
mod vault;

pub use crate::{
    crypto_kv_storage::CryptoKVStorage,
    crypto_storage::{CryptoStorage, PublicKeyResponse},
    error::Error,
    in_memory::InMemoryStorage,
    kv_storage::{GetResponse, KVStorage},
    namespaced::Namespaced,
    on_disk::OnDiskStorage,
    policy::{Capability, Identity, Permission, Policy},
    storage::Storage,
    vault::VaultStorage,
};

// Some common serializations for interacting with bytes these must be manually added to types via:
// #[serde(serialize_with = "to_base64", deserialize_with = "from_base64")]
// some_value: Vec<u8>

pub fn to_base64<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&base64::encode(bytes))
}

pub fn from_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    base64::decode(s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests;
