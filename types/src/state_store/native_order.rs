// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native order values — the decoded values of
//! `StateKeyInner::TradingNative(TradingNativeKey::{Order, BulkOrder})`
//! entries. The bytes are the BCS of the Move `NativeSingleOrder<M>` /
//! `NativeBulkOrder<M>` values, whose generic metadata `M` comes last: the
//! header fields are decoded here and the rest of the bytes, exactly the BCS
//! of `M`, is kept opaque in `metadata`. Field order and widths must track
//! the Move types.

use bytes::Bytes;
use move_core_types::account_address::AccountAddress;
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum NativeOrderEncodeError {
    #[error("length {0} exceeds the BCS maximum of u32::MAX")]
    LengthTooLarge(usize),
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum NativeOrderDecodeError {
    #[error("native order bytes end early while decoding {0}")]
    UnexpectedEnd(&'static str),
    #[error("invalid {field} tag {tag}")]
    InvalidTag { field: &'static str, tag: u8 },
    #[error("invalid ULEB128 length")]
    InvalidLength,
    #[error("client order id is not valid UTF-8")]
    InvalidUtf8,
}

/// Move `TimeInForce`. Variant ordinals are the BCS tags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeInForce {
    Gtc = 0,
    PostOnly = 1,
    Ioc = 2,
}

/// Move `TriggerCondition`. Variant ordinals are the BCS tags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TriggerCondition {
    PriceMoveAbove(u64),
    PriceMoveBelow(u64),
    /// Seconds since the Unix epoch.
    TimeBased(u64),
}

/// Move `NativeSingleOrder<M>`. Variant ordinals are the BCS tags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeSingleOrder {
    V1(SingleOrderV1),
}

/// Move `NativeBulkOrder<M>`. Variant ordinals are the BCS tags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeBulkOrder {
    V1(BulkOrderV1),
}

/// Fields of `NativeSingleOrder::V1`. `order_id` and `unique_priority_idx`
/// are the single `u128` fields of Move's `OrderId` / `IncreasingIdx`, which
/// BCS encodes identically to a bare `u128`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SingleOrderV1 {
    pub account: AccountAddress,
    pub order_id: u128,
    pub client_order_id: Option<String>,
    pub price: u64,
    pub orig_size: u64,
    pub remaining_size: u64,
    pub is_bid: bool,
    /// An order with a trigger is pending; without one it is active.
    pub trigger_condition: Option<TriggerCondition>,
    pub time_in_force: TimeInForce,
    pub creation_time_micros: u64,
    pub unique_priority_idx: u128,
    /// BCS of the market's metadata type `M`.
    pub metadata: Bytes,
}

/// Fields of `NativeBulkOrder::V1`. Price levels are best first on each side.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BulkOrderV1 {
    pub account: AccountAddress,
    pub order_id: u128,
    pub sequence_number: u64,
    pub bid_prices: Vec<u64>,
    pub bid_sizes: Vec<u64>,
    pub ask_prices: Vec<u64>,
    pub ask_sizes: Vec<u64>,
    pub unique_priority_idx: u128,
    pub creation_time_micros: u64,
    /// BCS of the market's metadata type `M`.
    pub metadata: Bytes,
}

const V1: u8 = 0;

impl NativeSingleOrder {
    pub fn serialize(&self) -> Result<Vec<u8>, NativeOrderEncodeError> {
        let mut w = Writer::default();
        match self {
            Self::V1(order) => {
                w.u8(V1);
                order.write(&mut w)?;
            },
        }
        Ok(w.0)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, NativeOrderDecodeError> {
        let mut r = Reader::new(bytes);
        match r.u8("NativeSingleOrder")? {
            V1 => Ok(Self::V1(SingleOrderV1::read(r)?)),
            tag => Err(NativeOrderDecodeError::InvalidTag {
                field: "NativeSingleOrder",
                tag,
            }),
        }
    }
}

impl NativeBulkOrder {
    pub fn serialize(&self) -> Result<Vec<u8>, NativeOrderEncodeError> {
        let mut w = Writer::default();
        match self {
            Self::V1(order) => {
                w.u8(V1);
                order.write(&mut w)?;
            },
        }
        Ok(w.0)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, NativeOrderDecodeError> {
        let mut r = Reader::new(bytes);
        match r.u8("NativeBulkOrder")? {
            V1 => Ok(Self::V1(BulkOrderV1::read(r)?)),
            tag => Err(NativeOrderDecodeError::InvalidTag {
                field: "NativeBulkOrder",
                tag,
            }),
        }
    }
}

impl SingleOrderV1 {
    fn write(&self, w: &mut Writer) -> Result<(), NativeOrderEncodeError> {
        w.address(&self.account);
        w.u128(self.order_id);
        match &self.client_order_id {
            None => w.u8(0),
            Some(id) => {
                w.u8(1);
                w.bytes(id.as_bytes())?;
            },
        }
        w.u64(self.price);
        w.u64(self.orig_size);
        w.u64(self.remaining_size);
        w.bool(self.is_bid);
        match &self.trigger_condition {
            None => w.u8(0),
            Some(trigger) => {
                w.u8(1);
                let (tag, value) = match trigger {
                    TriggerCondition::PriceMoveAbove(price) => (0, price),
                    TriggerCondition::PriceMoveBelow(price) => (1, price),
                    TriggerCondition::TimeBased(time_secs) => (2, time_secs),
                };
                w.u8(tag);
                w.u64(*value);
            },
        }
        w.u8(self.time_in_force as u8);
        w.u64(self.creation_time_micros);
        w.u128(self.unique_priority_idx);
        w.raw(&self.metadata);
        Ok(())
    }

    fn read(mut r: Reader) -> Result<Self, NativeOrderDecodeError> {
        let account = r.address("account")?;
        let order_id = r.u128("order_id")?;
        let client_order_id = match r.option_tag("client_order_id")? {
            false => None,
            true => Some(
                String::from_utf8(r.bytes("client_order_id")?.to_vec())
                    .map_err(|_| NativeOrderDecodeError::InvalidUtf8)?,
            ),
        };
        let price = r.u64("price")?;
        let orig_size = r.u64("orig_size")?;
        let remaining_size = r.u64("remaining_size")?;
        let is_bid = r.bool("is_bid")?;
        let trigger_condition = match r.option_tag("trigger_condition")? {
            false => None,
            true => {
                let tag = r.u8("trigger_condition")?;
                let value = r.u64("trigger_condition")?;
                Some(match tag {
                    0 => TriggerCondition::PriceMoveAbove(value),
                    1 => TriggerCondition::PriceMoveBelow(value),
                    2 => TriggerCondition::TimeBased(value),
                    tag => {
                        return Err(NativeOrderDecodeError::InvalidTag {
                            field: "trigger_condition",
                            tag,
                        })
                    },
                })
            },
        };
        let time_in_force = match r.u8("time_in_force")? {
            0 => TimeInForce::Gtc,
            1 => TimeInForce::PostOnly,
            2 => TimeInForce::Ioc,
            tag => {
                return Err(NativeOrderDecodeError::InvalidTag {
                    field: "time_in_force",
                    tag,
                })
            },
        };
        let creation_time_micros = r.u64("creation_time_micros")?;
        let unique_priority_idx = r.u128("unique_priority_idx")?;
        Ok(Self {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            creation_time_micros,
            unique_priority_idx,
            metadata: Bytes::copy_from_slice(r.rest()),
        })
    }
}

impl BulkOrderV1 {
    fn write(&self, w: &mut Writer) -> Result<(), NativeOrderEncodeError> {
        w.address(&self.account);
        w.u128(self.order_id);
        w.u64(self.sequence_number);
        for levels in [
            &self.bid_prices,
            &self.bid_sizes,
            &self.ask_prices,
            &self.ask_sizes,
        ] {
            w.u64_vec(levels)?;
        }
        w.u128(self.unique_priority_idx);
        w.u64(self.creation_time_micros);
        w.raw(&self.metadata);
        Ok(())
    }

    fn read(mut r: Reader) -> Result<Self, NativeOrderDecodeError> {
        Ok(Self {
            account: r.address("account")?,
            order_id: r.u128("order_id")?,
            sequence_number: r.u64("sequence_number")?,
            bid_prices: r.u64_vec("bid_prices")?,
            bid_sizes: r.u64_vec("bid_sizes")?,
            ask_prices: r.u64_vec("ask_prices")?,
            ask_sizes: r.u64_vec("ask_sizes")?,
            unique_priority_idx: r.u128("unique_priority_idx")?,
            creation_time_micros: r.u64("creation_time_micros")?,
            metadata: Bytes::copy_from_slice(r.rest()),
        })
    }
}

/// BCS writer for the header fields.
#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }

    fn bool(&mut self, v: bool) {
        self.0.push(v as u8);
    }

    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn u128(&mut self, v: u128) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn address(&mut self, v: &AccountAddress) {
        self.0.extend_from_slice(v.as_ref());
    }

    /// BCS ULEB128 length; BCS caps lengths at `u32::MAX`.
    fn uleb128(&mut self, len: usize) -> Result<(), NativeOrderEncodeError> {
        let mut v = u32::try_from(len).map_err(|_| NativeOrderEncodeError::LengthTooLarge(len))?;
        while v >= 0x80 {
            self.0.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
        self.0.push(v as u8);
        Ok(())
    }

    fn bytes(&mut self, v: &[u8]) -> Result<(), NativeOrderEncodeError> {
        self.uleb128(v.len())?;
        self.0.extend_from_slice(v);
        Ok(())
    }

    fn u64_vec(&mut self, v: &[u64]) -> Result<(), NativeOrderEncodeError> {
        self.uleb128(v.len())?;
        for x in v {
            self.u64(*x);
        }
        Ok(())
    }

    fn raw(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
}

/// BCS reader for the header fields; whatever it leaves is the metadata.
struct Reader<'a> {
    input: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input }
    }

    fn take(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], NativeOrderDecodeError> {
        if self.input.len() < n {
            return Err(NativeOrderDecodeError::UnexpectedEnd(field));
        }
        let (head, rest) = self.input.split_at(n);
        self.input = rest;
        Ok(head)
    }

    fn u8(&mut self, field: &'static str) -> Result<u8, NativeOrderDecodeError> {
        Ok(self.take(1, field)?[0])
    }

    fn bool(&mut self, field: &'static str) -> Result<bool, NativeOrderDecodeError> {
        match self.u8(field)? {
            0 => Ok(false),
            1 => Ok(true),
            tag => Err(NativeOrderDecodeError::InvalidTag { field, tag }),
        }
    }

    fn option_tag(&mut self, field: &'static str) -> Result<bool, NativeOrderDecodeError> {
        self.bool(field)
    }

    fn u64(&mut self, field: &'static str) -> Result<u64, NativeOrderDecodeError> {
        let bytes = self.take(8, field)?;
        Ok(u64::from_le_bytes(bytes.try_into().expect("8 bytes")))
    }

    fn u128(&mut self, field: &'static str) -> Result<u128, NativeOrderDecodeError> {
        let bytes = self.take(16, field)?;
        Ok(u128::from_le_bytes(bytes.try_into().expect("16 bytes")))
    }

    fn address(&mut self, field: &'static str) -> Result<AccountAddress, NativeOrderDecodeError> {
        let bytes = self.take(AccountAddress::LENGTH, field)?;
        Ok(AccountAddress::new(
            bytes.try_into().expect("address length"),
        ))
    }

    /// BCS ULEB128 length: canonical (no redundant trailing zero groups)
    /// and at most `u32::MAX`, as the BCS deserializer requires.
    fn uleb128(&mut self, field: &'static str) -> Result<usize, NativeOrderDecodeError> {
        let mut value: u64 = 0;
        for shift in (0..32).step_by(7) {
            let byte = self.u8(field)?;
            let digit = (byte & 0x7F) as u64;
            value |= digit << shift;
            if byte & 0x80 == 0 {
                if shift > 0 && digit == 0 {
                    return Err(NativeOrderDecodeError::InvalidLength);
                }
                return u32::try_from(value)
                    .map(|v| v as usize)
                    .map_err(|_| NativeOrderDecodeError::InvalidLength);
            }
        }
        Err(NativeOrderDecodeError::InvalidLength)
    }

    fn bytes(&mut self, field: &'static str) -> Result<&'a [u8], NativeOrderDecodeError> {
        let len = self.uleb128(field)?;
        self.take(len, field)
    }

    fn u64_vec(&mut self, field: &'static str) -> Result<Vec<u64>, NativeOrderDecodeError> {
        let len = self.uleb128(field)?;
        if self.input.len() / 8 < len {
            return Err(NativeOrderDecodeError::UnexpectedEnd(field));
        }
        (0..len).map(|_| self.u64(field)).collect()
    }

    fn rest(self) -> &'a [u8] {
        self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::value::{MoveStruct, MoveValue};

    fn addr(byte: u8) -> AccountAddress {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[AccountAddress::LENGTH - 1] = byte;
        AccountAddress::new(bytes)
    }

    /// A metadata type `M` with a struct and a vector, so its BCS has a
    /// nested shape and a length prefix.
    fn metadata_value() -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::U64(42),
            MoveValue::Vector(vec![MoveValue::U8(1), MoveValue::U8(2)]),
        ]))
    }

    fn metadata_bytes() -> Bytes {
        bcs::to_bytes(&metadata_value()).unwrap().into()
    }

    fn wrapped_u128(v: u128) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::U128(v)]))
    }

    fn move_option(v: Option<MoveValue>) -> MoveValue {
        // Move `Option<T>` is `struct Option { vec: vector<T> }`.
        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(
            v.into_iter().collect(),
        )]))
    }

    fn single_order_v1() -> SingleOrderV1 {
        SingleOrderV1 {
            account: addr(7),
            order_id: u128::MAX - 1,
            client_order_id: Some("coid-1".to_string()),
            price: 50_000,
            orig_size: 10,
            remaining_size: 4,
            is_bid: true,
            trigger_condition: Some(TriggerCondition::PriceMoveBelow(49_000)),
            time_in_force: TimeInForce::PostOnly,
            creation_time_micros: 1_700_000_000_000_000,
            unique_priority_idx: 123_456_789,
            metadata: metadata_bytes(),
        }
    }

    fn bulk_order_v1() -> BulkOrderV1 {
        BulkOrderV1 {
            account: addr(9),
            order_id: 77,
            sequence_number: 3,
            bid_prices: vec![100, 99, 98],
            bid_sizes: vec![1, 2, 3],
            ask_prices: vec![101, 102],
            ask_sizes: vec![4, 5],
            unique_priority_idx: 88,
            creation_time_micros: 1_700_000_000_000_001,
            metadata: metadata_bytes(),
        }
    }

    fn single(order: SingleOrderV1) -> Vec<u8> {
        NativeSingleOrder::V1(order).serialize().unwrap()
    }

    fn bulk(order: BulkOrderV1) -> Vec<u8> {
        NativeBulkOrder::V1(order).serialize().unwrap()
    }

    /// BCS length of the encoded prefix of `move_fields`, used to locate a
    /// field in the encoding without hardcoding widths.
    fn offset_of(move_fields: Vec<MoveValue>) -> usize {
        bcs::to_bytes(&MoveValue::Struct(MoveStruct::RuntimeVariant(
            0,
            move_fields,
        )))
        .unwrap()
        .len()
    }

    fn single_order_move_value(metadata: MoveValue) -> MoveValue {
        MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::Address(addr(7)),
            wrapped_u128(u128::MAX - 1),
            move_option(Some(MoveValue::Vector(
                b"coid-1".iter().map(|b| MoveValue::U8(*b)).collect(),
            ))),
            MoveValue::U64(50_000),
            MoveValue::U64(10),
            MoveValue::U64(4),
            MoveValue::Bool(true),
            move_option(Some(MoveValue::Struct(MoveStruct::RuntimeVariant(
                1,
                vec![MoveValue::U64(49_000)],
            )))),
            MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![])),
            MoveValue::U64(1_700_000_000_000_000),
            wrapped_u128(123_456_789),
            metadata,
        ]))
    }

    #[test]
    fn single_order_matches_move_bcs_encoding() {
        let move_bytes = bcs::to_bytes(&single_order_move_value(metadata_value())).unwrap();
        assert_eq!(move_bytes, single(single_order_v1()));
        assert_eq!(
            NativeSingleOrder::deserialize(&move_bytes).unwrap(),
            NativeSingleOrder::V1(single_order_v1())
        );
    }

    #[test]
    fn bulk_order_matches_move_bcs_encoding() {
        let u64s = |v: &[u64]| MoveValue::Vector(v.iter().map(|x| MoveValue::U64(*x)).collect());
        let move_value = MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![
            MoveValue::Address(addr(9)),
            wrapped_u128(77),
            MoveValue::U64(3),
            u64s(&[100, 99, 98]),
            u64s(&[1, 2, 3]),
            u64s(&[101, 102]),
            u64s(&[4, 5]),
            wrapped_u128(88),
            MoveValue::U64(1_700_000_000_000_001),
            metadata_value(),
        ]));
        let move_bytes = bcs::to_bytes(&move_value).unwrap();
        assert_eq!(move_bytes, bulk(bulk_order_v1()));
        assert_eq!(
            NativeBulkOrder::deserialize(&move_bytes).unwrap(),
            NativeBulkOrder::V1(bulk_order_v1())
        );
    }

    /// Move enum and variant ordinals are part of the BCS encoding; each is
    /// checked against the Move serializer.
    #[test]
    fn enum_ordinals_match_move() {
        for (time_in_force, ordinal) in [
            (TimeInForce::Gtc, 0u16),
            (TimeInForce::PostOnly, 1),
            (TimeInForce::Ioc, 2),
        ] {
            let order = SingleOrderV1 {
                time_in_force,
                ..single_order_v1()
            };
            let mut move_value = single_order_move_value(metadata_value());
            if let MoveValue::Struct(MoveStruct::RuntimeVariant(_, fields)) = &mut move_value {
                fields[8] = MoveValue::Struct(MoveStruct::RuntimeVariant(ordinal, vec![]));
            }
            assert_eq!(bcs::to_bytes(&move_value).unwrap(), single(order));
        }
        for (trigger, ordinal) in [
            (TriggerCondition::PriceMoveAbove(5), 0u16),
            (TriggerCondition::PriceMoveBelow(5), 1),
            (TriggerCondition::TimeBased(5), 2),
        ] {
            let order = SingleOrderV1 {
                trigger_condition: Some(trigger),
                ..single_order_v1()
            };
            let mut move_value = single_order_move_value(metadata_value());
            if let MoveValue::Struct(MoveStruct::RuntimeVariant(_, fields)) = &mut move_value {
                fields[7] = move_option(Some(MoveValue::Struct(MoveStruct::RuntimeVariant(
                    ordinal,
                    vec![MoveValue::U64(5)],
                ))));
            }
            assert_eq!(bcs::to_bytes(&move_value).unwrap(), single(order));
        }
    }

    #[test]
    fn round_trip_without_optional_fields() {
        let order = NativeSingleOrder::V1(SingleOrderV1 {
            client_order_id: None,
            trigger_condition: None,
            time_in_force: TimeInForce::Ioc,
            metadata: Bytes::new(),
            ..single_order_v1()
        });
        assert_eq!(
            NativeSingleOrder::deserialize(&order.serialize().unwrap()).unwrap(),
            order
        );

        let order = NativeBulkOrder::V1(BulkOrderV1 {
            bid_prices: vec![],
            bid_sizes: vec![],
            metadata: Bytes::new(),
            ..bulk_order_v1()
        });
        assert_eq!(
            NativeBulkOrder::deserialize(&order.serialize().unwrap()).unwrap(),
            order
        );
    }

    #[test]
    fn rejects_unknown_variant() {
        let mut bytes = single(single_order_v1());
        bytes[0] = 1;
        assert_eq!(
            NativeSingleOrder::deserialize(&bytes),
            Err(NativeOrderDecodeError::InvalidTag {
                field: "NativeSingleOrder",
                tag: 1
            }),
        );
        let mut bytes = bulk(bulk_order_v1());
        bytes[0] = 1;
        assert_eq!(
            NativeBulkOrder::deserialize(&bytes),
            Err(NativeOrderDecodeError::InvalidTag {
                field: "NativeBulkOrder",
                tag: 1
            }),
        );
    }

    #[test]
    fn rejects_truncated_header() {
        let bytes = single(single_order_v1());
        let header_len = bytes.len() - metadata_bytes().len();
        // Any cut inside the header fails; the metadata tail is opaque, so a
        // cut inside it decodes with shorter metadata.
        for len in 0..header_len {
            assert!(NativeSingleOrder::deserialize(&bytes[..len]).is_err());
        }
        let bytes = bulk(bulk_order_v1());
        let header_len = bytes.len() - metadata_bytes().len();
        for len in 0..header_len {
            assert!(NativeBulkOrder::deserialize(&bytes[..len]).is_err());
        }
    }

    #[test]
    fn rejects_invalid_tags() {
        let order = SingleOrderV1 {
            client_order_id: None,
            ..single_order_v1()
        };
        let fields = match single_order_move_value(metadata_value()) {
            MoveValue::Struct(MoveStruct::RuntimeVariant(_, fields)) => fields,
            _ => unreachable!(),
        };
        // `client_order_id` is None in `order`, so build prefixes with it.
        let mut prefix = fields[..2].to_vec();
        let coid = offset_of(prefix.clone());
        prefix.push(move_option(None));
        prefix.extend_from_slice(&fields[3..6]);
        let is_bid = offset_of(prefix.clone());
        prefix.push(fields[6].clone());
        let trigger_variant = offset_of(prefix.clone()) + 1;
        prefix.push(fields[7].clone());
        let time_in_force = offset_of(prefix);

        for (offset, tag, field) in [
            (coid, 2u8, "client_order_id"),
            (is_bid, 2, "is_bid"),
            (trigger_variant, 3, "trigger_condition"),
            (time_in_force, 3, "time_in_force"),
        ] {
            let mut bytes = single(order.clone());
            bytes[offset] = tag;
            assert_eq!(
                NativeSingleOrder::deserialize(&bytes),
                Err(NativeOrderDecodeError::InvalidTag { field, tag }),
            );
        }
    }

    #[test]
    fn rejects_non_canonical_length() {
        let order = BulkOrderV1 {
            bid_prices: vec![],
            metadata: Bytes::new(),
            ..bulk_order_v1()
        };
        let mut bytes = bulk(order);
        let offset = offset_of(vec![
            MoveValue::Address(addr(9)),
            wrapped_u128(77),
            MoveValue::U64(3),
        ]);
        assert_eq!(bytes[offset], 0);
        // Re-encode the empty bid_prices length 0 as the two-byte `0x80 0x00`.
        bytes.splice(offset..offset + 1, [0x80, 0x00]);
        assert_eq!(
            NativeBulkOrder::deserialize(&bytes),
            Err(NativeOrderDecodeError::InvalidLength),
        );
    }

    #[test]
    fn rejects_invalid_utf8_client_order_id() {
        let mut bytes = single(single_order_v1());
        // First byte of the client order id: past its option tag and length.
        let offset = offset_of(vec![
            MoveValue::Address(addr(7)),
            wrapped_u128(u128::MAX - 1),
        ]) + 2;
        bytes[offset] = 0xFF;
        assert_eq!(
            NativeSingleOrder::deserialize(&bytes),
            Err(NativeOrderDecodeError::InvalidUtf8),
        );
    }

    #[test]
    fn rejects_length_beyond_u32() {
        let mut w = Writer::default();
        assert_eq!(
            w.uleb128(u32::MAX as usize + 1),
            Err(NativeOrderEncodeError::LengthTooLarge(
                u32::MAX as usize + 1
            )),
        );
        assert_eq!(w.uleb128(u32::MAX as usize), Ok(()));
    }
}
