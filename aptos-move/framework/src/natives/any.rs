// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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
