// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Wide(pub u64);

impl From<u32> for Wide {
    fn from(value: u32) -> Self {
        Wide(value as u64)
    }
}

pub fn converted(x: Result<u32, u32>) -> Result<u32, Wide> {
    let v = x?;
    Ok(v)
}
