// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{backup::restore_handler::RestoreHandler, db::AptosDB};
use std::sync::Arc;

pub trait GetRestoreHandler {
    /// Gets an instance of `RestoreHandler` for data restore purpose.
    fn get_restore_handler(&self) -> RestoreHandler;
}

impl GetRestoreHandler for Arc<AptosDB> {
    fn get_restore_handler(&self) -> RestoreHandler {
        RestoreHandler::new(Arc::clone(self), Arc::clone(&self.state_store))
    }
}
