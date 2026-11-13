// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod bounds;

pub fn answer() -> bool {
    bounds::within(41)
}

pub fn limit() -> u32 {
    100
}

#[cfg(feature = "dependency-call")]
pub fn through_dependency() -> i32 {
    verex_rust_cargo_fixture_dependency::dependency_value()
}
