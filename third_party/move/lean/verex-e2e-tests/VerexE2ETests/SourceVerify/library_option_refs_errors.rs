// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn copied_value(value: Option<&u32>) -> Option<u32> {
    value.copied()
}

pub fn cloned_value(value: Option<&u32>) -> Option<u32> {
    value.cloned()
}
