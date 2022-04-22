// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::access_path::AccessPath;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use move_core_types::account_address::AccountAddress;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum StateKey {
    AccountAddressKey(AccountAddress),
    AccessPath(AccessPath),
    // Only used for testing
    #[serde(with = "serde_bytes")]
    Raw(Vec<u8>),
}

#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum StateKeyTag {
    AccountAddress,
    AccessPath,
    Raw = 255,
}

impl StateKey {
    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut out = vec![];

        let (prefix, raw_key) = match self {
            StateKey::AccountAddressKey(account_address) => {
                (StateKeyTag::AccountAddress, bcs::to_bytes(account_address)?)
            }
            StateKey::AccessPath(access_path) => {
                (StateKeyTag::AccessPath, bcs::to_bytes(access_path)?)
            }
            StateKey::Raw(raw_bytes) => (StateKeyTag::Raw, raw_bytes.to_vec()),
        };
        out.push(prefix as u8);
        out.extend(raw_key);
        Ok(out)
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> anyhow::Result<StateKey> {
        if val.is_empty() {
            return Err(StateKeyDecodeErr::EmptyInput.into());
        }
        let tag = val[0];
        let state_key_tag =
            StateKeyTag::from_u8(tag).ok_or(StateKeyDecodeErr::UnknownTag { unknown_tag: tag })?;
        match state_key_tag {
            StateKeyTag::AccountAddress => {
                Ok(StateKey::AccountAddressKey(bcs::from_bytes(&val[1..])?))
            }
            StateKeyTag::AccessPath => Ok(StateKey::AccessPath(bcs::from_bytes(&val[1..])?)),
            StateKeyTag::Raw => Ok(StateKey::Raw(val[1..].to_vec())),
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
#[derive(Debug, Error, Eq, PartialEq)]
pub enum StateKeyDecodeErr {
    /// Input is empty.
    #[error("Missing tag due to empty input")]
    EmptyInput,

    /// The first byte of the input is not a known tag representing one of the variants.
    #[error("lead tag byte is unknown: {}", unknown_tag)]
    UnknownTag { unknown_tag: u8 },
}
