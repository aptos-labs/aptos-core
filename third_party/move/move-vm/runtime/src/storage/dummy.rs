// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// TODO(loader_v2): Temporary infra to enable loader V2 to test & run things e2e locally, remove.
pub fn use_loader_v2_based_on_env() -> bool {
    std::env::var("USE_LOADER_V2").is_ok()
}
