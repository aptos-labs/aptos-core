// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::config::VMConfig;
use move_vm_types::loaded_data::runtime_types::TypeBuilder;

/// V2 implementation of loader, which is stateless - i.e., it does not contain module or script
/// cache. Instead, module and script storages are passed to all APIs by reference.
pub(crate) struct LoaderV2 {
    vm_config: VMConfig,
}

impl LoaderV2 {
    pub(crate) fn new(vm_config: VMConfig) -> Self {
        Self { vm_config }
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }
}
