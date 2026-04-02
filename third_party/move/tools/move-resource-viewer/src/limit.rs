// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
