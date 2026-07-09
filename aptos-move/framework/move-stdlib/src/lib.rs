// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod natives;

use move_command_line_common::files::{extension_equals, find_filenames, MOVE_EXTENSION};
use std::path::PathBuf;

const MODULES_DIR: &str = "sources";

fn path_in_crate<S: Into<String>>(relative: S) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

/// Paths to every Move source file in the stdlib.
pub fn move_stdlib_files() -> Vec<String> {
    let path = path_in_crate(MODULES_DIR);
    find_filenames(&[path], |p| extension_equals(p, MOVE_EXTENSION)).unwrap()
}

/// The `name=address` mappings the stdlib is compiled with.
pub fn move_stdlib_named_addresses_strings() -> Vec<String> {
    vec!["std=0x1".to_string(), "vm=0x0".to_string()]
}
