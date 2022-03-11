#!/usr/bin/env python3

# Copyright (c) The Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from typing import Any, Dict, Optional, Sequence
import sys

from first_transaction import Account, FaucetClient, RestClient

TESTNET_URL = "http://127.0.0.1:8080"
FAUCET_URL = "http://127.0.0.1:8081"


class HelloBlockchainClient(RestClient):
    def get_message(self, contract_address, account_address) -> str:
        resources = self.account_resources(account_address)
        for resource in resources:
            if resource["type"] == f"0x{contract_address}::Message::MessageHolder":
                return resource["data"]["message"]
        return None

    def publish_module(self, account_from: Account, module: str) -> str:
        """Publish a new module to the blockchain within the specified account"""

        payload = {
            "type": "module_bundle_payload",
            "modules": [
                {"bytecode": f"0x{module}"},
            ],
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn).json()
        return str(res["hash"])

    def set_message(self, contract_address: str, account_from: Account, message: str) -> str:
        payload = {
            "type": "script_function_payload",
            "function": f"0x{contract_address}::Message::set_message",
            "type_arguments": [],
            "arguments": [
                message.encode("utf-8").hex(),
            ]
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn).json()
        return str(res["hash"])

if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting an argument that points to the helloblockchain module"

    client = HelloBlockchainClient(TESTNET_URL)
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

    input("\nUpdate the modules path to Alice's address, build, copy to the provided path, and press enter.")
    module_path = sys.argv[1]
    with open(module_path, "rb") as f:
        module = f.read().hex()

    print("\n=== Testing Alice ===")
    print("Publishing...")
    tx_hash = client.publish_module(alice, module)
    client.wait_for_transaction(tx_hash)
    print(f"Initial value: {client.get_message(alice.address(), alice.address())}")
    print("Setting the message to \"Hello, Blockchain\"")
    tx_hash = client.set_message(alice.address(), alice, "Hello, Blockchain")
    client.wait_for_transaction(tx_hash)
    print(f"New value: {client.get_message(alice.address(), alice.address())}")

    print("\n=== Testing Bob ===")
    print(f"Initial value: {client.get_message(alice.address(), bob.address())}")
    print("Setting the message to \"Hello, Blockchain\"")
    tx_hash = client.set_message(alice.address(), bob, "Hello, Blockchain")
    client.wait_for_transaction(tx_hash)
    print(f"New value: {client.get_message(alice.address(), bob.address())}")
