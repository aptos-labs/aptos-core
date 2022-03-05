// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use diem_crypto::HashValue;

pub trait Hashable {
    fn hash(&self) -> HashValue;
}
