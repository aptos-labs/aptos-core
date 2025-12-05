// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::ARITY;
use anyhow::{ensure, Result};
use aptos_crypto::{
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
