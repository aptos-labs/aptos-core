// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply_mut<F: FnMut() -> u64>(callback: &mut F) -> u64 {
    callback();
    callback()
}

pub fn forward<F: FnMut() -> u64>(callback: &mut F) -> u64 {
    apply_mut(callback)
}

pub fn apply_bool<F: FnMut() -> bool>(callback: &mut F) -> bool {
    callback();
    callback()
}

pub fn counter(value: u64) -> u64 {
    let mut count = value;
    let mut callback = move || {
        count = count.wrapping_add(1);
        count
    };
    forward(&mut callback)
}

pub fn toggle(value: bool) -> bool {
    let mut state = value;
    let mut callback = move || {
        state = !state;
        state
    };
    apply_bool(&mut callback)
}

pub fn panicking(value: u64) -> u64 {
    let mut count = value;
    let mut callback = move || {
        count += 1;
        count
    };
    forward(&mut callback)
}

pub fn captured_loan(value: u64) -> u64 {
    let mut count = value;
    let mut callback = || {
        count = count.wrapping_add(1);
        count
    };
    forward(&mut callback)
}

pub fn wrong_body(value: u64) -> u64 {
    let mut count = value;
    let mut callback = move || {
        count = count.wrapping_add(1);
        count
    };
    forward(&mut callback)
}
