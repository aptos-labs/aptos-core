// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metadata::LanguageVersion;
use move_core_types::account_address::AccountAddress;

/// A module by address and name.
pub type ModuleRef = (AccountAddress, String);
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelBuilderOptions {
    /// The language version to use.
    pub language_version: LanguageVersion,

    /// Whether to compiler for testing. This will be reflected in the builtin constant
    /// `__COMPILE_FOR_TESTING__`.
    pub compile_for_testing: bool,

    /// Module dependencies the source does not show, as `(from, to)` pairs of
    /// `(address, name)`: an XIR interface calls modules its generated
    /// declarations do not name. The model keeps `to` whenever it keeps `from`.
    pub extra_dependencies: Vec<(ModuleRef, ModuleRef)>,
}
