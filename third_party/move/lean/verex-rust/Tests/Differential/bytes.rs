// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

macro_rules! common {
    ($t:ident) => {
        pub mod $t {
            use core::intrinsics;

            pub fn add(a: $t, b: $t) -> $t {
                a + b
            }
            pub fn sub(a: $t, b: $t) -> $t {
                a - b
            }
            pub fn mul(a: $t, b: $t) -> $t {
                a * b
            }
            pub fn div(a: $t, b: $t) -> $t {
                a / b
            }
            pub fn rem(a: $t, b: $t) -> $t {
                a % b
            }
            pub fn and(a: $t, b: $t) -> $t {
                a & b
            }
            pub fn or(a: $t, b: $t) -> $t {
                a | b
            }
            pub fn xor(a: $t, b: $t) -> $t {
                a ^ b
            }
            pub fn not(a: $t) -> $t {
                !a
            }
            pub fn shl(a: $t, b: $t) -> $t {
                a << b
            }
            pub fn shr(a: $t, b: $t) -> $t {
                a >> b
            }
            pub fn shl_u8(a: $t, b: u8) -> $t {
                a << b
            }
            pub fn shr_u8(a: $t, b: u8) -> $t {
                a >> b
            }
            pub fn shl_i8(a: $t, b: i8) -> $t {
                a << b
            }
            pub fn shr_i8(a: $t, b: i8) -> $t {
                a >> b
            }
            pub fn shl_u128(a: $t, b: u128) -> $t {
                a << b
            }
            pub fn shr_i128(a: $t, b: i128) -> $t {
                a >> b
            }
            pub fn compare(a: $t, b: $t) -> (bool, bool, bool, bool, bool, bool) {
                (a == b, a != b, a < b, a <= b, a > b, a >= b)
            }
            pub fn wrapping_add(a: $t, b: $t) -> $t {
                intrinsics::wrapping_add(a, b)
            }
            pub fn wrapping_sub(a: $t, b: $t) -> $t {
                intrinsics::wrapping_sub(a, b)
            }
            pub fn wrapping_mul(a: $t, b: $t) -> $t {
                intrinsics::wrapping_mul(a, b)
            }
            pub fn add_overflow(a: $t, b: $t) -> ($t, bool) {
                intrinsics::add_with_overflow(a, b)
            }
            pub fn sub_overflow(a: $t, b: $t) -> ($t, bool) {
                intrinsics::sub_with_overflow(a, b)
            }
            pub fn mul_overflow(a: $t, b: $t) -> ($t, bool) {
                intrinsics::mul_with_overflow(a, b)
            }
            pub unsafe fn unchecked_div(a: $t, b: $t) -> $t {
                unsafe { intrinsics::unchecked_div(a, b) }
            }
            pub unsafe fn unchecked_rem(a: $t, b: $t) -> $t {
                unsafe { intrinsics::unchecked_rem(a, b) }
            }
            pub unsafe fn unchecked_shl(a: $t, b: u32) -> $t {
                unsafe { intrinsics::unchecked_shl(a, b) }
            }
            pub unsafe fn unchecked_shr(a: $t, b: u32) -> $t {
                unsafe { intrinsics::unchecked_shr(a, b) }
            }
            pub unsafe fn unchecked_add(a: $t, b: $t) -> $t {
                unsafe { intrinsics::unchecked_add(a, b) }
            }
            pub unsafe fn unchecked_sub(a: $t, b: $t) -> $t {
                unsafe { intrinsics::unchecked_sub(a, b) }
            }
            pub unsafe fn unchecked_mul(a: $t, b: $t) -> $t {
                unsafe { intrinsics::unchecked_mul(a, b) }
            }
        }
    };
}

macro_rules! signed {
    ($t:ident, $m:ident) => {
        pub mod $m {
            pub fn neg(a: $t) -> $t {
                -a
            }
        }
    };
}

common!(u8);
common!(i8);
signed!(i8, neg_i8);
