// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply_mut<F: FnMut(u64) -> R, R>(callback: &mut F, x: u64) -> R {
    callback(x)
}

pub fn forward<F: Fn(u64) -> R, R>(callback: &mut F, x: u64) -> R {
    apply_mut(callback, x)
}

pub fn twice<F: Fn(u64) -> u64>(callback: &mut F, x: u64) -> u64 {
    let first = forward(callback, x);
    apply_mut(callback, first)
}

pub fn scalar(value: u64) -> u64 {
    let mut callback = move |x: u64| value.wrapping_add(x);
    twice(&mut callback, 1)
}

pub fn pair(value: u64) -> (u64, bool) {
    let mut callback = move |x: u64| (value.wrapping_add(x), true);
    forward(&mut callback, 1)
}

pub fn panicking(value: u64) -> u64 {
    let mut callback = move |x: u64| value + x;
    forward(&mut callback, 1)
}

pub fn wrong_body(value: u64) -> u64 {
    let mut callback = move |x: u64| value.wrapping_add(x);
    forward(&mut callback, 1)
}
