// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use api_integration_tests::*;
use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};

fn main() -> Result<()> {
    let tests = ForgeConfig::default().with_public_usage_tests(&[&GetIndex]);

    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
