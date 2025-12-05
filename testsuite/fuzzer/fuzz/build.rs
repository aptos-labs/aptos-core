// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::env;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(coverage_enabled)");

    let rustflags = env::var("RUSTFLAGS").unwrap_or_default();
    let sanitizer = env::var("RUSTC_SANITIZER").unwrap_or_default();

    let has_instr_cov = rustflags.contains("instrument-coverage");
    let has_san_cov = rustflags.contains("sanitizer=coverage") || sanitizer.contains("coverage");

    if has_instr_cov || has_san_cov {
        println!("cargo:rustc-cfg=coverage_enabled");
    }
}
