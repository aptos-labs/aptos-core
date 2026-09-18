// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Serves keyless validation the chain state it reads, out of this VM's
//! resolver. The validation itself lives in `aptos-keyless-validation`.

use crate::move_vm_ext::AptosMoveResolver;
use aptos_keyless_validation::{value_deserialization_error, KeylessStateView};
use aptos_types::{
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    vm_status::VMStatus,
};
use move_binary_format::errors::Location;
use move_core_types::{account_address::AccountAddress, move_resource::MoveStructType};
use move_vm_runtime::ModuleStorage;
use serde::Deserialize;

pub(crate) struct ResolverStateView<'a, R, M> {
    pub resolver: &'a R,
    pub module_storage: &'a M,
}

impl<R: AptosMoveResolver, M: ModuleStorage> KeylessStateView for ResolverStateView<'_, R, M> {
    fn current_time(&self) -> Result<CurrentTimeMicroseconds, VMStatus> {
        CurrentTimeMicroseconds::fetch_config(self.resolver)
            .ok()
            .flatten()
            .ok_or_else(|| {
                value_deserialization_error!(
                    "could not fetch CurrentTimeMicroseconds on-chain config"
                )
            })
    }

    fn patched_jwks(&self) -> Result<PatchedJWKs, VMStatus> {
        PatchedJWKs::fetch_config(self.resolver)
            .ok()
            .flatten()
            .ok_or_else(|| value_deserialization_error!("could not deserialize PatchedJWKs"))
    }

    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Result<FederatedJWKs, VMStatus> {
        self.get_resource_at_addr::<FederatedJWKs>(jwk_addr)
    }
}

impl<R: AptosMoveResolver, M: ModuleStorage> ResolverStateView<'_, R, M> {
    fn get_resource_at_addr<T: MoveStructType + for<'a> Deserialize<'a>>(
        &self,
        addr: &AccountAddress,
    ) -> Result<T, VMStatus> {
        let struct_tag = T::struct_tag();
        if !struct_tag.address.is_special() {
            let msg = format!(
                "[keyless-validation] Address {} is not special",
                struct_tag.address
            );
            return Err(VMStatus::error(
                aptos_types::vm_status::StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                Some(msg),
            ));
        }

        // INVARIANT:
        //   The struct should be defined at core (0x1) address, so we do not require metering for any
        //   module loading.
        let module = self
            .module_storage
            .unmetered_get_existing_deserialized_module(&struct_tag.address, &struct_tag.module)
            .map_err(|e| e.into_vm_status())?;

        let bytes = self
            .resolver
            .get_resource_bytes_with_metadata_and_layout(addr, &struct_tag, &module.metadata, None)
            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?
            .0
            .ok_or_else(|| {
                value_deserialization_error!(format!(
                    "get_resource failed on {}::{}::{}",
                    addr.to_hex_literal(),
                    T::struct_tag().module,
                    T::struct_tag().name
                ))
            })?;
        bcs::from_bytes::<T>(&bytes).map_err(|_| {
            value_deserialization_error!(format!(
                "could not deserialize {}::{}::{}",
                addr.to_hex_literal(),
                T::struct_tag().module,
                T::struct_tag().name
            ))
        })
    }
}
