// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

fn main() {
    // Disables telemetry for all x commands by setting the environment variable
    println!("cargo:rustc-env=APTOS_DISABLE_TELEMETRY={}", 1);
}
