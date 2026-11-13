// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serves keyless validation the chain state it reads, out of this VM's
//! resolver. The validation itself lives in `aptos-keyless-validation`.

use crate::move_vm_ext::AptosMoveResolver;
use aptos_keyless_validation::KeylessStateView;
use aptos_types::{
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
};
use move_core_types::{account_address::AccountAddress, move_resource::MoveStructType};
use move_vm_runtime::ModuleStorage;
use serde::Deserialize;

pub(crate) struct ResolverStateView<'a, R, M> {
    pub resolver: &'a R,
    pub module_storage: &'a M,
}

impl<R: AptosMoveResolver, M: ModuleStorage> KeylessStateView for ResolverStateView<'_, R, M> {
    fn current_time(&self) -> Option<CurrentTimeMicroseconds> {
        CurrentTimeMicroseconds::fetch_config(self.resolver)
            .ok()
            .flatten()
    }

    fn patched_jwks(&self) -> Option<PatchedJWKs> {
        PatchedJWKs::fetch_config(self.resolver).ok().flatten()
    }

    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Option<FederatedJWKs> {
        self.get_resource_at_addr::<FederatedJWKs>(jwk_addr)
    }
}

impl<R: AptosMoveResolver, M: ModuleStorage> ResolverStateView<'_, R, M> {
    fn get_resource_at_addr<T: MoveStructType + for<'a> Deserialize<'a>>(
        &self,
        addr: &AccountAddress,
    ) -> Option<T> {
        let struct_tag = T::struct_tag();
        // Defensive check to ensure this can only read a system-defined resource
        // type. Reading a resource loads its module unmetered, which is only
        // acceptable for framework modules. The account `addr` itself is not
        // restricted.
        if !struct_tag.address.is_special() {
            return None;
        }

        let module = self
            .module_storage
            .unmetered_get_existing_deserialized_module(&struct_tag.address, &struct_tag.module)
            .ok()?;
        let (bytes, _) = self
            .resolver
            .get_resource_bytes_with_metadata_and_layout(addr, &struct_tag, &module.metadata, None)
            .ok()?;
        bcs::from_bytes::<T>(&bytes?).ok()
    }
}
