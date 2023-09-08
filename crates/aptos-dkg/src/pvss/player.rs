// Copyright © Aptos Foundation

use derive_more::Display;
use serde::{Deserialize, Serialize};

/// An identifier from 0 to n-1 for the n players involved in the PVSS protocol.
#[derive(Copy, Display, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Player {
    /// A number from 0 to n-1.
    pub id: usize,
}

impl Player {
    pub fn get_id(&self) -> usize {
        self.id
    }
}
