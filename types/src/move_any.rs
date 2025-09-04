// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::as_move_value::AsMoveValue;
use anyhow::bail;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Rust representation of the Move Any type
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Any {
    pub type_name: String,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl Any {
    pub fn pack<T: Serialize>(move_name: &str, x: T) -> Any {
        Any {
            type_name: move_name.to_string(),
            data: bcs::to_bytes(&x).unwrap(),
        }
    }

    pub fn unpack<T: DeserializeOwned>(move_name: &str, x: Any) -> anyhow::Result<T> {
        let Any { type_name, data } = x;
        if type_name == move_name {
            let y = bcs::from_bytes::<T>(&data)?;
            Ok(y)
        } else {
            bail!("type mismatch")
        }
    }
}

impl AsMoveValue for Any {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.type_name.as_move_value(),
            self.data.as_move_value(),
        ]))
    }
}

pub trait AsMoveAny: Serialize {
    const MOVE_TYPE_NAME: &'static str;

    fn as_move_any(&self) -> Any
    where
        Self: Sized,
    {
        Any::pack(Self::MOVE_TYPE_NAME, self)
    }
}
