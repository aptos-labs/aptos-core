// Copyright © Aptos Foundation

use crate::{tests::common, MoveHarness};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use move_core_types::account_address::AccountAddress;

#[test]
fn test_large_type_identifier() -> anyhow::Result<()> {
    let mut h = MoveHarness::new();

    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    h.publish_package(&acc, &common::test_dir_path("too_large.data/pack"));

    {
        let tx = h.run_entry_function(
            &acc,
            str::parse("0xcafe::poc_module::f").unwrap(),
            vec![],
            vec![],
        );
        assert!(matches!(
            tx,
            TransactionStatus::Keep(ExecutionStatus::ExecutionFailure {
                code_offset: _,
                function: _,
                location: _
            })
        ));
        Ok(())
    }
}
