// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Paranoid Type checking returns an error
pub const EPARANOID_FAILURE: u64 = 1001;

// Reference safety checks failure
pub const EREFERENCE_COUNTING_FAILURE: u64 = 1002;

// User provided typetag failed to load.
pub const EUSER_TYPE_LOADING_FAILURE: u64 = 1003;
