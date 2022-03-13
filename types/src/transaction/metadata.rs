// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! This file implements Diem transaction metadata types to allow
//! easy parsing and introspection into metadata, whether the transaction
//! is using regular subaddressing, is subject to travel rule or corresponds
//! to an on-chain payment refund.

use serde::{Deserialize, Serialize};

/// List of all supported metadata types
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Metadata {
    Undefined,
    UnstructuredBytesMetadata(UnstructuredBytesMetadata),
}

/// Opaque binary transaction metadata
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnstructuredBytesMetadata {
    /// Unstructured byte vector metadata
    #[serde(with = "serde_bytes")]
    pub metadata: Option<Vec<u8>>,
}
