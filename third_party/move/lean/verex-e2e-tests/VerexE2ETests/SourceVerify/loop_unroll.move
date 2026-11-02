// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::loop_unroll {
    fun count(n: u8): u8 {
        let i = 0u8;
        while (i < n) { i = i + 1; };
        i
    }
    spec count {
        pragma unroll = 3;
        requires n <= 3;
        ensures result == n;
        aborts_if false;
    }

    fun early_return(n: u8): u8 {
        let i = 0u8;
        while (i < n) {
            if (i == 1) { return i };
            i = i + 1;
        };
        i
    }
    spec early_return {
        pragma unroll = 2;
        requires n <= 3;
        ensures result == if (n == 0) 0 else 1;
        aborts_if false;
    }
}
