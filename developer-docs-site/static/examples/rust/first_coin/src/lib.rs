// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use first_transaction::{Account, RestClient};

pub struct FirstCoinClient {
    pub rest_client: RestClient,
}

impl FirstCoinClient {
    /// Represents an account as well as the private, public key-pair for the Aptos blockchain.
    pub fn new(url: String) -> Self {
        Self {
            rest_client: RestClient::new(url),
        }
    }

    //:!:>section_1
    /// Initializes the new coin.
    pub fn initialize_coin(&self, account_from: &mut Account) -> String {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x1::ManagedCoin::initialize",
            "type_arguments": [format!("0x{}::MoonCoin::MoonCoin", account_from.address())],
            "arguments": [
                hex::encode("Moon Coin".as_bytes()),
                hex::encode("MOON".as_bytes()),
                "6",
                false,
            ]
        });
        self.rest_client.execution_transaction_with_payload(account_from, payload)
    }
    //<:!:section_1

    //:!:>section_2
    /// Receiver needs to register the coin before they can receive it.
    pub fn register_coin(
        &self,
        account_receiver: &mut Account,
        coin_type_address: &str,
    ) -> String {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x1::Coin::register",
            "type_arguments": [format!("0x{}::MoonCoin::MoonCoin", coin_type_address)],
            "arguments": []
        });
        self.rest_client.execution_transaction_with_payload(account_receiver, payload)
    }
    //<:!:section_2

    //:!:>section_3
    /// Receiver needs to register the coin before they can receive it.
    pub fn mint_coin(
        &self,
        account_owner: &mut Account,
        receiver_address: &str,
        amount: u64,
    ) -> String {
        let payload = serde_json::json!({
            "type": "script_function_payload",
            "function": "0x1::ManagedCoin::mint",
            "type_arguments": [format!("0x{}::MoonCoin::MoonCoin", account_owner.address())],
            "arguments": [
                receiver_address,
                amount.to_string(),
            ]
        });
        self.rest_client.execution_transaction_with_payload(account_owner, payload)
    }
    //<:!:section_3

    //:!:>section_4
    /// Receiver needs to register the coin before they can receive it.
    pub fn get_balance(
        &self,
        account_address: &str,
        coin_type_address: &str,
    ) -> u64 {
        let module_type = format!(
            "0x1::Coin::CoinStore<0x{}::MoonCoin::MoonCoin>",
            coin_type_address,
        );
        self.rest_client
            .account_resource(account_address, &module_type)
            .map(|value| value["data"]["coin"]["value"].as_str().unwrap().to_string().parse::<u64>().unwrap())
            .unwrap()
    }
    //<:!:section_4
}
