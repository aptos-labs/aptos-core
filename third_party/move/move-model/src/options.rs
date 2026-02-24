// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::metadata::LanguageVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelBuilderOptions {
    /// The language version to use.
    pub language_version: LanguageVersion,

    /// Whether to compiler for testing. This will be reflected in the builtin constant
    /// `__COMPILE_FOR_TESTING__`.
    pub compile_for_testing: bool,
}
