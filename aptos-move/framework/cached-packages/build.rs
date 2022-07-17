// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use framework::release::CORE_FRAMEWORK_RELEASE_SUFFIX;
use framework::release::TOKEN_RELEASE_SUFFIX;

fn main() {
    println!("cargo:rerun-if-changed=../aptos-framework/sources");
    println!("cargo:rerun-if-changed=../move-stdlib/sources");
    println!("cargo:rerun-if-changed=../move-stdlib/nursery/sources");
    let release = framework::release::ReleaseOptions {
        check_layout_compatibility: false,
        build_docs: false,
        with_diagram: false,
        script_builder: false,
        script_abis: false,
        errmap: false,
        package: PathBuf::from("aptos-framework"),
        output: PathBuf::from(format!(
            "{}/{}",
            std::env::var("OUT_DIR").unwrap(),
            CORE_FRAMEWORK_RELEASE_SUFFIX
        )),
    };
    release.create_release();

    println!("cargo:rerun-if-changed=../aptos-token/sources");
    let token_release = framework::release::ReleaseOptions {
        check_layout_compatibility: false,
        build_docs: true,
        with_diagram: true,
        script_builder: false,
        script_abis: false,
        errmap: false,
        package: PathBuf::from("aptos-token"),
        output: PathBuf::from(format!(
            "{}/{}",
            std::env::var("OUT_DIR").unwrap(),
            TOKEN_RELEASE_SUFFIX
        )),
    };
    token_release.create_release();
}
