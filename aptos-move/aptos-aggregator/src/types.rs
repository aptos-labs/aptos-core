// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub type AggregatorResult<T> = Result<T, AggregatorError>;

#[derive(Debug, Eq, PartialEq)]
pub enum AggregatorError {
    // TODO: Implement errors.
}

pub type AggregatorID = u64;
