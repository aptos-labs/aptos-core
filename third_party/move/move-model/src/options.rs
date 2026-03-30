// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
