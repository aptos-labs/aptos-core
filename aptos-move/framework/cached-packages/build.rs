// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn main() {
    let release = framework::release::ReleaseOptions {
        check_layout_compatibility: false,
        build_docs: false,
        with_diagram: false,
        script_builder: false,
        script_abis: false,
        errmap: false,
        package: PathBuf::from("aptos-framework"),
        output: PathBuf::from("fresh"),
    };
    release.create_release();
}
