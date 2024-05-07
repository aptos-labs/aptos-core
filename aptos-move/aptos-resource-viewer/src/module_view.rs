// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{deserializer::DeserializerConfig, CompiledModule};
use move_core_types::language_storage::ModuleId;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

#[allow(dead_code)]
pub struct ModuleView<'a, S> {
    module_cache: RefCell<HashMap<ModuleId, Arc<CompiledModule>>>,
    deserializer_config: DeserializerConfig,
    state_view: &'a S,
}
