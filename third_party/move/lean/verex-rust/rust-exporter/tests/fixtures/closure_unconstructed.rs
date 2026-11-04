// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[allow(unreachable_code)]
pub fn make(c: u32) -> impl Fn(u32) -> u32 {
    std::process::abort();
    move |a: u32| a + c
}
