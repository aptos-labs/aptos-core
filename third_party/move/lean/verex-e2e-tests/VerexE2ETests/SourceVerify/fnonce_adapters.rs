// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn shared_call<F: Fn() -> R, R>(callback: &F) -> R { callback() }
pub fn mutable_call<F: FnMut() -> R, R>(callback: &mut F) -> R { callback() }
pub fn once<F: FnOnce() -> R, R>(callback: F) -> R { callback() }

pub fn shared(value: u64) -> u64 {
    let callback = move || value;
    shared_call(&callback);
    once(callback)
}

pub fn mutable(value: u64) -> u64 {
    let mut state = value;
    let mut callback = move || { state = state.wrapping_add(1); state };
    mutable_call(&mut callback);
    once(callback)
}

pub fn pair(value: u64) -> (u64, bool) {
    let mut state = value;
    let mut callback = move || { state = state.wrapping_add(1); (state, true) };
    mutable_call(&mut callback);
    once(callback)
}

pub fn unit(value: u64) {
    let mut state = value;
    let mut callback = move || { state = state.wrapping_add(1); };
    mutable_call(&mut callback);
    once(callback)
}
