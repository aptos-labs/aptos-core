// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Calls every function `verex-rust/lib` states a contract of, so that a test finds
//! each contract's target declared.

macro_rules! integer {
    ($name:ident, $type:ty) => {
        pub fn $name(a: $type, b: $type, shift: u32, n: $type) -> $type {
            let c = a.checked_add(b).unwrap_or(0);
            let d = a.checked_sub(b).unwrap_or(c);
            let e = a.checked_mul(b).unwrap_or(d);
            let f = a.saturating_add(e);
            let g = a.saturating_sub(f);
            let h = g.wrapping_shl(shift);
            let i = h.wrapping_shr(shift);
            let j = Ord::max(i, a);
            let _ = a.cmp(&b);
            let mut last = Ord::min(j, b);
            for k in 0..n {
                last = k;
            }
            match a.cmp(&b) {
                std::cmp::Ordering::Less => a,
                _ => last,
            }
        }
    };
}

integer!(integer_u8, u8);
integer!(integer_u16, u16);
integer!(integer_u32, u32);
integer!(integer_u64, u64);
integer!(integer_u128, u128);
integer!(integer_usize, usize);
integer!(integer_i8, i8);
integer!(integer_i16, i16);
integer!(integer_i32, i32);
integer!(integer_i64, i64);
integer!(integer_i128, i128);
integer!(integer_isize, isize);

pub fn conversions(wide: u128, narrow: u64) -> u128 {
    let converted = u64::try_from(wide).unwrap_or(narrow);
    u128::from(converted)
}

pub fn vectors(flag: bool, count: usize, value: u32) -> usize {
    let flags = vec![flag; count];
    let mut values = Vec::new();
    values.push(value);
    let mut other: Vec<u32> = Vec::with_capacity(count);
    other.push(values[0]);
    other[0] = value;
    let removed = values.swap_remove(0);
    other.push(removed);
    let _popped = other.pop();
    let view: &[u32] = &other;
    let total = view.len();
    let writable: &mut [u32] = &mut other;
    writable[0] = value;
    if other.is_empty() { flags.len() } else { total }
}

pub fn options(value: Option<u32>, fallback: u32) -> u32 {
    if value.is_some() && !value.is_none() {
        value.unwrap()
    } else {
        value.unwrap_or(fallback)
    }
}

pub fn expected_option(value: Option<u32>) -> u32 {
    value.expect("a value")
}

pub fn expected_result(value: Result<u32, u32>) -> u32 {
    value.expect("a value")
}

pub fn question_option(value: Option<u32>) -> Option<u32> {
    let inner = value?;
    Some(inner)
}

pub fn question_result(value: Result<u32, u32>) -> Result<u32, u32> {
    let inner = value?;
    Ok(inner)
}

pub fn results(value: Result<u32, u32>, fallback: u32) -> u32 {
    if value.is_ok() && !value.is_err() {
        value.unwrap()
    } else {
        value.unwrap_or(fallback)
    }
}

pub fn slices(values: &mut [u32], value: u32) -> u32 {
    if values.is_empty() {
        return value;
    }
    values.swap(0, 0);
    values.fill(value);
    match values.get(0) {
        Some(first) => *first,
        None => value,
    }
}

pub fn vector_edits(
    values: &mut Vec<u32>,
    flags: &mut Vec<bool>,
    other: &[u32],
    other_flags: &[bool],
    at: usize,
) -> usize {
    values.insert(at, 1);
    let removed = values.remove(at);
    values.truncate(at);
    values.clear();
    values.extend_from_slice(other);
    flags.extend_from_slice(other_flags);
    let filled = vec![removed; at];
    filled.len() + values.as_slice().len()
}

pub fn slice_reads(values: &mut [u32], value: u32, mid: usize) -> bool {
    values.reverse();
    let (left, _right) = values.split_at(mid);
    let shared: &[u32] = left;
    shared.first().is_some() && shared.last().is_some() && shared.contains(&value)
}

pub fn option_reads(value: Option<u32>, slot: &mut Option<u32>, error: u8) -> Result<u32, u8> {
    let _view = slot.as_ref();
    let _taken = slot.take();
    let _old = slot.replace(1);
    let _default = value.unwrap_or_default();
    let _expected = value.expect("a value");
    value.ok_or(error)
}

pub fn result_reads(value: Result<u32, u8>, other: Result<u32, u8>, last: Result<u32, u8>) -> u32 {
    let _ok = value.ok();
    let _err = other.err();
    last.expect("a value")
}

pub fn string_reads(text: &String, view: &str) -> bool {
    let _bytes = text.len();
    let _text = text.as_str();
    text.is_empty() && view.is_empty()
}

macro_rules! integer_operations {
    ($name:ident, $type:ty) => {
        pub fn $name(a: $type, b: $type) -> u32 {
            let _quotient = a.checked_div(b);
            let _remainder = a.rem_euclid(b);
            a.leading_zeros() + a.trailing_zeros()
        }
    };
}

integer_operations!(operations_u8, u8);
integer_operations!(operations_u16, u16);
integer_operations!(operations_u32, u32);
integer_operations!(operations_u64, u64);
integer_operations!(operations_u128, u128);
integer_operations!(operations_usize, usize);
integer_operations!(operations_i8, i8);
integer_operations!(operations_i16, i16);
integer_operations!(operations_i32, i32);
integer_operations!(operations_i64, i64);
integer_operations!(operations_i128, i128);
integer_operations!(operations_isize, isize);

macro_rules! unsigned_operations {
    ($name:ident, $type:ty) => {
        pub fn $name(a: $type) -> bool {
            a.is_power_of_two()
        }
    };
}

unsigned_operations!(power_u8, u8);
unsigned_operations!(power_u16, u16);
unsigned_operations!(power_u32, u32);
unsigned_operations!(power_u64, u64);
unsigned_operations!(power_u128, u128);
unsigned_operations!(power_usize, usize);

macro_rules! signed_operations {
    ($name:ident, $type:ty) => {
        pub fn $name(a: $type) -> $type {
            a.abs()
        }
    };
}

signed_operations!(magnitude_i8, i8);
signed_operations!(magnitude_i16, i16);
signed_operations!(magnitude_i32, i32);
signed_operations!(magnitude_i64, i64);
signed_operations!(magnitude_i128, i128);
signed_operations!(magnitude_isize, isize);

pub fn clones(values: &Vec<u32>, view: &[u32], text: &String, part: &str, value: &Option<u32>) -> usize {
    let _vector = values.clone();
    let _string = text.clone();
    let _option = value.clone();
    let _slice = view.to_vec();
    let _owned = part.to_owned();
    let _shown = part.to_string();
    0
}

pub fn iterators(values: &[u32], vector: &Vec<u32>, n: u32) -> u32 {
    let mut last = 0;
    for value in values.iter() {
        last = *value;
    }
    for value in values {
        last = *value;
    }
    for value in vector {
        last = *value;
    }
    for k in 0..=n {
        last = k;
    }
    for k in (0..n).rev() {
        last = k;
    }
    for (_, value) in values.iter().enumerate() {
        last = *value;
    }
    for k in (0..n).step_by(2) {
        last = k;
    }
    last
}

macro_rules! integer_bits {
    ($name:ident, $type:ty) => {
        pub fn $name(a: $type, b: u32, values: &mut Vec<$type>) -> u32 {
            let _power = a.pow(b);
            values.sort();
            a.count_ones()
        }
    };
}

integer_bits!(bits_u8, u8);
integer_bits!(bits_u16, u16);
integer_bits!(bits_u32, u32);
integer_bits!(bits_u64, u64);
integer_bits!(bits_u128, u128);
integer_bits!(bits_usize, usize);
integer_bits!(bits_i8, i8);
integer_bits!(bits_i16, i16);
integer_bits!(bits_i32, i32);
integer_bits!(bits_i64, i64);
integer_bits!(bits_i128, i128);
integer_bits!(bits_isize, isize);

pub fn string_edits(text: &mut String, part: &str, value: char) -> usize {
    text.push_str(part);
    text.push(value);
    text.as_bytes().len() + part.as_bytes().len()
}

pub fn option_references(value: Option<&u32>, other: Option<&u32>) -> Option<u32> {
    let _copied = value.copied();
    other.cloned()
}

pub fn boxes(value: u32, shared: &Box<u32>, exclusive: &mut Box<u32>) -> u32 {
    let made = Box::new(value);
    *exclusive.as_mut() = *made;
    *shared.as_ref()
}

pub fn string_from_utf8(bytes: &[u8]) -> Result<&str, std::str::Utf8Error> {
    std::str::from_utf8(bytes)
}

pub fn owned_string_from_utf8(bytes: Vec<u8>) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(bytes)
}

pub fn string_into_bytes(text: String) -> Vec<u8> {
    text.into_bytes()
}

pub fn utf8_error_into_bytes(error: std::string::FromUtf8Error) -> Vec<u8> {
    error.into_bytes()
}

pub fn utf8_error_as_bytes(error: &std::string::FromUtf8Error) -> &[u8] {
    error.as_bytes()
}

pub fn slice_mutable_access(values: &mut [u32], index: usize, value: u32) -> u32 {
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    }
    for slot in values.iter_mut() {
        *slot = value;
    }
    for slot in &mut *values {
        *slot = value;
    }
    values.len() as u32
}

pub fn option_combinators(value: Option<u32>) -> Option<u32> {
    value.map(|x| x.wrapping_add(1))
}
