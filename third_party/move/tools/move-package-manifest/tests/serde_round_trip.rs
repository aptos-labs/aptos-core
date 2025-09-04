// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

datatest_stable::harness!(serde_round_trip, "tests", r".*\.toml$");

fn serde_round_trip(path: &Path) -> datatest_stable::Result<()> {
    let content = std::fs::read_to_string(path)?;

    if let Ok(parsed_manifest) = move_package_manifest::parse_package_manifest(&content) {
        let re_serialized =
            toml::to_string_pretty(&parsed_manifest).expect("failed to re-serialize-manifest");

        let re_parsed_manifest = move_package_manifest::parse_package_manifest(&re_serialized)
            .expect("failed to deserialize manifest again");

        assert_eq!(parsed_manifest, re_parsed_manifest)
    }

    Ok(())
}
