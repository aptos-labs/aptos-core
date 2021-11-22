// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Result};
use diem_api_types::mime_types;
use diem_sdk::client::AccountAddress;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::{
    io,
    io::Write,
    thread, time,
    time::{Duration, Instant},
};
use url::Url;

const DIEM_ACCOUNT_TYPE: &str = "0x1::DiemAccount::DiemAccount";

pub struct DevApiClient {
    client: Client,
    url: Url,
}

// Client that will make GET and POST requests based off of Dev API
impl DevApiClient {
    pub fn new(client: Client, url: Url) -> Result<Self> {
        Ok(Self { client, url })
    }

    pub async fn get_transactions_by_hash(&self, hash: &str) -> Result<Value> {
        let path = self.url.join(format!("transactions/{}", hash).as_str())?;

        DevApiClient::check_response(
            self.client.get(path.as_str()).send().await?,
            "GET /transactions failed",
        )
        .await
    }

    pub async fn post_transactions(&self, txn_bytes: Vec<u8>) -> Result<Value> {
        let path = self.url.join("transactions")?;

        DevApiClient::check_response(
            self.client
                .post(path.as_str())
                .header("Content-Type", mime_types::BCS_SIGNED_TRANSACTION)
                .body(txn_bytes)
                .send()
                .await?,
            "POST /transactions failed",
        )
        .await
    }

    pub async fn get_account_resources(&self, address: AccountAddress) -> Result<Value> {
        let path = self
            .url
            .join(format!("accounts/{}/resources", address.to_hex_literal()).as_str())?;

        DevApiClient::check_response(
            self.client.get(path.as_str()).send().await?,
            "Failed to get account resources with provided address",
        )
        .await
    }

    pub async fn get_account_transactions_response(
        &self,
        address: AccountAddress,
        start: u64,
        limit: u64,
    ) -> Result<Value> {
        let path = self
            .url
            .join(format!("accounts/{}/transactions", address).as_str())?;

        DevApiClient::check_response(
            self.client
                .get(path.as_str())
                .query(&[("start", start.to_string().as_str())])
                .query(&[("limit", limit.to_string().as_str())])
                .send()
                .await?,
            "Failed to get account transactions with provided address",
        )
        .await
    }

    async fn check_response(resp: Response, failure_message: &str) -> Result<Value> {
        let status = resp.status();
        let json = resp.json().await?;
        DevApiClient::check_response_status_code(
            &status,
            DevApiClient::response_context(failure_message, &json)?.as_str(),
        )?;
        Ok(json)
    }

    fn check_response_status_code(status: &StatusCode, context: &str) -> Result<()> {
        match status >= &StatusCode::from_u16(200)? && status < &StatusCode::from_u16(300)? {
            true => Ok(()),
            false => Err(anyhow!(context.to_string())),
        }
    }

    fn response_context(message: &str, json: &Value) -> Result<String> {
        Ok(format!(
            "{}. Here is the json block for the response that failed:\n{:?}",
            message, json
        ))
    }

    pub async fn get_account_sequence_number(&self, address: AccountAddress) -> Result<u64> {
        let account_resources_json = self.get_account_resources(address).await?;
        DevApiClient::parse_json_for_account_seq_num(account_resources_json)
    }

    fn parse_json_for_account_seq_num(json_objects: Value) -> Result<u64> {
        let json_arr = json_objects
            .as_array()
            .ok_or_else(|| anyhow!("Couldn't convert to array"))?
            .to_vec();
        let mut seq_number_string = "";
        for object in &json_arr {
            if object["type"] == DIEM_ACCOUNT_TYPE {
                seq_number_string = object["data"]["sequence_number"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Invalid sequence number string"))?;
                break;
            };
        }
        let seq_number: u64 = seq_number_string.parse()?;
        Ok(seq_number)
    }

    pub async fn check_txn_executed_from_hash(&self, hash: &str) -> Result<()> {
        let mut json = self.get_transactions_by_hash(hash).await?;
        let start = Instant::now();
        while json["type"] == "pending_transaction" {
            thread::sleep(time::Duration::from_secs(1));
            json = self.get_transactions_by_hash(hash).await?;
            let duration = start.elapsed();
            if duration > Duration::from_secs(15) {
                break;
            }
        }
        DevApiClient::confirm_successful_execution(&mut io::stdout(), &json, hash)
    }

    fn confirm_successful_execution<W>(writer: &mut W, json: &Value, hash: &str) -> Result<()>
    where
        W: Write,
    {
        if DevApiClient::is_execution_successful(json)? {
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

    pub fn get_hash_from_post_txn(json: Value) -> Result<String> {
        Ok(json["hash"].as_str().unwrap().to_string())
    }
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
        assert_eq!(
            DevApiClient::is_execution_successful(&successful_txn).unwrap(),
            true
        );

        let failed_txn = get_transactions_by_hash_json_output_fail();
        assert_eq!(
            DevApiClient::is_execution_successful(&failed_txn).unwrap(),
            false
        );
    }

    #[test]
    fn test_get_hash_from_post_txn() {
        let txn = post_txn_json_output();
        let hash = DevApiClient::get_hash_from_post_txn(txn).unwrap();
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

        DevApiClient::confirm_successful_execution(&mut stdout, &successful_txn, good_hash)
            .unwrap();
        assert_eq!(String::from_utf8(stdout).unwrap().as_str(), "".to_string());

        let failed_txn = get_transactions_by_hash_json_output_fail();
        let mut stdout = Vec::new();
        let bad_hash = "0xbad59bb50521334a69c06a315b6dd191a8da4c1c7a40ce27a8f96f12959496eb";
        assert_eq!(
            DevApiClient::confirm_successful_execution(&mut stdout, &failed_txn, bad_hash).is_err(),
            true
        );

        let fail_string = format!("{:#?}\n", &failed_txn);
        assert_eq!(String::from_utf8(stdout).unwrap().as_str(), fail_string)
    }

    #[test]
    fn test_parse_json_for_seq_num() {
        let value_obj = json!([{
            "type":"0x1::DiemAccount::DiemAccount",
            "data": {
                "authentication_key": "0x88cae30f0fea7879708788df9e7c9b7524163afcc6e33b0a9473852e18327fa9",
                "key_rotation_capability":{
                    "vec":[{"account_address":"0x24163afcc6e33b0a9473852e18327fa9"}]
                },
                "received_events":{
                    "counter":"0",
                    "guid":{}
                },
                "sent_events":{},
                "sequence_number":"3",
                "withdraw_capability":{
                    "vec":[{"account_address":"0x24163afcc6e33b0a9473852e18327fa9"}]
                }
            }
        }]);

        let ret_seq_num = DevApiClient::parse_json_for_account_seq_num(value_obj).unwrap();
        assert_eq!(ret_seq_num, 3);
    }

    #[test]
    fn test_check_response_status_code() {
        assert_eq!(
            DevApiClient::check_response_status_code(
                &StatusCode::from_u16(200).unwrap(),
                "Success"
            )
            .is_err(),
            false
        );
        assert_eq!(
            DevApiClient::check_response_status_code(&StatusCode::from_u16(404).unwrap(), "Failed")
                .is_err(),
            true
        );
    }

    #[test]
    fn test_response_context() {
        let failed_obj = json!({
            "code": 404,
            "message": "account not found by address(0x132412341234124) and ledger version(81)",
            "diem_ledger_version": "81"
        });
        let context = DevApiClient::response_context(
            "Failed to get account resources with provided address",
            &failed_obj,
        )
        .unwrap();

        let correct_string = format!("Failed to get account resources with provided address. Here is the json block for the response that failed:\n{:?}", failed_obj);
        assert_eq!(context, correct_string);
    }
}
