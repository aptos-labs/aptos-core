# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

"""
This example depends on the HelloBlockchain.move module having already been published to the destination blockchain.

One method to do so is to use the CLI:
    * Acquire the Aptos CLI
    * `cd ~`
    * `aptos init`
    * `cd ~/aptos-core/aptos-move/
    * `aptos move publish --named-address HelloBlockchain=${your_address_from_aptos_init}`
    * `python -m examples.hello-blockhain ${your_address_from_aptos_init}`
"""

import sys
from typing import Optional

from .common import NODE_URL, FAUCET_URL
from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.transactions import ScriptFunction, TransactionArgument, TransactionPayload

def get_message(rest_client: RestClient, contract_address: str, account_address: AccountAddress) -> Optional[str]:
    """ Retrieve the resource message::MessageHolder::message """
    return rest_client.account_resource(account_address, f"0x{contract_address}::message::MessageHolder")

def set_message(rest_client: RestClient, contract_address: str, sender: Account, message: str) -> str:
    """ Potentially initialize and set the resource message::MessageHolder::message """

    payload = ScriptFunction.natural(
        f"0x{contract_address}::message",
        "set_message",
        [],
        [TransactionArgument(message, Serializer.str)],
    )
    signed_transaction = rest_client.create_single_signer_bcs_transaction(sender, TransactionPayload(payload))
    txn_hash = rest_client.submit_bcs_transaction(signed_transaction)
    rest_client.wait_for_transaction(txn_hash)

if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting the contract address"
    contract_address = sys.argv[1]

    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    faucet_client.fund_account(alice.address(), 20_000)
    faucet_client.fund_account(bob.address(), 20_000)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    print("\n=== Testing Alice ===")
    print(f"Initial value: {get_message(rest_client, contract_address, alice.address())}")
    print("Setting the message to \"Hello, Blockchain\"")
    set_message(rest_client, contract_address, alice, "Hello, Blockchain")
    print(f"New value: {get_message(rest_client, contract_address, alice.address())}")

    print("\n=== Testing Bob ===")
    print(f"Initial value: {get_message(rest_client, contract_address, bob.address())}")
    print("Setting the message to \"Hello, Blockchain\"")
    set_message(rest_client, contract_address, bob, "Hello, Blockchain")
    print(f"New value: {get_message(rest_client, contract_address, bob.address())}")
