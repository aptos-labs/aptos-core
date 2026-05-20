// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{access_path::AccessPath, state_store::table::TableHandle};
use aptos_crypto_derive::CryptoHasher;
use bytes::{BufMut, Bytes, BytesMut};
use move_core_types::account_address::AccountAddress;
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
    /// Umbrella for the trading-native subsystem. Sub-entities
    /// (Position, future Collateral / Order / ...) are distinguished
    /// by [`TradingNativeKeyTag`] inside the payload, not by a
    /// top-level tag. This keeps the top-level tag space focused on
    /// subsystem-level categories.
    TradingNative = 2,
    Raw = 255,
}

/// Sub-tag distinguishing entities inside the
/// [`StateKeyInner::TradingNative`] umbrella. Encoded as the first
/// byte of the payload after the top-level [`StateKeyTag::TradingNative`]
/// byte. Variant ordinals are part of the on-disk byte format —
/// **do not reorder or insert before existing entries**.
#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum TradingNativeKeyTag {
    Position = 0,
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

    /// The sub-tag inside a `TradingNative` payload is unrecognized.
    #[error("unknown TradingNative sub-tag: {}", unknown_sub_tag)]
    UnknownTradingNativeSubTag { unknown_sub_tag: u8 },

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
    /// Umbrella variant for the trading-native subsystem. Specific
    /// entities are distinguished by the inner [`TradingNativeKey`].
    TradingNative(TradingNativeKey),
}

/// Sub-shape of a `StateKeyInner::TradingNative` key. Each variant
/// owns a fixed-width encoding; the [`TradingNativeKeyTag`] sub-tag
/// is written/read once per encode/decode.
#[derive(
    Clone, Debug, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum TradingNativeKey {
    /// Native-position persisted entry: per-(exchange, account, market) position.
    Position {
        exchange: AccountAddress,
        account: AccountAddress,
        market: AccountAddress,
    },
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
            StateKeyInner::TradingNative(key) => {
                writer.write_all(&[StateKeyTag::TradingNative as u8])?;
                match key {
                    TradingNativeKey::Position {
                        exchange,
                        account,
                        market,
                    } => {
                        writer.write_all(&[TradingNativeKeyTag::Position as u8])?;
                        writer.write_all(exchange.as_ref())?;
                        writer.write_all(account.as_ref())?;
                        writer.write_all(market.as_ref())?;
                    },
                }
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
            StateKeyInner::TradingNative(key) => match key {
                TradingNativeKey::Position {
                    exchange,
                    account,
                    market,
                } => write!(
                    f,
                    "StateKey::TradingNative::Position {{ exchange: {}, account: {}, market: {} }}",
                    exchange, account, market,
                ),
            },
        }
    }
}
