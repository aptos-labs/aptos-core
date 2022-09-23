use serde::{Serialize, Deserialize, Deserializer};
use serde::de::Error;

pub fn deserialize_bcs_from_string<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
{
    // convert string to bytes
    let s = <String>::deserialize(deserializer)?;
    let decoded = hex::decode(s).expect("Hex string to bcs decoding failed");
    let org_string = bcs::from_bytes::<String>(decoded.as_slice()).map_err(D::Error::custom);
    org_string
}

#[derive(Serialize, Deserialize, Debug)]
struct DummyEvent{
    #[serde(deserialize_with = "deserialize_bcs_from_string")]
    name: String,
}

#[test]
fn test_deserialize_bcs_from_string(){
    let hex_val = hex::encode(bcs::to_bytes("hello").unwrap());
    let test_struct = DummyEvent {
        name: hex_val
    };
    let val = serde_json::to_string(&test_struct).unwrap();
    let d: DummyEvent = serde_json::from_str(val.as_str()).unwrap();
    assert_eq!(d.name.as_str(), "hello");
}

