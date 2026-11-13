// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `on _ when …` takes the obligations left on the paths it selects.
module 0x42::proof_default_selector {
    fun pick(x: u64, c: u64): u64 {
        if (x > 10) {
            x / c * c
        } else {
            x
        }
    }
    spec pick {
        requires c > 0;
        ensures result <= x;
    }
}
