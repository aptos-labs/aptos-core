// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use aptos_crypto::HashValue;

pub trait Hashable {
    fn hash(&self) -> HashValue;
}
