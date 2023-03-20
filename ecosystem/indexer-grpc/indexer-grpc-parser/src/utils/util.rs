// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::models::property_map::PropertyMap;
use aptos_protos::{
    transaction::testing1::v1::{
        multisig_transaction_payload::Payload as MultisigPayloadType,
        transaction_payload::Payload as PayloadType, write_set::WriteSet as WriteSetType,
        EntryFunctionId, EntryFunctionPayload, MoveScriptBytecode, MoveType, ScriptPayload,
        TransactionPayload, UserTransactionRequest, WriteSet,
    },
    util::timestamp::Timestamp,
};
use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::str::FromStr;

// 9999-12-31 23:59:59, this is the max supported by Google BigQuery
pub const MAX_TIMESTAMP_SECS: i64 = 253_402_300_799;
// Max length of entry function id string to ensure that db doesn't explode
const MAX_ENTRY_FUNCTION_LENGTH: usize = 100;

// Supporting structs to get clean payload without escaped strings
#[derive(Debug, Deserialize, Serialize)]
pub struct EntryFunctionPayloadClean {
    pub function: Option<EntryFunctionId>,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScriptPayloadClean {
    pub code: Option<MoveScriptBytecode>,
    pub type_arguments: Vec<MoveType>,
    pub arguments: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScriptWriteSetClean {
    pub execute_as: String,
    pub script: ScriptPayloadClean,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultisigPayloadClean {
    pub multisig_address: String,
    pub transaction_payload: Option<Value>,
}

/// Standardizes all addresses and table handles to be length 66 (0x-64 length hash)
pub fn standardize_address(handle: &str) -> String {
    if let Some(handle) = handle.strip_prefix("0x") {
        format!("0x{:0>64}", handle)
    } else {
        format!("0x{:0>64}", handle)
    }
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

pub fn bigdecimal_to_u64(val: &BigDecimal) -> u64 {
    val.to_u64().expect("Unable to convert big decimal to u64")
}

pub fn ensure_not_negative(val: BigDecimal) -> BigDecimal {
    if val.is_negative() {
        return BigDecimal::zero();
    }
    val
}

pub fn get_entry_function_from_user_request(
    user_request: &UserTransactionRequest,
) -> Option<String> {
    let entry_function_id_str: String = match &user_request.payload.as_ref().unwrap().payload {
        Some(PayloadType::EntryFunctionPayload(payload)) => payload.entry_function_id_str.clone(),
        Some(PayloadType::MultisigPayload(payload)) => {
            if let Some(payload) = payload.transaction_payload.as_ref() {
                match payload.payload.as_ref().unwrap() {
                    MultisigPayloadType::EntryFunctionPayload(payload) => {
                        Some(payload.entry_function_id_str.clone())
                    },
                };
            }
            return None;
        },
        _ => return None,
    };
    Some(truncate_str(
        &entry_function_id_str,
        MAX_ENTRY_FUNCTION_LENGTH,
    ))
}

/// Part of the json comes escaped from the protobuf so we need to unescape in a safe way
pub fn get_clean_payload(payload: &TransactionPayload, version: i64) -> Option<Value> {
    match payload.payload.as_ref().unwrap() {
        PayloadType::EntryFunctionPayload(inner) => {
            let clean = get_clean_entry_function_payload(inner, version);
            Some(serde_json::to_value(clean).unwrap_or_else(|_| {
                aptos_logger::error!(version = version, "Unable to serialize payload into value");
                panic!()
            }))
        },
        PayloadType::ScriptPayload(inner) => {
            let clean = get_clean_script_payload(inner, version);
            Some(serde_json::to_value(clean).unwrap_or_else(|_| {
                aptos_logger::error!(version = version, "Unable to serialize payload into value");
                panic!()
            }))
        },
        PayloadType::ModuleBundlePayload(inner) => {
            Some(serde_json::to_value(inner).unwrap_or_else(|_| {
                aptos_logger::error!(version = version, "Unable to serialize payload into value");
                panic!()
            }))
        },
        PayloadType::WriteSetPayload(inner) => {
            if let Some(writeset) = inner.write_set.as_ref() {
                get_clean_writeset(writeset, version)
            } else {
                None
            }
        },
        PayloadType::MultisigPayload(inner) => {
            let clean = if let Some(payload) = inner.transaction_payload.as_ref() {
                let payload_clean = match payload.payload.as_ref().unwrap() {
                    MultisigPayloadType::EntryFunctionPayload(payload) => {
                        let clean = get_clean_entry_function_payload(payload, version);
                        Some(serde_json::to_value(clean).unwrap_or_else(|_| {
                            aptos_logger::error!(
                                version = version,
                                "Unable to serialize payload into value"
                            );
                            panic!()
                        }))
                    },
                };
                MultisigPayloadClean {
                    multisig_address: inner.multisig_address.clone(),
                    transaction_payload: payload_clean,
                }
            } else {
                MultisigPayloadClean {
                    multisig_address: inner.multisig_address.clone(),
                    transaction_payload: None,
                }
            };
            Some(serde_json::to_value(clean).unwrap_or_else(|_| {
                aptos_logger::error!(version = version, "Unable to serialize payload into value");
                panic!()
            }))
        },
    }
}

/// Part of the json comes escaped from the protobuf so we need to unescape in a safe way
/// Note that DirectWriteSet is just events + writeset which is already represented separately
pub fn get_clean_writeset(writeset: &WriteSet, version: i64) -> Option<Value> {
    match writeset.write_set.as_ref().unwrap() {
        WriteSetType::ScriptWriteSet(inner) => {
            let payload = inner.script.as_ref().unwrap();
            Some(
                serde_json::to_value(get_clean_script_payload(payload, version)).unwrap_or_else(
                    |_| {
                        aptos_logger::error!(
                            version = version,
                            "Unable to serialize payload into value"
                        );
                        panic!()
                    },
                ),
            )
        },
        WriteSetType::DirectWriteSet(_) => None,
    }
}

/// Part of the json comes escaped from the protobuf so we need to unescape in a safe way
fn get_clean_entry_function_payload(
    payload: &EntryFunctionPayload,
    version: i64,
) -> EntryFunctionPayloadClean {
    EntryFunctionPayloadClean {
        function: payload.function.clone(),
        type_arguments: payload.type_arguments.clone(),
        arguments: payload
            .arguments
            .iter()
            .map(|arg| {
                serde_json::from_str(arg).unwrap_or_else(|_| {
                    aptos_logger::error!(
                        version = version,
                        "Unable to serialize payload into value"
                    );
                    panic!()
                })
            })
            .collect(),
    }
}

/// Part of the json comes escaped from the protobuf so we need to unescape in a safe way
fn get_clean_script_payload(payload: &ScriptPayload, version: i64) -> ScriptPayloadClean {
    ScriptPayloadClean {
        code: payload.code.clone(),
        type_arguments: payload.type_arguments.clone(),
        arguments: payload
            .arguments
            .iter()
            .map(|arg| {
                serde_json::from_str(arg).unwrap_or_else(|_| {
                    aptos_logger::error!(
                        version = version,
                        "Unable to serialize payload into value"
                    );
                    panic!()
                })
            })
            .collect(),
    }
}

pub fn parse_timestamp(ts: &Timestamp, version: i64) -> chrono::NaiveDateTime {
    chrono::NaiveDateTime::from_timestamp_opt(ts.seconds, ts.nanos as u32)
        .unwrap_or_else(|| panic!("Could not parse timestamp {:?} for version {}", ts, version))
}

pub fn parse_timestamp_secs(ts: u64, version: i64) -> chrono::NaiveDateTime {
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
        "u8" => bcs::from_bytes::<u8>(decoded.as_slice()).map(|e| format!("{}", e)),
        "u64" => bcs::from_bytes::<u64>(decoded.as_slice()).map(|e| format!("{}", e)),
        "u128" => bcs::from_bytes::<u128>(decoded.as_slice()).map(|e| format!("{}", e)),
        "bool" => bcs::from_bytes::<bool>(decoded.as_slice()).map(|e| format!("{}", e)),
        // TODO(larry): add the address type.
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

/// Convert the vector<u8> that is directly generated from b"xxx"
pub fn convert_hex(val: String) -> Option<String> {
    let decoded = hex::decode(val.strip_prefix("0x").unwrap_or(&*val)).ok()?;
    String::from_utf8(decoded).ok()
}

/// Deserialize from string to type T
pub fn deserialize_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    s.parse::<T>().map_err(D::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
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

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp(
            &Timestamp {
                seconds: 1649560602,
                nanos: 0,
            },
            1,
        );
        assert_eq!(ts.timestamp(), 1649560602);
        assert_eq!(ts.year(), 2022);

        let ts2 = parse_timestamp_secs(600000000000000, 2);
        assert_eq!(ts2.year(), 9999);

        let ts3 = parse_timestamp_secs(1659386386, 2);
        assert_eq!(ts3.timestamp(), 1659386386);
    }

    #[test]
    fn test_deserialize_string_from_bcs() {
        let test_struct = TypeInfoMock {
            module_name: String::from("0x6170746f735f636f696e"),
            struct_name: String::from("0x4170746f73436f696e"),
        };
        let val = serde_json::to_string(&test_struct).unwrap();
        let d: TypeInfoMock = serde_json::from_str(val.as_str()).unwrap();
        assert_eq!(d.module_name.as_str(), "aptos_coin");
        assert_eq!(d.struct_name.as_str(), "AptosCoin");
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
}
