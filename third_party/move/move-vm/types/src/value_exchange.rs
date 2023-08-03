// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{AnnotatedValue, SeedWrapper, Value};
use move_core_types::value::MoveTypeLayout;
use std::fmt::{Display, Formatter};

/// Type for errors occurred while swapping values.
#[derive(Debug)]
pub struct ExchangeError(pub String);

impl ExchangeError {
    pub fn new(s: &impl ToString) -> Self {
        Self(s.to_string())
    }
}

impl Display for ExchangeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error during value exchange: {}.", self.0)
    }
}

pub type ExchangeResult<T> = Result<T, ExchangeError>;

/// Trait which allows to swap values at (de)-serialization time.
/// **WARNING:** it is the responsibility of trait implementation to ensure
/// layouts of exchanged values are the same.
pub trait ValueExchange {
    fn try_exchange(&self, value_to_exchange: Value) -> ExchangeResult<Value>;

    fn try_claim_back(&self, value_to_exchange: Value) -> ExchangeResult<Value>;
}

pub fn deserialize_and_exchange(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    exchange: &dyn ValueExchange,
) -> Option<Value> {
    let seed = SeedWrapper {
        exchange: Some(exchange),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_exchange(
    value: &Value,
    layout: &MoveTypeLayout,
    exchange: &dyn ValueExchange,
) -> Option<Vec<u8>> {
    let value = AnnotatedValue {
        exchange: Some(exchange),
        layout,
        val: &value.0,
    };
    bcs::to_bytes(&value).ok()
}
