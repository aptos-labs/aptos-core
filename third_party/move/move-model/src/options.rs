// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::simplifier::SimplificationPass;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelBuilderOptions {
    /// Whether compilation of the Move code into bytecode should be handled by the new compiler
    /// part of the model.
    pub compile_via_model: bool,

    /// Ignore the "opaque" pragma on internal function (i.e., functions with no unknown callers)
    /// specs when possible. The opaque can be ignored as long as the function spec has no property
    /// marked as `[concrete]` or `[abstract]`.
    pub ignore_pragma_opaque_internal_only: bool,

    /// Ignore the "opaque" pragma on all function specs when possible. The opaque can be ignored
    /// as long as the function spec has no property marked as `[concrete]` or `[abstract]`.
    pub ignore_pragma_opaque_when_possible: bool,

    /// List of simplification passes and the order each pass to be executed
    pub simplification_pipeline: Vec<SimplificationPass>,
}
