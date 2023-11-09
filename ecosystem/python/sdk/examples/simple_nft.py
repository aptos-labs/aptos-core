# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import json

from aptos_sdk.account import Account
from aptos_sdk.aptos_tokenv1_client import AptosTokenV1Client
from aptos_sdk.async_client import FaucetClient, RestClient

from .common import FAUCET_URL, NODE_URL


async def main():
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)
    token_client = AptosTokenV1Client(rest_client)

    # :!:>section_2
    alice = Account.generate()
    bob = Account.generate()  # <:!:section_2

    collection_name = "Alice's"
    token_name = "Alice's first token"
    property_version = 0

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    # :!:>section_3
    bob_fund = faucet_client.fund_account(alice.address(), 100_000_000)
    alice_fund = faucet_client.fund_account(bob.address(), 100_000_000)  # <:!:section_3
    await asyncio.gather(*[bob_fund, alice_fund])

    print("\n=== Initial Coin Balances ===")
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")

    print("\n=== Creating Collection and Token ===")

    # :!:>section_4
    txn_hash = await token_client.create_collection(
        alice, collection_name, "Alice's simple collection", "https://aptos.dev"
    )  # <:!:section_4
    await rest_client.wait_for_transaction(txn_hash)

    # :!:>section_5
    txn_hash = await token_client.create_token(
        alice,
        collection_name,
        token_name,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
        0,
    )  # <:!:section_5
    await rest_client.wait_for_transaction(txn_hash)

    # :!:>section_6
    collection_data = await token_client.get_collection(
        alice.address(), collection_name
    )
    print(
        f"Alice's collection: {json.dumps(collection_data, indent=4, sort_keys=True)}"
    )  # <:!:section_6
    # :!:>section_7
    balance = await token_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    print(f"Alice's token balance: {balance}")  # <:!:section_7
    # :!:>section_8
    token_data = await token_client.get_token_data(
        alice.address(), collection_name, token_name, property_version
    )
    print(
        f"Alice's token data: {json.dumps(token_data, indent=4, sort_keys=True)}"
    )  # <:!:section_8

    print("\n=== Transferring the token to Bob ===")
    # :!:>section_9
    txn_hash = await token_client.offer_token(
        alice,
        bob.address(),
        alice.address(),
        collection_name,
        token_name,
        property_version,
        1,
    )  # <:!:section_9
    await rest_client.wait_for_transaction(txn_hash)

    # :!:>section_10
    txn_hash = await token_client.claim_token(
        bob,
        alice.address(),
        alice.address(),
        collection_name,
        token_name,
        property_version,
    )  # <:!:section_10
    await rest_client.wait_for_transaction(txn_hash)

    alice_balance = token_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    bob_balance = token_client.get_token_balance(
        bob.address(), alice.address(), collection_name, token_name, property_version
    )
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice's token balance: {alice_balance}")
    print(f"Bob's token balance: {bob_balance}")

    print("\n=== Transferring the token back to Alice using MultiAgent ===")
    txn_hash = await token_client.direct_transfer_token(
        bob, alice, alice.address(), collection_name, token_name, 0, 1
    )
    await rest_client.wait_for_transaction(txn_hash)

    alice_balance = token_client.get_token_balance(
        alice.address(), alice.address(), collection_name, token_name, property_version
    )
    bob_balance = token_client.get_token_balance(
        bob.address(), alice.address(), collection_name, token_name, property_version
    )
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice's token balance: {alice_balance}")
    print(f"Bob's token balance: {bob_balance}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())
