// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{TokenDataId, TokenMutabilityConfig},
    move_utils::move_event_v1::MoveEventV1Type,
};
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTokenDataEvent {
    id: TokenDataId,
    description: String,
    maximum: u64,
    uri: String,
    royalty_payee_address: AccountAddress,
    royalty_points_denominator: u64,
    royalty_points_numerator: u64,
    name: String,
    mutability_config: TokenMutabilityConfig,
    property_keys: Vec<String>,
    property_values: Vec<Vec<u8>>,
    property_types: Vec<String>,
}

impl CreateTokenDataEvent {
    pub fn new(
        id: TokenDataId,
        description: String,
        maximum: u64,
        uri: String,
        royalty_payee_address: AccountAddress,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        name: String,
        mutability_config: TokenMutabilityConfig,
        property_keys: Vec<String>,
        property_values: Vec<Vec<u8>>,
        property_types: Vec<String>,
    ) -> Self {
        Self {
            id,
            description,
            maximum,
            uri,
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_numerator,
            name,
            mutability_config,
            property_keys,
            property_values,
            property_types,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn id(&self) -> &TokenDataId {
        &self.id
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn maximum(&self) -> u64 {
        self.maximum
    }

    pub fn uri(&self) -> &String {
        &self.uri
    }

    pub fn royalty_payee_address(&self) -> &AccountAddress {
        &self.royalty_payee_address
    }

    pub fn royalty_points_denominator(&self) -> u64 {
        self.royalty_points_denominator
    }

    pub fn royalty_points_numerator(&self) -> u64 {
        self.royalty_points_numerator
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn mutability_config(&self) -> &TokenMutabilityConfig {
        &self.mutability_config
    }

    pub fn property_keys(&self) -> &Vec<String> {
        &self.property_keys
    }

    pub fn property_values(&self) -> &Vec<Vec<u8>> {
        &self.property_values
    }

    pub fn property_types(&self) -> &Vec<String> {
        &self.property_types
    }
}

impl MoveStructType for CreateTokenDataEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CreateTokenDataEvent");
}

impl MoveEventV1Type for CreateTokenDataEvent {}

pub static CREATE_TOKEN_DATA_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("CreateTokenDataEvent").to_owned(),
        type_args: vec![],
    }))
});
