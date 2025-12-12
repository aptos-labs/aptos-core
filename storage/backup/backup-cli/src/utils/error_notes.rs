// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::error;
use std::fmt::{Debug, Display};

pub(crate) trait ErrorNotes<T, E: Display, N: Debug> {
    fn err_notes(self, notes: N) -> Result<T, E>;
}

impl<T, E: Display, N: Debug> ErrorNotes<T, E, N> for Result<T, E> {
    fn err_notes(self, notes: N) -> Result<T, E> {
        if let Err(e) = &self {
            error!(error = %e, notes = ?notes, "Error raised, see notes.");
        }
        self
    }
}
