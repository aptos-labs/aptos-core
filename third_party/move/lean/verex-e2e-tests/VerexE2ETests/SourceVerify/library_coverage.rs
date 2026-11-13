// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Reaches every integer contract of the library, each elaborated at its type.

pub fn all_u8(a: u8, b: u8, c: u32) -> (u8, u8, u8, u8, Option<u8>, Option<u8>, Option<u8>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_u16(a: u16, b: u16, c: u32) -> (u16, u16, u16, u16, Option<u16>, Option<u16>, Option<u16>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_u32(a: u32, b: u32, c: u32) -> (u32, u32, u32, u32, Option<u32>, Option<u32>, Option<u32>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_u64(a: u64, b: u64, c: u32) -> (u64, u64, u64, u64, Option<u64>, Option<u64>, Option<u64>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_u128(a: u128, b: u128, c: u32) -> (u128, u128, u128, u128, Option<u128>, Option<u128>, Option<u128>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_usize(a: usize, b: usize, c: u32) -> (usize, usize, usize, usize, Option<usize>, Option<usize>, Option<usize>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_i8(a: i8, b: i8, c: u32) -> (i8, i8, i8, i8, Option<i8>, Option<i8>, Option<i8>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_i16(a: i16, b: i16, c: u32) -> (i16, i16, i16, i16, Option<i16>, Option<i16>, Option<i16>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_i32(a: i32, b: i32, c: u32) -> (i32, i32, i32, i32, Option<i32>, Option<i32>, Option<i32>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_i64(a: i64, b: i64, c: u32) -> (i64, i64, i64, i64, Option<i64>, Option<i64>, Option<i64>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_i128(a: i128, b: i128, c: u32) -> (i128, i128, i128, i128, Option<i128>, Option<i128>, Option<i128>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}

pub fn all_isize(a: isize, b: isize, c: u32) -> (isize, isize, isize, isize, Option<isize>, Option<isize>, Option<isize>) {
    (a.saturating_add(b), a.saturating_sub(b), a.wrapping_shl(c), a.wrapping_shr(c),
     a.checked_add(b), a.checked_sub(b), a.checked_mul(b))
}
