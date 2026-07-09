// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Each labeled state can bind only one `&mut` witness tuple. Two labeled
// calls on the same state with different `&mut` parameter shapes must be
// rejected; otherwise the second call's extra slot has no witness and falls
// through to the ambient argument.

module 0x42::labeled_state_shape_mismatch {
    struct Counter has copy, drop, store { value: u64 }

    spec fun single_progressed(c: &mut Counter): bool {
        old(c).value < c.value
    }

    spec fun pair_progressed(a: &mut Counter, b: &mut Counter): bool {
        old(a).value < a.value && old(b).value < b.value
    }

    fun step_both(a: &mut Counter, b: &mut Counter) {
        a.value = a.value + 1;
        b.value = b.value + 1;
    }
    spec step_both {
        ensures exists S in *:
            (..S |~ single_progressed(a)) &&
            (S.. |~ pair_progressed(a, b));
    }
}
