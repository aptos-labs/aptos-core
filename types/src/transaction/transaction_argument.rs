// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use move_deps::move_core_types::transaction_argument::VecBytes as MoveVecBytes;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use serde_bytes::ByteBuf;
use std::fmt::{Debug, Formatter};

pub use move_deps::move_core_types::{
    parser::parse_transaction_argument, transaction_argument::TransactionArgument,
};

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct VecBytes(MoveVecBytes);

impl Debug for VecBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.clone().into_vec(), f)
    }
}

impl Serialize for VecBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data = self.0.clone().into_vec();
        let length = data.len();
        let mut seq = serializer.serialize_seq(Some(length))?;
        for e in data {
            seq.serialize_element(&ByteBuf::from(&e[..]))?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for VecBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(::serde::Deserialize)]
        #[serde(rename = "VecBytes")]
        struct Value(Vec<ByteBuf>);

        let value = Value::deserialize(deserializer)?;

        Ok(VecBytes::from(
            value.0.iter().map(|e| e.clone().into_vec()).collect(),
        ))
    }
}

impl VecBytes {
    pub fn from(vec_bytes: Vec<Vec<u8>>) -> Self {
        VecBytes(MoveVecBytes::from(vec_bytes))
    }

    pub fn into_vec(self) -> Vec<Vec<u8>> {
        self.0.into_vec()
    }
}
