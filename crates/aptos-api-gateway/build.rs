// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    println!("cargo:rerun-if-changed=src/src/testing/proto/testing.proto");
    #[cfg(test)]
    compile_protos(&["tests/proto/testing.proto"], &["tests/proto"]).unwrap();
}
