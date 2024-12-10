// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v1::MoveEventV1Type;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_OBJECTS_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CollectionMutationEvent {
    mutated_field_name: String,
}

impl CollectionMutationEvent {
    pub fn new(mutated_field_name: String) -> Self {
        Self { mutated_field_name }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn mutated_field_name(&self) -> &String {
        &self.mutated_field_name
    }
}

impl MoveStructType for CollectionMutationEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MutationEvent");
}

impl MoveEventV1Type for CollectionMutationEvent {}

pub static COLLECTION_MUTATION_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("collection").to_owned(),
        name: ident_str!("MutationEvent").to_owned(),
        type_args: vec![],
    }))
});
