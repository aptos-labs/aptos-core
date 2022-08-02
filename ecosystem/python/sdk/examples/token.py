# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from aptos_sdk.account import Account
from aptos_sdk.client import FaucetClient, RestClient, TESTNET_URL, FAUCET_URL


if __name__ == "__main__":
    rest_client = RestClient(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    bob = Account.generate()

    collection_name = "Alice's"
    token_name = "Alice's first token"

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.address(), 20_000)
    faucet_client.fund_account(bob.address(), 20_000)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    print("\n=== Creating Collection and Token ===")

    txn_hash = rest_client.create_collection(
        alice, collection_name, "Alice's simple collection", "https://aptos.dev"
    )
    rest_client.wait_for_transaction(txn_hash)

    txn_hash = rest_client.create_token(
        alice,
        collection_name,
        token_name,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
        0,
    )
    rest_client.wait_for_transaction(txn_hash)

    print(
        f"Alice's collection: {rest_client.get_collection(alice.address(), collection_name)}"
    )
    print(
        f"Alice's token balance: {rest_client.get_token_balance(alice.address(), alice.address(), collection_name, token_name, 0)}"
    )
    print(
        f"Alice's token data: {rest_client.get_token_data(alice.address(), collection_name, token_name, 0)}"
    )

    print("\n=== Transferring the token to Bob ===")
    txn_hash = rest_client.offer_token(
        alice,
        bob.address(),
        alice.address(),
        collection_name,
        token_name,
        0,
        1,
    )
    rest_client.wait_for_transaction(txn_hash)

    txn_hash = rest_client.claim_token(
        bob,
        alice.address(),
        alice.address(),
        collection_name,
        token_name,
        0,
    )
    rest_client.wait_for_transaction(txn_hash)

    print(
        f"Alice's token balance: {rest_client.get_token_balance(alice.address(), alice.address(), collection_name, token_name, 0)}"
    )
    print(
        f"Bob's token balance: {rest_client.get_token_balance(bob.address(), alice.address(), collection_name, token_name, 0)}"
    )

    print("\n=== Transferring the token back to Alice using MultiAgent ===")
    txn_hash = rest_client.direct_transfer_token(
        bob, alice, alice.address(), collection_name, token_name, 0, 1
    )
    rest_client.wait_for_transaction(txn_hash)

    print(
        f"Alice's token balance: {rest_client.get_token_balance(alice.address(), alice.address(), collection_name, token_name, 0)}"
    )
    print(
        f"Bob's token balance: {rest_client.get_token_balance(bob.address(), alice.address(), collection_name, token_name, 0)}"
    )

    rest_client.close()
