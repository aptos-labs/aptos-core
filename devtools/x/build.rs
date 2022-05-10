// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

fn main() {
    // disables telemetry for all x commands by setting environment variable
    println!("cargo:rustc-env=APTOS_TELEMETRY_DISABLE={}", 1);
}
