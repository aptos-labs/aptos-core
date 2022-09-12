// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_openapi::{impl_poem_parameter, impl_poem_type};
use indoc::indoc;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// DEPRECATED, to be removed soon. See API changelog.
///
/// Event key is a global index for an event stream.
///
/// It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
/// a combination of a `uint64` creation number and account address (without
/// trimming leading zeros).
///
/// For example, event key `0x010000000000000088fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1` is combined by the following 2 parts:
/// 1. `0100000000000000`: little endian `uint64` representation of `1`.
/// 2. `88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1`: 32 bytes of account address.
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct EventKey(pub aptos_types::event::EventKey);

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

impl_poem_type!(
    EventKey,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x000000000000000088fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1 "
                .to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            Event key is a global index for an event stream.

            It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
            a combination of a `uint64` creation number and account address (without
            trimming leading zeros).

            For example, event key `0x000000000000000088fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1` is combined by the following 2 parts:
              1. `0000000000000000`: `uint64` representation of `0`.
              2. `88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1`: 32 bytes of account address.
        "})
    )
);

impl_poem_parameter!(EventKey);
