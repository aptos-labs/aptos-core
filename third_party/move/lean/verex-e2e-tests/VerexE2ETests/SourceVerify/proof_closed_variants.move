// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A parameter's variants its data invariant excludes close while the
/// theorem is set up: no goal is left of them for a proof's steps.
module 0x42::proof_closed_variants {
    enum Slot has copy, drop {
        Empty,
        Full { value: u64 },
    }

    struct Pair has copy, drop {
        first: Slot,
        second: Slot,
    }
    spec Pair {
        invariant (first is Empty) == (second is Empty);
    }

    fun positive(x: u64): u64 {
        x
    }
    spec positive {
        requires x > 0;
    }

    fun read(self: &Pair): u64 {
        match (self.first) {
            Slot::Full { value } => if (value > 0) positive(value) else 0,
            Slot::Empty => 0,
        }
    }
    spec read {
        pragma verify = true;
    }
}
