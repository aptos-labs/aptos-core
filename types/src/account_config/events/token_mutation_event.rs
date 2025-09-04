// Copyright © Velor Foundation
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
pub struct TokenMutationEvent {
    mutated_field_name: String,
    old_value: String,
    new_value: String,
}

impl TokenMutationEvent {
    pub fn new(mutated_field_name: String, old_value: String, new_value: String) -> Self {
        Self {
            mutated_field_name,
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

    pub fn old_value(&self) -> &String {
        &self.old_value
    }

    pub fn new_value(&self) -> &String {
        &self.new_value
    }
}

impl MoveStructType for TokenMutationEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MutationEvent");
}

impl MoveEventV1Type for TokenMutationEvent {}

pub static TOKEN_MUTATION_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("MutationEvent").to_owned(),
        type_args: vec![],
    }))
});
