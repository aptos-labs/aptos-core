// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::aptos::move_test_helpers;
use aptos_rest_client::Resource;
use aptos_types::transaction::{ScriptFunction, TransactionPayload};
use forge::{AptosContext, AptosTest, Result, Test};
use move_deps::move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId,
};

pub struct StringArgs;

impl Test for StringArgs {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::string-args"
    }
}

#[async_trait::async_trait]
impl AptosTest for StringArgs {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let base_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/aptos/move_modules/");
        let txn_factory = move_test_helpers::publish_code(ctx, base_path).await?;

        // After publish, expect state == "init"
        assert_eq!(
            &format!("{:?}", get_resource(ctx).await?.data.get("state").unwrap()),
            "String(\"init\")"
        );
        let txn = ctx
            .root_account()
            .sign_with_transaction_builder(txn_factory.payload(encode_hello_world_hi("hi there")));
        ctx.client().submit_and_wait(&txn).await?;
        let rsc = get_resource(ctx).await?;
        //assert_eq!(rsc.data.get("state").unwrap().as_str().unwrap(), "hi there");
        assert_eq!(
            &format!("{:?}", rsc.data.get("state").unwrap()),
            "String(\"hi there\")"
        );
        Ok(())
    }
}

async fn get_resource(ctx: &mut AptosContext<'_>) -> Result<Resource> {
    let resp = ctx
        .client()
        .get_account_resource(
            ctx.root_account().address(),
            "0xA550C18::HelloWorld::ModuleData",
        )
        .await?;
    Ok(resp.into_inner().unwrap())
}

/// Transaction builder generator currently does not support struct types
fn encode_hello_world_hi(msg: &str) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::from_hex_literal("0xA550C18").unwrap(),
            ident_str!("HelloWorld").to_owned(),
        ),
        ident_str!("hi").to_owned(),
        vec![],
        vec![bcs::to_bytes(msg.as_bytes()).unwrap()],
    ))
}
