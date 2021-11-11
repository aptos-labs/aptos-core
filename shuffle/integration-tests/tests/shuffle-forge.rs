// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use forge::{forge_main, ForgeConfig, LocalFactory, Options, Result};
use shuffle_integration_tests::*;

fn main() -> Result<()> {
    let tests = ForgeConfig::default().with_admin_tests(&[&SetMessageHelloBlockchain]);
    let options = Options::from_args();
    forge_main(tests, LocalFactory::from_workspace()?, &options)
}
