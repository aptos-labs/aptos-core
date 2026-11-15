// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Token { pub value: u64 }

pub fn take(token: Token) -> u64 { token.value }

pub fn apply<F: FnOnce() -> u64>(callback: F) -> u64 { callback() }

pub fn wrong(value: u64) -> u64 {
    let token = Token { value };
    apply(move || take(token))
}

pub fn panicking(value: u64) -> u64 {
    let token = Token { value };
    apply(move || { let n = take(token); assert!(n != 0); n })
}
