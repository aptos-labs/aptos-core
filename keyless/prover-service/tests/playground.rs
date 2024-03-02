use aptos_crypto::poseidon_bn254;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pepper(
    #[serde(with = "hex::serde")] pub(crate) [u8; poseidon_bn254::BYTES_PACKED_PER_SCALAR],
);

impl Pepper {
    pub const NUM_BYTES: usize = poseidon_bn254::BYTES_PACKED_PER_SCALAR;

    pub fn new(bytes: [u8; Self::NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> &[u8; Self::NUM_BYTES] {
        &self.0
    }

    pub fn from_hex(hex: &str) -> Self {
        let bytes = hex::decode(hex).unwrap();
        let mut extended_bytes = [0u8; Self::NUM_BYTES];
        extended_bytes.copy_from_slice(&bytes);
        Self(extended_bytes)
    }

    // Used for testing. #[cfg(test)] doesn't seem to allow for use in smoke tests.
    pub fn from_number(num: u128) -> Self {
        let big_int = num_bigint::BigUint::from(num);
        let bytes: Vec<u8> = big_int.to_bytes_le();
        let mut extended_bytes = [0u8; Self::NUM_BYTES];
        extended_bytes[..bytes.len()].copy_from_slice(&bytes);
        Self(extended_bytes)
    }
}

#[test]
fn test_pepper_encoding() {
    let pepper = Pepper::from_number(42);
    println!("{}", serde_json::to_string(&pepper).unwrap());
}

#[test]
fn test_epk_encoding() {
    let pepper = Pepper::from_number(42);
    println!("{}", serde_json::to_string(&pepper).unwrap());
}

#[test]
fn test_hex_serialization() {
    #[derive(Serialize, Deserialize)]
    struct T1 {
        #[serde(with = "hex")]
        bytes: [u8; 4],
    }

    #[derive(Serialize, Deserialize)]
    struct T2 {
        #[serde(with = "hex")]
        bytes: Vec<u8>,
    }

    println!(
        "{}",
        serde_json::to_string(&T1 {
            bytes: [1, 2, 3, 4]
        })
        .unwrap()
    );
    println!(
        "{}",
        serde_json::to_string(&T2 {
            bytes: Vec::from([1, 2, 3, 4])
        })
        .unwrap()
    );

    assert_eq!(
        serde_json::to_string(&T1 {
            bytes: [1, 2, 3, 4]
        })
        .unwrap(),
        serde_json::to_string(&T2 {
            bytes: Vec::from([1, 2, 3, 4])
        })
        .unwrap()
    );
}
