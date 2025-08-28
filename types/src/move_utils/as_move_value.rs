// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::value::{MoveStruct, MoveValue};

pub trait AsMoveValue {
    fn as_move_value(&self) -> MoveValue;
}

impl<T: AsMoveValue> AsMoveValue for Option<T> {
    fn as_move_value(&self) -> MoveValue {
        let items = if let Some(obj) = self.as_ref() {
            vec![obj.as_move_value()]
        } else {
            vec![]
        };

        MoveValue::Struct(MoveStruct::Runtime(vec![MoveValue::Vector(items)]))
    }
}

impl AsMoveValue for String {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self
            .clone()
            .into_bytes()
            .as_move_value()]))
    }
}

impl<T: AsMoveValue> AsMoveValue for Vec<T> {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Vector(self.iter().map(T::as_move_value).collect())
    }
}

impl AsMoveValue for bool {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Bool(*self)
    }
}

impl AsMoveValue for u8 {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::U8(*self)
    }
}

impl AsMoveValue for u16 {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::U16(*self)
    }
}

impl AsMoveValue for u32 {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::U32(*self)
    }
}

impl AsMoveValue for u64 {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::U64(*self)
    }
}
