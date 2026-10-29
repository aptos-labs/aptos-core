// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `NativeCollateral` — the decoded value of a
//! `StateKeyInner::TradingNative(TradingNativeKey::Collateral)` entry,
//! encoded with BCS. The bytes match the BCS of the Move
//! `aptos_experimental::native_collateral::Collateral` value once its
//! aggregator snapshot is materialized: a snapshot serializes as its `u64`,
//! so `primary_offset` is that `u64`, the balance offset by 2^63.

use move_core_types::value::{IdentifierMappingKind, MoveStructLayout, MoveTypeLayout};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

const SIGN_OFFSET: u64 = 1 << 63;

/// Runtime layout of the Move `Collateral` value. `primary` is an
/// `AggregatorSnapshot<u64>`, whose single field is the delayed field, as the
/// VM lays it out. Used to swap the snapshot's identifier for its committed
/// value when the write is materialized.
static COLLATERAL_LAYOUT: Lazy<MoveTypeLayout> = Lazy::new(|| {
    let snapshot = MoveTypeLayout::new_struct(MoveStructLayout::new(vec![MoveTypeLayout::Native(
        IdentifierMappingKind::Snapshot,
        Box::new(MoveTypeLayout::U64),
    )]));
    let secondary = MoveTypeLayout::new_struct(MoveStructLayout::new(vec![
        MoveTypeLayout::Address,
        MoveTypeLayout::U64,
    ]));
    MoveTypeLayout::new_struct(MoveStructLayout::new_variants(vec![vec![
        snapshot,
        MoveTypeLayout::Vector(Box::new(secondary)),
        MoveTypeLayout::U64,
    ]]))
});

pub fn collateral_layout() -> &'static MoveTypeLayout {
    &COLLATERAL_LAYOUT
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NativeCollateral {
    V1 {
        primary_offset: u64,
        secondaries: Vec<SecondaryBalance>,
        reserved: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SecondaryBalance {
    pub asset: move_core_types::account_address::AccountAddress,
    pub amount: u64,
}

impl NativeCollateral {
    /// Signed primary balance, undoing the 2^63 offset.
    pub fn primary(&self) -> i64 {
        match self {
            NativeCollateral::V1 { primary_offset, .. } => (primary_offset ^ SIGN_OFFSET) as i64,
        }
    }

    /// BCS-encoded length, computed without allocating the buffer.
    pub fn serialized_len(&self) -> usize {
        bcs::serialized_size(self).expect("NativeCollateral size is computable")
    }

    pub fn serialize(&self) -> Result<Vec<u8>, bcs::Error> {
        bcs::to_bytes(self)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, bcs::Error> {
        bcs::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::{
        account_address::AccountAddress,
        value::{MoveStruct, MoveValue},
    };

    fn sample() -> NativeCollateral {
        NativeCollateral::V1 {
            primary_offset: SIGN_OFFSET + 1_000_000,
            secondaries: vec![SecondaryBalance {
                asset: AccountAddress::from_hex_literal("0x42").unwrap(),
                amount: 7,
            }],
            reserved: 5,
        }
    }

    #[test]
    fn serialized_len_matches_serialize() {
        let collateral = sample();
        assert_eq!(
            collateral.serialized_len(),
            collateral.serialize().unwrap().len()
        );
    }

    #[test]
    fn roundtrip() {
        let collateral = sample();
        assert_eq!(
            NativeCollateral::deserialize(&collateral.serialize().unwrap()).unwrap(),
            collateral
        );
    }

    #[test]
    fn primary_undoes_offset() {
        let zero = NativeCollateral::V1 {
            primary_offset: SIGN_OFFSET,
            secondaries: vec![],
            reserved: 0,
        };
        assert_eq!(zero.primary(), 0);
        let negative = NativeCollateral::V1 {
            primary_offset: SIGN_OFFSET - 5,
            secondaries: vec![],
            reserved: 0,
        };
        assert_eq!(negative.primary(), -5);
        assert_eq!(sample().primary(), 1_000_000);
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = sample().serialize().unwrap();
        bytes.push(0xFF);
        assert!(NativeCollateral::deserialize(&bytes).is_err());
    }

    fn without_delayed_fields(layout: &MoveTypeLayout) -> MoveTypeLayout {
        match layout {
            MoveTypeLayout::Native(_, inner) => without_delayed_fields(inner),
            MoveTypeLayout::Vector(inner) => {
                MoveTypeLayout::Vector(Box::new(without_delayed_fields(inner)))
            },
            MoveTypeLayout::Struct(s) => MoveTypeLayout::new_struct(match s.as_ref() {
                MoveStructLayout::Runtime(fields) => {
                    MoveStructLayout::new(fields.iter().map(without_delayed_fields).collect())
                },
                MoveStructLayout::RuntimeVariants(variants) => MoveStructLayout::new_variants(
                    variants
                        .iter()
                        .map(|fields| fields.iter().map(without_delayed_fields).collect())
                        .collect(),
                ),
                other => panic!("unexpected struct layout {:?}", other),
            }),
            other => other.clone(),
        }
    }

    #[test]
    fn layout_matches_value_encoding() {
        let bytes = sample().serialize().unwrap();
        let layout = without_delayed_fields(collateral_layout());
        let value = MoveValue::simple_deserialize(&bytes, &layout).unwrap();
        assert_eq!(value.simple_serialize().unwrap(), bytes);
    }

    #[test]
    fn layout_marks_primary_as_snapshot() {
        let MoveTypeLayout::Struct(s) = collateral_layout() else {
            panic!("collateral layout is not a struct");
        };
        let MoveStructLayout::RuntimeVariants(variants) = s.as_ref() else {
            panic!("collateral layout is not an enum");
        };
        assert_eq!(variants.len(), 1);
        let MoveTypeLayout::Struct(primary) = &variants[0][0] else {
            panic!("primary is not a struct");
        };
        assert!(matches!(
            primary.as_ref(),
            MoveStructLayout::Runtime(fields)
                if matches!(
                    fields.as_slice(),
                    [MoveTypeLayout::Native(IdentifierMappingKind::Snapshot, inner)]
                        if matches!(inner.as_ref(), MoveTypeLayout::U64)
                )
        ));
    }

    #[test]
    fn matches_move_bcs_encoding() {
        // BCS of the Move-side `Collateral::V1` once its snapshot is
        // materialized to its u64, built via the Move serializer.
        let native = sample();
        let move_value = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::U64(SIGN_OFFSET + 1_000_000),
            MoveValue::Vector(vec![MoveValue::Struct(MoveStruct::Runtime(vec![
                MoveValue::Address(AccountAddress::from_hex_literal("0x42").unwrap()),
                MoveValue::U64(7),
            ]))]),
            MoveValue::U64(5),
        ]));
        let move_bytes = bcs::to_bytes(&move_value).unwrap();
        assert_eq!(move_bytes, native.serialize().unwrap());
        assert_eq!(NativeCollateral::deserialize(&move_bytes).unwrap(), native);
    }
}
