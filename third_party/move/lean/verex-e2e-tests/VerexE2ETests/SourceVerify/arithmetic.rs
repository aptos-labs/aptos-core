// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn add_u8(a: u8, b: u8) -> u8 { a + b }
pub fn add_u16(a: u16, b: u16) -> u16 { a + b }
pub fn add_u32(a: u32, b: u32) -> u32 { a + b }
pub fn add_u64(a: u64, b: u64) -> u64 { a + b }
pub fn add_u128(a: u128, b: u128) -> u128 { a + b }
pub fn add_usize(a: usize, b: usize) -> usize { a + b }
pub fn add_i8(a: i8, b: i8) -> i8 { a + b }
pub fn add_i16(a: i16, b: i16) -> i16 { a + b }
pub fn add_i32(a: i32, b: i32) -> i32 { a + b }
pub fn add_i64(a: i64, b: i64) -> i64 { a + b }
pub fn add_i128(a: i128, b: i128) -> i128 { a + b }
pub fn add_isize(a: isize, b: isize) -> isize { a + b }

pub fn sub_usize(a: usize, b: usize) -> usize { a - b }
pub fn mul_u32(a: u32, b: u32) -> u32 { a * b }

pub fn div_u32(a: u32, b: u32) -> u32 { a / b }
pub fn rem_usize(a: usize, b: usize) -> usize { a % b }
pub fn div_i32(a: i32, b: i32) -> i32 { a / b }
pub fn half(a: u32) -> u32 { a / 2 }
pub fn neg_i32(a: i32) -> i32 { -a }

pub fn not_u8(a: u8) -> u8 { !a }
pub fn not_i8(a: i8) -> i8 { !a }

pub fn and_u32(a: u32, b: u32) -> u32 { a & b }
pub fn and_i32(a: i32, b: i32) -> i32 { a & b }
pub fn or_i8(a: i8, b: i8) -> i8 { a | b }
pub fn xor_isize(a: isize, b: isize) -> isize { a ^ b }

pub fn shl_u8(a: u8, n: u32) -> u8 { a << n }
pub fn shr_i32(a: i32, n: u32) -> i32 { a >> n }
pub fn shl_signed_distance(a: u32, n: i32) -> u32 { a << n }

pub fn bool_and(a: bool, b: bool) -> bool { a & b }
pub fn bool_or(a: bool, b: bool) -> bool { a | b }
pub fn bool_xor(a: bool, b: bool) -> bool { a ^ b }
pub fn bool_less(a: bool, b: bool) -> bool { a < b }
pub fn bool_at_least(a: bool, b: bool) -> bool { a >= b }
