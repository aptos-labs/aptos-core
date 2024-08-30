// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// TODO(loader_v2): Temporary infra to still have loader V1 to test, run
//                  and compare things e2e locally.
pub fn use_loader_v1_based_on_env() -> bool {
    std::env::var("USE_LOADER_V1").is_ok()
}
