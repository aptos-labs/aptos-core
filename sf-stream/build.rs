// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use prost_wkt_build::*;
use std::path::PathBuf;

fn main() {
    let proto_path = std::path::Path::new(&std::env::current_dir().unwrap())
        .join("../crates/aptos-protos/indexer/");
    let out_dir = "src/pb";
    let out = PathBuf::from(out_dir);
    let descriptor_file = out.join("descriptors.bin");
    let mut prost_build = prost_build::Config::new();
    prost_build
        .out_dir(out_dir)
        .type_attribute(".", "#[derive(serde::Serialize,serde::Deserialize)]")
        .file_descriptor_set_path(&descriptor_file)
        .compile_protos(
            &[proto_path
                .clone()
                .join("extractor.proto")
                .into_os_string()
                .into_string()
                .unwrap()],
            &[proto_path.into_os_string().into_string().unwrap()],
        )
        .unwrap();

    let descriptor_bytes = std::fs::read(descriptor_file).unwrap();

    let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();

    prost_wkt_build::add_serde(out, descriptor);
}
