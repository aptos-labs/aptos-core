// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
            args.fail_fast,
        )
        .unwrap();
    }
}
