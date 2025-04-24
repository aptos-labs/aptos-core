// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[tokio::main]
async fn main() {
    #[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
    raikou::simulation_test::main().await;
}
