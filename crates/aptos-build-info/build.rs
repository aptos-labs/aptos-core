// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

fn main() -> shadow_rs::SdResult<()> {
    // CARGO_CFG env vars don't make it to the program at runtime, so we
    // propagate it here by adding a new env var via this cargo directive.
    println!(
        "cargo:rustc-env=USING_TOKIO_UNSTABLE={}",
        std::env::var("CARGO_CFG_TOKIO_UNSTABLE").is_ok()
    );
    println!("cargo:rerun-if-changed=build.rs");
    // Check for this path first, otherwise it will force a rebuild every time
    // https://github.com/rust-lang/cargo/issues/4213
    let git_head = std::path::Path::new("../../.git/HEAD");
    if git_head.exists() {
        println!("cargo:rerun-if-changed=../../.git/HEAD");
    }
    shadow_rs::new()
}
