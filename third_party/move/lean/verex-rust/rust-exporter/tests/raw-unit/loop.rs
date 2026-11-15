// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn clear(mut flag: bool) -> bool {
    while flag {
        flag = false;
    }
    flag
}
