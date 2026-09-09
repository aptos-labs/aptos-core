// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Stamps the build identity echoed in every response payload: the compiling
//! rustc's version line and the Cargo profile name. The Lean harness logs
//! this next to its Move frontend's identity so a stale build is visible.

fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=LEAN_LINK_PROFILE={profile}");
    println!("cargo:rustc-env=LEAN_LINK_RUSTC={version}");
    println!("cargo:rerun-if-changed=build.rs");
}
