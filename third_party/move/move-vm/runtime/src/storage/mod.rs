// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod loader;
pub(crate) mod struct_name_index_map;

pub mod code_storage;
pub mod environment;
pub mod module_storage;
pub mod verifier;

pub mod implementations;
pub mod publishing;

// TODO(loader_v2): Temporary infra to still have loader V1 to test, run
//                  and compare things e2e locally.
pub fn use_loader_v1_based_on_env() -> bool {
    std::env::var("USE_LOADER_V1").is_ok()
}
