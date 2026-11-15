// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::abort_codes {
    fun declared(x: u64) {
        if (x == 0) abort 1;
        if (x == 1) abort 2;
    }
    spec declared {
        pragma opaque;
        aborts_with 1, 2;
    }

    fun caller(x: u64) { declared(x); }
    spec caller { aborts_with 1, 2; }

    fun wrong(x: u64) { declared(x); }
    spec wrong { aborts_with 1, 3; }

    fun partial(x: u64) {
        if (x == 0) abort 7;
        if (x == 1) abort 8;
    }
    spec partial {
        pragma aborts_if_is_partial;
        aborts_if x == 0 with 7;
        aborts_with 8;
    }

    fun partial_wrong(x: u64) {
        if (x == 0) abort 7;
        if (x == 1) abort 7;
    }
    spec partial_wrong {
        pragma aborts_if_is_partial;
        aborts_if x == 0 with 7;
        aborts_with 8;
    }

    // Partial conditions still constrain codes when there is no aborts_with.
    fun partial_codes_only(x: u64) {
        if (x == 0) abort 7;
        if (x == 1) abort 8;
    }
    spec partial_codes_only {
        pragma aborts_if_is_partial;
        aborts_if x == 0 with 7;
    }

    // A standalone code is allowed even when a conditioned clause also holds.
    fun alternative(x: u64) { if (x == 0) abort 2; }
    spec alternative {
        pragma aborts_if_is_partial;
        aborts_if x == 0 with 1;
        aborts_with 2;
    }
}
