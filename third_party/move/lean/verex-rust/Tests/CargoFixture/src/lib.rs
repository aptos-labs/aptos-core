// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod bounds;

pub fn answer() -> bool {
    bounds::within(41)
}

pub fn limit() -> u32 {
    100
}
