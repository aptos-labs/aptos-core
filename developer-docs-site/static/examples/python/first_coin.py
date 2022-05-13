#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from hello_blockchain import HelloBlockchainClient
from typing import Optional
import sys

from first_transaction import Account, FaucetClient, RestClient, TESTNET_URL, FAUCET_URL

class FirstCoin(RestClient):

#:!:>section_1
    def initialize_coin(self, account_from: Account) -> Optional[str]:
        """ Initialize a new coin with the given coin type. """
        payload = {
            "type": "script_function_payload",
            "function": "0x1::ManagedCoin::initialize",
            "type_arguments": [f"0x{account_from.address()}::MoonCoin::MoonCoin"],
            "arguments": [
                "Moon Coin".encode("utf-8").hex(),
                "MOON".encode("utf-8").hex(),
                "6",
                False
            ]
        }
        res = self.execute_transaction_with_payload(account_from, payload)
        return str(res["hash"])
#<:!:section_1

#:!:>section_2
    def register_coin(self, account_receiver: Account, coin_type_address: str) -> str:
        """ Register the receiver account to receive transfers for the new coin. """

        payload = {
            "type": "script_function_payload",
            "function": "0x1::Coin::register",
            "type_arguments": [f"0x{coin_type_address}::MoonCoin::MoonCoin"],
            "arguments": []
        }
        res = self.execute_transaction_with_payload(account_receiver, payload)
        return str(res["hash"])
#<:!:section_2

#:!:>section_3
    def mint_coin(
        self,
        account_coin_owner: Account,
        receiver_address: str,
        amount: int
    ) -> str:
        """ Register the receiver account to receive transfers for the new coin. """

        payload = {
            "type": "script_function_payload",
            "function": "0x1::ManagedCoin::mint",
            "type_arguments": [f"0x{account_coin_owner.address()}::MoonCoin::MoonCoin"],
            "arguments": [
                receiver_address,
                f"{amount}"
            ]
        }
        res = self.execute_transaction_with_payload(account_coin_owner, payload)
        return str(res["hash"])
#<:!:section_3

#:!:>section_4
    def get_balance(
        self,
        account_address: str,
        coin_type_address: str,
    ) -> str:
        """ Returns the coin balance of the given account """

        return self.account_resource(account_address, f"0x1::Coin::CoinStore<0x{coin_type_address}::MoonCoin::MoonCoin>")
#<:!:section_4

if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting an argument that points to the helloblockchain module"

    client = FirstCoin(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, client)

    alice = Account()
    bob = Account()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.address(), 10_000_000)
    faucet_client.fund_account(bob.address(), 10_000_000)

    input("\nUpdate the module with Alice's address, build, copy to the provided path, and press enter.")
    module_path = sys.argv[1]
    with open(module_path, "rb") as f:
        module_hex = f.read().hex()

    print("Publishing MoonCoinType module...")
    hello_blockchain_client = HelloBlockchainClient(TESTNET_URL)
    tx_hash = hello_blockchain_client.publish_module(alice, module_hex)
    hello_blockchain_client.wait_for_transaction(tx_hash)

    print("Alice will initialize the new coin")
    tx_hash = client.initialize_coin(alice)
    client.wait_for_transaction(tx_hash)
    
    print("Bob registers the newly created coin so he can receive it from Alice")
    tx_hash = client.register_coin(bob, alice.address())
    client.wait_for_transaction(tx_hash)
    print(f"Bob's initial MoonCoinType balance: {client.get_balance(bob.address(), alice.address())}")
    
    print("Alice mints Bob some of the new coin")
    tx_hash = client.mint_coin(alice, bob.address(), 100)
    client.wait_for_transaction(tx_hash)
    print(f"Bob's updated MoonCoinType balance: {client.get_balance(bob.address(), alice.address())}")
