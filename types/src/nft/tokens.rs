// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
};

pub const BARS_NAME: &str = "BARSToken";
pub const BARS_IDENTIFIER: &IdentStr = ident_str!(BARS_NAME);

pub fn bars_tag() -> TypeTag {
    TypeTag::Struct(StructTag {
        address: CORE_CODE_ADDRESS,
        module: Identifier::from(BARS_IDENTIFIER),
        name: Identifier::from(BARS_IDENTIFIER),
        type_params: vec![],
    })
}
