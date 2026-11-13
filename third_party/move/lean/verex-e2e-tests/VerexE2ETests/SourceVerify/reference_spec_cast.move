// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::reference_spec_cast {
    fun increment(x: &mut u8) { *x = *x + 1; }
    spec increment {
        requires (x as num) < 255;
        aborts_if false;
        ensures (x as num) == old(x as num) + 1;
    }

    fun unchanged(x: &u8): u8 { *x }
    spec unchanged {
        aborts_if false;
        ensures (result as num) == (x as num);
    }

    fun false_increment(x: &mut u8) { *x = *x + 1; }
    spec false_increment {
        requires (x as num) < 255;
        aborts_if false;
        // Reading the final reference must not silently select its old value.
        ensures (x as num) == old(x as num);
    }
}
