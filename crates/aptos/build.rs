// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

fn main() -> shadow_rs::SdResult<()> {
    println!("cargo:rerun-if-changed=build.rs");
    // Check for this path first, otherwise it will force a rebuild every time
    // https://github.com/rust-lang/cargo/issues/4213
    let git_head = std::path::Path::new("../../.git/HEAD");
    if git_head.exists() {
        println!("cargo:rerun-if-changed=../../.git/HEAD");
    }
    shadow_rs::new()
}
