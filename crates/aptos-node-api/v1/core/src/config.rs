// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_node_api_context::Context;
use std::sync::Arc;

pub struct ApiV1Config {
    pub context: Arc<Context>,
}

impl ApiV1Config {
    pub fn new(context: Arc<Context>) -> Self {
        Self { context }
    }
}
