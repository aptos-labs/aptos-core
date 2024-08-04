// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod loader;
pub(crate) mod struct_name_index_map;
pub(crate) mod struct_type_storage;

// Note: these traits should be defined elsewhere, along with Script and Module types.
//       We keep them here for now so that it is easier to land new changes.
pub mod module_storage;
pub mod script_storage;
pub mod verifier;

// TODO(loader_v2): Remove when we no longer need the dummy implementation.
pub mod dummy;
mod test_utils;
#[cfg(any(test, feature = "testing"))]
pub use test_utils::{TestModuleStorage, TestScriptStorage};
