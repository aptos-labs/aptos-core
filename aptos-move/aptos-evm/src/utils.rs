// Copyright © Aptos Foundation

use primitive_types::{H256, U256};

pub fn u256_to_arr(value: &U256) -> [u8; 32] {
    let mut result = [0u8; 32];
    value.to_big_endian(&mut result);
    result
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

pub fn read_h256_from_bytes(bytes: &[u8]) -> H256 {
    if bytes.len() != 32 {
        panic!("InvalidU256 length expected 32, got {}", bytes.len());
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    H256(buf)
}
