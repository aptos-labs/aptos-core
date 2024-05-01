// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0


// FIXME(aldenhu): remove
#![allow(dead_code)]

use aptos_crypto::hash::CryptoHasher;
use aptos_crypto::HashValue;

/// FIXME(aldenhu): doc


mod reader;
mod updater;

enum Error {}

type Result<T, E=Error> = std::result::Result<T, E>;

trait HashValueArrayRead {
    fn at(&self, index: u64) -> Result<HashValue>;
}
