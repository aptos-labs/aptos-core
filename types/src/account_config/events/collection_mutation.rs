// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::Object, move_utils::move_event_v2::MoveEventV2Type};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_OBJECTS_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CollectionMutation {
    mutated_field_name: String,
    collection: Object,
    old_value: String,
    new_value: String,
}

impl CollectionMutation {
    pub fn new(
        mutated_field_name: String,
        collection: Object,
        old_value: String,
        new_value: String,
    ) -> Self {
        Self {
            mutated_field_name,
            collection,
            old_value,
            new_value,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn mutated_field_name(&self) -> &String {
        &self.mutated_field_name
    }

    pub fn collection(&self) -> &Object {
        &self.collection
    }

    pub fn old_value(&self) -> &String {
        &self.old_value
    }

    pub fn new_value(&self) -> &String {
        &self.new_value
    }
}

impl MoveStructType for CollectionMutation {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Mutation");
}

impl MoveEventV2Type for CollectionMutation {}

pub static COLLECTION_MUTATION_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("collection").to_owned(),
        name: ident_str!("Mutation").to_owned(),
        type_args: vec![],
    }))
});
