// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Common code for unused item checks.

/// Attribute names that suppress unused warnings for all item types.
/// - `deprecated`: Marks items that are deprecated but may not be removed.
pub const SHARED_SUPPRESSION_ATTRS: &[&str] = &["deprecated"];
