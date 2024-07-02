// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{counters::TIMER, move_vm_ext::AptosMoveResolver, natives::aptos_natives_with_builder};
use aptos_framework::natives::code::PackageRegistry;
use aptos_infallible::RwLock;
use aptos_metrics_core::TimerHelper;
use aptos_native_interface::SafeNativeBuilder;
use aptos_types::{on_chain_config::OnChainConfig, state_store::state_key::StateKey};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    vm_status::StatusCode,
};
use move_vm_runtime::{config::VMConfig, move_vm::MoveVM};
use once_cell::sync::Lazy;
use std::collections::HashMap;

const WARM_VM_CACHE_SIZE: usize = 8;

pub(crate) struct WarmVmCache {
    cache: RwLock<HashMap<WarmVmId, MoveVM>>,
}

static WARM_VM_CACHE: Lazy<WarmVmCache> = Lazy::new(|| WarmVmCache {
    cache: RwLock::new(HashMap::new()),
});

impl WarmVmCache {
    pub(crate) fn get_warm_vm(
        native_builder: SafeNativeBuilder,
        vm_config: VMConfig,
        resolver: &impl AptosMoveResolver,
        bin_v7_enabled: bool,
    ) -> VMResult<MoveVM> {
        WARM_VM_CACHE.get(native_builder, vm_config, resolver, bin_v7_enabled)
    }

    fn get(
        &self,
        mut native_builder: SafeNativeBuilder,
        vm_config: VMConfig,
        resolver: &impl AptosMoveResolver,
        bin_v7_enabled: bool,
    ) -> VMResult<MoveVM> {
        let _timer = TIMER.timer_with(&["warm_vm_get"]);
        let id = {
            let _timer = TIMER.timer_with(&["get_warm_vm_id"]);
            WarmVmId::new(&native_builder, &vm_config, resolver, bin_v7_enabled)?
        };

        if let Some(vm) = self.cache.read().get(&id) {
            let _timer = TIMER.timer_with(&["warm_vm_cache_hit"]);
            return Ok(vm.clone());
        }

        {
            let _timer = TIMER.timer_with(&["warm_vm_cache_miss"]);
            let mut cache_locked = self.cache.write();
            if let Some(vm) = cache_locked.get(&id) {
                // Another thread has loaded it
                return Ok(vm.clone());
            }

            let vm =
                MoveVM::new_with_config(aptos_natives_with_builder(&mut native_builder), vm_config);
            Self::warm_vm_up(&vm, resolver);

            // Not using LruCache because its `::get()` requires &mut self
            if cache_locked.len() >= WARM_VM_CACHE_SIZE {
                cache_locked.clear();
            }
            cache_locked.insert(id, vm.clone());
            Ok(vm)
        }
    }

    fn warm_vm_up(vm: &MoveVM, resolver: &impl AptosMoveResolver) {
        let _timer = TIMER.timer_with(&["vm_warm_up"]);

        // Loading `0x1::account` and its transitive dependency into the code cache.
        //
        // This should give us a warm VM to avoid the overhead of VM cold start.
        // Result of this load could be omitted as this is a best effort approach and won't hurt if that fails.
        //
        // Loading up `0x1::account` should be sufficient as this is the most common module
        // used for prologue, epilogue and transfer functionality.
        let _ = vm.load_module(
            &ModuleId::new(CORE_CODE_ADDRESS, ident_str!("account").to_owned()),
            resolver,
        );
    }
}

#[derive(Eq, Hash, PartialEq)]
struct WarmVmId {
    natives: Bytes,
    vm_config: Bytes,
    core_packages_registry: Option<Bytes>,
    bin_v7_enabled: bool,
}

impl WarmVmId {
    fn new(
        native_builder: &SafeNativeBuilder,
        vm_config: &VMConfig,
        resolver: &impl AptosMoveResolver,
        bin_v7_enabled: bool,
    ) -> VMResult<Self> {
        let natives = {
            let _timer = TIMER.timer_with(&["serialize_native_builder"]);
            native_builder.id_bytes()
        };
        Ok(Self {
            natives,
            vm_config: Self::vm_config_bytes(vm_config),
            core_packages_registry: Self::core_packages_id_bytes(resolver)?,
            bin_v7_enabled,
        })
    }

    fn vm_config_bytes(vm_config: &VMConfig) -> Bytes {
        let _timer = TIMER.timer_with(&["serialize_vm_config"]);
        bcs::to_bytes(vm_config)
            .expect("Failed to serialize VMConfig.")
            .into()
    }

    fn core_packages_id_bytes(resolver: &impl AptosMoveResolver) -> VMResult<Option<Bytes>> {
        let bytes = {
            let _timer = TIMER.timer_with(&["fetch_pkgreg"]);
            resolver.fetch_config_bytes(&StateKey::on_chain_config::<PackageRegistry>().map_err(
                |err| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("failed to create StateKey: {}", err))
                        .finish(Location::Undefined)
                },
            )?)
        };

        let core_package_registry = {
            let _timer = TIMER.timer_with(&["deserialize_pkgreg"]);
            bytes
                .as_ref()
                .map(|bytes| PackageRegistry::deserialize_into_config(bytes))
                .transpose()
                .map_err(|err| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("Failed to deserialize PackageRegistry: {}", err))
                        .finish(Location::Undefined)
                })?
        };

        {
            let _timer = TIMER.timer_with(&["ensure_no_ext_deps"]);
            core_package_registry
                .as_ref()
                .map(Self::ensure_no_external_dependency)
                .transpose()?;
        }

        Ok(bytes)
    }

    /// If core packages depend on external packages, the warm vm caches needs a better way to
    /// identify a stale entry.
    fn ensure_no_external_dependency(core_package_registry: &PackageRegistry) -> VMResult<()> {
        for package in &core_package_registry.packages {
            for dep in &package.deps {
                if dep.account != CORE_CODE_ADDRESS {
                    return Err(
                        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                            .with_message("External dependency found in core packages.".to_string())
                            .finish(Location::Undefined),
                    );
                }
            }
        }
        Ok(())
    }
}
