// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

pub mod framework;
pub mod tasks;
pub mod templates;
#[cfg(feature = "fuzzing")]
pub mod transactional_ops;
pub mod vm_test_harness;
