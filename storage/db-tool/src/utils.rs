// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;

pub fn parse_maxable_u64(input: &str) -> Result<u64> {
    if &input.to_lowercase() == "max" {
        Ok(u64::MAX)
    } else {
        Ok(input.parse()?)
    }
}
