// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness};
use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecutorMode, FakeExecutor},
};
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::SignedTransaction,
};
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use std::path::PathBuf;

pub fn initialize(
    path: PathBuf,
    mode: ExecutorMode,
    aggregator_execution_enabled: bool,
    txns: usize,
) -> AggV2TestHarness {
    // Aggregator tests should use parallel execution.
    let executor = FakeExecutor::from_head_genesis().set_executor_mode(mode);

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

    let txn_accounts: Vec<Account> = (0..txns)
        .map(|_i| harness.new_account_with_key_pair())
        .collect();

    AggV2TestHarness {
        harness,
        account,
        txn_accounts,
        txn_index: 0,
    }
}

pub struct AggV2TestHarness {
    pub harness: MoveHarness,
    pub account: Account,
    pub txn_accounts: Vec<Account>,
    pub txn_index: usize,
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

// For a generic test, so we can test any combination of features, with the same "aggregator equation test case",
// we define a generic aggregator (or snapshot) location.
// It is defined by:
// - the address of the account that resource is stored in
// - what type the stored element is (u64/u128/string)
// - what type of resource it is stored in (resource/table/resource group)
// - the index inside of a vector in resource/resource group,
//   or (key, index) pair inside table (where key=i / 10, index=i % 10)
#[derive(Debug)]
pub struct AggregatorLocation {
    address: AccountAddress,
    element_type: ElementType,
    use_type: UseType,
    index: u64,
}

impl AggregatorLocation {
    pub fn new(
        address: AccountAddress,
        element_type: ElementType,
        use_type: UseType,
        index: u64,
    ) -> AggregatorLocation {
        AggregatorLocation {
            address,
            use_type,
            index,
            element_type,
        }
    }
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

impl AggV2TestHarness {
    fn create_entry_agg_func_with_args(
        &mut self,
        name: &str,
        agg_loc: &AggregatorLocation,
        arguments: &[u128],
    ) -> SignedTransaction {
        self.txn_index += 1;

        let mut args = vec![
            bcs::to_bytes(&agg_loc.address).unwrap(),
            bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
            bcs::to_bytes(&agg_loc.index).unwrap(),
        ];
        for arg in arguments {
            args.push(agg_loc.element_type.value_to_bcs(*arg))
        }
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse(name).unwrap(),
            vec![agg_loc.element_type.get_type_tag()],
            args,
        )
    }

    pub fn check(&mut self, agg_loc: &AggregatorLocation, expected: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::check", agg_loc, &[expected])
    }

    pub fn check_snapshot(
        &mut self,
        snap_loc: &AggregatorLocation,
        expected: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::check_snapshot",
            snap_loc,
            &[expected],
        )
    }

    #[allow(clippy::new_ret_no_self)]
    pub fn new(&mut self, agg_loc: &AggregatorLocation, max_value: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::new", agg_loc, &[max_value])
    }

    pub fn add(&mut self, agg_loc: &AggregatorLocation, value: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::add", agg_loc, &[value])
    }

    pub fn try_add(&mut self, agg_loc: &AggregatorLocation, value: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::try_add", agg_loc, &[value])
    }

    pub fn sub(&mut self, agg_loc: &AggregatorLocation, value: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::sub", agg_loc, &[value])
    }

    pub fn try_sub(&mut self, agg_loc: &AggregatorLocation, value: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::try_sub", agg_loc, &[value])
    }

    pub fn new_add(
        &mut self,
        agg_loc: &AggregatorLocation,
        max_value: u128,
        a: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::new_add", agg_loc, &[
            max_value, a,
        ])
    }

    pub fn sub_add(&mut self, agg_loc: &AggregatorLocation, a: u128, b: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::sub_add", agg_loc, &[a, b])
    }

    pub fn add_sub(&mut self, agg_loc: &AggregatorLocation, a: u128, b: u128) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::add_sub", agg_loc, &[a, b])
    }

    pub fn materialize(&mut self, agg_loc: &AggregatorLocation) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::materialize", agg_loc, &[])
    }

    pub fn materialize_and_add(
        &mut self,
        agg_loc: &AggregatorLocation,
        value: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::materialize_and_add",
            agg_loc,
            &[value],
        )
    }

    pub fn materialize_and_sub(
        &mut self,
        agg_loc: &AggregatorLocation,
        value: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::materialize_and_sub",
            agg_loc,
            &[value],
        )
    }

    pub fn add_and_materialize(
        &mut self,
        agg_loc: &AggregatorLocation,
        value: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::add_and_materialize",
            agg_loc,
            &[value],
        )
    }

    pub fn sub_and_materialize(
        &mut self,
        agg_loc: &AggregatorLocation,
        value: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::sub_and_materialize",
            agg_loc,
            &[value],
        )
    }

    pub fn add_2(
        &mut self,
        agg_loc_a: &AggregatorLocation,
        agg_loc_b: &AggregatorLocation,
        value_a: u128,
        value_b: u128,
    ) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::add_2").unwrap(),
            vec![
                agg_loc_a.element_type.get_type_tag(),
                agg_loc_b.element_type.get_type_tag(),
            ],
            vec![
                bcs::to_bytes(&agg_loc_a.address).unwrap(),
                bcs::to_bytes(&(agg_loc_a.use_type as u32)).unwrap(),
                bcs::to_bytes(&agg_loc_a.index).unwrap(),
                agg_loc_a.element_type.value_to_bcs(value_a),
                bcs::to_bytes(&agg_loc_b.address).unwrap(),
                bcs::to_bytes(&(agg_loc_b.use_type as u32)).unwrap(),
                bcs::to_bytes(&agg_loc_b.index).unwrap(),
                agg_loc_b.element_type.value_to_bcs(value_b),
            ],
        )
    }

    pub fn snapshot(
        &mut self,
        agg_loc: &AggregatorLocation,
        snap_loc: &AggregatorLocation,
    ) -> SignedTransaction {
        assert_eq!(agg_loc.element_type, snap_loc.element_type);
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::snapshot").unwrap(),
            vec![agg_loc.element_type.get_type_tag()],
            vec![
                bcs::to_bytes(&agg_loc.address).unwrap(),
                bcs::to_bytes(&(agg_loc.use_type as u32)).unwrap(),
                bcs::to_bytes(&agg_loc.index).unwrap(),
                bcs::to_bytes(&snap_loc.address).unwrap(),
                bcs::to_bytes(&(snap_loc.use_type as u32)).unwrap(),
                bcs::to_bytes(&snap_loc.index).unwrap(),
            ],
        )
    }

    pub fn concat(
        &mut self,
        input_loc: &AggregatorLocation,
        output_loc: &AggregatorLocation,
        prefix: &str,
        suffix: &str,
    ) -> SignedTransaction {
        assert_eq!(output_loc.element_type, ElementType::String);
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::concat").unwrap(),
            vec![input_loc.element_type.get_type_tag()],
            vec![
                bcs::to_bytes(&input_loc.address).unwrap(),
                bcs::to_bytes(&(input_loc.use_type as u32)).unwrap(),
                bcs::to_bytes(&input_loc.index).unwrap(),
                bcs::to_bytes(&output_loc.address).unwrap(),
                bcs::to_bytes(&(output_loc.use_type as u32)).unwrap(),
                bcs::to_bytes(&output_loc.index).unwrap(),
                bcs::to_bytes(&prefix.to_string()).unwrap(),
                bcs::to_bytes(&suffix.to_string()).unwrap(),
            ],
        )
    }

    pub fn read_snapshot(&mut self, agg_loc: &AggregatorLocation) -> SignedTransaction {
        self.create_entry_agg_func_with_args("0x1::aggregator_v2_test::read_snapshot", agg_loc, &[])
    }

    pub fn add_and_read_snapshot_u128(
        &mut self,
        agg_loc: &AggregatorLocation,
        value: u128,
    ) -> SignedTransaction {
        self.create_entry_agg_func_with_args(
            "0x1::aggregator_v2_test::add_and_read_snapshot",
            agg_loc,
            &[value],
        )
    }

    // idempotent verify functions:

    pub fn verify_copy_snapshot(&mut self) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::verify_copy_snapshot").unwrap(),
            vec![],
            vec![],
        )
    }

    pub fn verify_copy_string_snapshot(&mut self) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::verify_copy_string_snapshot").unwrap(),
            vec![],
            vec![],
        )
    }

    pub fn verify_string_concat(&mut self) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::verify_string_concat").unwrap(),
            vec![],
            vec![],
        )
    }

    pub fn verify_string_snapshot_concat(&mut self) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::aggregator_v2_test::verify_string_snapshot_concat").unwrap(),
            vec![],
            vec![],
        )
    }
}
