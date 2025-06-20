// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Key-Value definitions for macros
//!
//! Example:
//! ```
//! use aptos_logger::info;
//! info!(
//!   key = "value"
//! );
//! ```

use serde::Serialize;
use std::{borrow::Cow, fmt};

/// The key part of a logging key value pair e.g. `info!(key = value)`
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct Key(Cow<'static, str>);

impl Key {
    pub fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    pub fn new_owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    pub fn as_str(&self) -> &'_ Self {
        self
    }
}

/// The value part of a logging key value pair e.g. `info!(key = value)`
#[derive(Clone, Copy)]
pub enum Value<'v> {
    Debug(&'v (dyn fmt::Debug)),
    Display(&'v (dyn fmt::Display)),
    Serde(&'v (dyn erased_serde::Serialize)),
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Value::Debug(d) => fmt::Debug::fmt(d, f),
            Value::Display(d) => fmt::Display::fmt(d, f),
            Value::Serde(s) => {
                fmt::Debug::fmt(&serde_json::to_value(s).map_err(|_| fmt::Error)?, f)
            },
        }
    }
}

impl<'v> Value<'v> {
    /// Get a value from a debuggable type.
    pub fn from_serde<T: serde::Serialize>(value: &'v T) -> Self {
        Value::Serde(value)
    }

    /// Get a value from a debuggable type.
    pub fn from_debug<T: fmt::Debug>(value: &'v T) -> Self {
        Value::Debug(value)
    }

    /// Get a value from a displayable type.
    pub fn from_display<T: fmt::Display>(value: &'v T) -> Self {
        Value::Display(value)
    }
}

/// The logging key value pair e.g. `info!(key = value)`
#[derive(Clone, Debug)]
pub struct KeyValue<'v> {
    key: Key,
    value: Value<'v>,
}

impl<'v> KeyValue<'v> {
    pub fn new(key: &'static str, value: Value<'v>) -> Self {
        Self {
            key: Key::new(key),
            value,
        }
    }
}

impl Schema for KeyValue<'_> {
    fn visit(&self, visitor: &mut dyn Visitor) {
        visitor.visit_pair(self.key.clone(), self.value)
    }
}

/// A schema of key-value pairs.
///
/// The schema may be a single pair, a set of pairs, or a filter over a set of pairs.
/// Use the [`Visitor`](trait.Visitor.html) trait to inspect the structured data
/// in a schema.
pub trait Schema {
    /// Visit key-value pairs.
    fn visit(&self, visitor: &mut dyn Visitor);
}

/// A visitor for the key-value pairs in a [`Schema`](trait.Schema.html).
pub trait Visitor {
    /// Visit a key-value pair.
    fn visit_pair(&mut self, key: Key, value: Value<'_>);
}

impl<'a, 'b: 'a> Visitor for fmt::DebugMap<'a, 'b> {
    fn visit_pair(&mut self, key: Key, value: Value<'_>) {
        self.entry(&key, &value);
    }
}
