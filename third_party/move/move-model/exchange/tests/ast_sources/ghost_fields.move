// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::ghost_fields {
    struct Counter has copy, drop { value: u64 }
    spec Counter {
        ghost bumps: num;
        ghost doubled: num = value * 2;
    }

    enum Slot has copy, drop {
        Empty,
        Full { item: u64 },
    }
    spec Slot {
        ghost seen: bool;
    }

    fun bump(c: &mut Counter) {
        c.value = c.value + 1;
        spec {
            update c.bumps = c.bumps + 1;
        };
    }
    spec bump {
        ensures c.bumps == old(c.bumps) + 1;
    }

    fun fresh(): Counter { Counter { value: 3 } }
    spec fresh {
        ensures result.doubled == 6;
    }

    fun fill(): Slot { Slot::Full { item: 1 } }
}
