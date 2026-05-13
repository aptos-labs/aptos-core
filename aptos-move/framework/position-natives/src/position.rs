// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compact-binary codec for `NativePosition`.
//!
//! Layout (little-endian payload; the preceding `StateValue` / RocksDB-key
//! encoding is handled elsewhere):
//!
//! ```text
//! PerpV1  tag=0x00  68 bytes
//!   [tag:1][size:u64:8][is_long:u8:1]
//!   [entry_px_times_size_sum:u128:16][avg_entry_px:u64:8]
//!   [user_leverage:u8:1][is_isolated:u8:1]
//!   [funding_index:u128:16][unrealized_funding_before:u64:8]
//!   [timestamp:u64:8]
//!
//! SpotV1  tag=0x01  42 bytes
//!   [tag:1][size:u64:8][is_long:u8:1]
//!   [entry_px_times_size_sum:u128:16][avg_entry_px:u64:8]
//!   [timestamp:u64:8]
//! ```

use move_core_types::account_address::AccountAddress;
use std::convert::TryInto;

/// Deserialized form of a persisted position. Mirrors the `Position` enum
/// in `aptos_experimental::native_position`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativePosition {
    PerpV1 {
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        user_leverage: u8,
        is_isolated: bool,
        /// Signed. Matches etna's `AccumulativeIndex { index: i128 }`.
        funding_index: i128,
        /// Signed. Matches etna's `unrealized_funding_amount_before_last_update: i64`.
        unrealized_funding_before: i64,
        timestamp: u64,
    },
    SpotV1 {
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_entry_px: u64,
        timestamp: u64,
    },
}

const TAG_PERP_V1: u8 = 0x00;
const TAG_SPOT_V1: u8 = 0x01;

const PERP_V1_LEN: usize = 68;
const SPOT_V1_LEN: usize = 42;

/// Errors from the Position codec.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("position payload too short: expected {expected} bytes, got {got}")]
    TooShort { expected: usize, got: usize },
    #[error("unknown position variant tag: 0x{0:02x}")]
    UnknownTag(u8),
    #[error("invalid boolean byte: {0}")]
    InvalidBool(u8),
}

/// Helper for slice → fixed-array conversions during deserialize.
/// The length checks at each call site guarantee the slice length;
/// `expect` documents that this is unreachable.
macro_rules! fixed_slice {
    ($bytes:expr, $start:expr, $len:expr) => {
        $bytes[$start..$start + $len]
            .try_into()
            .expect("slice length checked by caller")
    };
}

impl NativePosition {
    /// Number of bytes [`serialize`] will produce for this variant.
    /// Used to pre-size `Vec::with_capacity` so the allocation is
    /// exact and the codec-size invariant is implicit (no debug-only
    /// assertion needed).
    pub fn serialized_len(&self) -> usize {
        match self {
            NativePosition::PerpV1 { .. } => PERP_V1_LEN,
            NativePosition::SpotV1 { .. } => SPOT_V1_LEN,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.serialized_len());
        match self {
            NativePosition::PerpV1 {
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                user_leverage,
                is_isolated,
                funding_index,
                unrealized_funding_before,
                timestamp,
            } => {
                out.push(TAG_PERP_V1);
                out.extend_from_slice(&size.to_le_bytes());
                out.push(u8::from(*is_long));
                out.extend_from_slice(&entry_px_times_size_sum.to_le_bytes());
                out.extend_from_slice(&avg_entry_px.to_le_bytes());
                out.push(*user_leverage);
                out.push(u8::from(*is_isolated));
                out.extend_from_slice(&funding_index.to_le_bytes());
                out.extend_from_slice(&unrealized_funding_before.to_le_bytes());
                out.extend_from_slice(&timestamp.to_le_bytes());
            },
            NativePosition::SpotV1 {
                size,
                is_long,
                entry_px_times_size_sum,
                avg_entry_px,
                timestamp,
            } => {
                out.push(TAG_SPOT_V1);
                out.extend_from_slice(&size.to_le_bytes());
                out.push(u8::from(*is_long));
                out.extend_from_slice(&entry_px_times_size_sum.to_le_bytes());
                out.extend_from_slice(&avg_entry_px.to_le_bytes());
                out.extend_from_slice(&timestamp.to_le_bytes());
            },
        }
        out
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, CodecError> {
        let tag = *bytes.first().ok_or(CodecError::TooShort {
            expected: 1,
            got: 0,
        })?;
        match tag {
            TAG_PERP_V1 => {
                if bytes.len() < PERP_V1_LEN {
                    return Err(CodecError::TooShort {
                        expected: PERP_V1_LEN,
                        got: bytes.len(),
                    });
                }
                let size = u64::from_le_bytes(fixed_slice!(bytes, 1, 8));
                let is_long = decode_bool(bytes[9])?;
                let entry_px_times_size_sum = u128::from_le_bytes(fixed_slice!(bytes, 10, 16));
                let avg_entry_px = u64::from_le_bytes(fixed_slice!(bytes, 26, 8));
                let user_leverage = bytes[34];
                let is_isolated = decode_bool(bytes[35])?;
                let funding_index = i128::from_le_bytes(fixed_slice!(bytes, 36, 16));
                let unrealized_funding_before = i64::from_le_bytes(fixed_slice!(bytes, 52, 8));
                let timestamp = u64::from_le_bytes(fixed_slice!(bytes, 60, 8));
                Ok(NativePosition::PerpV1 {
                    size,
                    is_long,
                    entry_px_times_size_sum,
                    avg_entry_px,
                    user_leverage,
                    is_isolated,
                    funding_index,
                    unrealized_funding_before,
                    timestamp,
                })
            },
            TAG_SPOT_V1 => {
                if bytes.len() < SPOT_V1_LEN {
                    return Err(CodecError::TooShort {
                        expected: SPOT_V1_LEN,
                        got: bytes.len(),
                    });
                }
                let size = u64::from_le_bytes(fixed_slice!(bytes, 1, 8));
                let is_long = decode_bool(bytes[9])?;
                let entry_px_times_size_sum = u128::from_le_bytes(fixed_slice!(bytes, 10, 16));
                let avg_entry_px = u64::from_le_bytes(fixed_slice!(bytes, 26, 8));
                let timestamp = u64::from_le_bytes(fixed_slice!(bytes, 34, 8));
                Ok(NativePosition::SpotV1 {
                    size,
                    is_long,
                    entry_px_times_size_sum,
                    avg_entry_px,
                    timestamp,
                })
            },
            other => Err(CodecError::UnknownTag(other)),
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            NativePosition::PerpV1 { size, .. } => *size,
            NativePosition::SpotV1 { size, .. } => *size,
        }
    }

    pub fn is_long(&self) -> bool {
        match self {
            NativePosition::PerpV1 { is_long, .. } => *is_long,
            NativePosition::SpotV1 { is_long, .. } => *is_long,
        }
    }

    /// User-configured leverage. `Some(N)` for perp positions; `None`
    /// for spot, which has no leverage concept. Callers that previously
    /// relied on a default of 1 should explicitly choose between
    /// `unwrap_or(1)` (treat spot as 1×) and `is_some()` (perp-only
    /// branch).
    pub fn user_leverage(&self) -> Option<u8> {
        match self {
            NativePosition::PerpV1 { user_leverage, .. } => Some(*user_leverage),
            NativePosition::SpotV1 { .. } => None,
        }
    }
}

fn decode_bool(byte: u8) -> Result<bool, CodecError> {
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(CodecError::InvalidBool(other)),
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct PositionKey {
    pub exchange: AccountAddress,
    pub account: AccountAddress,
    pub market: AccountAddress,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perp_v1_roundtrip() {
        let position = NativePosition::PerpV1 {
            size: 1_000,
            is_long: true,
            entry_px_times_size_sum: 50_000_000_000,
            avg_entry_px: 50_000_000,
            user_leverage: 10,
            is_isolated: false,
            funding_index: 0,
            unrealized_funding_before: 0,
            timestamp: 1_700_000_000,
        };
        let bytes = position.serialize();
        assert_eq!(bytes.len(), PERP_V1_LEN);
        assert_eq!(NativePosition::deserialize(&bytes).unwrap(), position);
    }

    #[test]
    fn spot_v1_roundtrip() {
        let position = NativePosition::SpotV1 {
            size: 42,
            is_long: false,
            entry_px_times_size_sum: 123,
            avg_entry_px: 999,
            timestamp: 1,
        };
        let bytes = position.serialize();
        assert_eq!(bytes.len(), SPOT_V1_LEN);
        assert_eq!(NativePosition::deserialize(&bytes).unwrap(), position);
    }

    #[test]
    fn rejects_invalid_bool() {
        let mut bytes = NativePosition::SpotV1 {
            size: 1,
            is_long: false,
            entry_px_times_size_sum: 0,
            avg_entry_px: 0,
            timestamp: 0,
        }
        .serialize();
        bytes[9] = 2;
        assert!(matches!(
            NativePosition::deserialize(&bytes),
            Err(CodecError::InvalidBool(2))
        ));
    }
}
