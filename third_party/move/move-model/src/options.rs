// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::metadata::LanguageVersion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelBuilderOptions {
    /// Whether compilation of the Move code into bytecode should be handled via
    /// the model builder.
    pub compile_via_model: bool,

    /// The language version to use.
    pub language_version: LanguageVersion,

    /// Ignore the "opaque" pragma on internal function (i.e., functions with no unknown callers)
    /// specs when possible. The opaque can be ignored as long as the function spec has no property
    /// marked as `[concrete]` or `[abstract]`.
    pub ignore_pragma_opaque_internal_only: bool,

    /// Ignore the "opaque" pragma on all function specs when possible. The opaque can be ignored
    /// as long as the function spec has no property marked as `[concrete]` or `[abstract]`.
    pub ignore_pragma_opaque_when_possible: bool,
}
