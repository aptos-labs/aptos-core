// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A [`ModuleProvider`] over the base state view. Mid-block publishing is
//! unsupported on the mono path, so module bytes are always read from the
//! committed state under the block.

use anyhow::{anyhow, bail, Result};
use aptos_types::state_store::{state_key::StateKey, TStateView};
use bytes::Bytes;
use mono_move_core::ModuleProvider;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// Serves module bytes from the committed state under the block.
pub struct StateViewModuleProvider<'a, S> {
    base_view: &'a S,
}

impl<'a, S> StateViewModuleProvider<'a, S> {
    pub fn new(base_view: &'a S) -> Self {
        Self { base_view }
    }
}

impl<S> ModuleProvider for StateViewModuleProvider<'_, S>
where
    S: TStateView<Key = StateKey>,
{
    fn get_module_bytes(&self, address: &AccountAddress, name: &str) -> Result<Option<Bytes>> {
        let Ok(name) = Identifier::new(name) else {
            return Ok(None);
        };
        self.base_view
            .get_state_value_bytes(&StateKey::module(address, &name))
            .map_err(|e| anyhow!("module read failed for {}::{}: {}", address, name, e))
    }

    fn deserialize_module(&self, bytes: &[u8]) -> Result<CompiledModule> {
        CompiledModule::deserialize(bytes).map_err(|e| anyhow!("deserialization failed: {e:?}"))
    }

    fn verify_module(&self, _module: &CompiledModule) -> Result<()> {
        // Committed on-chain modules have already been verified when
        // published.
        Ok(())
    }

    fn get_same_package_modules(
        &self,
        address: &AccountAddress,
        module_name: &str,
    ) -> Result<Vec<Identifier>> {
        // Only reachable under `LoadingPolicy::Package`; the Block-STM
        // integration loads lazily.
        bail!(
            "package membership requested for {}::{}: unsupported over a state view",
            address,
            module_name
        )
    }
}
