// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_command_line_common::files::MOVE_ERROR_DESC_EXTENSION;
use std::path::{Path, PathBuf};

fn main() {
    let temppath = aptos_temppath::TempPath::new();

    let release = framework::release::ReleaseOptions {
        check_layout_compatibility: false,
        build_docs: false,
        with_diagram: false,
        script_builder: false,
        script_abis: false,
        errmap: false,
        package: PathBuf::from("aptos-framework"),
        output: temppath.path().to_path_buf(),
    };
    release.create_release();

    let base_path = temppath.path(); //.join("aptos-framework").join("releases").join("artifacts").join("current");
    let mut errmap = base_path
        .join("error_description")
        .join("error_description");
    errmap.set_extension(MOVE_ERROR_DESC_EXTENSION);

    std::fs::create_dir_all("errmap").unwrap_or_else(|_| panic!("Unable to create path: errmap"));
    let mut errmap_out = PathBuf::from("errmap").join("error_description");
    errmap_out.set_extension(MOVE_ERROR_DESC_EXTENSION);
    read_and_write(&errmap, &errmap_out);

    let transaction_script_builder = base_path.join("transaction_script_builder.rs");
    let transaction_script_builder_out = PathBuf::from("src").join("aptos_stdlib.rs");
    read_and_write(&transaction_script_builder, &transaction_script_builder_out);
}

fn read_and_write(inpath: &Path, outpath: &Path) {
    let data = std::fs::read(inpath).unwrap_or_else(|_| panic!("Unable to read: {:?}", inpath));
    std::fs::write(outpath, data).unwrap_or_else(|_| panic!("Unable to write: {:?}", outpath));
}
