// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use serde_json::Value;
use sha2::Digest;

// 9999-12-31 23:59:59, this is the max supported by Google BigQuery
pub const MAX_TIMESTAMP_SECS: u64 = 253_402_300_799;

/// Standardizes all addresses and table handles to be length 66 (0x-64 length hash)
pub fn standardize_address(handle: &str) -> String {
    format!("0x{:0>64}", &handle[2..])
}

pub fn hash_str(val: &str) -> String {
    hex::encode(sha2::Sha256::digest(val.as_bytes()))
}

pub fn truncate_str(val: &str, max_chars: usize) -> String {
    let mut trunc = val.to_string();
    trunc.truncate(max_chars);
    trunc
}

pub fn u64_to_bigdecimal(val: u64) -> BigDecimal {
    BigDecimal::from(val)
}

#[allow(dead_code)]
pub fn bigdecimal_to_u64(val: &BigDecimal) -> u64 {
    val.to_u64().expect("Unable to convert big decimal to u64")
}

pub fn ensure_not_negative(val: BigDecimal) -> BigDecimal {
    if val.is_negative() {
        return BigDecimal::zero();
    }
    val
}

pub fn parse_timestamp(ts: u64, version: i64) -> chrono::NaiveDateTime {
    let defaulted_secs = std::cmp::min(ts / 1000000, MAX_TIMESTAMP_SECS);
    chrono::NaiveDateTime::from_timestamp_opt(defaulted_secs as i64, 0).unwrap_or_else(|| {
        panic!(
            "Could not parse timestamp (ms) {:?} for version {}",
            ts, version
        )
    })
}

pub fn parse_timestamp_secs(ts: u64, version: i64) -> chrono::NaiveDateTime {
    let defaulted_secs = std::cmp::min(ts, MAX_TIMESTAMP_SECS);
    chrono::NaiveDateTime::from_timestamp_opt(defaulted_secs as i64, 0).unwrap_or_else(|| {
        panic!(
            "Could not parse timestamp (s) {:?} for version {}",
            ts, version
        )
    })
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

        let ts = parse_timestamp(1649560602763949, 1);
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), current_year);

        let ts2 = parse_timestamp_secs(600000000000000, 2);
        assert_eq!(ts2.year(), 9999);

        let ts3 = parse_timestamp_secs(1659386386, 2);
        assert_eq!(ts3.timestamp(), 1659386386);

        let ts4 = parse_timestamp(ts3.timestamp_micros() as u64, 2);
        assert_eq!(ts3, ts4);

        let ts5 = parse_timestamp_secs(MAX_TIMESTAMP_SECS, 2);
        assert_eq!(ts5.year(), 9999);

        let ts6 = parse_timestamp_secs(MAX_TIMESTAMP_SECS + 1, 2);
        let ts7 = parse_timestamp_secs(u64::MAX, 2);
        assert_eq!(ts5, ts6);
        assert_eq!(ts6, ts7);

        let ts8 = parse_timestamp(MAX_TIMESTAMP_SECS * 1000000, 2);
        assert_eq!(ts5, ts8);

        let ts9 = parse_timestamp(ts7.timestamp_micros() as u64, 2);
        assert_eq!(ts7, ts9);
    }
}
