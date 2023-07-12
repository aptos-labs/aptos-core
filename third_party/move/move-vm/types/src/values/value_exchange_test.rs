// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{ExchangeError, ExchangeResult, Identifier, Struct, Value, ValueExchange};
use move_core_types::value::{LayoutTag, MoveStructLayout::Runtime, MoveTypeLayout};
use std::{cell::RefCell, collections::BTreeMap};

#[cfg(test)]
#[derive(Debug, Default)]
struct TestExchange {
    // For testing purposes, all swapped data is stored in a map.
    data: RefCell<BTreeMap<Identifier, Value>>,
}

#[cfg(test)]
impl ValueExchange for TestExchange {
    fn record_value(&self, value: Value) -> ExchangeResult<Identifier> {
        let mut data = self.data.borrow_mut();
        // Identifiers can be generated using the number of entries stored
        // so far.
        let id = Identifier(data.len() as u64);
        data.insert(id, value);
        Ok(id)
    }

    fn claim_value(&self, id: Identifier) -> ExchangeResult<Value> {
        self.data
            .borrow()
            .get(&id)
            .ok_or_else(|| ExchangeError(format!("Value for id {:?} does not exist", id)))
            .map(|v| {
                // Because we only have a reference to a value, we need to copy
                // it out. In general, should not be a big problem.
                v.copy_value()
                    .map_err(|_| ExchangeError("Error while copying a value".to_string()))
            })?
    }
}

#[test]
fn test() {
    let exchange = TestExchange::default();

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
    // marked values with identifiers.
    let blob = value.simple_serialize(&layout).unwrap();
    let patched_value = Value::deserialize_with_exchange(&blob, &layout, &exchange).unwrap();

    let expected_patched_value = Value::struct_(Struct::pack(vec![
        Value::u64(100),
        Value::u128(0),
        Value::u64(1),
    ]));
    assert!(patched_value.equals(&expected_patched_value).unwrap());

    // Then patch the value back while serializing, and reconstruct the
    // original one.
    let blob = patched_value
        .serialize_with_exchange(&layout, &exchange)
        .unwrap();
    let final_value = Value::simple_deserialize(&blob, &layout).unwrap();
    assert!(value.equals(&final_value).unwrap());
}
