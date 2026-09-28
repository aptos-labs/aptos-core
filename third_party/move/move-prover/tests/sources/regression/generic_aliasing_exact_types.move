// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Resources whose types can never be equal, such as `R<u8>` and `R<u64>`, give no aliasing
// case.

module 0x42::generic_aliasing_exact_types {

    struct R<phantom T> has key {
        value: bool,
    }

    struct Q<phantom A, phantom B> has key {
        value: bool,
    }

    struct S<phantom T> has key {
        value: bool,
    }

    /// Must verify: ten distinct concrete resources, none of which can alias.
    public fun concrete(a: address): bool {
        exists<R<u8>>(a) || exists<R<u16>>(a) || exists<R<u32>>(a) || exists<R<u64>>(a)
            || exists<R<u128>>(a) || exists<R<u256>>(a) || exists<Q<u8, u8>>(a)
            || exists<Q<u8, u16>>(a) || exists<Q<u8, u32>>(a) || exists<Q<u8, u64>>(a)
    }
    spec concrete {
        aborts_if false;
    }

    /// Must verify: the type parameter occurs only in `S<T>`, which aliases none of the others.
    public fun generic<T>(a: address): bool {
        exists<S<T>>(a) || exists<R<u8>>(a) || exists<R<u16>>(a) || exists<R<u32>>(a)
            || exists<R<u64>>(a) || exists<R<u128>>(a) || exists<R<u256>>(a)
            || exists<Q<u8, u8>>(a) || exists<Q<u8, u16>>(a) || exists<Q<u8, u32>>(a)
            || exists<Q<u8, u64>>(a)
    }
    spec generic {
        aborts_if false;
    }
}
