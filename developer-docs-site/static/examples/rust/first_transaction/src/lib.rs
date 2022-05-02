// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{ExpandedSecretKey, PublicKey, SecretKey};
use hex::ToHex;
use rand::{rngs::OsRng, Rng, SeedableRng};
use reqwest;
use tiny_keccak::{Hasher, Sha3};

pub const TESTNET_URL: &str = "https://fullnode.devnet.aptoslabs.com";
pub const FAUCET_URL: &str = "https://faucet.devnet.aptoslabs.com";

//:!:>section_1
pub struct Account {
    signing_key: SecretKey,
}

impl Account {
    /// Represents an account as well as the private, public key-pair for the Aptos blockchain.
    pub fn new(priv_key_bytes: Option<Vec<u8>>) -> Self {
        let signing_key = match priv_key_bytes {
            Some(key) => SecretKey::from_bytes(&key).unwrap(),
            None => SecretKey::generate(&mut rand::rngs::StdRng::from_seed(OsRng.gen())),
        };

        Account { signing_key }
    }
    /// Returns the address associated with the given account
    pub fn address(&self) -> String {
        self.auth_key()
    }

    /// Returns the auth_key for the associated account
    pub fn auth_key(&self) -> String {
        let mut sha3 = Sha3::v256();
        sha3.update(PublicKey::from(&self.signing_key).as_bytes());
        sha3.update(&vec![0u8]);

        let mut output = [0u8; 32];
        sha3.finalize(&mut output);
        hex::encode(output)
    }

    /// Returns the public key for the associated account
    pub fn pub_key(&self) -> String {
        hex::encode(PublicKey::from(&self.signing_key).as_bytes())
    }
}
//<:!:section_1

//:!:>section_2
#[derive(Clone)]
pub struct RestClient {
    url: String,
}

impl RestClient {
    /// A wrapper around the Aptos-core Rest API
    pub fn new(url: String) -> Self {
        Self { url }
    }
    //<:!:section_2
    //:!:>section_3
    /// Returns the sequence number and authentication key for an account
    pub fn account(&self, account_address: &str) -> serde_json::Value {
        let res =
            reqwest::blocking::get(format!("{}/accounts/{}", self.url, account_address)).unwrap();

        if res.status() != 200 {
            assert_eq!(
                res.status(),
                200,
                "{} - {}",
                res.text().unwrap_or("".to_string()),
                account_address,
            );
        }

        res.json().unwrap()
    }

    /// Returns all resources associated with the account
    pub fn account_resource(
        &self,
        account_address: &str,
        resource_type: &str,
    ) -> Option<serde_json::Value> {
        let res = reqwest::blocking::get(format!(
            "{}/accounts/{}/resource/{}",
            self.url, account_address, resource_type,
        ))
        .unwrap();

        if res.status() == 404 {
            None
        } else if res.status() != 200 {
            assert_eq!(
                res.status(),
                200,
                "{} - {}",
                res.text().unwrap_or("".to_string()),
                account_address,
            );
            unreachable!()
        } else {
            Some(res.json().unwrap())
        }
    }
    //<:!:section_3
    //:!:>section_4
    /// Generates a transaction request that can be submitted to produce a raw transaction that can be signed, which upon being signed can be submitted to the blockchain.
    pub fn generate_transaction(
        &self,
        sender: &str,
        payload: serde_json::Value,
    ) -> serde_json::Value {
        let account_res = self.account(sender);

        let seq_num = account_res
            .get("sequence_number")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        // Unix timestamp, in seconds + 10 minutes
        let expiration_time_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + 600;

        serde_json::json!({
            "sender": format!("0x{}", sender),
            "sequence_number": seq_num.to_string(),
            "max_gas_amount": "1000",
            "gas_unit_price": "1",
            "gas_currency_code": "XUS",
            "expiration_timestamp_secs": expiration_time_secs.to_string(),
            "payload": payload,
        })
    }

    /// Converts a transaction request produced by `generate_transaction` into a properly signed transaction, which can then be submitted to the blockchain.
    pub fn sign_transaction(
        &self,
        account_from: &mut Account,
        mut txn_request: serde_json::Value,
    ) -> serde_json::Value {
        let res = reqwest::blocking::Client::new()
            .post(format!("{}/transactions/signing_message", self.url))
            .body(txn_request.to_string())
            .send()
            .unwrap();

        if res.status() != 200 {
            assert_eq!(
                res.status(),
                200,
                "{} - {}",
                res.text().unwrap_or("".to_string()),
                txn_request.as_str().unwrap_or(""),
            );
        }
        let body: serde_json::Value = res.json().unwrap();
        let to_sign_hex = Box::new(body.get("message").unwrap().as_str()).unwrap();
        let to_sign = hex::decode(&to_sign_hex[2..]).unwrap();
        let signature: String = ExpandedSecretKey::from(&account_from.signing_key)
            .sign(&to_sign, &PublicKey::from(&account_from.signing_key))
            .encode_hex();

        let signature_payload = serde_json::json!({
            "type": "ed25519_signature",
            "public_key": format!("0x{}", account_from.pub_key()),
            "signature": format!("0x{}", signature),
        });
        txn_request
            .as_object_mut()
            .unwrap()
            .insert("signature".to_string(), signature_payload);
        txn_request
    }

    /// Submits a signed transaction to the blockchain.
    pub fn submit_transaction(&self, txn_request: &serde_json::Value) -> serde_json::Value {
        let res = reqwest::blocking::Client::new()
            .post(format!("{}/transactions", self.url))
            .body(txn_request.to_string())
            .header("Content-Type", "application/json")
            .send()
            .unwrap();

        if res.status() != 202 {
            assert_eq!(
                res.status(),
                202,
                "{} - {}",
                res.text().unwrap_or("".to_string()),
                txn_request.as_str().unwrap_or(""),
            );
        }
        res.json().unwrap()
    }

    pub fn transaction_pending(&self, transaction_hash: &str) -> bool {
        let res = reqwest::blocking::get(format!("{}/transactions/{}", self.url, transaction_hash))
            .unwrap();

        if res.status() == 404 {
            return true;
        }

        if res.status() != 200 {
            assert_eq!(
                res.status(),
                200,
                "{} - {}",
                res.text().unwrap_or("".to_string()),
                transaction_hash,
            );
        }

        res.json::<serde_json::Value>()
            .unwrap()
            .get("type")
            .unwrap()
            .as_str()
            .unwrap()
            == "pending_transaction"
    }

    /// Waits up to 10 seconds for a transaction to move past pending state.
    pub fn wait_for_transaction(&self, txn_hash: &str) {
        let mut count = 0;
        while self.transaction_pending(txn_hash) {
            assert!(count < 10, "transaction {} timed out", txn_hash);
            thread::sleep(Duration::from_secs(1));
            count += 1;
        }
    }
    //<:!:section_4
    //:!:>section_5
    /// Returns the test coin balance associated with the account
    pub fn account_balance(&self, account_address: &str) -> Option<u64> {
        self.account_resource(account_address, "0x1::TestCoin::Balance")
            .unwrap()["data"]["coin"]["value"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
    }

    /// Transfer a given coin amount from a given Account to the recipient's account address.
    /// Returns the sequence number of the transaction used to transfer
    pub fn transfer(&self, account_from: &mut Account, recipient: &str, amount: u64) -> String {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x1::TestCoin::transfer",
            "type_arguments": [],
            "arguments": [format!("0x{}", recipient), amount.to_string()]
        });
        let txn_request = self.generate_transaction(&account_from.address(), payload);
        let signed_txn = self.sign_transaction(account_from, txn_request);
        let res = self.submit_transaction(&signed_txn);

        res.get("hash").unwrap().as_str().unwrap().to_string()
    }
}
//<:!:section_5

//:!:>section_6
pub struct FaucetClient {
    url: String,
    rest_client: RestClient,
}

impl FaucetClient {
    /// Faucet creates and funds accounts. This is a thin wrapper around that.
    pub fn new(url: String, rest_client: RestClient) -> Self {
        Self { url, rest_client }
    }

    /// This creates an account if it does not exist and mints the specified amount of coins into that account.
    pub fn fund_account(&self, auth_key: &str, amount: u64) {
        let res = reqwest::blocking::Client::new()
            .post(format!(
                "{}/mint?amount={}&auth_key={}",
                self.url, amount, auth_key
            ))
            .send()
            .unwrap();

        if res.status() != 200 {
            assert_eq!(
                res.status(),
                200,
                "{}",
                res.text().unwrap_or("".to_string()),
            );
        }
        for txn_hash in res.json::<serde_json::Value>().unwrap().as_array().unwrap() {
            self.rest_client
                .wait_for_transaction(txn_hash.as_str().unwrap())
        }
    }
}
//<:!:section_6
