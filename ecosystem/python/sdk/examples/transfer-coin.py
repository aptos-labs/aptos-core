# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from aptos_sdk.account import Account
from aptos_sdk.client import FaucetClient, RestClient

from .common import FAUCET_URL, NODE_URL

if __name__ == "__main__":
    #:!:>section_1
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)  # <:!:section_1

    #:!:>section_2
    alice = Account.generate()
    bob = Account.generate()  # <:!:section_2

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    #:!:>section_3
    faucet_client.fund_account(alice.address(), 100_000_000)
    faucet_client.fund_account(bob.address(), 0)  # <:!:section_3

    print("\n=== Initial Balances ===")
    #:!:>section_4
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")  # <:!:section_4

    # Have Alice give Bob 1_000 coins
    #:!:>section_5
    txn_hash = rest_client.transfer(alice, bob.address(), 1_000)  # <:!:section_5
    #:!:>section_6
    rest_client.wait_for_transaction(txn_hash)  # <:!:section_6

    print("\n=== Intermediate Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    # Have Alice give Bob another 1_000 coins using BCS
    txn_hash = rest_client.bcs_transfer(alice, bob.address(), 1_000)
    rest_client.wait_for_transaction(txn_hash)

    print("\n=== Final Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    rest_client.close()
