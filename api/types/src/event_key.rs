// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_openapi::{impl_poem_parameter, impl_poem_type};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct EventKey(aptos_types::event::EventKey);

impl From<aptos_types::event::EventKey> for EventKey {
    fn from(val: aptos_types::event::EventKey) -> Self {
        Self(val)
    }
}

impl From<EventKey> for aptos_types::event::EventKey {
    fn from(val: EventKey) -> Self {
        val.0
    }
}

impl FromStr for EventKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        let value = s.strip_prefix("0x").unwrap_or(s);
        let inner_event: aptos_types::event::EventKey = bcs::from_bytes(&hex::decode(value)?)?;
        Ok(inner_event.into())
    }
}

impl Serialize for EventKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EventKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hash = <String>::deserialize(deserializer)?;
        hash.parse().map_err(D::Error::custom)
    }
}

impl fmt::Display for EventKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl_poem_type!(EventKey);
impl_poem_parameter!(EventKey);

#[cfg(test)]
mod tests {
    use crate::event_key::EventKey;

    use serde_json::{json, Value};

    #[test]
    fn test_from_and_to_string() {
        let hash =
            "0x0000000000000000000000000000000000000000000000000000000000000000000000000a550c18";
        assert_eq!(hash.parse::<EventKey>().unwrap().to_string(), hash);
    }

    #[test]
    fn test_from_and_to_json() {
        let hex =
            "0x0000000000000000000000000000000000000000000000000000000000000000000000000a550c18";
        let hash: EventKey = serde_json::from_value(json!(hex)).unwrap();
        assert_eq!(hash, hex.parse().unwrap());

        let val: Value = serde_json::to_value(hash).unwrap();
        assert_eq!(val, json!(hex));
    }
}
