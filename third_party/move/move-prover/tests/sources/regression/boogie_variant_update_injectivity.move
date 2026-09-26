// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The merged enum `$Update` wrapper's name must identify one field.
//
// The name carries the field's rendered type so that same-named fields of
// different types stay apart. While the type came first and was joined to the
// field name by a bare `_`, the two were not recoverable, because a rendered
// function type contains `_` itself:
//
//   |u64, u8| bool  renders  $fun_u64_u8_bool , field `x`      -> .._$fun_u64_u8_bool_x
//   |u64| u8        renders  $fun_u64_u8      , field `bool_x` -> .._$fun_u64_u8_bool_x
//
// Both wrappers were then declared under one name and Boogie rejected the
// duplicate, so the enum could not be verified at all. The field name now comes
// first, separated by `.`: a Move field name contains no `.`, so the boundary is
// recoverable whatever the type renders as.

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
