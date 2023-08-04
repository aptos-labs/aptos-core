// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    value_exchange::{
        deserialize_and_exchange, serialize_and_exchange, AsIdentifier, ExchangeResult,
        IdentifierBuilder, ValueExchange,
    },
    values::{Struct, Value},
};
use move_core_types::value::{LayoutTag, MoveStructLayout::Runtime, MoveTypeLayout};
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(test)]
#[derive(Debug, Default)]
struct TestExchange {
    // For testing purposes, all swapped data is stored in a map.
    liftings: RefCell<BTreeMap<u64, Value>>,
}

#[cfg(test)]
impl ValueExchange for TestExchange {
    fn try_exchange(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        // Identifiers are generated using the number of entries stored
        // so far.
        let mut liftings = self.liftings.borrow_mut();
        let identifier = liftings.len() as u64;

        let identifier_value = value_to_exchange.build_identifier(identifier).unwrap();
        liftings.insert(identifier, value_to_exchange);
        Ok(identifier_value)
    }

    fn try_claim_back(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        let liftings = self.liftings.borrow();

        let identifier = value_to_exchange.as_identifier().unwrap();
        Ok(liftings.get(&identifier).unwrap().copy_value().unwrap())
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
