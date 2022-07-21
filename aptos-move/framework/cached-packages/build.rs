// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use framework::release::CORE_FRAMEWORK_RELEASE_SUFFIX;
use framework::release::TOKEN_RELEASE_SUFFIX;

fn main() {
    println!("cargo:rerun-if-changed=../aptos-framework/sources");
    println!("cargo:rerun-if-changed=../aptos-stdlib/sources");
    println!("cargo:rerun-if-changed=../move-stdlib/sources");
    let release = framework::release::ReleaseOptions {
        no_check_layout_compatibility: false,
        no_build_docs: false,
        with_diagram: false,
        no_script_builder: true, // TODO: consider bringing this back
        no_script_abis: false,
        no_errmap: false,
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
        no_check_layout_compatibility: false,
        no_build_docs: true,
        with_diagram: true,
        no_script_builder: true,
        no_script_abis: false,
        no_errmap: false,
        package: PathBuf::from("aptos-token"),
        output: PathBuf::from(format!(
            "{}/{}",
            std::env::var("OUT_DIR").unwrap(),
            TOKEN_RELEASE_SUFFIX
        )),
    };
    token_release.create_release();
}
