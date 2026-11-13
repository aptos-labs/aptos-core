// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn tuple_mut<F: FnMut() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    callback()
}

pub fn tuple_forward<F: Fn() -> (u64, bool)>(callback: &mut F) -> (u64, bool) {
    tuple_mut(callback)
}

pub fn tuple_value(value: u64) -> (u64, bool) {
    tuple_forward(&mut move || (value, true))
}
