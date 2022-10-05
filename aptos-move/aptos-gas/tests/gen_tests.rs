// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::gen::{generate_update_proposal, GenArgs};
use framework::{BuildOptions, BuiltPackage};

#[test]
fn can_generate_and_build_update_proposal() {
    let output_dir = tempfile::tempdir().unwrap();

    generate_update_proposal(&GenArgs {
        output: Some(output_dir.path().to_string_lossy().to_string()),
    })
    .unwrap();

    BuiltPackage::build(output_dir.path().to_path_buf(), BuildOptions::default()).unwrap();
}
