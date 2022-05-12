// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{ModuleBundle, TransactionPayload};
use forge::{AptosContext, AptosTest, Result, Test};
use move_deps::move_package;

pub struct ModulePublish;

impl Test for ModulePublish {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::module-publish"
    }
}

#[async_trait::async_trait]
impl AptosTest for ModulePublish {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let base_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/aptos/move_modules/");

        let build_config = move_package::BuildConfig {
            generate_docs: true,
            generate_abis: true,
            install_dir: Some(base_path.clone()),
            ..Default::default()
        };

        let compiled_package = build_config
            .clone()
            .compile_package(&base_path, &mut std::io::stdout())
            .unwrap();

        let mut blobs = vec![];
        compiled_package
            .compiled_modules()
            .iter_modules()
            .first()
            .unwrap()
            .serialize(&mut blobs)
            .unwrap();

        let txn_factory = ctx.aptos_transaction_factory();
        let publish_txn = ctx
            .root_account()
            .sign_with_transaction_builder(txn_factory.payload(TransactionPayload::ModuleBundle(
                ModuleBundle::singleton(blobs.clone()),
            )));
        ctx.client().submit_and_wait(&publish_txn).await?;
        let publish_txn = ctx
            .root_account()
            .sign_with_transaction_builder(txn_factory.payload(TransactionPayload::ModuleBundle(
                ModuleBundle::singleton(blobs),
            )));
        // republish should fail
        ctx.client()
            .submit_and_wait(&publish_txn)
            .await
            .unwrap_err();
        Ok(())
    }
}
