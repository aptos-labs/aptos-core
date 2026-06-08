// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `NativePosition` — the decoded value of a
//! `StateKeyInner::TradingNative(TradingNativeKey::Position)` entry,
//! encoded with BCS. The bytes match the BCS of the Move
//! `aptos_trading::native_position_types::Position` value, so the field
//! order and widths here must track the Move type.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NativePosition {
    PerpV1 {
        size: u64,
        is_long: bool,
        entry_px_times_size_sum: u128,
        avg_acquire_entry_px: u64,
        user_leverage: u8,
        is_isolated: bool,
        // Move wraps this in `AccumulativeIndex { index: i128 }`, which is
        // BCS-identical to a bare `i128`.
        funding_index_at_last_update: i128,
        unrealized_funding_amount_before_last_update: i64,
        timestamp: u64,
    },
}

impl NativePosition {
    /// BCS-encoded length, for gas pre-sizing. Computed without
    /// allocating the buffer.
    pub fn serialized_len(&self) -> usize {
        bcs::serialized_size(self).expect("NativePosition size is computable")
    }

    pub fn serialize(&self) -> Result<Vec<u8>, bcs::Error> {
        bcs::to_bytes(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bcs::Error> {
        bcs::from_bytes(bytes)
    }

    pub fn size(&self) -> u64 {
        match self {
            NativePosition::PerpV1 { size, .. } => *size,
        }
    }

    pub fn is_long(&self) -> bool {
        match self {
            NativePosition::PerpV1 { is_long, .. } => *is_long,
        }
    }

    pub fn user_leverage(&self) -> u8 {
        match self {
            NativePosition::PerpV1 { user_leverage, .. } => *user_leverage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NativePosition {
        NativePosition::PerpV1 {
            size: 1_000,
            is_long: true,
            entry_px_times_size_sum: 50_000_000_000,
            avg_acquire_entry_px: 50_000_000,
            user_leverage: 10,
            is_isolated: false,
            funding_index_at_last_update: 0,
            unrealized_funding_amount_before_last_update: 0,
            timestamp: 1_700_000_000,
        }
    }

    #[test]
    fn serialized_len_matches_serialize() {
        let position = sample();
        assert_eq!(
            position.serialized_len(),
            position.serialize().unwrap().len()
        );
    }

    #[test]
    fn perp_v1_roundtrip() {
        let position = sample();
        assert_eq!(
            NativePosition::deserialize(&position.serialize().unwrap()).unwrap(),
            position
        );
    }

    #[test]
    fn perp_v1_roundtrip_negative_funding() {
        let position = NativePosition::PerpV1 {
            size: 7,
            is_long: false,
            entry_px_times_size_sum: 1,
            avg_acquire_entry_px: 1,
            user_leverage: 3,
            is_isolated: true,
            funding_index_at_last_update: i128::MIN,
            unrealized_funding_amount_before_last_update: i64::MIN,
            timestamp: 42,
        };
        assert_eq!(
            NativePosition::deserialize(&position.serialize().unwrap()).unwrap(),
            position
        );
    }

    #[test]
    fn rejects_invalid_bool() {
        let mut bytes = sample().serialize().unwrap();
        bytes[9] = 2; // is_long byte
        assert!(NativePosition::deserialize(&bytes).is_err());
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = sample().serialize().unwrap();
        bytes.push(0xFF);
        assert!(NativePosition::deserialize(&bytes).is_err());
    }

    #[test]
    fn matches_move_bcs_encoding() {
        // BCS of the Move-side `Position::PerpV1` value, built via the Move
        // serializer, must match `NativePosition`'s BCS byte-for-byte.
        use move_core_types::value::{MoveStruct, MoveValue};

        let native = NativePosition::PerpV1 {
            size: 1_000,
            is_long: true,
            entry_px_times_size_sum: 50_000_000_000,
            avg_acquire_entry_px: 50_000_000,
            user_leverage: 10,
            is_isolated: false,
            funding_index_at_last_update: -123_456_789,
            unrealized_funding_amount_before_last_update: -42,
            timestamp: 1_700_000_000,
        };

        let move_value = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::U64(1_000),
            MoveValue::Bool(true),
            MoveValue::U128(50_000_000_000),
            MoveValue::U64(50_000_000),
            MoveValue::U8(10),
            MoveValue::Bool(false),
            MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::I128(-123_456_789)])),
            MoveValue::I64(-42),
            MoveValue::U64(1_700_000_000),
        ]));

        let move_bytes = bcs::to_bytes(&move_value).unwrap();
        assert_eq!(move_bytes, native.serialize().unwrap());
        assert_eq!(NativePosition::deserialize(&move_bytes).unwrap(), native);
    }
}
