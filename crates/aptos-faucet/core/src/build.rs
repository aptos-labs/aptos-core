// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

fn main() {
    println!("cargo:rerun-if-changed=../doc/.version");
    println!("cargo:rerun-if-changed=../move_scripts/build/Minter/bytecode_scripts/main.mv");
}
