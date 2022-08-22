// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::ReleaseTarget;
use std::path::PathBuf;

fn main() {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if std::env::var("SKIP_FRAMEWORK_BUILD").is_err() {
        println!("cargo:rerun-if-changed=../aptos-token/sources");
        println!("cargo:rerun-if-changed=../aptos-token/Move.toml");
        println!("cargo:rerun-if-changed=../aptos-framework/sources");
        println!("cargo:rerun-if-changed=../aptos-framework/Move.toml");
        println!("cargo:rerun-if-changed=../aptos-stdlib/sources");
        println!("cargo:rerun-if-changed=../aptos-stdlib/Move.toml");
        println!("cargo:rerun-if-changed=../move-stdlib/sources");
        println!("cargo:rerun-if-changed=../move-stdlib/Move.toml");
        ReleaseTarget::Head
            .create_release(
                true,
                Some(
                    PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR defined"))
                        .join("head.mrb"),
                ),
            )
            .expect("release build failed");
    }
}
