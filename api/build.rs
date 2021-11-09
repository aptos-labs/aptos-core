// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::{env, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=doc/openapi.yaml");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let path = Path::new(&out_dir).join("spec.html");
    let dest_path = path.to_str().unwrap();
    let args = vec![
        "redoc-cli",
        "bundle",
        "doc/openapi.yaml",
        "-o",
        dest_path,
        "--title",
        "Diem API Specification",
    ];
    let result = Command::new("npx").args(&args).output();

    if result.is_err() {
        println!(
            "cargo:warning=Run `scripts/dev_setup.sh` to install build tools(nodejs, npm & npx)."
        );
    }
    let exec_cmd = format!("executing `npx {}`", args.join(" "));
    let output = result.expect(&exec_cmd);
    if !output.status.success() {
        println!("cargo:warning={}", &exec_cmd);
        println!("cargo:warning={}", output.status);
        println!(
            "cargo:warning=stdout: {}",
            String::from_utf8(output.stdout).unwrap()
        );
        println!(
            "cargo:warning=stderr: {}",
            String::from_utf8(output.stderr).unwrap()
        );
        panic!("redoc-cli bundle failed");
    }
}
