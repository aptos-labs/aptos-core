// Copyright © Aptos Foundation

use primitive_types::{H160, H256, U256};
use move_core_types::value::{MoveTypeLayout, MoveValue};

pub fn u256_to_h160(value: &U256) -> H160 {
    let mut result = [0u8; 32];
    value.to_big_endian(&mut result);
    H160::from_slice(&result[12..])
}

pub fn vec_to_h160(value: &[u8]) -> H160 {
    let mut result = [0u8; 20];
    result.copy_from_slice(value);
    H160::from_slice(&result)
}

pub fn u256_to_arr(value: &U256) -> [u8; 32] {
    let mut result = [0u8; 32];
    value.to_big_endian(&mut result);
    result
}

pub fn u256_to_move_arr(value: &U256) -> Vec<u8> {
    let move_value = MoveValue::U256(move_core_types::u256::U256::from_inner(value.clone()));
    move_value.simple_serialize().unwrap()
}

pub fn h256_to_arr(value: &H256) -> [u8; 32] {
    let mut result = [0u8; 32];
    result.copy_from_slice(value.as_bytes());
    result
}

/// Convenience function to read a 256-bit unsigned integer from storage
/// (assumes big-endian encoding).
pub fn read_u256_from_bytes(bytes: &[u8]) -> U256 {
    if bytes.len() != 32 {
        panic!("InvalidU256 length expected 32, got {}", bytes.len());
    }
    U256::from_big_endian(bytes)
}

pub fn read_u256_from_move_bytes(bytes: &[u8]) -> U256 {
    let move_value: MoveValue = MoveValue::simple_deserialize(&bytes, &MoveTypeLayout::U256).unwrap();
    move_value.to_u256().into_inner()
}

pub fn read_h256_from_bytes(bytes: &[u8]) -> H256 {
    if bytes.len() != 32 {
        panic!("InvalidU256 length expected 32, got {}", bytes.len());
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    H256(buf)
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use primitive_types::U256;

    #[test]
    fn test_ser_de() {
        let value = U256::from("111000");
        let bytes  = super::u256_to_move_arr(&value);
        let value2 = super::read_u256_from_move_bytes(&bytes);
        assert_eq!(value, value2);
    }
}
