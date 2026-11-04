// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply<F: Fn(u64) -> R, R>(callback: &F, value: u64) -> R {
    callback(value)
}

pub fn forward<F: Fn(u64) -> R, R>(callback: &F, value: u64) -> R {
    apply(callback, value)
}

pub fn scalar(value: u64) -> u64 {
    let callback = move |x: u64| value.wrapping_add(x);
    forward(&callback, 1)
}

pub fn pair(value: u64) -> (u64, bool) {
    let callback = move |x: u64| (value.wrapping_add(x), true);
    forward(&callback, 1)
}

pub fn two<A: Fn() -> u64, B: Fn() -> u64>(a: &A, b: &B) -> u64 {
    a().wrapping_add(b())
}

pub fn multiple(value: u64) -> u64 {
    let a = move || value;
    let b = || 1u64;
    two(&a, &b)
}

pub fn apply_unit<F: Fn()>(callback: &F) {
    callback();
    callback();
}

pub fn unit() {
    apply_unit(&|| ());
}
