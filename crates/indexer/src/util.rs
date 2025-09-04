// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::models::property_map::{PropertyMap, TokenObjectPropertyMap};
use velor_api_types::Address;
use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use sha2::Digest;

// 9999-12-31 23:59:59, this is the max supported by Google BigQuery
pub const MAX_TIMESTAMP_SECS: i64 = 253_402_300_799;

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
    let seconds = ts / 1000000;
    let ns = (ts % 1000000 * 1000).try_into().unwrap_or_else(|_| {
        panic!(
            "Could not get nanoseconds for timestamp {:?} for version {}",
            ts, version
        )
    });
    #[allow(deprecated)]
    chrono::NaiveDateTime::from_timestamp_opt(seconds as i64, ns)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

pub fn parse_timestamp_secs(ts: u64, version: i64) -> chrono::NaiveDateTime {
    #[allow(deprecated)]
    chrono::NaiveDateTime::from_timestamp_opt(
        std::cmp::min(ts, MAX_TIMESTAMP_SECS as u64) as i64,
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
        },
        Value::Object(object) => {
            for (_key, value) in object {
                recurse_remove_null_bytes_from_json(value);
            }
        },
        Value::String(str) => {
            if !str.is_empty() {
                let replacement = string_null_byte_replacement(str);
                *str = replacement;
            }
        },
        _ => {},
    }
}

fn string_null_byte_replacement(value: &mut str) -> String {
    value.replace('\u{0000}', "").replace("\\u0000", "")
}

/// convert the bcs encoded inner value of property_map to its original value in string format
pub fn deserialize_property_map_from_bcs_hexstring<'de, D>(
    deserializer: D,
) -> core::result::Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let s = serde_json::Value::deserialize(deserializer)?;
    // iterate the json string to convert key-value pair
    // assume the format of {“map”: {“data”: [{“key”: “Yuri”, “value”: {“type”: “String”, “value”: “0x42656e”}}, {“key”: “Tarded”, “value”: {“type”: “String”, “value”: “0x446f766572"}}]}}
    // if successfully parsing we return the decoded property_map string otherwise return the original string
    Ok(convert_bcs_propertymap(s.clone()).unwrap_or(s))
}

/// convert the bcs encoded inner value of property_map to its original value in string format
pub fn deserialize_token_object_property_map_from_bcs_hexstring<'de, D>(
    deserializer: D,
) -> core::result::Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let s = serde_json::Value::deserialize(deserializer)?;
    // iterate the json string to convert key-value pair
    Ok(convert_bcs_token_object_propertymap(s.clone()).unwrap_or(s))
}

pub fn deserialize_string_from_hexstring<'de, D>(
    deserializer: D,
) -> core::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;
    Ok(convert_hex(s.clone()).unwrap_or(s))
}

/// Convert the bcs serialized vector<u8> to its original string format
pub fn convert_bcs_hex(typ: String, value: String) -> Option<String> {
    let decoded = hex::decode(value.strip_prefix("0x").unwrap_or(&*value)).ok()?;

    match typ.as_str() {
        "0x1::string::String" => bcs::from_bytes::<String>(decoded.as_slice()),
        "u8" => bcs::from_bytes::<u8>(decoded.as_slice()).map(|e| e.to_string()),
        "u64" => bcs::from_bytes::<u64>(decoded.as_slice()).map(|e| e.to_string()),
        "u128" => bcs::from_bytes::<u128>(decoded.as_slice()).map(|e| e.to_string()),
        "bool" => bcs::from_bytes::<bool>(decoded.as_slice()).map(|e| e.to_string()),
        "address" => bcs::from_bytes::<Address>(decoded.as_slice()).map(|e| e.to_string()),
        _ => Ok(value),
    }
    .ok()
}

/// Convert the bcs serialized vector<u8> to its original string format for token v2 property map.
pub fn convert_bcs_hex_new(typ: u8, value: String) -> Option<String> {
    let decoded = hex::decode(value.strip_prefix("0x").unwrap_or(&*value)).ok()?;

    match typ {
        0 /* bool */ => bcs::from_bytes::<bool>(decoded.as_slice()).map(|e| e.to_string()),
        1 /* u8 */ => bcs::from_bytes::<u8>(decoded.as_slice()).map(|e| e.to_string()),
        2 /* u16 */ => bcs::from_bytes::<u16>(decoded.as_slice()).map(|e| e.to_string()),
        3 /* u32 */ => bcs::from_bytes::<u32>(decoded.as_slice()).map(|e| e.to_string()),
        4 /* u64 */ => bcs::from_bytes::<u64>(decoded.as_slice()).map(|e| e.to_string()),
        5 /* u128 */ => bcs::from_bytes::<u128>(decoded.as_slice()).map(|e| e.to_string()),
        6 /* u256 */ => bcs::from_bytes::<BigDecimal>(decoded.as_slice()).map(|e| e.to_string()),
        7 /* address */ => bcs::from_bytes::<Address>(decoded.as_slice()).map(|e| e.to_string()),
        8 /* byte_vector */ => bcs::from_bytes::<Vec<u8>>(decoded.as_slice()).map(|e| format!("0x{}", hex::encode(e))),
        9 /* string */ => bcs::from_bytes::<String>(decoded.as_slice()),
        _ => Ok(value),
    }
        .ok()
}

/// Convert the json serialized PropertyMap's inner BCS fields to their original value in string format
pub fn convert_bcs_propertymap(s: Value) -> Option<Value> {
    match PropertyMap::from_bcs_encode_str(s) {
        Some(e) => match serde_json::to_value(&e) {
            Ok(val) => Some(val),
            Err(_) => None,
        },
        None => None,
    }
}

pub fn convert_bcs_token_object_propertymap(s: Value) -> Option<Value> {
    match TokenObjectPropertyMap::from_bcs_encode_str(s) {
        Some(e) => match serde_json::to_value(&e) {
            Ok(val) => Some(val),
            Err(_) => None,
        },
        None => None,
    }
}

/// Convert the vector<u8> that is directly generated from b"xxx"
pub fn convert_hex(val: String) -> Option<String> {
    let decoded = hex::decode(val.strip_prefix("0x").unwrap_or(&*val)).ok()?;
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use serde::Serialize;

    #[derive(Serialize, Deserialize, Debug)]
    struct TypeInfoMock {
        #[serde(deserialize_with = "deserialize_string_from_hexstring")]
        pub module_name: String,
        #[serde(deserialize_with = "deserialize_string_from_hexstring")]
        pub struct_name: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TokenDataMock {
        #[serde(deserialize_with = "deserialize_property_map_from_bcs_hexstring")]
        pub default_properties: serde_json::Value,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TokenObjectDataMock {
        #[serde(deserialize_with = "deserialize_token_object_property_map_from_bcs_hexstring")]
        pub default_properties: serde_json::Value,
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp(1649560602763949, 1);
        assert_eq!(ts.and_utc().timestamp(), 1649560602);
        assert_eq!(ts.nanosecond(), 763949000);
        assert_eq!(ts.year(), 2022);

        let ts2 = parse_timestamp_secs(600000000000000, 2);
        assert_eq!(ts2.year(), 9999);

        let ts3 = parse_timestamp_secs(1659386386, 2);
        assert_eq!(ts3.and_utc().timestamp(), 1659386386);
    }

    #[test]
    fn test_deserialize_string_from_bcs() {
        let test_struct = TypeInfoMock {
            module_name: String::from("0x6170746f735f636f696e"),
            struct_name: String::from("0x4170746f73436f696e"),
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TypeInfoMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.module_name.as_str(), "velor_coin");
        assert_eq!(d.struct_name.as_str(), "VelorCoin");
    }

    #[test]
    fn test_deserialize_property_map() {
        let test_property_json = r#"
        {
            "map":{
               "data":[
                  {
                     "key":"type",
                     "value":{
                        "type":"0x1::string::String",
                        "value":"0x06646f6d61696e"
                     }
                  },
                  {
                     "key":"creation_time_sec",
                     "value":{
                        "type":"u64",
                        "value":"0x140f4f6300000000"
                     }
                  },
                  {
                     "key":"expiration_time_sec",
                     "value":{
                        "type":"u64",
                        "value":"0x9442306500000000"
                     }
                  }
               ]
            }
        }"#;
        let test_property_json: serde_json::Value =
            serde_json::from_str(test_property_json).unwrap();
        let test_struct = TokenDataMock {
            default_properties: test_property_json,
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TokenDataMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.default_properties["type"], "domain");
        assert_eq!(d.default_properties["creation_time_sec"], "1666125588");
        assert_eq!(d.default_properties["expiration_time_sec"], "1697661588");
    }

    #[test]
    fn test_empty_property_map() {
        let test_property_json = r#"{"map": {"data": []}}"#;
        let test_property_json: serde_json::Value =
            serde_json::from_str(test_property_json).unwrap();
        let test_struct = TokenDataMock {
            default_properties: test_property_json,
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TokenDataMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.default_properties, Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn test_deserialize_token_object_property_map() {
        let test_property_json = r#"
        {
            "data": [{
                    "key": "Rank",
                    "value": {
                        "type": 9,
                        "value": "0x0642726f6e7a65"
                    }
                },
                {
                    "key": "address_property",
                    "value": {
                        "type": 7,
                        "value": "0x2b4d540735a4e128fda896f988415910a45cab41c9ddd802b32dd16e8f9ca3cd"
                    }
                },
                {
                    "key": "bytes_property",
                    "value": {
                        "type": 8,
                        "value": "0x0401020304"
                    }
                },
                {
                    "key": "u64_property",
                    "value": {
                        "type": 4,
                        "value": "0x0000000000000001"
                    }
                }
            ]
        }
        "#;
        let test_property_json: serde_json::Value =
            serde_json::from_str(test_property_json).unwrap();
        let test_struct = TokenObjectDataMock {
            default_properties: test_property_json,
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TokenObjectDataMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.default_properties["Rank"], "Bronze");
        assert_eq!(
            d.default_properties["address_property"],
            "0x2b4d540735a4e128fda896f988415910a45cab41c9ddd802b32dd16e8f9ca3cd"
        );
        assert_eq!(d.default_properties["bytes_property"], "0x01020304");
        assert_eq!(d.default_properties["u64_property"], "72057594037927936");
    }

    #[test]
    fn test_empty_token_object_property_map() {
        let test_property_json = r#"{"data": []}"#;
        let test_property_json: serde_json::Value =
            serde_json::from_str(test_property_json).unwrap();
        let test_struct = TokenObjectDataMock {
            default_properties: test_property_json,
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TokenObjectDataMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.default_properties, Value::Object(serde_json::Map::new()));
    }
}
