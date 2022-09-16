// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::U64;
use bigdecimal::{FromPrimitive, Signed, ToPrimitive, Zero};
use serde_json::Value;
use sha2::Digest;

pub fn hash_str(val: &str) -> String {
    hex::encode(sha2::Sha256::digest(val.as_bytes()))
}
pub fn u64_to_bigdecimal(val: u64) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_u64(val).expect("Unable to convert u64 to big decimal")
}

#[allow(dead_code)]
pub fn bigdecimal_to_u64(val: &bigdecimal::BigDecimal) -> u64 {
    val.to_u64().expect("Unable to convert big decimal to u64")
}

pub fn ensure_not_negative(val: bigdecimal::BigDecimal) -> bigdecimal::BigDecimal {
    if val.is_negative() {
        return bigdecimal::BigDecimal::zero();
    }
    val
}

pub fn parse_timestamp(ts: U64, version: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt((*ts.inner() / 1000000) as i64, 0)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

pub fn parse_timestamp_secs(ts: U64, version: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(
        std::cmp::min(*ts.inner(), chrono::NaiveDateTime::MAX.timestamp() as u64) as i64,
        0,
    )
    .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

pub fn remove_null_bytes<T: serde::Serialize + for<'de> serde::Deserialize<'de>>(input: &T) -> T {
    let mut txn_json = serde_json::to_value(input).unwrap();
    recurse_remove_null_bytes_from_json(&mut txn_json);
    serde_json::from_value::<T>(txn_json).unwrap()
}

fn recurse_remove_null_bytes_from_json(sub_json: &mut Value) {
    match sub_json {
        Value::Array(array) => {
            for item in array {
                recurse_remove_null_bytes_from_json(item);
            }
        }
        Value::Object(object) => {
            for (_key, value) in object {
                recurse_remove_null_bytes_from_json(value);
            }
        }
        Value::String(str) => {
            if !str.is_empty() {
                let replacement = string_null_byte_replacement(str);
                *str = replacement;
            }
        }
        _ => {}
    }
}

fn string_null_byte_replacement(value: &mut str) -> String {
    value.replace('\u{0000}', "").replace("\\u0000", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_timestamp() {
        let current_year = chrono::offset::Utc::now().year();

        let ts = parse_timestamp(U64::from(1649560602763949), 1);
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), current_year);

        let ts2 = parse_timestamp_secs(U64::from(600000000000000), 2);
        assert_eq!(ts2.year(), chrono::NaiveDateTime::MAX.date().year());

        let ts3 = parse_timestamp_secs(U64::from(1659386386), 2);
        assert_eq!(ts3.timestamp(), 1659386386);
    }
}
