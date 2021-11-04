// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use crate::shared::DevApiClient;
use anyhow::{anyhow, Result};
use diem_types::account_address::AccountAddress;
use serde_json::Value;
use std::{cmp::max, io, io::Write, thread, time};
use url::Url;

// Will list the last 10 transactions and has the ability to block and stream future transactions.
pub async fn handle(network: Url, tail: bool, address: AccountAddress, raw: bool) -> Result<()> {
    let client = DevApiClient::new(reqwest::Client::new(), network)?;
    let account_seq_num = client.get_account_sequence_number(address).await?;
    let mut prev_seq_num = max(account_seq_num as i64 - 10, 0);
    let resp = client
        .get_account_transactions_response(address, prev_seq_num as u64, 10)
        .await?;
    let json_with_txns: serde_json::Value = serde_json::from_str(resp.text().await?.as_str())?;

    let all_transactions = json_with_txns
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Failed to get transactions"))?;

    write_out_txns(all_transactions.to_vec(), &mut io::stdout(), raw)?;

    if !all_transactions.is_empty() {
        prev_seq_num = parse_txn_for_seq_num(
            all_transactions
                .last()
                .ok_or_else(|| anyhow!("Couldn't get last transaction"))?,
        )?
    } else {
        // Setting to -1 to handle the case where sequence number is 0 and
        // we don't have a previous sequence number
        prev_seq_num = -1;
    }

    if tail {
        // listening for incoming transactions
        loop {
            thread::sleep(time::Duration::from_millis(1000));
            let resp = client
                .get_account_transactions_response(address, (prev_seq_num + 1) as u64, 100)
                .await?;
            let json_with_txns: serde_json::Value =
                serde_json::from_str(resp.text().await?.as_str())?;
            let txn_array = json_with_txns
                .as_array()
                .ok_or_else(|| anyhow!("Couldn't convert to array"))?
                .to_vec();

            // checking if there are transactions
            if txn_array.is_empty() {
                continue;
            }
            let last_txn_seq_num = parse_txn_for_seq_num(
                txn_array
                    .last()
                    .ok_or_else(|| anyhow!("Couldn't get last transaction"))?,
            )?;
            if last_txn_seq_num > prev_seq_num {
                write_out_txns(txn_array, &mut io::stdout(), raw)?;
            }
            prev_seq_num = last_txn_seq_num;
        }
    }
    Ok(())
}

fn write_out_txns<W: Write>(all_transactions: Vec<Value>, mut stdout: W, raw: bool) -> Result<()> {
    for txn in all_transactions.iter() {
        write_into(&mut stdout, txn, raw)?;
    }

    Ok(())
}

fn parse_txn_for_seq_num(last_txn: &Value) -> Result<i64> {
    Ok(last_txn["sequence_number"]
        .to_string()
        .replace('"', "")
        .parse::<i64>()?)
}

fn write_into<W>(writer: &mut W, json: &serde_json::Value, raw: bool) -> io::Result<()>
where
    W: Write,
{
    match raw {
        true => writeln!(writer, "{}", json),
        false => writeln!(writer, "{:#}", json),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::{json, Value};

    fn get_sample_txn() -> Value {
        json!([{
            "type":"user_transaction",
            "version":"268",
            "hash":"0x8be63c23e88f9d0290f060e33fe09e8a755e45f41ee0fc9447f3b5d97a8b88d1",
            "state_root_hash":"0x8d903f9df4092946f036164fa925c6716a172ee0140f2404371393e517b7058e",
            "event_root_hash":"0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
            "gas_used":"8",
            "success":true,
            "vm_status":"Executed successfully",
            "sender":"0x24163afcc6e33b0a9473852e18327fa9",
            "sequence_number":"2",
            "max_gas_amount":"1000000",
            "gas_unit_price":"0",
            "gas_currency_code":"XUS",
            "expiration_timestamp_secs":"1635800460",
            "payload":{}
        },{
            "type":"user_transaction",
            "version":"270",
            "hash":"0x42343251",
            "state_root_hash":"0x3434235",
            "event_root_hash":"0x3434235",
            "gas_used":"5",
            "success":true,
            "vm_status":"Executed successfully",
            "sender":"0x24163afcc6e33b0a9473852e18327fa9",
            "sequence_number":"3",
            "max_gas_amount":"1000000",
            "gas_unit_price":"0",
            "gas_currency_code":"XUS",
            "expiration_timestamp_secs":"1635800460",
            "payload":{}
        }])
    }

    #[test]
    fn test_write_into() {
        let mut stdout = Vec::new();
        let txn = get_sample_txn();
        write_into(&mut stdout, &txn[0], false).unwrap();
        assert_eq!(
            format!("{:#}\n", txn[0]),
            String::from_utf8(stdout).unwrap().as_str()
        );

        stdout = Vec::new();
        write_into(&mut stdout, &txn[0], true).unwrap();
        assert_eq!(
            format!("{}\n", txn[0]),
            String::from_utf8(stdout).unwrap().as_str()
        );
    }

    #[test]
    fn test_write_out_txns_stdout() {
        let all_txns = get_sample_txn();
        let txn_array = all_txns.as_array().unwrap();
        let mut stdout = Vec::new();
        write_out_txns(txn_array.to_vec(), &mut stdout, false).unwrap();
        assert_eq!(
            String::from_utf8(stdout).unwrap().as_str(),
            format!("{:#}\n{:#}\n", txn_array[0], txn_array[1])
        );

        stdout = Vec::new();
        write_out_txns(txn_array.to_vec(), &mut stdout, true).unwrap();
        assert_eq!(
            String::from_utf8(stdout).unwrap().as_str(),
            format!("{}\n{}\n", txn_array[0], txn_array[1])
        )
    }

    #[test]
    fn test_parse_seq_num() {
        let txn = get_sample_txn();
        let seq_num = parse_txn_for_seq_num(&txn[0]).unwrap();
        assert_eq!(seq_num, 2);
    }
}
