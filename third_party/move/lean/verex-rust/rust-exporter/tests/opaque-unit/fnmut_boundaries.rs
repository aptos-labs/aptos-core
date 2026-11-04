// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn argument<F: FnMut(u64) -> u64>(callback: &mut F, value: u64) -> u64 {
    callback(value)
}

pub fn result_parameter<F: FnMut() -> R, R>(callback: &mut F) -> R {
    callback()
}

pub fn result_pair<F: FnMut() -> (u64, u64)>(callback: &mut F) -> (u64, u64) {
    callback()
}

pub fn result_reference<F: FnMut() -> &'static u64>(callback: &mut F) -> &'static u64 {
    callback()
}

pub fn shared<F: Fn() -> u64>(callback: &F) -> u64 {
    callback()
}

pub fn once<F: FnOnce() -> u64>(callback: F) -> u64 {
    callback()
}

pub fn apply<F: FnMut() -> u64>(callback: &mut F) -> u64 {
    callback()
}

// This Fn closure uses the checked shared-receiver adapter for FnMut.
pub fn adapter(value: u64) -> u64 {
    let mut callback = || value;
    apply(&mut callback)
}

// Borrowed closure bodies still need consuming adapters for FnOnce evidence.
pub fn shared_once(value: u64) -> u64 {
    let callback = move || value;
    shared(&callback);
    once(callback)
}

pub fn mutable_once(value: u64) -> u64 {
    let mut state = value;
    let mut callback = move || { state = state.wrapping_add(1); state };
    apply(&mut callback);
    once(callback)
}
