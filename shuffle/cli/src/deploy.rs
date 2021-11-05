// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{build_move_package, DevApiClient, Home, MAIN_PKG_PATH};
use anyhow::{anyhow, Result};
use diem_crypto::PrivateKey;
use diem_sdk::{
    transaction_builder::TransactionFactory,
    types::{
        transaction::{ModuleBundle, TransactionPayload},
        LocalAccount,
    },
};
use diem_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use generate_key::load_key;
use serde_json::Value;
use std::{
    io,
    io::Write,
    path::Path,
    thread, time,
    time::{Duration, Instant},
};
use url::Url;

/// Deploys shuffle's main Move Package to the sender's address.
pub async fn handle(home: &Home, project_path: &Path, network: Url) -> Result<()> {
    let client = DevApiClient::new(reqwest::Client::new(), network)?;
    if !home.get_latest_key_path().exists() {
        return Err(anyhow!(
            "An account hasn't been created yet! Run shuffle account first."
        ));
    }
    let new_account_key = load_key(home.get_latest_key_path());
    println!("Using Public Key {}", &new_account_key.public_key());
    let derived_address =
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address();
    println!(
        "Sending txn from address {}",
        derived_address.to_hex_literal()
    );

    let account_seq_number = client.get_account_sequence_number(derived_address).await?;
    let mut new_account = LocalAccount::new(derived_address, new_account_key, account_seq_number);

    let compiled_package = build_move_package(project_path.join(MAIN_PKG_PATH).as_ref())?;
    for module in compiled_package
        .transitive_compiled_modules()
        .compute_dependency_graph()
        .compute_topological_order()?
    {
        let module_id = module.self_id();
        if module_id.address() != &new_account.address() {
            println!("Skipping Module: {}", module_id);
            continue;
        }
        println!("Deploying Module: {}", module_id);
        let mut binary = vec![];
        module.serialize(&mut binary)?;

        let hash = send_module_transaction(&client, &mut new_account, binary).await?;
        check_txn_executed_from_hash(&client, hash.as_str()).await?;
    }

    Ok(())
}

async fn send_module_transaction(
    client: &DevApiClient,
    account: &mut LocalAccount,
    module_binary: Vec<u8>,
) -> Result<String> {
    let factory = TransactionFactory::new(ChainId::test());
    let publish_txn = account.sign_with_transaction_builder(factory.payload(
        TransactionPayload::ModuleBundle(ModuleBundle::singleton(module_binary)),
    ));
    let bytes = bcs::to_bytes(&publish_txn)?;
    let resp = client.post_transactions(bytes).await?;
    let json: serde_json::Value = serde_json::from_str(resp.text().await?.as_str())?;
    let hash = get_hash_from_post_txn(json)?;
    Ok(hash)
}

async fn check_txn_executed_from_hash(client: &DevApiClient, hash: &str) -> Result<()> {
    let mut resp = client.get_transactions_by_hash(hash).await?;
    let mut json: serde_json::Value = serde_json::from_str(resp.text().await?.as_str())?;
    let start = Instant::now();
    while json["type"] == "pending_transaction" {
        thread::sleep(time::Duration::from_secs(1));
        resp = client.get_transactions_by_hash(hash).await?;
        json = serde_json::from_str(resp.text().await?.as_str())?;
        let duration = start.elapsed();
        if duration > Duration::from_secs(10) {
            break;
        }
    }
    confirm_successful_execution(&mut io::stdout(), &json, hash)
}

fn confirm_successful_execution<W>(writer: &mut W, json: &Value, hash: &str) -> Result<()>
where
    W: Write,
{
    if is_execution_successful(json)? {
        return Ok(());
    }
    writeln!(writer, "{:#?}", json)?;
    Err(anyhow!(format!(
        "Transaction with hash {} didn't execute successfully",
        hash
    )))
}

fn is_execution_successful(json: &Value) -> Result<bool> {
    json["success"]
        .as_bool()
        .ok_or_else(|| anyhow!("Unable to access success key"))
}

fn get_hash_from_post_txn(json: Value) -> Result<String> {
    Ok(json["hash"].as_str().unwrap().to_string())
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    fn post_txn_json_output() -> Value {
        json!({
        "type":"pending_transaction",
        "hash":"0xbca2738726dc456f23762372ab0dd2f450ec3ec20271e5318ae37e9d42ee2bb8",
        "sender":"0x24163afcc6e33b0a9473852e18327fa9",
        "sequence_number":"10",
        "max_gas_amount":"1000000",
        "gas_unit_price":"0",
        "gas_currency_code":"XUS",
        "expiration_timestamp_secs":"1635872777",
        "payload":{}
        })
    }

    fn get_transactions_by_hash_json_output_success() -> Value {
        json!({
            "type":"user_transaction",
            "version":"3997",
            "hash":"0x89e59bb50521334a69c06a315b6dd191a8da4c1c7a40ce27a8f96f12959496eb",
            "state_root_hash":"0x7a0b81379ab8786f34fcff804e5fb413255467c28f09672e8d22bfaa4e029102",
            "event_root_hash":"0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
            "gas_used":"8",
            "success":true,
            "vm_status":"Executed successfully",
            "sender":"0x24163afcc6e33b0a9473852e18327fa9",
            "sequence_number":"14",
            "max_gas_amount":"1000000",
            "gas_unit_price":"0",
            "gas_currency_code":"XUS",
            "expiration_timestamp_secs":"1635873470",
            "payload":{}
        })
    }

    fn get_transactions_by_hash_json_output_fail() -> Value {
        json!({
            "type":"user_transaction",
            "version":"3997",
            "hash":"0xbad59bb50521334a69c06a315b6dd191a8da4c1c7a40ce27a8f96f12959496eb",
            "state_root_hash":"0x7a0b81379ab8786f34fcff804e5fb413255467c28f09672e8d22bfaa4e029102",
            "event_root_hash":"0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
            "gas_used":"8",
            "success":false,
            "vm_status":"miscellaneous error",
            "sender":"0x24163afcc6e33b0a9473852e18327fa9",
            "sequence_number":"14",
            "max_gas_amount":"1000000",
            "gas_unit_price":"0",
            "gas_currency_code":"XUS",
            "expiration_timestamp_secs":"1635873470",
            "payload":{}
        })
    }

    #[test]
    fn test_confirm_is_execution_successful() {
        let successful_txn = get_transactions_by_hash_json_output_success();
        assert_eq!(is_execution_successful(&successful_txn).unwrap(), true);

        let failed_txn = get_transactions_by_hash_json_output_fail();
        assert_eq!(is_execution_successful(&failed_txn).unwrap(), false);
    }

    #[test]
    fn test_get_hash_from_post_txn() {
        let txn = post_txn_json_output();
        let hash = get_hash_from_post_txn(txn).unwrap();
        assert_eq!(
            hash,
            "0xbca2738726dc456f23762372ab0dd2f450ec3ec20271e5318ae37e9d42ee2bb8"
        );
    }

    #[test]
    fn test_print_confirmation_with_success_value() {
        let successful_txn = get_transactions_by_hash_json_output_success();
        let mut stdout = Vec::new();
        let good_hash = "0xbca2738726dc456f23762372ab0dd2f450ec3ec20271e5318ae37e9d42ee2bb8";

        confirm_successful_execution(&mut stdout, &successful_txn, good_hash).unwrap();
        assert_eq!(String::from_utf8(stdout).unwrap().as_str(), "".to_string());

        let failed_txn = get_transactions_by_hash_json_output_fail();
        let mut stdout = Vec::new();
        let bad_hash = "0xbad59bb50521334a69c06a315b6dd191a8da4c1c7a40ce27a8f96f12959496eb";
        assert_eq!(
            confirm_successful_execution(&mut stdout, &failed_txn, bad_hash).is_err(),
            true
        );

        let fail_string = format!("{:#?}\n", &failed_txn);
        assert_eq!(String::from_utf8(stdout).unwrap().as_str(), fail_string)
    }
}
