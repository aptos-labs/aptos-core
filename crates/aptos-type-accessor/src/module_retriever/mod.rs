// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api_types::MoveModule;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use move_core_types::language_storage::ModuleId;
use std::collections::BTreeMap;

mod api;

pub use api::ApiModuleRetriever;

#[async_trait]
#[enum_dispatch]
pub trait ModuleRetrieverTrait: Clone + Send + Sync + 'static {
    /// Retrieve modules from somewhere.
    async fn retrieve_modules(
        &self,
        module_ids: &[ModuleId],
    ) -> Result<BTreeMap<ModuleId, MoveModule>>;

    /// Retrieve a module from somewhere.
    async fn retrieve_module(&self, module_id: ModuleId) -> Result<MoveModule>;
}

/// This enum has as its variants all possible implementations of ModuleRetrieverTrait.
#[enum_dispatch(ModuleRetrieverTrait)]
#[derive(Clone, Debug)]
pub enum ModuleRetriever {
    ApiModuleRetriever,
}
