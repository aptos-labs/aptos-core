# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.aptos_token_client import AptosTokenClient, Property, PropertyMap
from aptos_sdk.async_client import FaucetClient, RestClient

from .common import FAUCET_URL, NODE_URL


async def main():
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)
    token_client = AptosTokenClient(rest_client)
    alice = Account.generate()
    bob = Account.generate()

    collection_name = "Alice's"
    token_name = "Alice's first token"

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    bob_fund = faucet_client.fund_account(alice.address(), 100_000_000)
    alice_fund = faucet_client.fund_account(bob.address(), 100_000_000)
    await asyncio.gather(*[bob_fund, alice_fund])

    print("\n=== Initial Coin Balances ===")
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")

    print("\n=== Creating Collection and Token ===")

    txn_hash = await token_client.create_collection(
        alice,
        "Alice's simple collection",
        1,
        collection_name,
        "https://aptos.dev",
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        True,
        0,
        1,
    )
    await rest_client.wait_for_transaction(txn_hash)

    # This is a hack, once we add support for reading events or indexer, this will be easier
    resp = await rest_client.account_resource(alice.address(), "0x1::account::Account")
    int(resp["data"]["guid_creation_num"])

    txn_hash = await token_client.mint_token(
        alice,
        collection_name,
        "Alice's simple token",
        token_name,
        "https://aptos.dev/img/nyan.jpeg",
        PropertyMap([Property.string("string", "string value")]),
    )
    await rest_client.wait_for_transaction(txn_hash)

    minted_tokens = await token_client.tokens_minted_from_transaction(txn_hash)
    assert len(minted_tokens) == 1
    token_addr = minted_tokens[0]

    collection_addr = AccountAddress.for_named_collection(
        alice.address(), collection_name
    )
    collection_data = await token_client.read_object(collection_addr)
    print(f"Alice's collection: {collection_data}")
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token: {token_data}")

    txn_hash = await token_client.add_token_property(
        alice, token_addr, Property.bool("test", False)
    )
    await rest_client.wait_for_transaction(txn_hash)
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token: {token_data}")
    txn_hash = await token_client.remove_token_property(alice, token_addr, "string")
    await rest_client.wait_for_transaction(txn_hash)
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token: {token_data}")
    txn_hash = await token_client.update_token_property(
        alice, token_addr, Property.bool("test", True)
    )
    await rest_client.wait_for_transaction(txn_hash)
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token: {token_data}")
    txn_hash = await token_client.add_token_property(
        alice, token_addr, Property.bytes("bytes", b"\x00\x01")
    )
    await rest_client.wait_for_transaction(txn_hash)
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token: {token_data}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())
