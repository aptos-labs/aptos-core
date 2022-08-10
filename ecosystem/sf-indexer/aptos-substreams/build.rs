// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn main() {
    let out_dir = "src/pb";
    let mut prost_build = prost_build::Config::new();

    let proto_path =
        std::path::Path::new(&std::env::current_dir().unwrap()).join("../../sf-stream/proto/");

    prost_build
        .out_dir(out_dir)
        .include_file("mod.rs")
        .compile_well_known_types()
        .compile_protos(
            &[
                proto_path
                    .clone()
                    .join("extractor.proto")
                    .into_os_string()
                    .into_string()
                    .unwrap()
                    .to_string(),
                "proto/block_output.proto".to_string(),
            ],
            &[
                proto_path
                    .into_os_string()
                    .into_string()
                    .unwrap()
                    .to_string(),
                "proto/".to_string(),
            ],
        )
        .unwrap();
}
