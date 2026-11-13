// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A quantifier range sees the binders of the ranges before it, not an enclosing `let` of the
// same name.

module 0x42::quant_range_binder_scope {

    /// Must fail: `exists i in 1..2, j in 0..i: true` is true.
    public fun shadowed(): bool {
        false
    }
    spec shadowed {
        let i = 0u64;
        ensures result == (exists i in 1u64..2u64, j in 0u64..i: true);
    }

    /// Must fail: the same quantifier with no enclosing `let` named `i`.
    public fun unshadowed(): bool {
        false
    }
    spec unshadowed {
        let k = 0u64;
        ensures result == (exists i in 1u64..2u64, j in 0u64..i: true);
    }

    /// Must verify: the dependent range is empty for `i = 0`.
    public fun empty_inner(): bool {
        false
    }
    spec empty_inner {
        let i = 5u64;
        ensures result == (exists i in 0u64..1u64, j in 0u64..i: true);
    }

    spec fun lt_100(x: &mut u64): bool {
        x < 100
    }

    fun keep(x: &mut u64) {
        *x = *x;
    }

    /// Must verify: a state label is not a binder, so `0..n` uses the enclosing `n`.
    public fun state_label(x: &mut u64) {
        keep(x)
    }
    spec state_label {
        let n = 1;
        ensures forall n in *, j in 0..n: j == 0 || (n.. |~ lt_100(x));
    }
}
