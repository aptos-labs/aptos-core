// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Reaches the library's contracts of `Ord`, conversions, and the `Vec` and slice
// operations the benchmark's Rust problems use.

pub fn order_u8(a: u8, b: u8) -> (u8, u8) {
    (a.max(b), a.min(b))
}

pub fn order_u16(a: u16, b: u16) -> (u16, u16) {
    (a.max(b), a.min(b))
}

pub fn order_u32(a: u32, b: u32) -> (u32, u32) {
    (a.max(b), a.min(b))
}

pub fn order_u64(a: u64, b: u64) -> (u64, u64) {
    (a.max(b), a.min(b))
}

pub fn order_u128(a: u128, b: u128) -> (u128, u128) {
    (a.max(b), a.min(b))
}

pub fn order_usize(a: usize, b: usize) -> (usize, usize) {
    (a.max(b), a.min(b))
}

pub fn order_i8(a: i8, b: i8) -> (i8, i8) {
    (a.max(b), a.min(b))
}

pub fn order_i16(a: i16, b: i16) -> (i16, i16) {
    (a.max(b), a.min(b))
}

pub fn order_i32(a: i32, b: i32) -> (i32, i32) {
    (a.max(b), a.min(b))
}

pub fn order_i64(a: i64, b: i64) -> (i64, i64) {
    (a.max(b), a.min(b))
}

pub fn order_i128(a: i128, b: i128) -> (i128, i128) {
    (a.max(b), a.min(b))
}

pub fn order_isize(a: isize, b: isize) -> (isize, isize) {
    (a.max(b), a.min(b))
}

pub fn narrow(value: u128) -> u64 {
    u64::try_from(value).unwrap()
}

pub fn widen(value: u64) -> u128 {
    u128::from(value)
}

pub fn repeated(length: usize) -> Vec<bool> {
    vec![true; length]
}

pub fn reserved(capacity: usize) -> Vec<u32> {
    Vec::with_capacity(capacity)
}

pub fn take(values: &mut Vec<u32>, position: usize) -> u32 {
    values.swap_remove(position)
}

pub fn none(values: &[u32]) -> bool {
    values.is_empty()
}

pub fn exchange(values: &mut Vec<u32>, first: usize, second: usize) {
    values.swap(first, second);
}

pub fn clear(values: &mut Vec<bool>) {
    values.fill(false);
}
