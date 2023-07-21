// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// We hide these tests behind a feature flag because these are not standard unit tests,
// these are integration tests that rely on a variety of outside pieces such as a local
// testnet and a running Redis instance.
#[cfg(feature = "integration-tests")]
mod tests;
