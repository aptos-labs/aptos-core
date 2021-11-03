// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
#![allow(unused_imports)]
use anyhow::{Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::{Currency, TransactionFactory},
    types::LocalAccount,
};
use diem_types::{
    account_address::AccountAddress,
    account_config,
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, Script, ScriptFunction, TransactionArgument,
        TransactionPayload, VecBytes,
    },
};
use generate_key::load_key;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};
use shuffle_transaction_builder::framework::{
    encode_create_parent_vasp_account_script_function, encode_mint_coin_script_function,
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Helper rust app to invoke script functions")]
pub struct PublicScriptHelper {
    #[structopt(long = "account-key-path")]
    account_key_path: PathBuf,
    #[structopt(long = "account-address")]
    account_address: String,
}

fn main() -> Result<()> {
    let args = PublicScriptHelper::from_args();
    let account_key = load_key(args.account_key_path);
    let address = AccountAddress::from_hex_literal(&args.account_address).unwrap();

    let json_rpc_url = "http://0.0.0.0:8080".to_string();
    println!("Connecting to {}...", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);

    let seq_num = client
        .get_account(address)?
        .into_inner()
        .unwrap()
        .sequence_number;
    let mut account = LocalAccount::new(address, account_key, seq_num);

    // Create a new account.
    println!("Running script function");
    let create_new_account_txn =
        account.sign_with_transaction_builder(TransactionFactory::new(ChainId::test()).payload(
            // See examples in this file for script function construction using various ty_args and args
            // diem-move/diem-framework/DPN/releases/artifacts/current/transaction_script_builder.rs
            // Example for constructing TypeTag for ty_args
            // let token = TypeTag::Struct(StructTag {
            //     address: AccountAddress::from_hex_literal("0x1").unwrap(),
            //     module: Identifier("XDX".into()),
            //     name: Identifier("XDX".into()),
            //     type_params: Vec::new(),
            // });
            TransactionPayload::ScriptFunction(ScriptFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal("0x1").unwrap(),
                    ident_str!("DiemTransactionPublishingOption").to_owned(),
                ),
                ident_str!("set_module_publish_pre_approval").to_owned(),
                vec![],
                vec![bcs::to_bytes(&false).unwrap()],
            )),
        ));
    send(&client, create_new_account_txn)?;
    println!("Success!");

    Ok(())
}

/// Send a transaction to the blockchain through the blocking client.
fn send(client: &BlockingClient, tx: diem_types::transaction::SignedTransaction) -> Result<()> {
    use diem_json_rpc_types::views::VMStatusView;

    client.submit(&tx)?;
    assert_eq!(
        client
            .wait_for_signed_transaction(&tx, Some(std::time::Duration::from_secs(60)), None)?
            .into_inner()
            .vm_status,
        VMStatusView::Executed,
    );
    Ok(())
}
