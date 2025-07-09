// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, harness::MoveHarness, BlockSplit};
use aptos_language_e2e_tests::{
    account::Account,
    executor::{assert_outputs_equal, ExecutorMode, FakeExecutor},
};
use aptos_types::{
    account_address::AccountAddress, on_chain_config::FeatureFlag, transaction::SignedTransaction,
};
use std::path::PathBuf;

pub fn initialize(
    path: PathBuf,
    mode: ExecutorMode,
    resource_group_charge_as_sum_enabled: bool,
    txns: usize,
) -> ResourceGroupsTestHarness {
    let (mut harness, account) =
        initialize_harness(mode, resource_group_charge_as_sum_enabled, path);
    harness.executor.disable_block_executor_fallback();

    let mut rg_harness = ResourceGroupsTestHarness {
        harness,
        comparison_harnesses: vec![],
        account,
        txn_accounts: vec![],
        txn_index: 0,
    };

    rg_harness.initialize_issuer_accounts(txns);
    rg_harness
}

pub fn initialize_enabled_disabled_comparison(
    path: PathBuf,
    mode: ExecutorMode,
    txns: usize,
) -> ResourceGroupsTestHarness {
    let (harness_base, account_base) = initialize_harness(mode, false, path.clone());
    let (harness_comp, _account_comp) = initialize_harness(mode, true, path);

    let mut rg_harness = ResourceGroupsTestHarness {
        harness: harness_base,
        comparison_harnesses: vec![harness_comp],
        account: account_base,
        txn_accounts: vec![],
        txn_index: 0,
    };

    rg_harness.initialize_issuer_accounts(txns);
    rg_harness
}

fn initialize_harness(
    mode: ExecutorMode,
    resource_group_charge_as_sum_enabled: bool,
    path: PathBuf,
) -> (MoveHarness, Account) {
    let executor = FakeExecutor::from_head_genesis().set_executor_mode(mode);

    let mut harness = MoveHarness::new_with_executor(executor);
    // Reduce gas scaling, so that smaller differences in gas are caught in comparison testing.
    harness.modify_gas_scaling(1000);
    if resource_group_charge_as_sum_enabled {
        harness.enable_features(
            vec![FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET],
            vec![FeatureFlag::APTOS_VM_V2],
        );
    } else {
        harness.enable_features(vec![], vec![
            FeatureFlag::APTOS_VM_V2,
            FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
        ]);
    }
    let account = harness.new_account_at(AccountAddress::ONE);
    assert_success!(harness.publish_package_cache_building(&account, &path));
    (harness, account)
}

pub struct ResourceGroupsTestHarness {
    pub harness: MoveHarness,
    pub comparison_harnesses: Vec<MoveHarness>,
    pub account: Account,
    pub txn_accounts: Vec<Account>,
    pub txn_index: usize,
}

impl ResourceGroupsTestHarness {
    // TODO[agg_v2](cleanup): These 3 functions are common with aggregator_v2 tests.
    // Should I create struct PropTestHarness with these 3 functions, and set
    // ResourceGroupsTestHarness { harness: PropTestHarness }?
    pub fn run_block_in_parts_and_check(
        &mut self,
        block_split: BlockSplit,
        txn_block: Vec<(u64, SignedTransaction)>,
    ) {
        let result = self
            .harness
            .run_block_in_parts_and_check(block_split, txn_block.clone());

        for (idx, h) in self.comparison_harnesses.iter_mut().enumerate() {
            let new_result = h.run_block_in_parts_and_check(block_split, txn_block.clone());
            assert_outputs_equal(
                &result,
                "baseline",
                &new_result,
                &format!("comparison {}", idx),
            );
        }
    }

    pub fn initialize_issuer_accounts(&mut self, num_accounts: usize) {
        self.txn_accounts = (0..num_accounts)
            .map(|_i| self.new_account_with_key_pair())
            .collect();
    }

    pub fn new_account_with_key_pair(&mut self) -> Account {
        let acc = Account::new();
        let seq_num = 0;
        // Mint the account 10M Aptos coins (with 8 decimals).
        let balance = 1_000_000_000_000_000;

        let result = self.harness.store_and_fund_account(&acc, balance, seq_num);

        for h in self.comparison_harnesses.iter_mut() {
            h.store_and_fund_account(&acc, balance, seq_num);
        }

        result
    }

    pub fn init_signer(&mut self, seed: Vec<u8>) -> SignedTransaction {
        self.harness.create_entry_function(
            &self.account,
            str::parse("0x1::resource_groups_test::init_signer").unwrap(),
            vec![],
            vec![bcs::to_bytes(&seed).unwrap()],
        )
    }

    pub fn set_resource(&mut self, index: u32, name: String, value: u32) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::set_resource").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index).unwrap(),
                bcs::to_bytes(&name).unwrap(),
                bcs::to_bytes(&value).unwrap(),
            ],
        )
    }

    pub fn unset_resource(&mut self, index: u32) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::unset_resource").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index).unwrap(),
            ],
        )
    }

    pub fn read_or_init(&mut self, index: u32) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::read_or_init").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index).unwrap(),
            ],
        )
    }

    pub fn set_3_group_members(
        &mut self,
        index1: u32,
        index2: u32,
        index3: u32,
        name: String,
        value: u32,
    ) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::set_3_group_members").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index1).unwrap(),
                bcs::to_bytes(&index2).unwrap(),
                bcs::to_bytes(&index3).unwrap(),
                bcs::to_bytes(&name).unwrap(),
                bcs::to_bytes(&value).unwrap(),
            ],
        )
    }

    pub fn check(&mut self, index: u32, name: String, value: u32) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::check").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index).unwrap(),
                bcs::to_bytes(&name).unwrap(),
                bcs::to_bytes(&value).unwrap(),
            ],
        )
    }

    pub fn set_and_check(
        &mut self,
        index1: u32,
        index2: u32,
        name1: String,
        value1: u32,
        name2: String,
        value2: u32,
    ) -> SignedTransaction {
        self.txn_index += 1;
        self.harness.create_entry_function(
            &self.txn_accounts[self.txn_index % self.txn_accounts.len()],
            str::parse("0x1::resource_groups_test::set_and_check").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(self.account.address()).unwrap(),
                bcs::to_bytes(&index1).unwrap(),
                bcs::to_bytes(&index2).unwrap(),
                bcs::to_bytes(&name1).unwrap(),
                bcs::to_bytes(&value1).unwrap(),
                bcs::to_bytes(&name2).unwrap(),
                bcs::to_bytes(&value2).unwrap(),
            ],
        )
    }
}
