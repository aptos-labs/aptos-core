// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;

pub trait Hashable {
    fn hash(&self) -> HashValue;
}
