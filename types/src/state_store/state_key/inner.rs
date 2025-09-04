// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{access_path::AccessPath, state_store::table::TableHandle};
use velor_crypto_derive::CryptoHasher;
use bytes::{BufMut, Bytes, BytesMut};
use num_derive::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Debug, Formatter},
    io::Write,
};
use thiserror::Error;

#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum StateKeyTag {
    AccessPath,
    TableItem,
    Raw = 255,
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

    #[error(transparent)]
    AnyHow(#[from] anyhow::Error),
}

#[derive(Clone, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[serde(rename = "StateKey")]
pub enum StateKeyInner {
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

impl StateKeyInner {
    /// Serializes to bytes for physical storage.
    pub(crate) fn encode(&self) -> anyhow::Result<Bytes> {
        let mut writer = BytesMut::new().writer();

        match self {
            StateKeyInner::AccessPath(access_path) => {
                writer.write_all(&[StateKeyTag::AccessPath as u8])?;
                bcs::serialize_into(&mut writer, access_path)?;
            },
            StateKeyInner::TableItem { handle, key } => {
                writer.write_all(&[StateKeyTag::TableItem as u8])?;
                bcs::serialize_into(&mut writer, &handle)?;
                writer.write_all(key)?;
            },
            StateKeyInner::Raw(raw_bytes) => {
                writer.write_all(&[StateKeyTag::Raw as u8])?;
                writer.write_all(raw_bytes)?;
            },
        };

        Ok(writer.into_inner().into())
    }
}

impl Debug for StateKeyInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StateKeyInner::AccessPath(ap) => {
                write!(f, "StateKey::{:?}", ap)
            },
            StateKeyInner::TableItem { handle, key } => {
                write!(
                    f,
                    "StateKey::TableItem {{ handle: {:x}, key: {} }}",
                    handle.0,
                    hex::encode(key),
                )
            },
            StateKeyInner::Raw(bytes) => {
                write!(f, "StateKey::Raw({})", hex::encode(bytes),)
            },
        }
    }
}
