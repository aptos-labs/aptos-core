// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module-publishing verification shared between AptosVM's legacy publish path
//! and the `code::verify_package` native. Kept in the framework natives crate so
//! the native can reuse it (AptosVM cannot be a dependency of the natives crate).

pub mod event_validation;
pub mod native_validation;
pub mod publish;
pub mod resource_groups;
