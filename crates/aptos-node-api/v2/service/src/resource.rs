// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::QueryRoot;
use anyhow::{Context as AnyhowContext, Result};
use aptos_move_graphql_values::annotated_struct_to_graphql_object;
use aptos_node_api_context::Context as ApiContext;
use aptos_node_api_v1_types::{Address, AsConverter, BasicErrorWith404};
use aptos_vm::data_cache::AsMoveResolver;
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, Json, Object, Schema, SimpleObject, Value,
};
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use std::{str::FromStr, sync::Arc};

#[derive(Clone, Debug, SimpleObject)]
pub struct Resource {
    json_data_v1: Json<Value>,
}

impl Resource {
    pub fn new(json_data_v1: Json<Value>) -> Self {
        Self { json_data_v1 }
    }
}

#[Object]
impl QueryRoot {
    async fn resource(
        &self,
        ctx: &Context<'_>,
        address: String,
        resource_type: String,
    ) -> Result<Resource> {
        let context = ctx.data_unchecked::<Arc<ApiContext>>();
        let resource_type =
            StructTag::from_str(&resource_type).context("Failed to parse given resource type")?;

        let address = Address::from_str(&address).context("Failed to parse given address")?;

        let (_ledger_info, _ledger_version, state_view) =
            context.state_view::<BasicErrorWith404>(None)?;
        let bytes = state_view
            .as_move_resolver()
            .get_resource(&address.into(), &resource_type)
            .context(format!(
                "Failed to query DB to check for {} at {}",
                resource_type, address
            ))?
            .ok_or_else(|| {
                anyhow::anyhow!("Could not find resource {} at {}", resource_type, address)
            })?;

        let resource = state_view
            .as_move_resolver()
            .as_converter(context.db.clone())
            .try_into_inner_resource(&resource_type, &bytes)
            .context("Failed to deserialize resource data retrieved from DB")?;

        let out = annotated_struct_to_graphql_object(resource)
            .context("Failed to convert resource to GraphQL object")?;

        Ok(Resource::new(Json(out)))
    }
}
