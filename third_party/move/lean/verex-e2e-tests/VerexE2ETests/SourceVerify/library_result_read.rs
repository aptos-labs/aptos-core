// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn or_panic(value: Result<u32, u8>) -> u32 {
    value.expect("a value")
}

pub fn success(value: Result<u32, u8>) -> Option<u32> {
    value.ok()
}

pub fn failure(value: Result<u32, u8>) -> Option<u8> {
    value.err()
}
