// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply<F: FnMut() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    callback()
}

pub fn forward<F: FnMut() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    apply(callback)
}

pub fn pair(value: u64) -> (u64, bool) {
    let mut n = value;
    let mut callback = move || {
        n = n.wrapping_add(1);
        (n, true)
    };
    forward(&mut callback)
}

pub fn apply_generic<F: FnMut() -> R, R>(callback: &mut F) -> R {
    callback()
}

pub fn generic_pair(value: u64) -> (u64, bool) {
    let mut n = value;
    let mut callback = move || {
        n = n.wrapping_add(1);
        (n, true)
    };
    apply_generic(&mut callback)
}

pub fn singleton(value: u64) -> (u64,) {
    let mut callback = move || (value,);
    apply_generic(&mut callback)
}

pub fn apply_nested<F: FnMut() -> (u64, (bool, u64))>(callback: &mut F) -> (u64, (bool, u64)) {
    callback()
}

pub fn nested_tuple(value: u64) -> (u64, (bool, u64)) {
    let mut callback = move || (value, (true, value.wrapping_add(1)));
    apply_nested(&mut callback)
}
