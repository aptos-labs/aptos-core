# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

"""
This example depends on the hello_blockchain.move module having already been published to the destination blockchain.

One method to do so is to use the CLI:
    * Acquire the Aptos CLI
    * `cd ~`
    * `aptos init`
    * `cd ~/aptos-core/aptos-move/
    * `aptos move publish --named-address hello_blockchain=${your_address_from_aptos_init}`
    * `python -m examples.hello-blockhain ${your_address_from_aptos_init}`
"""

import sys
from typing import Optional

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.transactions import (
    EntryFunction,
    TransactionArgument,
    TransactionPayload,
)

from .common import FAUCET_URL, NODE_URL


class HelloBlockchainClient(RestClient):
    def get_message(
        self, contract_address: str, account_address: AccountAddress
    ) -> Optional[str]:
        """Retrieve the resource message::MessageHolder::message"""
        return self.account_resource(
            account_address, f"0x{contract_address}::message::MessageHolder"
        )

    def set_message(self, contract_address: str, sender: Account, message: str) -> str:
        """Potentially initialize and set the resource message::MessageHolder::message"""

        payload = EntryFunction.natural(
            f"0x{contract_address}::message",
            "set_message",
            [],
            [TransactionArgument(message, Serializer.str)],
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)


if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting the contract address"
    contract_address = sys.argv[1]

    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    rest_client = HelloBlockchainClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    faucet_client.fund_account(alice.address(), 20_000)
    faucet_client.fund_account(bob.address(), 20_000)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    print("\n=== Testing Alice ===")
    print(
        f"Initial value: {rest_client.get_message(contract_address, alice.address())}"
    )
    print('Setting the message to "Hello, Blockchain"')
    txn_hash = rest_client.set_message(contract_address, alice, "Hello, Blockchain")
    rest_client.wait_for_transaction(txn_hash)

    print(f"New value: {rest_client.get_message(contract_address, alice.address())}")

    print("\n=== Testing Bob ===")
    print(f"Initial value: {rest_client.get_message(contract_address, bob.address())}")
    print('Setting the message to "Hello, Blockchain"')
    txn_hash = rest_client.set_message(contract_address, bob, "Hello, Blockchain")
    rest_client.wait_for_transaction(txn_hash)

    print(f"New value: {rest_client.get_message(contract_address, bob.address())}")
