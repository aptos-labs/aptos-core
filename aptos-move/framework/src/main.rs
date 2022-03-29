// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use clap::Parser;
use framework::release::ReleaseOptions;

// Generates the compiled stdlib and transaction scripts. Until this is run changes to the source
// modules/scripts, and changes in the Move compiler will not be reflected in the stdlib used for
// genesis, and everywhere else across the code-base unless otherwise specified.
fn main() {
    let release_options = ReleaseOptions::parse();
    release_options.create_release();
}
