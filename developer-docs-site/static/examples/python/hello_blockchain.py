#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from typing import Optional
import sys

from first_transaction import Account, FaucetClient, RestClient, TESTNET_URL, FAUCET_URL

#:!:>section_1
class HelloBlockchainClient(RestClient):
    def publish_module(self, account_from: Account, module_hex: str) -> str:
        """Publish a new module to the blockchain within the specified account"""

        payload = {
            "type": "module_bundle_payload",
            "modules": [
                {"bytecode": f"0x{module_hex}"},
            ],
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn)
        return str(res["hash"])
#<:!:section_1

#:!:>section_2
    def get_message(self, contract_address: str, account_address: str) -> Optional[str]:
        """ Retrieve the resource Message::MessageHolder::message """
        return self.account_resource(account_address, f"0x{contract_address}::Message::MessageHolder")
#<:!:section_2

#:!:>section_3
    def set_message(self, contract_address: str, account_from: Account, message: str) -> str:
        """ Potentially initialize and set the resource Message::MessageHolder::message """

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
        res = self.submit_transaction(signed_txn)
        return str(res["hash"])
#<:!:section_3

if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting an argument that points to the helloblockchain module"

    client = HelloBlockchainClient(TESTNET_URL)
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

    input("\nUpdate the module with Alice's address, build, copy to the provided path, and press enter.")
    module_path = sys.argv[1]
    with open(module_path, "rb") as f:
        module_hex = f.read().hex()

    print("\n=== Testing Alice ===")
    print("Publishing...")
    tx_hash = client.publish_module(alice, module_hex)
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
