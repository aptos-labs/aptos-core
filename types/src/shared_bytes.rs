// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use bytes::Bytes;
use serde::{Deserialize, Serialize, Serializer};
use std::ops::Deref;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SharedBytes(Bytes);

impl SharedBytes {
    pub fn into_vec(self) -> Vec<u8> {
        let Self(bytes) = self;
        bytes.into()
    }

    pub fn copy(bytes: &[u8]) -> Self {
        Self(Bytes::copy_from_slice(bytes))
    }

    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self(Bytes::from_static(bytes))
    }
}

impl Serialize for SharedBytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for SharedBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: &[u8] = Deserialize::deserialize(deserializer)?;
        Ok(Self(Bytes::copy_from_slice(bytes)))
    }
}

impl<T> From<T> for SharedBytes
where
    T: Into<Bytes>,
{
    fn from(bytes: T) -> Self {
        Self(bytes.into())
    }
}

impl Deref for SharedBytes {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
