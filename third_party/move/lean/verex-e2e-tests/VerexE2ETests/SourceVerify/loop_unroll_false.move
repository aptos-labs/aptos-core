// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::loop_unroll_false {
    // A bounded proof must not silently discard executions beyond its bound.
    fun insufficient(): u8 {
        let i = 0u8;
        while (i < 3) { i = i + 1; };
        i
    }
    spec insufficient {
        pragma unroll = 0;
        ensures result == 0;
        aborts_if false;
    }

    // A sufficient bound does not establish an incorrect postcondition.
    fun incorrect(): u8 {
        let i = 0u8;
        while (i < 3) { i = i + 1; };
        i
    }
    spec incorrect {
        pragma unroll = 3;
        ensures result == 4;
        aborts_if false;
    }
}
