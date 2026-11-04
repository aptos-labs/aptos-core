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

pub fn both<A: Fn(u64) -> u64, B: Fn(u64) -> u64>(a: &mut A, b: &mut B, x: u64) -> u64 {
    apply_mut(a, x).wrapping_add(apply_mut(b, x))
}

pub fn multiple(value: u64) -> u64 {
    let mut a = move |x: u64| value.wrapping_add(x);
    let mut b = |x: u64| x;
    both(&mut a, &mut b, 1)
}

pub fn unit_mut<F: FnMut()>(callback: &mut F) {
    callback();
}

pub fn unit_forward<F: Fn()>(callback: &mut F) {
    unit_mut(callback);
    unit_mut(callback);
}

pub fn unit() {
    unit_forward(&mut || ());
}

pub fn tuple_mut<F: FnMut() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    callback()
}

pub fn tuple_forward<F: Fn() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    tuple_mut(callback)
}

pub fn tuple_value(value: u64) -> (u64, bool) {
    tuple_forward(&mut move || (value, true))
}
