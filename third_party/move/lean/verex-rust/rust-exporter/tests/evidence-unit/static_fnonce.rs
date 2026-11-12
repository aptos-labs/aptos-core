// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Token {
    pub value: u64,
}

pub fn take(token: Token) -> u64 {
    token.value
}

pub fn apply_once<F: FnOnce(A) -> R, A, R>(callback: F, argument: A) -> R {
    callback(argument)
}

pub fn forward<F: FnOnce(A) -> R, A, R>(callback: F, argument: A) -> R {
    apply_once(callback, argument)
}

pub fn scalar(value: u64) -> u64 {
    let token = Token { value };
    forward(move |x: u64| take(token).wrapping_add(x), 1)
}

pub fn pair(value: u64) -> (u64, bool) {
    let token = Token { value };
    forward(move |x: u64| (take(token).wrapping_add(x), true), 1)
}

pub fn apply_unit<F: FnOnce()>(callback: F) {
    callback();
}

pub fn unit(value: u64) {
    let token = Token { value };
    apply_unit(move || { take(token); });
}
