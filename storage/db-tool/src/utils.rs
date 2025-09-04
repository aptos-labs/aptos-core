// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

pub fn parse_maxable_u64(input: &str) -> Result<u64> {
    if &input.to_lowercase() == "max" {
        Ok(u64::MAX)
    } else {
        Ok(input.parse()?)
    }
}
