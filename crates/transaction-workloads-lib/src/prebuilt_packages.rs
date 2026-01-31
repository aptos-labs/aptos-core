// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Prebuilt packages are automatically compiled by build.rs during cargo build.
// To skip the build step (e.g., for debugging), set SKIP_PREBUILT_PACKAGES_BUILD=1.
//
// This file includes the auto-generated prebuilt_packages.rs from OUT_DIR.

// This provides PreBuiltPackagesImpl, to be used to access the prebuilt packages.

include!(concat!(env!("OUT_DIR"), "/prebuilt_transaction_generator_packages.rs"));
