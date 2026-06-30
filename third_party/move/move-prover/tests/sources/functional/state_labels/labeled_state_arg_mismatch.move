// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The per-label witness is keyed only by `MemoryLabel`. Two labeled calls
// sharing a label but referring to different `&mut` arguments must be
// rejected; otherwise the second call silently reuses the first call's
// witness and proves the wrong intermediate state.

module 0x42::labeled_state_arg_mismatch {
    struct Counter has copy, drop, store { value: u64 }

    spec fun progressed(c: &mut Counter): bool {
        old(c).value < c.value
    }

    fun step_both(a: &mut Counter, b: &mut Counter) {
        a.value = a.value + 1;
        b.value = b.value + 1;
    }
    spec step_both {
        ensures exists S in *:
            (..S |~ progressed(a)) &&
            (S.. |~ progressed(b));
    }
}
