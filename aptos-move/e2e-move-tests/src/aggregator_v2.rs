// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness};
use aptos_language_e2e_tests::{account::Account, executor::FakeExecutor};
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::SignedTransaction,
};
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorMode {
    Sequential,
    // Runs sequential, then parallel, and compares outputs.
    Both,
}

pub fn initialize(
    path: PathBuf,
    mode: ExecutorMode,
    aggregator_execution_enabled: bool,
) -> (MoveHarness, Account) {
    // Aggregator tests should use parallel execution.
    let executor = FakeExecutor::from_head_genesis();
    let executor = match mode {
        ExecutorMode::Sequential => executor.set_not_parallel(),
        // TODO Poorly named function, to rename.
        ExecutorMode::Both => executor.set_parallel(),
    };

    let mut harness = MoveHarness::new_with_executor(executor);
    if aggregator_execution_enabled {
        harness.enable_features(
            vec![
                FeatureFlag::AGGREGATOR_V2_API,
                FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
            ],
            vec![],
        );
    } else {
        harness.enable_features(vec![FeatureFlag::AGGREGATOR_V2_API], vec![
            FeatureFlag::AGGREGATOR_V2_DELAYED_FIELDS,
        ]);
    }
    let account = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package_cache_building(&account, &path));
    (harness, account)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UseType {
    UseResourceType = 0,
    UseTableType = 1,
    UseResourceGroupType = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementType {
    U64,
    U128,
    String,
}

impl ElementType {
    fn get_type_tag(&self) -> TypeTag {
        match self {
            ElementType::U64 => TypeTag::U64,
            ElementType::U128 => TypeTag::U128,
            ElementType::String => TypeTag::Struct(Box::new(StructTag {
                address: AccountAddress::ONE,
                module: ident_str!("string").to_owned(),
                name: ident_str!("String").to_owned(),
                type_params: vec![],
            })),
        }
    }

    fn value_to_bcs(&self, value: u128) -> Vec<u8> {
        match self {
            ElementType::U64 => bcs::to_bytes(&(value as u64)),
            ElementType::U128 => bcs::to_bytes(&value),
            ElementType::String => bcs::to_bytes(&value.to_string()),
        }
        .unwrap()
    }
}

#[derive(Debug)]
pub struct AggLocation<'a> {
    account: &'a Account,
    element_type: ElementType,
    use_type: UseType,
    index: u64,
}

impl<'a> AggLocation<'a> {
    pub fn new(
        account: &'a Account,
        element_type: ElementType,
        use_type: UseType,
        index: u64,
    ) -> AggLocation {
        AggLocation {
            account,
            use_type,
            index,
            element_type,
        }
    }
}

fn create_entry_agg_func_no_arg(
    harness: &mut MoveHarness,
    name: &str,
    agg_loc: &AggLocation,
) -> SignedTransaction {
    harness.create_entry_function(
        agg_loc.account,
        str::parse(name).unwrap(),
        vec![agg_loc.element_type.get_type_tag()],
        vec![
            bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc.index).unwrap(),
        ],
    )
}

fn create_entry_agg_func_with_arg(
    harness: &mut MoveHarness,
    name: &str,
    agg_loc: &AggLocation,
    argument: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        agg_loc.account,
        str::parse(name).unwrap(),
        vec![agg_loc.element_type.get_type_tag()],
        vec![
            bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc.index).unwrap(),
            agg_loc.element_type.value_to_bcs(argument),
        ],
    )
}

fn create_entry_agg_func_with_two_args(
    harness: &mut MoveHarness,
    name: &str,
    agg_loc: &AggLocation,
    argument_1: u128,
    argument_2: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        agg_loc.account,
        str::parse(name).unwrap(),
        vec![agg_loc.element_type.get_type_tag()],
        vec![
            bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc.index).unwrap(),
            agg_loc.element_type.value_to_bcs(argument_1),
            agg_loc.element_type.value_to_bcs(argument_2),
        ],
    )
}

pub fn init(
    harness: &mut MoveHarness,
    account: &Account,
    use_type: UseType,
    element_type: ElementType,
    aggregator: bool,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse(
            if aggregator {
                "0x1::aggregator_v2_test::init_aggregator"
            } else {
                "0x1::aggregator_v2_test::init_snapshot"
            },
        )
        .unwrap(),
        vec![element_type.get_type_tag()],
        vec![bcs::to_bytes(&(use_type as u32)).unwrap()],
    )
}

pub fn check(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    expected: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::check", agg_loc, expected)
}

pub fn check_snapshot(
    harness: &mut MoveHarness,
    snap_loc: &AggLocation,
    expected: u128,
) -> SignedTransaction {
    println!(
        "Check snapshot argument: {:?}",
        snap_loc.element_type.value_to_bcs(expected)
    );
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::check_snapshot",
        snap_loc,
        expected,
    )
}

pub fn new(harness: &mut MoveHarness, agg_loc: &AggLocation, max_value: u128) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::new", agg_loc, max_value)
}

pub fn add(harness: &mut MoveHarness, agg_loc: &AggLocation, value: u128) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::add", agg_loc, value)
}

pub fn try_add(harness: &mut MoveHarness, agg_loc: &AggLocation, value: u128) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::try_add", agg_loc, value)
}

pub fn sub(harness: &mut MoveHarness, agg_loc: &AggLocation, value: u128) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::sub", agg_loc, value)
}

pub fn try_sub(harness: &mut MoveHarness, agg_loc: &AggLocation, value: u128) -> SignedTransaction {
    create_entry_agg_func_with_arg(harness, "0x1::aggregator_v2_test::try_sub", agg_loc, value)
}

pub fn new_add(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    max_value: u128,
    a: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_two_args(
        harness,
        "0x1::aggregator_v2_test::new_add",
        agg_loc,
        max_value,
        a,
    )
}

pub fn sub_add(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    a: u128,
    b: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_two_args(harness, "0x1::aggregator_v2_test::sub_add", agg_loc, a, b)
}

pub fn add_sub(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    a: u128,
    b: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_two_args(harness, "0x1::aggregator_v2_test::add_sub", agg_loc, a, b)
}

pub fn materialize(harness: &mut MoveHarness, agg_loc: &AggLocation) -> SignedTransaction {
    create_entry_agg_func_no_arg(harness, "0x1::aggregator_v2_test::materialize", agg_loc)
}

pub fn materialize_and_add(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    value: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::materialize_and_add",
        agg_loc,
        value,
    )
}

pub fn materialize_and_sub(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    value: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::materialize_and_sub",
        agg_loc,
        value,
    )
}

pub fn add_and_materialize(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    value: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::add_and_materialize",
        agg_loc,
        value,
    )
}

pub fn sub_and_materialize(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    value: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::sub_and_materialize",
        agg_loc,
        value,
    )
}

pub fn add_2(
    harness: &mut MoveHarness,
    agg_loc_a: &AggLocation,
    agg_loc_b: &AggLocation,
    value_a: u128,
    value_b: u128,
) -> SignedTransaction {
    harness.create_entry_function(
        agg_loc_a.account,
        str::parse("0x1::aggregator_v2_test::add_2").unwrap(),
        vec![
            agg_loc_a.element_type.get_type_tag(),
            agg_loc_b.element_type.get_type_tag(),
        ],
        vec![
            bcs::to_bytes(&(agg_loc_a.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc_a.index).unwrap(),
            agg_loc_a.element_type.value_to_bcs(value_a),
            bcs::to_bytes(&agg_loc_b.account.address()).unwrap(),
            bcs::to_bytes(&(agg_loc_b.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc_b.index).unwrap(),
            agg_loc_b.element_type.value_to_bcs(value_b),
        ],
    )
}

pub fn snapshot(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    snap_loc: &AggLocation,
) -> SignedTransaction {
    assert_eq!(agg_loc.element_type, snap_loc.element_type);
    harness.create_entry_function(
        agg_loc.account,
        str::parse("0x1::aggregator_v2_test::snapshot").unwrap(),
        vec![agg_loc.element_type.get_type_tag()],
        vec![
            bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc.index).unwrap(),
            bcs::to_bytes(&snap_loc.account.address()).unwrap(),
            bcs::to_bytes(&(snap_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&snap_loc.index).unwrap(),
        ],
    )
}

pub fn concat(
    harness: &mut MoveHarness,
    input_loc: &AggLocation,
    output_loc: &AggLocation,
    prefix: &str,
    suffix: &str,
) -> SignedTransaction {
    assert_eq!(output_loc.element_type, ElementType::String);
    harness.create_entry_function(
        input_loc.account,
        str::parse("0x1::aggregator_v2_test::concat").unwrap(),
        vec![input_loc.element_type.get_type_tag()],
        vec![
            bcs::to_bytes(&(input_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&input_loc.index).unwrap(),
            bcs::to_bytes(&output_loc.account.address()).unwrap(),
            bcs::to_bytes(&(output_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&output_loc.index).unwrap(),
            bcs::to_bytes(&prefix.to_string()).unwrap(),
            bcs::to_bytes(&suffix.to_string()).unwrap(),
        ],
    )
}

pub fn read_snapshot(harness: &mut MoveHarness, agg_loc: &AggLocation) -> SignedTransaction {
    create_entry_agg_func_no_arg(harness, "0x1::aggregator_v2_test::read_snapshot", agg_loc)
}

pub fn add_and_read_snapshot_u128(
    harness: &mut MoveHarness,
    agg_loc: &AggLocation,
    value: u128,
) -> SignedTransaction {
    create_entry_agg_func_with_arg(
        harness,
        "0x1::aggregator_v2_test::add_and_read_snapshot",
        agg_loc,
        value,
    )
}

// indempotent verify functions:

pub fn verify_copy_snapshot(harness: &mut MoveHarness, account: &Account) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_copy_snapshot").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_copy_string_snapshot(
    harness: &mut MoveHarness,
    account: &Account,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_copy_string_snapshot").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_string_concat(harness: &mut MoveHarness, account: &Account) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_string_concat").unwrap(),
        vec![],
        vec![],
    )
}

pub fn verify_string_snapshot_concat(
    harness: &mut MoveHarness,
    account: &Account,
) -> SignedTransaction {
    harness.create_entry_function(
        account,
        str::parse("0x1::aggregator_v2_test::verify_string_snapshot_concat").unwrap(),
        vec![],
        vec![],
    )
}
