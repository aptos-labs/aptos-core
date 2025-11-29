#![no_main]
// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ethnum::U256 as EthnumU256;
use libfuzzer_sys::{fuzz_target, Corpus};
use primitive_types::U256 as PrimitiveU256;
use std::ops::{Shl, Shr};

const NUM_BITS_PER_BYTE: usize = 8;
const U256_NUM_BITS: usize = 256;
pub const U256_NUM_BYTES: usize = U256_NUM_BITS / NUM_BITS_PER_BYTE;

/// Trait for 256-bit unsigned integers, defining operations used by aptos-core
pub trait U256: Sized {
    fn get_from_le_bytes(slice: &[u8; U256_NUM_BYTES]) -> Self;
    fn turn_to_le_bytes(&self) -> [u8; U256_NUM_BYTES];

    fn checked_add(lhs: Self, rhs: Self) -> Option<Self>;
    fn checked_sub(lhs: Self, rhs: Self) -> Option<Self>;
    fn checked_mul(lhs: Self, rhs: Self) -> Option<Self>;
    fn checked_div(lhs: Self, rhs: Self) -> Option<Self>;
    fn checked_rem(lhs: Self, rhs: Self) -> Option<Self>;

    fn checked_shl_u8(lhs: Self, rhs: u8) -> Option<Self>;
    fn checked_shr_u8(lhs: Self, rhs: u8) -> Option<Self>;

    fn bit_or(lhs: Self, rhs: Self) -> Self;
    fn bit_and(lhs: Self, rhs: Self) -> Self;
    fn bit_xor(lhs: Self, rhs: Self) -> Self;

    fn try_into_u8(self) -> Option<u8>;
    fn try_into_u16(self) -> Option<u16>;
    fn try_into_u32(self) -> Option<u32>;
    fn try_into_u64(self) -> Option<u64>;
    fn try_into_u128(self) -> Option<u128>;

    fn from_u8(n: u8) -> Self;
    fn from_u16(n: u16) -> Self;
    fn from_u32(n: u32) -> Self;
    fn from_u64(n: u64) -> Self;
    fn from_u128(n: u128) -> Self;
}

impl U256 for EthnumU256 {
    fn get_from_le_bytes(slice: &[u8; U256_NUM_BYTES]) -> Self {
        EthnumU256::from_le_bytes(*slice)
    }

    fn turn_to_le_bytes(&self) -> [u8; U256_NUM_BYTES] {
        EthnumU256::to_le_bytes(*self)
    }

    fn checked_add(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_add(rhs)
    }

    fn checked_sub(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_sub(rhs)
    }

    fn checked_mul(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_mul(rhs)
    }

    fn checked_div(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_div(rhs)
    }

    fn checked_rem(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_rem(rhs)
    }

    fn checked_shl_u8(lhs: Self, rhs: u8) -> Option<Self> {
        (lhs << rhs).into()
    }

    fn checked_shr_u8(lhs: Self, rhs: u8) -> Option<Self> {
        (lhs >> rhs).into()
    }

    fn bit_and(lhs: Self, rhs: Self) -> Self {
        lhs & rhs
    }

    fn bit_or(lhs: Self, rhs: Self) -> Self {
        lhs | rhs
    }

    fn bit_xor(lhs: Self, rhs: Self) -> Self {
        lhs ^ rhs
    }

    fn try_into_u8(self) -> Option<u8> {
        self.try_into().ok()
    }

    fn try_into_u16(self) -> Option<u16> {
        self.try_into().ok()
    }

    fn try_into_u32(self) -> Option<u32> {
        self.try_into().ok()
    }

    fn try_into_u64(self) -> Option<u64> {
        self.try_into().ok()
    }

    fn try_into_u128(self) -> Option<u128> {
        self.try_into().ok()
    }

    fn from_u8(n: u8) -> Self {
        EthnumU256::from(n)
    }

    fn from_u16(n: u16) -> Self {
        EthnumU256::from(n)
    }

    fn from_u32(n: u32) -> Self {
        EthnumU256::from(n)
    }

    fn from_u64(n: u64) -> Self {
        EthnumU256::from(n)
    }

    fn from_u128(n: u128) -> Self {
        EthnumU256::from(n)
    }
}

impl U256 for PrimitiveU256 {
    fn get_from_le_bytes(slice: &[u8; U256_NUM_BYTES]) -> Self {
        PrimitiveU256::from_little_endian(slice)
    }

    fn turn_to_le_bytes(&self) -> [u8; U256_NUM_BYTES] {
        let mut bytes = [0u8; U256_NUM_BYTES];
        self.to_little_endian(&mut bytes);
        bytes
    }

    fn checked_add(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_add(rhs)
    }

    fn checked_sub(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_sub(rhs)
    }

    fn checked_mul(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_mul(rhs)
    }

    fn checked_div(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_div(rhs)
    }

    fn checked_rem(lhs: Self, rhs: Self) -> Option<Self> {
        lhs.checked_rem(rhs)
    }

    fn checked_shl_u8(lhs: Self, rhs: u8) -> Option<Self> {
        Some(lhs.shl(rhs))
    }

    fn checked_shr_u8(lhs: Self, rhs: u8) -> Option<Self> {
        Some(lhs.shr(rhs))
    }

    fn bit_and(lhs: Self, rhs: Self) -> Self {
        lhs & rhs
    }

    fn bit_or(lhs: Self, rhs: Self) -> Self {
        lhs | rhs
    }

    fn bit_xor(lhs: Self, rhs: Self) -> Self {
        lhs ^ rhs
    }

    fn try_into_u8(self) -> Option<u8> {
        if self > PrimitiveU256::from(u8::MAX) {
            None
        } else {
            Some(self.low_u64() as u8)
        }
    }

    fn try_into_u16(self) -> Option<u16> {
        if self > PrimitiveU256::from(u16::MAX) {
            None
        } else {
            Some(self.low_u64() as u16)
        }
    }

    fn try_into_u32(self) -> Option<u32> {
        if self > PrimitiveU256::from(u32::MAX) {
            None
        } else {
            Some(self.low_u64() as u32)
        }
    }

    fn try_into_u64(self) -> Option<u64> {
        if self > PrimitiveU256::from(u64::MAX) {
            None
        } else {
            Some(self.low_u128() as u64)
        }
    }

    fn try_into_u128(self) -> Option<u128> {
        if self > PrimitiveU256::from(u128::MAX) {
            None
        } else {
            Some(self.low_u128())
        }
    }

    fn from_u8(n: u8) -> Self {
        PrimitiveU256::from(n)
    }

    fn from_u16(n: u16) -> Self {
        PrimitiveU256::from(n)
    }

    fn from_u32(n: u32) -> Self {
        PrimitiveU256::from(n)
    }

    fn from_u64(n: u64) -> Self {
        PrimitiveU256::from(n)
    }

    fn from_u128(n: u128) -> Self {
        PrimitiveU256::from(n)
    }
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.len() < 64 {
        return Corpus::Reject;
    }

    let mut bytes = [0u8; 32];

    bytes.copy_from_slice(&data[0..32]);
    let pri_u256_1 = PrimitiveU256::get_from_le_bytes(&bytes);
    let eth_u256_1 = EthnumU256::get_from_le_bytes(&bytes);

    bytes.copy_from_slice(&data[32..64]);
    let pri_u256_2 = PrimitiveU256::get_from_le_bytes(&bytes);
    let eth_u256_2 = EthnumU256::get_from_le_bytes(&bytes);

    // Check conversion consistency
    assert_eq!(pri_u256_1.turn_to_le_bytes(), eth_u256_1.turn_to_le_bytes());
    assert_eq!(pri_u256_2.turn_to_le_bytes(), eth_u256_2.turn_to_le_bytes());

    // Check arithmetic operations
    let add1 = PrimitiveU256::checked_add(pri_u256_1, pri_u256_2);
    let add2 = EthnumU256::checked_add(eth_u256_1, eth_u256_2);
    assert_eq!(
        add1.map(|x| x.turn_to_le_bytes()),
        add2.map(|x| x.turn_to_le_bytes())
    );

    let sub1 = PrimitiveU256::checked_sub(pri_u256_1, pri_u256_2);
    let sub2 = EthnumU256::checked_sub(eth_u256_1, eth_u256_2);
    assert_eq!(
        sub1.map(|x| x.turn_to_le_bytes()),
        sub2.map(|x| x.turn_to_le_bytes())
    );

    let mul1 = PrimitiveU256::checked_mul(pri_u256_1, pri_u256_2);
    let mul2 = EthnumU256::checked_mul(eth_u256_1, eth_u256_2);
    assert_eq!(
        mul1.map(|x| x.turn_to_le_bytes()),
        mul2.map(|x| x.turn_to_le_bytes())
    );

    let div1 = PrimitiveU256::checked_div(pri_u256_1, pri_u256_2);
    let div2 = EthnumU256::checked_div(eth_u256_1, eth_u256_2);
    assert_eq!(
        div1.map(|x| x.turn_to_le_bytes()),
        div2.map(|x| x.turn_to_le_bytes())
    );

    let rem1 = PrimitiveU256::checked_rem(pri_u256_1, pri_u256_2);
    let rem2 = EthnumU256::checked_rem(eth_u256_1, eth_u256_2);
    assert_eq!(
        rem1.map(|x| x.turn_to_le_bytes()),
        rem2.map(|x| x.turn_to_le_bytes())
    );

    // Check bitwise operations
    let or1 = PrimitiveU256::bit_or(pri_u256_1, pri_u256_2);
    let or2 = EthnumU256::bit_or(eth_u256_1, eth_u256_2);
    assert_eq!(or1.turn_to_le_bytes(), or2.turn_to_le_bytes());

    let and1 = PrimitiveU256::bit_and(pri_u256_1, pri_u256_2);
    let and2 = EthnumU256::bit_and(eth_u256_1, eth_u256_2);
    assert_eq!(and1.turn_to_le_bytes(), and2.turn_to_le_bytes());

    let xor1 = PrimitiveU256::bit_xor(pri_u256_1, pri_u256_2);
    let xor2 = EthnumU256::bit_xor(eth_u256_1, eth_u256_2);
    assert_eq!(xor1.turn_to_le_bytes(), xor2.turn_to_le_bytes());

    // Check shift operations
    let shl_amount = data[32];
    let shr_amount = data[33];

    let shl1 = PrimitiveU256::checked_shl_u8(pri_u256_1, shl_amount);
    let shl2 = EthnumU256::checked_shl_u8(eth_u256_1, shl_amount);
    assert_eq!(
        shl1.map(|x| x.turn_to_le_bytes()),
        shl2.map(|x| x.turn_to_le_bytes())
    );

    let shr1 = PrimitiveU256::checked_shr_u8(pri_u256_1, shr_amount);
    let shr2 = EthnumU256::checked_shr_u8(eth_u256_1, shr_amount);
    assert_eq!(
        shr1.map(|x| x.turn_to_le_bytes()),
        shr2.map(|x| x.turn_to_le_bytes())
    );

    // Check conversions to smaller types
    let v1 = pri_u256_1.try_into_u8();
    let v2 = eth_u256_1.try_into_u8();
    assert_eq!(v1, v2);

    let v1 = pri_u256_1.try_into_u16();
    let v2 = eth_u256_1.try_into_u16();
    assert_eq!(v1, v2);

    let v1 = pri_u256_1.try_into_u32();
    let v2 = eth_u256_1.try_into_u32();
    assert_eq!(v1, v2);

    let v1 = pri_u256_1.try_into_u64();
    let v2 = eth_u256_1.try_into_u64();
    assert_eq!(v1, v2);

    let v1 = pri_u256_1.try_into_u128();
    let v2 = eth_u256_1.try_into_u128();
    assert_eq!(v1, v2);

    // Check conversions from smaller types
    let v1 = PrimitiveU256::from_u8(data[0]);
    let v2 = EthnumU256::from_u8(data[0]);
    assert_eq!(v1.turn_to_le_bytes(), v2.turn_to_le_bytes());

    let v1 = PrimitiveU256::from_u16(u16::from_le_bytes([data[0], data[1]]));
    let v2 = EthnumU256::from_u16(u16::from_le_bytes([data[0], data[1]]));
    assert_eq!(v1.turn_to_le_bytes(), v2.turn_to_le_bytes());

    let v1 = PrimitiveU256::from_u32(u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
    let v2 = EthnumU256::from_u32(u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
    assert_eq!(v1.turn_to_le_bytes(), v2.turn_to_le_bytes());

    let v1 = PrimitiveU256::from_u64(u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]));
    let v2 = EthnumU256::from_u64(u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]));
    assert_eq!(v1.turn_to_le_bytes(), v2.turn_to_le_bytes());

    let v1 = PrimitiveU256::from_u128(u128::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
        data[10], data[11], data[12], data[13], data[14], data[15],
    ]));
    let v2 = EthnumU256::from_u128(u128::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
        data[10], data[11], data[12], data[13], data[14], data[15],
    ]));
    assert_eq!(v1.turn_to_le_bytes(), v2.turn_to_le_bytes());

    Corpus::Keep
});
