// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn unchecked(value: Option<u64>) -> u64 {
    unsafe { value.unwrap_unchecked() }
}
