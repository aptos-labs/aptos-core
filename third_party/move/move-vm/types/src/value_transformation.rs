// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{AnnotatedValue, SeedWrapper, Value, ValueImpl};
use move_core_types::value::{LayoutTag, MoveTypeLayout};
use std::fmt::{Display, Formatter};

/// Type for errors occurred while transforming values.
#[derive(Debug)]
pub struct TransformationError(pub String);

impl TransformationError {
    pub fn new(s: &impl ToString) -> Self {
        Self(s.to_string())
    }
}

impl Display for TransformationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error during value transformation: {}.", self.0)
    }
}

pub type TransformationResult<T> = Result<T, TransformationError>;

/// Trait to allow transforming one Move value into another Move value.
/// The transformation only takes place if `matches` returns true.
pub trait OnTagTransformation {
    fn matches(&self, tag: &LayoutTag) -> bool;

    fn try_apply(
        &self,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value>;

    fn try_revert(&self, value_to_transform: Value) -> TransformationResult<Value>;
}

/// Trait which allows to swap values at (de)-serialization time.
pub trait ValueExchange {
    fn try_exchange(
        &self,
        layout: &MoveTypeLayout,
        value_to_exchange: Value,
    ) -> TransformationResult<Value>;

    fn try_claim_back(&self, value_to_exchange: Value) -> TransformationResult<Value>;
}

impl<T: ValueExchange> OnTagTransformation for T {
    fn matches(&self, tag: &LayoutTag) -> bool {
        matches!(tag, LayoutTag::AggregatorLifting)
    }

    fn try_apply(
        &self,
        layout: &MoveTypeLayout,
        value_to_transform: Value,
    ) -> TransformationResult<Value> {
        // Currently, aggregators / snapshots support only 64/128-bit integers.
        if !matches!(layout, MoveTypeLayout::U64 | MoveTypeLayout::U128) {
            return Err(TransformationError(format!(
                "Unable to exchange values of type {}, only u64 and u128 are supported",
                layout
            )));
        }

        self.try_exchange(layout, value_to_transform)
    }

    fn try_revert(&self, value_to_transform: Value) -> TransformationResult<Value> {
        // Aggregators / snapshots support only 64/128-bit integers and this
        // should be checked when we apply the transformation. Let's guard just
        // in case.
        assert!(matches!(
            value_to_transform.0,
            ValueImpl::U64(_) | ValueImpl::U128(_)
        ));

        self.try_claim_back(value_to_transform)
    }
}

pub fn deserialize_and_exchange(
    bytes: &[u8],
    layout: &MoveTypeLayout,
    exchange: &impl ValueExchange,
) -> Option<Value> {
    let seed = SeedWrapper {
        transformation: Some(exchange),
        layout,
    };
    bcs::from_bytes_seed(seed, bytes).ok()
}

pub fn serialize_and_exchange(
    value: &Value,
    layout: &MoveTypeLayout,
    exchange: &impl ValueExchange,
) -> Option<Vec<u8>> {
    let value = AnnotatedValue {
        transformation: Some(exchange),
        layout,
        val: &value.0,
    };
    bcs::to_bytes(&value).ok()
}

/// Types which implement this trait can be interpreted as 64-bit identifiers.
pub trait AsIdentifier {
    fn as_identifier(&self) -> Option<u64>;
}

impl AsIdentifier for Value {
    fn as_identifier(&self) -> Option<u64> {
        self.0.as_identifier()
    }
}

impl AsIdentifier for ValueImpl {
    fn as_identifier(&self) -> Option<u64> {
        match self {
            ValueImpl::U64(x) => Some(*x),
            // SAFETY: If the value is identifier, it has been previously set
            // from u64.
            ValueImpl::U128(x) => Some(*x as u64),
            // Do not support anything else for now.
            _ => None,
        }
    }
}

/// Types which implement this trait can convert 64-bit identifiers
/// into specified targets.
pub trait IdentifierBuilder {
    type Target: AsIdentifier;

    fn embed_identifier(layout: &MoveTypeLayout, identifier: u64) -> Option<Self::Target>;
}

impl IdentifierBuilder for Value {
    type Target = Value;

    fn embed_identifier(layout: &MoveTypeLayout, identifier: u64) -> Option<Self::Target> {
        match layout {
            MoveTypeLayout::U64 => Some(Value::u64(identifier)),
            MoveTypeLayout::U128 => Some(Value::u128(identifier as u128)),
            // Do not support anything else for now.
            _ => None,
        }
    }
}
