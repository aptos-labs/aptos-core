// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn shared_view(x: &mut u32) -> u32 {
    let read = move || *x;
    read()
}
