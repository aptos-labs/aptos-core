// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The merged enum `$Update` wrapper's name identifies one field, even when a rendered function
// type contains `_`: field `x` of type `|u64, u8| bool` and field `bool_x` of type `|u64| u8`
// get distinct wrappers.

module 0x42::boogie_variant_update_injectivity {

    enum F has copy, drop {
        V1 { x: |u64, u8|bool has copy + drop },
        V2 { bool_x: |u64|u8 has copy + drop },
    }

    /// Must verify. `$Update` is emitted for every field of an emitted enum, so
    /// reaching `F` at all is what exercises the two colliding names.
    public fun distinct_update_wrappers(
        f: |u64, u8|bool has copy + drop,
        g: |u64|u8 has copy + drop,
    ): bool {
        let l = F::V1 { x: f };
        let r = F::V2 { bool_x: g };
        (l is V1) && (r is V2)
    }

    spec distinct_update_wrappers {
        aborts_if false;
        ensures result == true;
    }
}
