// Copyright © Aptos Foundation

use derive_more::Display;

/// An identifier from 0 to n-1 for the n players involved in the PVSS protocol.
#[derive(Display, PartialEq, Eq, Clone)]
pub struct Player {
    /// A number from 0 to n-1.
    pub(crate) id: usize,
}

impl Player {
    pub fn get_id(&self) -> usize {
        self.id
    }
}
