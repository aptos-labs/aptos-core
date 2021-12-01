// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
#![allow(unused_imports)]

use anyhow::Result;
use diem_sdk::{
    client::BlockingClient,
    move_types::{identifier::Identifier, language_storage::StructTag},
    transaction_builder::TransactionFactory,
    types::LocalAccount,
};
use diem_transaction_builder::experimental_stdlib as stdlib;
use diem_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{ScriptFunction, TransactionPayload},
};
use generate_key::load_key;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Demo for trove hackathon")]
pub struct DemoCli {
    #[structopt(long)]
    account_key_path: PathBuf,
    #[structopt(long)]
    account_address: String,
    #[structopt(long, default_value = "http://0.0.0.0:8080")]
    jsonrpc_endpoint: String,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    CreateBasicAccount {
        new_account_address: String,
        new_auth_key_prefix: String,
    },
    InitMultiToken {},
    RegisterBarsUser {},
    MintBarsNft {
        #[structopt(long)]
        creator_addr: String,
        #[structopt(long)]
        creator_name: String,
        #[structopt(long)]
        content_uri: String,
        #[structopt(long)]
        amount: u64,
    },
    TransferBarsNft {
        #[structopt(long)]
        to: String,
        #[structopt(long)]
        amount: u64,
        #[structopt(long)]
        creator: String,
        #[structopt(long)]
        creation_num: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: DemoCli = DemoCli::from_args();
    let account_key = load_key(args.account_key_path);
    let address = AccountAddress::from_hex_literal(&args.account_address).unwrap();

    let json_rpc_url = args.jsonrpc_endpoint;
    println!("Connecting to {}...", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);

    let seq_num = client
        .get_account(address)?
        .into_inner()
        .unwrap()
        .sequence_number;
    let mut account = LocalAccount::new(address, account_key, seq_num);

    match args.cmd {
        Command::CreateBasicAccount {
            new_account_address,
            new_auth_key_prefix,
        } => create_basic_account(
            &mut account,
            &client,
            new_account_address,
            new_auth_key_prefix,
        )?,
        Command::InitMultiToken { .. } => init_multi_token(&mut account, &client)?,
        Command::RegisterBarsUser { .. } => register_bars_user(&mut account, &client)?,
        Command::MintBarsNft {
            creator_addr,
            creator_name,
            content_uri,
            amount,
        } => mint_bars_nft(
            &mut account,
            &client,
            creator_addr,
            creator_name,
            content_uri,
            amount,
        )?,
        Command::TransferBarsNft {
            to,
            amount,
            creator,
            creation_num,
        } => transfer_bars_nft(&mut account, &client, to, amount, creator, creation_num)?,
    }

    Ok(())
}

fn create_basic_account(
    account: &mut LocalAccount,
    client: &BlockingClient,
    new_address: String,
    new_auth_key_prefix: String,
) -> Result<()> {
    let txn =
        account.sign_with_transaction_builder(TransactionFactory::new(ChainId::test()).payload(
            stdlib::encode_create_account_script_function(
                AccountAddress::from_hex_literal(&new_address).unwrap(),
                hex::decode(&new_auth_key_prefix).unwrap(),
            ),
        ));
    send(client, txn)?;
    println!("Success");
    Ok(())
}

fn init_multi_token(account: &mut LocalAccount, client: &BlockingClient) -> Result<()> {
    let txn = account.sign_with_transaction_builder(
        TransactionFactory::new(ChainId::test())
            .payload(stdlib::encode_nft_initialize_script_function()),
    );
    send(client, txn)?;
    println!("Success");
    Ok(())
}

fn register_bars_user(account: &mut LocalAccount, client: &BlockingClient) -> Result<()> {
    let txn = account.sign_with_transaction_builder(
        TransactionFactory::new(ChainId::test())
            .payload(stdlib::encode_register_bars_user_script_function()),
    );
    send(client, txn)?;
    println!("Success");
    Ok(())
}

fn mint_bars_nft(
    account: &mut LocalAccount,
    client: &BlockingClient,
    creator_addr: String,
    creator_name: String,
    content_uri: String,
    amount: u64,
) -> Result<()> {
    let txn = account.sign_with_transaction_builder(
        TransactionFactory::new(ChainId::test()).payload(stdlib::encode_mint_bars_script_function(
            AccountAddress::from_hex_literal(&creator_addr).unwrap(),
            creator_name.as_bytes().to_vec(),
            content_uri.as_bytes().to_vec(),
            amount,
        )),
    );
    send(client, txn)?;
    println!("Success");
    Ok(())
}

fn transfer_bars_nft(
    account: &mut LocalAccount,
    client: &BlockingClient,
    to: String,
    amount: u64,
    creator: String,
    creation_num: u64,
) -> Result<()> {
    let token = TypeTag::Struct(StructTag {
        address: AccountAddress::from_hex_literal("0x1").unwrap(),
        module: Identifier::new("BARSToken").unwrap(),
        name: Identifier::new("BARSToken").unwrap(),
        type_params: Vec::new(),
    });
    let txn =
        account.sign_with_transaction_builder(TransactionFactory::new(ChainId::test()).payload(
            stdlib::encode_transfer_token_between_galleries_script_function(
                token,
                AccountAddress::from_hex_literal(&to).unwrap(),
                amount,
                AccountAddress::from_hex_literal(&creator).unwrap(),
                creation_num,
            ),
        ));
    send(client, txn)?;
    println!("Success");
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
