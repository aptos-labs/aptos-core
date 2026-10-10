// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn choose(flag: bool, when_true: u32, when_false: u32) -> u32 {
    if flag { when_true } else { when_false }
}
