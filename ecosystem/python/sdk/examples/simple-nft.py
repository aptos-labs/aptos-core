# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import json

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

    collection_name = "Alice's"
    token_name = "Alice's first token"
    property_version = 0

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    #:!:>section_3
    faucet_client.fund_account(alice.address(), 100_000_000)
    faucet_client.fund_account(bob.address(), 100_000_000)  # <:!:section_3

    print("\n=== Initial Coin Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    print("\n=== Creating Collection and Token ===")

    #:!:>section_4
    txn_hash = rest_client.create_collection(
        alice, collection_name, "Alice's simple collection", "https://aptos.dev"
    )  # <:!:section_4
    rest_client.wait_for_transaction(txn_hash)

    #:!:>section_5
    txn_hash = rest_client.create_token(
        alice,
        collection_name,
        token_name,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
        0,
    )  # <:!:section_5
    rest_client.wait_for_transaction(txn_hash)

    #:!:>section_6
    collection_data = rest_client.get_collection(alice.address(), collection_name)
    print(
        f"Alice's collection: {json.dumps(collection_data, indent=4, sort_keys=True)}"
    )  # <:!:section_6
    #:!:>section_7
    balance = rest_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Alice's token balance: {balance}")  # <:!:section_7
    #:!:>section_8
    token_data = rest_client.get_token_data(
        alice.address(), collection_name, token_name, property_version
    )
    print(
        f"Alice's token data: {json.dumps(token_data, indent=4, sort_keys=True)}"
    )  # <:!:section_8

    print("\n=== Transferring the token to Bob ===")
    #:!:>section_9
    txn_hash = rest_client.offer_token(
        alice,
        bob.address(),
        alice.address(),
        collection_name,
        token_name,
        property_version,
        1,
    )  # <:!:section_9
    rest_client.wait_for_transaction(txn_hash)

    #:!:>section_10
    txn_hash = rest_client.claim_token(
        bob,
        alice.address(),
        alice.address(),
        collection_name,
        token_name,
        property_version,
    )  # <:!:section_10
    rest_client.wait_for_transaction(txn_hash)

    balance = rest_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Alice's token balance: {balance}")
    balance = rest_client.get_token_balance(
        bob.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Bob's token balance: {balance}")

    print("\n=== Transferring the token back to Alice using MultiAgent ===")
    #:!:>section_11
    txn_hash = rest_client.direct_transfer_token(
        bob, alice, alice.address(), collection_name, token_name, 0, 1
    )  # <:!:section_11
    rest_client.wait_for_transaction(txn_hash)

    balance = rest_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Alice's token balance: {balance}")
    balance = rest_client.get_token_balance(
        bob.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Bob's token balance: {balance}")

    rest_client.close()
