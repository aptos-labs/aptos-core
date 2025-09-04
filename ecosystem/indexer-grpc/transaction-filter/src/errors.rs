// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Serialize, Serializer};
use std::fmt::Display;
use thiserror::Error as ThisError;

#[derive(Debug, Serialize)]
pub struct FilterStepTrace {
    pub serialized_filter: String,
    pub filter_type: String,
}

impl Display for FilterStepTrace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:   {}", self.filter_type, self.serialized_filter)
    }
}
#[derive(Debug)]
pub struct SerializableError {
    pub inner: Box<dyn std::error::Error + Send + Sync>,
}

impl Display for SerializableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::error::Error for SerializableError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

/// Custom error that allows for keeping track of the filter type/path that caused the error
#[derive(Debug, Serialize, ThisError)]
pub struct FilterError {
    pub filter_path: Vec<FilterStepTrace>,
    #[source]
    pub error: SerializableError,
}

impl FilterError {
    pub fn new(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self {
            filter_path: Vec::new(),
            error: SerializableError::new(error),
        }
    }

    pub fn add_trace(&mut self, serialized_filter: String, filter_type: String) {
        self.filter_path.push(FilterStepTrace {
            serialized_filter,
            filter_type,
        });
    }
}

impl From<anyhow::Error> for FilterError {
    fn from(error: anyhow::Error) -> Self {
        Self::new(error.into())
    }
}

impl Display for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trace_path = self
            .filter_path
            .iter()
            .map(|trace| format!("{}", trace))
            .collect::<Vec<String>>()
            .join("\n");
        write!(
            f,
            "Filter Error: {:?}\nTrace Path:\n{}",
            self.error.inner, trace_path
        )
    }
}

impl SerializableError {
    fn new(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SerializableError { inner: error }
    }
}

// Implement Serialize for the wrapper
impl Serialize for SerializableError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize the error as its string representation
        serializer.serialize_str(&self.inner.to_string())
    }
}
