use crate::models::property_map::PropertyMap;
use aptos_api_types::Address;
use bcs;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Convert the bcs serialized vector<u8> to its original string format
pub fn convert_bcs_hex(typ: String, value: String) -> Option<String> {
    let decoded = hex::decode(value.strip_prefix("0x").unwrap_or(&*value)).ok()?;

    match typ.as_str() {
        "0x1::string::String" => bcs::from_bytes::<String>(decoded.as_slice()),
        "u8" => bcs::from_bytes::<u8>(decoded.as_slice()).map(|e| format!("{}", e)),
        "u64" => bcs::from_bytes::<u64>(decoded.as_slice()).map(|e| format!("{}", e)),
        "u128" => bcs::from_bytes::<u128>(decoded.as_slice()).map(|e| format!("{}", e)),
        "bool" => bcs::from_bytes::<bool>(decoded.as_slice()).map(|e| format!("{}", e)),
        "address" => bcs::from_bytes::<Address>(decoded.as_slice()).map(|e| format!("{}", e)),
        _ => Ok(value),
    }
    .ok()
}

/// Convert the vector<u8> that is directly generated from b"xxx"
pub fn convert_hex(val: String) -> Option<String> {
    let decoded = hex::decode(val.strip_prefix("0x").unwrap_or(&*val)).ok()?;
    String::from_utf8(decoded).ok()
}

/// Convert the json serialized PropertyMap's inner BCS fields to their original value in string format
pub fn convert_bcs_propertymap(s: Value) -> Option<Value> {
    match PropertyMap::from_bsc_encode_str(s) {
        Some(e) => match serde_json::to_value(&e) {
            Ok(val) => Some(val),
            Err(_) => None,
        },
        None => None,
    }
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

pub fn deserialize_string_from_bcs_hexstring<'de, D>(
    deserializer: D,
) -> core::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;
    Ok(convert_bcs_hex("0x1::string::String".to_string(), s.clone()).unwrap_or(s))
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
    Ok(convert_bcs_propertymap(s.clone()).unwrap_or_else(|| s))
}

#[derive(Serialize, Deserialize, Debug)]
struct DummyEvent {
    #[serde(deserialize_with = "deserialize_string_from_bcs_hexstring")]
    name: String,
}

#[test]
fn test_deserialize_string_from_hexstring() {
    let mut val = "6170746f735f636f696e".to_string();
    val = convert_hex(val).unwrap();
    assert_eq!(val, "aptos_coin");
}

#[test]
fn test_deserialize_string_from_bcs() {
    let hex_val = hex::encode(bcs::to_bytes("hello").unwrap());
    let test_struct = DummyEvent {
        name: hex_val,
    };
    let val = serde_json::to_string(&test_struct).unwrap();
    let d: DummyEvent = serde_json::from_str(val.as_str()).unwrap();
    assert_eq!(d.name.as_str(), "hello");
}

#[test]
fn test_deserialize_propertymap_from_bcs() {
    let data = r#"
        {
            "map":
                {
                    "data": [
                        {
                            "key": "Yuri",
                            "value": {"type": "0x1::string::String", "value": "0568656c6c6f"}
                        },
                        {
                            "key": "Sergi",
                            "value": {"type": "u8", "value": "0a"}
                        }
                    ]
                }
        }"#;
    let v: Value = serde_json::from_str(data).unwrap();
    let a = convert_bcs_propertymap(v).unwrap();
    assert_eq!(a["data"]["Sergi"]["value"], "10");
    assert_eq!(a["data"]["Yuri"]["value"], "hello");
}
