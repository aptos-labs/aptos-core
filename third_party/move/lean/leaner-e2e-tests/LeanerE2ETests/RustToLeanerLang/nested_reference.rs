// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn read_nested(value: &&u32) -> u32 {
    **value
}
