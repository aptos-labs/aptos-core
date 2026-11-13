// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn promoted() -> &'static u64 {
    &5
}

pub fn no_contract() -> u64 {
    *promoted()
}
