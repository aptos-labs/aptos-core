// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn once<F: FnOnce(u64) -> R, R>(callback: F, value: u64) -> R {
    callback(value)
}
pub fn shared<F: Fn(u64) -> R, R>(callback: F, value: u64) -> R {
    once(callback, value)
}
pub fn mutable<F: FnMut(u64) -> R, R>(callback: F, value: u64) -> R {
    once(callback, value)
}

// Two forwarding hops require the original Fn dictionary to be scoped.
pub fn forward<F: Fn(u64) -> R, R>(callback: F, value: u64) -> R {
    mutable(callback, value)
}

pub fn relay<F: Fn(u64) -> R, R>(callback: F, value: u64) -> R {
    forward(callback, value)
}

// A borrowed dictionary not used by consuming adapters remains unrestricted.
pub fn direct<F: Fn(u64) -> R, R>(callback: &F, value: u64) -> R {
    callback(value)
}

pub fn scalar(value: u64) -> u64 {
    relay(move |x: u64| value.wrapping_add(x), 1)
}

pub fn counter(value: u64) -> u64 {
    let mut state = value;
    mutable(
        move |x: u64| {
            state = state.wrapping_add(x);
            state
        },
        1,
    )
}

pub fn pair(value: u64) -> (u64, bool) {
    let mut state = value;
    mutable(
        move |x: u64| {
            state = state.wrapping_add(x);
            (state, true)
        },
        1,
    )
}
