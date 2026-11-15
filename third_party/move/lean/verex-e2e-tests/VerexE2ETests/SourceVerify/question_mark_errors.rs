// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn option_add(x: Option<u32>) -> Option<u32> {
    let v = x?;
    Some(v / 2)
}

pub fn option_two(x: Option<u32>, y: Option<u32>) -> Option<u32> {
    let a = x?;
    let b = y?;
    Some(a / 2 + b / 2)
}

pub fn result_add(x: Result<u32, u64>) -> Result<u32, u64> {
    let v = x?;
    Ok(v / 2)
}

pub fn result_two(x: Result<u32, u64>, y: Result<u32, u64>) -> Result<u32, u64> {
    let a = x?;
    let b = y?;
    Ok(a / 2 + b / 2)
}

pub fn result_to_other_ok(x: Result<u32, u64>) -> Result<bool, u64> {
    let v = x?;
    Ok(v == 0)
}

pub fn option_increment(x: Option<u32>) -> Option<u32> {
    let v = x?;
    Some(v + 1)
}
