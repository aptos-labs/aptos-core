// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A selector of `on _` that takes no path.
module 0x42::proof_default_selector_errors {
    fun pick(x: u64): u64 {
        if (x > 10) {
            x
        } else {
            x
        }
    }
    spec pick {
        ensures result <= x;
    }
}
