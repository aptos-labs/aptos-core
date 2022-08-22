// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{access_path::AccessPath, state_store::table::TableHandle};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use thiserror::Error;

#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum StateKey {
    AccessPath(AccessPath),
    TableItem {
        handle: TableHandle,
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
    },
    // Only used for testing
    #[serde(with = "serde_bytes")]
    Raw(Vec<u8>),
}

#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum StateKeyTag {
    AccessPath,
    TableItem,
    Raw = 255,
}

impl StateKey {
    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut out = vec![];

        let (prefix, raw_key) = match self {
            StateKey::AccessPath(access_path) => {
                (StateKeyTag::AccessPath, bcs::to_bytes(access_path)?)
            }
            StateKey::TableItem { handle, key } => {
                let mut bytes = bcs::to_bytes(&handle)?;
                bytes.extend(key);
                (StateKeyTag::TableItem, bytes)
            }
            StateKey::Raw(raw_bytes) => (StateKeyTag::Raw, raw_bytes.to_vec()),
        };
        out.push(prefix as u8);
        out.extend(raw_key);
        Ok(out)
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<StateKey, StateKeyDecodeErr> {
        if val.is_empty() {
            return Err(StateKeyDecodeErr::EmptyInput);
        }
        let tag = val[0];
        let state_key_tag =
            StateKeyTag::from_u8(tag).ok_or(StateKeyDecodeErr::UnknownTag { unknown_tag: tag })?;
        match state_key_tag {
            StateKeyTag::AccessPath => Ok(StateKey::AccessPath(bcs::from_bytes(&val[1..])?)),
            StateKeyTag::TableItem => {
                const HANDLE_SIZE: usize = std::mem::size_of::<TableHandle>();
                if val.len() < 1 + HANDLE_SIZE {
                    return Err(StateKeyDecodeErr::NotEnoughBytes {
                        tag,
                        num_bytes: val.len(),
                    });
                }
                let handle = bcs::from_bytes(
                    val[1..1 + HANDLE_SIZE]
                        .try_into()
                        .expect("Bytes too short."),
                )?;
                let key = val[1 + HANDLE_SIZE..].to_vec();
                Ok(StateKey::table_item(handle, key))
            }
            StateKeyTag::Raw => Ok(StateKey::Raw(val[1..].to_vec())),
        }
    }

    pub fn table_item(handle: TableHandle, key: Vec<u8>) -> Self {
        StateKey::TableItem { handle, key }
    }

    pub fn size(&self) -> usize {
        match self {
            StateKey::AccessPath(access_path) => access_path.size(),
            StateKey::TableItem { handle, key } => handle.size() + key.len(),
            StateKey::Raw(bytes) => bytes.len(),
        }
    }
}

impl CryptoHash for StateKey {
    type Hasher = StateKeyHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(
            self.encode()
                .expect("Failed to serialize the state key")
                .as_ref(),
        );
        state.finish()
    }
}

/// Error thrown when a [`StateKey`] fails to be deserialized out of a byte sequence stored in physical
/// storage, via [`StateKey::decode`].
#[derive(Debug, Error)]
pub enum StateKeyDecodeErr {
    /// Input is empty.
    #[error("Missing tag due to empty input")]
    EmptyInput,

    /// The first byte of the input is not a known tag representing one of the variants.
    #[error("lead tag byte is unknown: {}", unknown_tag)]
    UnknownTag { unknown_tag: u8 },

    #[error("Not enough bytes: tag: {}, num bytes: {}", tag, num_bytes)]
    NotEnoughBytes { tag: u8, num_bytes: usize },

    #[error(transparent)]
    BcsError(#[from] bcs::Error),
}
