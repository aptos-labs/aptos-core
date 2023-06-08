// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::value::{MoveStructLayout, MoveTypeLayout};

pub(crate) fn is_valid_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;

    match layout {
        L::Bool | L::U8 | L::U16 | L::U32 | L::U64 | L::U128 | L::U256 | L::Address | L::Signer => {
            true
        },
        L::Vector(layout) => is_valid_layout(layout),
        L::Struct(struct_layout) => {
            if !matches!(struct_layout, MoveStructLayout::Runtime(_))
                || struct_layout.fields().is_empty()
            {
                return false;
            }
            struct_layout.fields().iter().all(is_valid_layout)
        },
    }
}
