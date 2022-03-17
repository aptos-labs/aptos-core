// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_vm::natives::aptos_natives;
use framework::diem_framework_named_addresses;
use move_compiler::shared::NumericalAddress;
use move_unit_test::UnitTestingConfig;

#[test]
fn move_unit_tests() {
    let mut named_addresses = diem_framework_named_addresses();
    named_addresses.insert(
        "HelloBlockchain".to_owned(),
        NumericalAddress::parse_str("0xe110").unwrap(),
    );

    let config =
        UnitTestingConfig::default_with_bound(Some(100_000)).with_named_addresses(named_addresses);

    move_unit_test::cargo_runner::run_tests_with_config_and_filter(
        config,
        ".",
        r"sources/.*\.move$",
        Some(&move_stdlib::move_stdlib_modules_full_path()),
        Some(aptos_natives()),
    );
}
