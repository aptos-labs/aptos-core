// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct HashValue(diem_crypto::hash::HashValue);

impl From<diem_crypto::hash::HashValue> for HashValue {
    fn from(val: diem_crypto::hash::HashValue) -> Self {
        Self(val)
    }
}

impl From<HashValue> for diem_crypto::hash::HashValue {
    fn from(val: HashValue) -> Self {
        val.0
    }
}

impl FromStr for HashValue {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        if let Some(hex) = s.strip_prefix("0x") {
            Ok(diem_crypto::hash::HashValue::from_str(hex)?.into())
        } else {
            Ok(diem_crypto::hash::HashValue::from_str(s)?.into())
        }
    }
}

impl Serialize for HashValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HashValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hash = <String>::deserialize(deserializer)?;
        HashValue::from_str(&hash).map_err(D::Error::custom)
    }
}

impl fmt::Display for HashValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::HashValue;

    use serde_json::{json, Value};
    use std::str::FromStr;

    #[test]
    fn test_from_and_to_string() {
        let hash = "0xb78e1ba6fa7f7b3a3f3ac2a31e6675d84f2261c711c3b438a252f648b26df3ed";
        assert_eq!(HashValue::from_str(hash).unwrap().to_string(), hash);

        let hash_without_prefix =
            "b78e1ba6fa7f7b3a3f3ac2a31e6675d84f2261c711c3b438a252f648b26df3ed";
        assert_eq!(
            HashValue::from_str(hash_without_prefix)
                .unwrap()
                .to_string(),
            hash
        );
    }

    #[test]
    fn test_from_and_to_json() {
        let hex = "0xb78e1ba6fa7f7b3a3f3ac2a31e6675d84f2261c711c3b438a252f648b26df3ed";
        let hash: HashValue = serde_json::from_value(json!(hex)).unwrap();
        assert_eq!(hash, HashValue::from_str(hex).unwrap());

        let val: Value = serde_json::to_value(hash).unwrap();
        assert_eq!(val, json!(hex));
    }
}
