// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    value_exchange::{
        deserialize_and_exchange, serialize_and_exchange, ExchangeError, ExchangeResult,
        ValueExchange,
    },
    values::{Struct, Value, ValueImpl},
};
use move_core_types::value::{LayoutTag, MoveStructLayout::Runtime, MoveTypeLayout};
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(test)]
#[derive(Debug, Default)]
struct TestExchange {
    // For testing purposes, all swapped data is stored in a map.
    data: RefCell<BTreeMap<u64, u128>>,
}

#[cfg(test)]
impl ValueExchange for TestExchange {
    fn try_exchange(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        // Identifiers are generated using the number of entries stored
        // so far.
        let mut data = self.data.borrow_mut();
        let id = data.len() as u64;

        match value_to_exchange.0 {
            ValueImpl::U64(x) => {
                data.insert(id, x as u128);
                Ok(Value(ValueImpl::U64(id)))
            },
            ValueImpl::U128(x) => {
                data.insert(id, x);
                Ok(Value(ValueImpl::U128(id as u128)))
            },
            _ => {
                Err(ExchangeError(format!(
                    "Cannot exchange value {:?}",
                    value_to_exchange
                )))
            },
        }
    }

    fn try_claim_back(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        match value_to_exchange.0 {
            ValueImpl::U64(x) => {
                let v = *self
                    .data
                    .borrow()
                    .get(&x)
                    .expect("Claimed value should always exist");
                // SAFETY: we previously upcasted u64 to u128.
                Ok(Value(ValueImpl::U64(v as u64)))
            },
            ValueImpl::U128(x) => {
                // SAFETY: x is an identifier and is a u64.
                let v = *self
                    .data
                    .borrow()
                    .get(&(x as u64))
                    .expect("Claimed value should always exist");
                Ok(Value(ValueImpl::U128(v)))
            },
            _ => {
                Err(ExchangeError(format!(
                    "Cannot claim back with value {:?}",
                    value_to_exchange
                )))
            },
        }
    }
}

#[test]
fn test() {
    let exchange = TestExchange::default();

    // We should swap 2nd and 3rd fields.
    let value = Value::struct_(Struct::pack(vec![
        Value::u64(100),
        Value::u128(101),
        Value::u64(102),
    ]));

    let layout = MoveTypeLayout::Struct(Runtime(vec![
        MoveTypeLayout::U64,
        MoveTypeLayout::Tagged(LayoutTag::AggregatorLifting, Box::new(MoveTypeLayout::U128)),
        MoveTypeLayout::Tagged(LayoutTag::AggregatorLifting, Box::new(MoveTypeLayout::U64)),
    ]));

    // Construct a blob, and then deserialize it, at the same time replacing
    // tagged values with identifiers.
    let blob = value.simple_serialize(&layout).unwrap();
    let patched_value = deserialize_and_exchange(&blob, &layout, &exchange).unwrap();

    let expected_patched_value = Value::struct_(Struct::pack(vec![
        Value::u64(100),
        Value::u128(0),
        Value::u64(1),
    ]));
    assert!(patched_value.equals(&expected_patched_value).unwrap());

    // Then patch the value back while serializing, and reconstruct the
    // original one.
    let blob = serialize_and_exchange(&patched_value, &layout, &exchange).unwrap();
    let final_value = Value::simple_deserialize(&blob, &layout).unwrap();
    assert!(value.equals(&final_value).unwrap());
}
