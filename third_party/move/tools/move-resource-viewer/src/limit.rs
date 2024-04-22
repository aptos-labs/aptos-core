// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;

// Default limit set to 100mb per query.
const DEFAULT_LIMIT: usize = 100_000_000;

pub struct Limiter(usize);

impl Limiter {
    pub fn charge(&mut self, cost: usize) -> PartialVMResult<()> {
        if self.0 < cost {
            return Err(PartialVMError::new(StatusCode::ABORTED)
                .with_message("Query exceeds size limit".to_string()));
        }
        self.0 -= cost;
        Ok(())
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Limiter(DEFAULT_LIMIT)
    }
}
