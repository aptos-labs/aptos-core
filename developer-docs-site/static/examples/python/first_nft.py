#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import requests
from typing import Any, Dict
import sys

from first_transaction import Account, FaucetClient, RestClient, TESTNET_URL, FAUCET_URL

class TokenClient(RestClient):
    def submit_transaction_helper(self, account: Account, payload: Dict[str, Any]):
        txn_request = self.generate_transaction(account.address(), payload)
        signed_txn = self.sign_transaction(account, txn_request)
        res = self.submit_transaction(signed_txn)
        self.wait_for_transaction(res["hash"])

#:!:>section_1
    def create_collection(self, account: Account, description: str, name: str, uri: str):
        """Creates a new collection within the specified account"""

        payload = {
            "type": "script_function_payload",
            "function": f"0x1::Token::create_unlimited_collection_script",
            "type_arguments": [],
            "arguments": [
                description.encode("utf-8").hex(),
                name.encode("utf-8").hex(),
                uri.encode("utf-8").hex(),
            ]
        }
        self.submit_transaction_helper(account, payload)
#<:!:section_1

#:!:>section_2
    def create_token(
            self,
            account: Account,
            collection_name: str,
            description: str,
            name: str,
            supply: int,
            uri: str,
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::Token::create_token_script",
            "type_arguments": [],
            "arguments": [
                collection_name.encode("utf-8").hex(),    
                description.encode("utf-8").hex(),
                name.encode("utf-8").hex(),
                str(supply),
                uri.encode("utf-8").hex(),
            ]
        }
        self.submit_transaction_helper(account, payload)
#<:!:section_2

#:!:>section_4
    def offer_token(
            self,
            account: Account,
            receiver: str,
            creator: str,
            token_creation_num: int,
            amount: int
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::TokenTransfers::offer_script",
            "type_arguments": [],
            "arguments": [
                receiver,
                creator,
                str(token_creation_num),
                str(amount),
            ]
        }
        self.submit_transaction_helper(account, payload)
#<:!:section_4
            
#:!:>section_5
    def claim_token(
            self,
            account: Account,
            sender: str,
            creator: str,
            token_creation_num: int,
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::TokenTransfers::claim_script",
            "type_arguments": [],
            "arguments": [
                sender,
                creator,
                str(token_creation_num),
            ]
        }
        self.submit_transaction_helper(account, payload)
#<:!:section_5
            
    def cancel_token_offer(
            self,
            account: Account,
            receiver: str,
            creator: str,
            token_creation_num: int,
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::TokenTransfers::cancel_offer_script",
            "type_arguments": [],
            "arguments": [
                receiver,
                creator,
                str(token_creation_num),
            ]
        }
        self.submit_transaction_helper(account, payload)
            
#:!:>section_3
    def table_item(self, handle: str, key_type: str, value_type: str, key: Any) -> Any:
        response = requests.post(f"{self.url}/tables/{handle}/item", json={
            "key_type": key_type,
            "value_type": value_type,
            "key": key,
        })
        assert response.status_code == 200, response.text
        return response.json()

    def get_token_id(self, creator: str, collection_name: str, token_name: str) -> int:
        """ Retrieve the token's creation_num, which is useful for non-creator operations """

        collections = self.account_resource(creator, "0x1::Token::Collections")
        collection = self.table_item(
            collections["data"]["collections"]["handle"],
            "0x1::ASCII::String",
            "0x1::Token::Collection",
            collection_name,
        )
        token_data = self.table_item(
            collection["tokens"]["handle"],
            "0x1::ASCII::String",
            "0x1::Token::TokenData",
            token_name,
        )
        return token_data["id"]["creation_num"]

#<:!:section_3


if __name__ == "__main__":
    client = TokenClient(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, client)

    alice = Account()
    bob = Account()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.address(), 10_000_000)
    faucet_client.fund_account(bob.address(), 10_000_000)

    print("\n=== Initial Balances ===")
    print(f"Alice: {client.account_balance(alice.address())}")
    print(f"Bob: {client.account_balance(bob.address())}")

    client.create_collection(alice, "Alice's simple collection", "Alice's", "https://aptos.dev")
    client.create_token(alice, "Alice's", "Alice's simple token", "Alice's first token", 1, "https://aptos.dev/img/nyan.jpeg")

    print("\n=== Creating Collection and Token ===")
    token_id = client.get_token_id(alice.address(), "Alice's", "Alice's first token")
    print(f"Alice's token's identifier: {token_id}")
    print(f"See {client.url}/accounts/{alice.address()}/resources")

    print("\n=== Transferring the token to Bob ===")
    client.offer_token(alice, bob.address(), alice.address(), token_id, 1)
    client.claim_token(bob, alice.address(), alice.address(), token_id)

    print(f"See {client.url}/accounts/{bob.address()}/resources")
