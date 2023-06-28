// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::schema::QueryRoot;
use async_graphql::{Context, Object, SimpleObject};

#[derive(Clone, Debug, SimpleObject)]
pub struct Module {
    module_id: String,
}

#[Object]
impl QueryRoot {
    async fn module(&self, _ctx: &Context<'_>, module_id: String) -> Module {
        Module { module_id }
    }
}
