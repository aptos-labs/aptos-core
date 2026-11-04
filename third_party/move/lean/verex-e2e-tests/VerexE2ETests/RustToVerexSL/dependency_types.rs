// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn unwrap_or_zero(value: Option<u32>) -> u32 {
    match value {
        Some(inner) => inner,
        None => 0,
    }
}

pub fn wrap(value: u32) -> Option<u32> {
    Some(value)
}

pub fn ok_or_error(value: Result<u32, u8>) -> u32 {
    match value {
        Ok(inner) => inner,
        Err(error) => error as u32,
    }
}

pub fn keep(values: Vec<u32>) -> Vec<u32> {
    values
}
