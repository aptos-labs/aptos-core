// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Exceeding the limit on aliasing cases is an error.

module 0x42::generic_aliasing_cap {

    struct R<phantom T> has key {
        value: bool,
    }

    public fun seven<T1, T2, T3, T4, T5, T6, T7>(a: address) {
        R<T1>[a].value = true;
        R<T2>[a].value = true;
        R<T3>[a].value = true;
        R<T4>[a].value = true;
        R<T5>[a].value = true;
        R<T6>[a].value = true;
        R<T7>[a].value = true;
    }
}
