// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn through_subslice(values: &mut [u32]) {
    let [_, tail @ ..] = values else {
        return;
    };
    if tail.len() != 0 {
        tail[0] = 7;
    }
}
