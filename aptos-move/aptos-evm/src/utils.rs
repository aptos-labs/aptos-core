// Copyright © Aptos Foundation

use move_core_types::value::{MoveTypeLayout, MoveValue};
use primitive_types::{H160, H256, U256};

pub fn vec_to_h160(value: &[u8]) -> H160 {
    let mut result = [0u8; 20];
    result.copy_from_slice(value);
    H160::from_slice(&result)
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

pub fn read_u256_from_move_bytes(bytes: &[u8]) -> U256 {
    let move_value: MoveValue =
        MoveValue::simple_deserialize(&bytes, &MoveTypeLayout::U256).unwrap();
    move_value.to_u256().into_inner()
}

pub fn read_bytes_from_move_bytes(bytes: &[u8]) -> Vec<u8> {
    let bytes: Vec<u8> = bcs::from_bytes(bytes).unwrap();
    bytes
}

pub fn write_bytes_to_move_bytes(bytes: &[u8]) -> Vec<u8> {
    bcs::to_bytes(bytes).unwrap()
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
    use primitive_types::U256;
    use std::str::FromStr;

    #[test]
    fn test_ser_de() {
        let value = U256::from("111000");
        let bytes = super::u256_to_move_arr(&value);
        let value2 = super::read_u256_from_move_bytes(&bytes);
        assert_eq!(value, value2);
    }
}
