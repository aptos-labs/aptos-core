// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn power(base: u32, exponent: u32) -> u32 {
    base.pow(exponent)
}

pub fn signed_power(base: i8, exponent: u32) -> i8 {
    base.pow(exponent)
}

pub fn ones(value: u8) -> u32 {
    value.count_ones()
}

pub fn signed_ones(value: i8) -> u32 {
    value.count_ones()
}

pub fn sorted(values: &mut Vec<u32>) {
    values.sort();
}
