// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn apply_mut<F: FnMut(u64) -> R, R>(callback: &mut F, x: u64) -> R {
    callback(x)
}

pub fn both<A: Fn(u64) -> u64, B: Fn(u64) -> u64>(a: &mut A, b: &mut B, x: u64) -> u64 {
    apply_mut(a, x).wrapping_add(apply_mut(b, x))
}

pub fn multiple(value: u64) -> u64 {
    let mut a = move |x: u64| value.wrapping_add(x);
    let mut b = |x: u64| x;
    both(&mut a, &mut b, 1)
}
