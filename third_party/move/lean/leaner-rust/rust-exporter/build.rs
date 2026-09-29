// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! The exporter links against the pinned toolchain's `rustc_driver`, whose
//! LLVM shared library the `rustc-dev` component ships in the sysroot's
//! `lib` directory on some hosts (aarch64 Linux among them) rather than
//! beside `librustc_driver`. rustc adds only the latter to the linker's
//! search path, so the link then fails with "library not found: LLVM-…"
//! whatever the linker. Name the sysroot `lib` directory explicitly; where
//! the library already sits beside the driver this is redundant and
//! harmless. Running the binary still needs that directory on
//! `LD_LIBRARY_PATH`, as the README says.

use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTC");
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(rustc)
        .arg("--print")
        .arg("sysroot")
        .output()
        .expect("`rustc --print sysroot` must run to locate the rustc-dev libraries");
    let sysroot = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    println!(
        "cargo:rustc-link-search=native={}",
        sysroot.join("lib").display()
    );
}
