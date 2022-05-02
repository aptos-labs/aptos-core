// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../aptos-framework/sources");
    println!("cargo:rerun-if-changed=../move-stdlib/sources");
    let release = framework::release::ReleaseOptions {
        check_layout_compatibility: false,
        build_docs: false,
        with_diagram: false,
        script_builder: false,
        script_abis: false,
        errmap: false,
        package: PathBuf::from("aptos-framework"),
        output: PathBuf::from(std::env::var("OUT_DIR").unwrap()),
    };
    release.create_release();
}
