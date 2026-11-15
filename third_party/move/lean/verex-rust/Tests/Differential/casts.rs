// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! casts {
    ($t:ident) => {
        pub mod $t {
            pub fn to_u8(a: $t) -> u8 {
                a as u8
            }
            pub fn to_i8(a: $t) -> i8 {
                a as i8
            }
            pub fn to_u16(a: $t) -> u16 {
                a as u16
            }
            pub fn to_i16(a: $t) -> i16 {
                a as i16
            }
            pub fn to_u32(a: $t) -> u32 {
                a as u32
            }
            pub fn to_i32(a: $t) -> i32 {
                a as i32
            }
            pub fn to_u64(a: $t) -> u64 {
                a as u64
            }
            pub fn to_i64(a: $t) -> i64 {
                a as i64
            }
            pub fn to_u128(a: $t) -> u128 {
                a as u128
            }
            pub fn to_i128(a: $t) -> i128 {
                a as i128
            }
            pub fn to_usize(a: $t) -> usize {
                a as usize
            }
            pub fn to_isize(a: $t) -> isize {
                a as isize
            }
            pub fn from_char(a: char) -> $t {
                a as $t
            }
        }
    };
}

casts!(u8);
casts!(i8);
casts!(u16);
casts!(i16);
casts!(u32);
casts!(i32);
casts!(u64);
casts!(i64);
casts!(u128);
casts!(i128);
casts!(usize);
casts!(isize);

pub fn char_of_byte(a: u8) -> char {
    a as char
}

pub fn char_order(a: char, b: char) -> (bool, bool, bool, bool, bool, bool) {
    (a == b, a != b, a < b, a <= b, a > b, a >= b)
}

pub fn char_match(a: char) -> u8 {
    match a {
        '\0' => 0,
        'a'..='z' => 1,
        '\u{10FFFF}' => 2,
        _ => 3,
    }
}
