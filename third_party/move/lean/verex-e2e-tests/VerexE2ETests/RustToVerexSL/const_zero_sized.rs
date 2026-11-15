// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;

pub fn none() -> Option<Infallible> {
    Option::<Infallible>::None
}

pub fn question(value: Option<u64>) -> Option<u64> {
    let inner = value?;
    Some(inner)
}
