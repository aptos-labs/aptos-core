// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::HashValue;

pub trait Hashable {
    fn hash(&self) -> HashValue;
}
