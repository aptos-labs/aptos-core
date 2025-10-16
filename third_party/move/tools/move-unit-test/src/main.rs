// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use clap::*;
use move_unit_test::{test_reporter::UnitTestFactoryWithCostTable, UnitTestingConfig};

pub fn main() {
    let args = UnitTestingConfig::parse();

    let test_plan = args.build_test_plan();
    if let Some(test_plan) = test_plan {
        args.run_and_report_unit_tests(
            test_plan,
            None,
            None,
            std::io::stdout(),
            UnitTestFactoryWithCostTable::new(None, None),
            false,
        )
        .unwrap();
    }
}
