// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};

/// An identifier from 0 to n-1 for the n players involved in the PVSS protocol.
#[derive(Copy, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Player {
    /// A number from 0 to n-1.
    pub id: usize,
}

impl Player {
    pub fn get_id(&self) -> usize {
        self.id
    }
}
