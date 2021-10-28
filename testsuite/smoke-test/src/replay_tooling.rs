// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_sdk::{
    client::views::VMStatusView, transaction_builder::Currency,
    types::account_address::AccountAddress,
};
use diem_transaction_replay::DiemDebugger;
use forge::{PublicUsageContext, PublicUsageTest, Result, Test};

pub struct ReplayTooling;

impl Test for ReplayTooling {
    fn name(&self) -> &'static str {
        "smoke-test::replay-tooling"
    }
}

impl PublicUsageTest for ReplayTooling {
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()> {
        let client = ctx.client();
        let json_debugger = DiemDebugger::json_rpc(ctx.url())?;

        let treasury_account_address =
            AccountAddress::from_hex("0000000000000000000000000b1e55ed")?;

        let treasury_account = client
            .get_account(treasury_account_address)?
            .into_inner()
            .unwrap();
        let mut account1 = ctx.random_account();
        let account2 = ctx.random_account();
        ctx.create_parent_vasp_account(account1.authentication_key())?;
        ctx.create_parent_vasp_account(account2.authentication_key())?;
        ctx.fund(account1.address(), 100)?;
        ctx.fund(account2.address(), 100)?;
        let txn = account1.sign_with_transaction_builder(ctx.transaction_factory().peer_to_peer(
            Currency::XUS,
            account2.address(),
            3,
        ));
        client.submit(&txn)?;
        let txn = client
            .wait_for_signed_transaction(&txn, None, None)?
            .into_inner();

        let replay_result = json_debugger
            .execute_past_transactions(txn.version, 1, false)?
            .pop()
            .unwrap();

        let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../diem-move/transaction-replay/examples/account_exists.move")
            .canonicalize()?;

        let bisect_result = json_debugger
            .bisect_transactions_by_script(
                script_path.to_str().unwrap(),
                account1.address(),
                0,
                txn.version,
                None,
            )?
            .unwrap();

        let account_creation_txn = client
            .get_account_transaction(
                treasury_account_address,
                treasury_account.sequence_number,
                false,
            )?
            .into_inner()
            .unwrap();

        assert_eq!(account_creation_txn.version + 1, bisect_result);
        assert_eq!(replay_result.gas_used(), txn.gas_used);
        assert_eq!(VMStatusView::Executed, txn.vm_status);

        Ok(())
    }
}
