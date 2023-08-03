// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::Value;
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
pub trait ValueExchange {
    /// Returns a unique identifier which can be transformed into a Move value.
    /// If transformed, the value has exactly the same layout as the recorded value.
    ///
    /// The mapping between an identifier and a swapped value is recorded for later
    /// reuse. For example, clients can serialize the value back and replace
    /// identifiers with values. Returns an error if a mapping already exists.
    fn record_value(&self, value_to_swap: Value) -> ExchangeResult<Identifier>;

    /// Returns the previously swapped value based on the identifier. If a
    /// value has not been swapped, returns an error.
    fn claim_value(&self, id: Identifier) -> ExchangeResult<Value>;
}

/// A unique (at least per-block) identifier which can be used to identify
/// swapped values.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Identifier(pub u64);

impl Identifier {
    /// Given a type layout, tries to embed the identifier into a Move value
    /// which the caller can embed at the right place.
    pub fn try_into_value(self, layout: &MoveTypeLayout) -> ExchangeResult<Value> {
        match layout {
            MoveTypeLayout::U64 => Ok(Value::u64(self.0)),
            MoveTypeLayout::U128 => Ok(Value::u128(self.0 as u128)),
            _ => Err(ExchangeError::new(&format!(
                "converting identifier into {} is not supported",
                layout
            ))),
        }
    }
}

/// Trait (similar to TryInto<>) to reinterpret values as identifiers.
pub trait TryAsIdentifier {
    fn try_as_identifier(&self) -> ExchangeResult<Identifier>;
}
