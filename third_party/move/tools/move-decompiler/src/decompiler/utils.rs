// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_model::model::{ModuleEnv, ModuleId};

pub fn shortest_prefix(module_env: &ModuleEnv<'_>, target_mod_id: &ModuleId) -> String {
    if *target_mod_id == module_env.get_id() {
        String::new()
    } else {
        let module = module_env.env.get_module(*target_mod_id);
        format!("{}::", module.get_full_name_str())
    }
}
