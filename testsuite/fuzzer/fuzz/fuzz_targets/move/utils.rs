// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::CompiledModule;
use move_core_types::value::{MoveStructLayout, MoveTypeLayout};

#[allow(dead_code)]
pub(crate) fn is_valid_layout(layout: &MoveTypeLayout) -> bool {
    use MoveTypeLayout as L;

    match layout {
        L::Bool | L::U8 | L::U16 | L::U32 | L::U64 | L::U128 | L::U256 | L::Address | L::Signer => {
            true
        },

        L::Vector(layout) | L::Native(_, layout) => is_valid_layout(layout),
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

#[allow(dead_code)]
pub(crate) fn compiled_module_serde(module: &CompiledModule) -> Result<(), ()> {
    let mut blob = vec![];
    module.serialize(&mut blob).map_err(|_| ())?;
    CompiledModule::deserialize(&blob).map_err(|_| ())?;
    Ok(())
}
