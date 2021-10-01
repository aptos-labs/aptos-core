// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_crypto::HashValue;

pub trait Hashable {
    fn hash(&self) -> HashValue;
}
