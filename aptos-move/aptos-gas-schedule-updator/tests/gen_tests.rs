// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_gas_schedule_updator::GenerateNewSchedule;

#[test]
fn can_generate_and_build_update_proposal() {
    let output_dir = tempfile::tempdir().unwrap();

    GenerateNewSchedule {
        gas_feature_version: None,
        output: Some(output_dir.path().to_string_lossy().to_string()),
    }
    .execute()
    .unwrap();

    BuiltPackage::build(output_dir.path().to_path_buf(), BuildOptions::default()).unwrap();
}
