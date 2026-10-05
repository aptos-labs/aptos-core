// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Specifications read a reference parameter as the value it refers to, and
/// name the largest integer of a width with `max_uN()`.
module 0x42::spec_references {
    fun inc(r: &u64): u64 {
        *r + 1
    }
    spec inc {
        aborts_if r + 1 > max_u64();
        ensures result == r + 1;
    }

    fun bump(r: &mut u64) {
        *r = *r + 1
    }
    spec bump {
        aborts_if r + 1 > MAX_U64;
        ensures r == old(r) + 1;
    }

    fun below(r: &u8, s: &u8): bool {
        *r < *s
    }
    spec below {
        ensures result == (r < s);
    }
}
