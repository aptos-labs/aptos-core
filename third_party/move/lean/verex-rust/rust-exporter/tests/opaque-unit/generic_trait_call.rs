// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Non-unit methods remain outside the first method-evidence slice.
pub trait Step { fn step(&self) -> u64; }

pub fn step<T: Step>(value: &T) -> u64 { value.step() }
