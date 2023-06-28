// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_node_api_context::Context as ApiContext;
use std::sync::Arc;

pub struct ApiV2Config {
    pub context: Arc<ApiContext>,
}

impl ApiV2Config {
    pub fn new(context: Arc<ApiContext>) -> Self {
        Self { context }
    }
}
