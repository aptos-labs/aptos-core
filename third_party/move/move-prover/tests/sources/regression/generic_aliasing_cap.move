// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Past the cap on type-aliasing cases, verification must fail with a diagnostic rather than
// silently skip the remaining cases.
//
// A generic function is verified once per way its accessed resources can alias. The number
// of cases grows with the Bell numbers of the set of resources that can coincide: seven such
// resources give 877 cases, above the cap. Truncating silently would leave aliasing cases
// unverified -- the unsoundness the derivation exists to prevent -- so it is an error.

module 0x42::generic_aliasing_cap {

    struct R<phantom T> has key {
        value: bool,
    }

    public fun seven<T1, T2, T3, T4, T5, T6, T7>(a: address): bool {
        R<T1>[a].value && R<T2>[a].value && R<T3>[a].value && R<T4>[a].value
            && R<T5>[a].value && R<T6>[a].value && R<T7>[a].value
    }
    spec seven {
        pragma aborts_if_is_partial;
        ensures result == true;
    }
}
