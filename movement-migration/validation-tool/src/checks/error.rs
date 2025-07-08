// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("the criterion was not satisfied: {0}")]
    Unsatisfied(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("criterion encountered an internal error: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}
