// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::ARITY;
use anyhow::{ensure, Result};
use velor_crypto::{
    hash::{CryptoHasher, HexyHasher},
    HashValue,
};

#[derive(Default)]
pub struct HexyHashBuilder {
    hasher: HexyHasher,
    seen_children: usize,
}

impl HexyHashBuilder {
    pub fn add_child(&mut self, hash: &HashValue) -> Result<()> {
        ensure!(self.seen_children < ARITY, "Too many children");

        self.hasher.update(hash.as_ref());
        self.seen_children += 1;

        Ok(())
    }

    pub fn finish(self) -> Result<HashValue> {
        ensure!(self.seen_children == ARITY, "Not enough children");
        Ok(self.hasher.finish())
    }
}
