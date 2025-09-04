// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{backup::restore_handler::RestoreHandler, db::VelorDB};
use std::sync::Arc;

pub trait GetRestoreHandler {
    /// Gets an instance of `RestoreHandler` for data restore purpose.
    fn get_restore_handler(&self) -> RestoreHandler;
}

impl GetRestoreHandler for Arc<VelorDB> {
    fn get_restore_handler(&self) -> RestoreHandler {
        RestoreHandler::new(Arc::clone(self), Arc::clone(&self.state_store))
    }
}
