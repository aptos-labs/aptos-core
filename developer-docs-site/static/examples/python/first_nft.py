#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

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
            "function": f"0x1::SimpleToken::create_unlimited_simple_collection",
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
            "function": f"0x1::SimpleToken::create_simple_token",
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
    def transfer_token_to(
            self,
            account: Account,
            receiver: str,
            creator: str,
            token_creation_num: int,
            amount: int
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::SimpleToken::transfer_simple_token_to",
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
    def receive_token_from(
            self,
            account: Account,
            sender: str,
            creator: str,
            token_creation_num: int,
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::SimpleToken::receive_simple_token_from",
            "type_arguments": [],
            "arguments": [
                sender,
                creator,
                str(token_creation_num),
            ]
        }
        self.submit_transaction_helper(account, payload)
#<:!:section_5
            
    def stop_token_transfer_to(
            self,
            account: Account,
            receiver: str,
            creator: str,
            token_creation_num: int,
    ):
        payload = {
            "type": "script_function_payload",
            "function": f"0x1::SimpleToken::stop_simple_token_transfer_to",
            "type_arguments": [],
            "arguments": [
                receiver,
                creator,
                str(token_creation_num),
            ]
        }
        self.submit_transaction_helper(account, payload)
            
#:!:>section_3
    def get_token_id(self, creator: str, collection_name: str, token_name: str) -> int:
        """ Retrieve the token's creation_num, which is useful for non-creator operations """

        resources = self.account_resources(creator)
        collections = []
        tokens = []
        for resource in resources:
            if resource["type"] == f"0x1::Token::Collections<0x1::SimpleToken::NoMetadata>":
                collections = resource["data"]["collections"]["data"]
        for collection in collections:
            if collection["key"] == collection_name:
                tokens = collection["value"]["tokens"]["data"]
        for token in tokens:
            if token["key"] == token_name:
                return int(token["value"]["id"]["creation_num"])
            
        assert False
#<:!:section_3


if __name__ == "__main__":
    client = TokenClient(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, client)

    alice = Account()
    bob = Account()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.pub_key(), 10_000_000)
    faucet_client.fund_account(bob.pub_key(), 10_000_000)

    print("\n=== Initial Balances ===")
    print(f"Alice: {client.account_balance(alice.address())}")
    print(f"Bob: {client.account_balance(bob.address())}")

    client.create_collection(alice, "Alice's simple collection", "Alice's", "https://aptos.dev")
    client.create_token(alice, "Alice's", "Alice's simple token", "Alice's first token", 1, "https://aptos.dev")

    print("\n=== Creating Collection and Token ===")
    token_id = client.get_token_id(alice.address(), "Alice's", "Alice's first token")
    print(f"Alice's token's identifier: {token_id}")
    print(f"See {client.url}/accounts/{alice.address()}/resources")

    print("\n=== Transferring the token to Bob ===")
    client.transfer_token_to(alice, bob.address(), alice.address(), token_id, 1)
    client.receive_token_from(bob, alice.address(), alice.address(), token_id)

    print(f"See {client.url}/accounts/{bob.address()}/resources")
